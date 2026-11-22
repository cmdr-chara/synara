//! `orchestration.getSnapshot` — the OrchestrationReadModel rebuilt natively
//! from a byte copy of `state.sqlite` (the upstream process holds an exclusive
//! lock). Column names in the projection tables mirror the wire fields in
//! snake_case; `*_json` columns embed JSON sub-objects, INTEGER 0/1 columns
//! are booleans on the wire.

use std::path::PathBuf;

use rusqlite::Connection;
use serde_json::{Map, Value, json};

use crate::todo::{WsErr, state_dir};

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("ORCHESTRATION"))
}

/// Same byte-copy trick as `stats`: upstream keeps `state.sqlite` under
/// `PRAGMA locking_mode = EXCLUSIVE`, so we copy db + WAL to a temp file and
/// read that.
pub(crate) struct StateCopy {
    pub(crate) conn: Connection,
    db: PathBuf,
    wal: PathBuf,
}

impl Drop for StateCopy {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.db);
        let _ = std::fs::remove_file(&self.wal);
    }
}

pub(crate) fn open_state_copy() -> Option<StateCopy> {
    let db = PathBuf::from(state_dir()).join("state.sqlite");
    let tmp = std::env::temp_dir().join(format!("synara-orch-snap-{}.sqlite", std::process::id()));
    let tmp_wal = PathBuf::from(format!("{}-wal", tmp.display()));
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp_wal);
    std::fs::copy(&db, &tmp).ok()?;
    let wal = db.with_file_name("state.sqlite-wal");
    if wal.exists() {
        let _ = std::fs::copy(&wal, &tmp_wal);
    }
    let conn = Connection::open(&tmp).ok()?;
    Some(StateCopy {
        conn,
        db: tmp,
        wal: tmp_wal,
    })
}

fn snake_to_camel(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper = false;
    for c in s.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn row_value(v: rusqlite::types::ValueRef<'_>, parse_json: bool, to_bool: bool) -> Value {
    match v {
        rusqlite::types::ValueRef::Null => Value::Null,
        rusqlite::types::ValueRef::Integer(n) => {
            if to_bool {
                Value::Bool(n != 0)
            } else {
                json!(n)
            }
        }
        rusqlite::types::ValueRef::Real(f) => json!(f),
        rusqlite::types::ValueRef::Text(t) => {
            let s = String::from_utf8_lossy(t).into_owned();
            if parse_json {
                serde_json::from_str(&s).unwrap_or(Value::Null)
            } else {
                Value::String(s)
            }
        }
        rusqlite::types::ValueRef::Blob(b) => {
            json!(b.iter().map(|x| *x).collect::<Vec<u8>>())
        }
    }
}

/// Read a table into wire objects. `renames` overrides the default
/// snake_case->camelCase key, `json_cols` are `*_json` columns whose contents
/// are parsed (key emitted without the suffix), `bool_cols` are 0/1 integers
/// emitted as booleans.
pub(crate) fn query_objects(
    conn: &Connection,
    sql: &str,
    renames: &[(&str, &str)],
    json_cols: &[&str],
    bool_cols: &[&str],
) -> Vec<Value> {
    let Ok(mut stmt) = conn.prepare(sql) else {
        return Vec::new();
    };
    let names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let Ok(mut rows) = stmt.query([]) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    while let Ok(Some(row)) = rows.next() {
        let mut obj = Map::new();
        for (i, name) in names.iter().enumerate() {
            let col = name.as_str();
            if col == "row_id" {
                continue;
            }
            let key: String = renames
                .iter()
                .find(|(c, _)| *c == col)
                .map(|(_, k)| k.to_string())
                .unwrap_or_else(|| {
                    if let Some(base) = col.strip_suffix("_json") {
                        snake_to_camel(base)
                    } else {
                        snake_to_camel(col)
                    }
                });
            if key.is_empty() {
                continue;
            }
            let Ok(v) = row.get_ref(i) else { continue };
            let val = row_value(v, col.ends_with("_json") || json_cols.contains(&col), bool_cols.contains(&col));
            obj.insert(key, val);
        }
        out.push(Value::Object(obj));
    }
    out
}

fn get<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}

fn is_present(v: &Value) -> bool {
    !v.is_null()
}

fn load_spaces(conn: &Connection) -> Vec<Value> {
    query_objects(
        conn,
        "SELECT space_id AS id, name, icon, sort_order, created_at, updated_at, deleted_at \
         FROM projection_spaces ORDER BY sort_order, space_id",
        &[],
        &[],
        &[],
    )
}

fn load_projects(conn: &Connection) -> Vec<Value> {
    query_objects(
        conn,
        "SELECT project_id AS id, kind, title, workspace_root, scripts_json, \
         default_model_selection_json, is_pinned, space_id, additional_folders_json, \
         created_at, updated_at, deleted_at \
         FROM projection_projects ORDER BY created_at, project_id",
        &[],
        &[],
        &["is_pinned"],
    )
}

fn load_turn_objects(conn: &Connection, thread_id: &str) -> Vec<Value> {
    let sql = "SELECT turn_id, pending_message_id, assistant_message_id, state, requested_at, \
         started_at, completed_at, checkpoint_turn_count, checkpoint_ref, checkpoint_status, \
         checkpoint_files_json, source_proposed_plan_thread_id, source_proposed_plan_id, \
         started_without_git_workspace \
         FROM projection_turns WHERE thread_id = ?1 ORDER BY requested_at, row_id";
    let Ok(mut stmt) = conn.prepare(sql) else {
        return Vec::new();
    };
    let names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    let Ok(mut rows) = stmt.query([thread_id]) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    while let Ok(Some(row)) = rows.next() {
        let mut obj = Map::new();
        let mut source_plan_thread = Value::Null;
        let mut source_plan_id = Value::Null;
        for (i, col) in names.iter().enumerate() {
            let Ok(v) = row.get_ref(i) else { continue };
            match col.as_str() {
                "source_proposed_plan_thread_id" => source_plan_thread = row_value(v, false, false),
                "source_proposed_plan_id" => source_plan_id = row_value(v, false, false),
                "checkpoint_turn_count" | "started_without_git_workspace" => {
                    let key = if col.as_str() == "started_without_git_workspace" {
                        "startedWithoutGitWorkspace"
                    } else {
                        "checkpointTurnCount"
                    };
                    obj.insert(
                        key.into(),
                        row_value(v, false, col.as_str() == "started_without_git_workspace"),
                    );
                }
                "checkpoint_files_json" => {
                    obj.insert("checkpointFiles".into(), row_value(v, true, false));
                }
                "checkpoint_ref" => {
                    obj.insert("checkpointRef".into(), row_value(v, false, false));
                }
                "checkpoint_status" => {
                    obj.insert("checkpointStatus".into(), row_value(v, false, false));
                }
                "pending_message_id" => {
                    obj.insert("pendingMessageId".into(), row_value(v, false, false));
                }
                "assistant_message_id" => {
                    obj.insert("assistantMessageId".into(), row_value(v, false, false));
                }
                c => {
                    obj.insert(snake_to_camel(c), row_value(v, false, false));
                }
            }
        }
        if is_present(&source_plan_thread) || is_present(&source_plan_id) {
            obj.insert(
                "sourceProposedPlan".into(),
                json!({"threadId": source_plan_thread, "planId": source_plan_id}),
            );
        }
        out.push(Value::Object(obj));
    }
    out
}

fn load_checkpoints(turns: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for t in turns {
        if get(t, "checkpointTurnCount").is_null() && get(t, "checkpointStatus").is_null() {
            continue;
        }
        out.push(json!({
            "turnId": get(t, "turnId"),
            "files": get(t, "checkpointFiles"),
            "startedAt": get(t, "startedAt"),
            "completedAt": get(t, "completedAt"),
            "status": get(t, "checkpointStatus"),
            "assistantMessageId": get(t, "assistantMessageId"),
            "checkpointTurnCount": get(t, "checkpointTurnCount"),
        }));
    }
    out
}

fn load_threads(conn: &Connection) -> Vec<Value> {
    let cols = "thread_id AS id, project_id, title, branch, worktree_path, latest_turn_id, \
        runtime_mode, interaction_mode, model_selection_json, handoff_json, env_mode, \
        fork_source_thread_id, fork_source_message_id, associated_worktree_path, \
        associated_worktree_branch, associated_worktree_ref, archived_at, parent_thread_id, \
        subagent_agent_id, subagent_nickname, subagent_role, latest_user_message_at, \
        latest_human_message_at, has_actionable_proposed_plan, last_known_pr_json, \
        create_branch_flow_completed, sidechat_source_thread_id, sidechat_last_activity_at, \
        sidechat_expired_at, sidechat_context_json, is_pinned, pinned_messages_json, notes, \
        creation_source, source_thread_id, source_turn_id, gateway_operation_id, \
        gateway_operation_index, working_directory, settled_at, goal, goal_started_at, \
        goal_paused_at, goal_achievements_json, claude_cache_review_json, snoozed_until, \
        snooze_reminder_at, usage_limit_recovery_source_json, usage_limit_recovery_json, \
        usage_limit_recovery_cancelled_at, usage_limit_recovery_cancelled_turn_id, \
        usage_limit_recovery_last_reset_at, pull_requests_json, pending_approval_count, \
        pending_user_input_count, created_at, updated_at, deleted_at";
    let sql = format!("SELECT {cols} FROM projection_threads ORDER BY created_at, thread_id");
    let mut threads = query_objects(
        conn,
        &sql,
        &[("pending_approval_count", ""), ("pending_user_input_count", "")],
        &[],
        &[
            "is_pinned",
            "has_actionable_proposed_plan",
            "create_branch_flow_completed",
        ],
    );

    for thread in threads.iter_mut() {
        let tid = get(thread, "id").as_str().unwrap_or_default().to_string();
        let latest_turn_id = get(thread, "latestTurnId").as_str().map(|s| s.to_string());
        let Some(tobj) = thread.as_object_mut() else { continue };
        let approvals = tobj
            .get("pendingApprovalCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let user_inputs = tobj
            .get("pendingUserInputCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        tobj.insert("hasPendingApprovals".into(), Value::Bool(approvals > 0));
        tobj.insert("hasPendingUserInput".into(), Value::Bool(user_inputs > 0));

        let session = {
            let sql = "SELECT thread_id, status, provider_name, provider_session_id, \
                 provider_thread_id, active_turn_id, last_error, runtime_mode, \
                 last_activity_at, last_progress_at, provider_instance_id, updated_at \
                 FROM projection_thread_sessions WHERE thread_id = ?1";
            let mut out = Value::Null;
            if let Ok(mut stmt) = conn.prepare(sql) {
                let names: Vec<String> =
                    stmt.column_names().iter().map(|s| s.to_string()).collect();
                if let Ok(mut rows) = stmt.query([tid.as_str()]) {
                    if let Ok(Some(row)) = rows.next() {
                        let mut obj = Map::new();
                        for (i, c) in names.iter().enumerate() {
                            if let Ok(v) = row.get_ref(i) {
                                obj.insert(snake_to_camel(c), row_value(v, false, false));
                            }
                        }
                        out = Value::Object(obj);
                    }
                }
            }
            out
        };
        tobj.insert("session".into(), session);

        let turns = load_turn_objects(conn, &tid);
        tobj.insert("checkpoints".into(), json!(load_checkpoints(&turns)));
        let latest = latest_turn_id
            .as_deref()
            .and_then(|lt| turns.iter().find(|t| get(t, "turnId").as_str() == Some(lt)))
            .cloned()
            .or_else(|| turns.last().cloned())
            .unwrap_or(Value::Null);
        tobj.insert("latestTurn".into(), latest);

        tobj.insert(
            "activities".into(),
            json!(query_objects(
                conn,
                &format!(
                    "SELECT activity_id AS id, turn_id, tone, kind, summary, payload_json, \
                     created_at, sequence FROM projection_thread_activities \
                     WHERE thread_id = '{}' ORDER BY created_at, activity_id",
                    tid.replace('\'', "''")
                ),
                &[],
                &[],
                &[],
            )),
        );
        tobj.insert(
            "messages".into(),
            json!(query_objects(
                conn,
                &format!(
                    "SELECT message_id AS id, turn_id, role, text, is_streaming, created_at, \
                     updated_at, attachments_json, source, skills_json, mentions_json, \
                     dispatch_mode, dispatch_origin, sequence, text_event_sequence, text_json, \
                     starts_new_turn, async_user_input_json \
                     FROM projection_thread_messages \
                     WHERE thread_id = '{}' ORDER BY created_at, sequence, message_id",
                    tid.replace('\'', "''")
                ),
                &[("is_streaming", "streaming"), ("text_json", "textSegments")],
                &[],
                &["is_streaming"],
            )),
        );
        tobj.insert(
            "pendingInteractions".into(),
            json!(query_objects(
                conn,
                &format!(
                    "SELECT interaction_kind, request_id, turn_id, lifecycle_generation, status, \
                     decision, response_command_id, response_requested_at, created_at, \
                     resolved_at FROM projection_pending_interactions \
                     WHERE thread_id = '{}' ORDER BY created_at, request_id",
                    tid.replace('\'', "''")
                ),
                &[],
                &[],
                &[],
            )),
        );
        tobj.insert(
            "proposedPlans".into(),
            json!(query_objects(
                conn,
                &format!(
                    "SELECT plan_id AS id, turn_id, plan_markdown, implemented_at, \
                     implementation_thread_id, created_at, updated_at \
                     FROM projection_thread_proposed_plans \
                     WHERE thread_id = '{}' ORDER BY created_at, plan_id",
                    tid.replace('\'', "''")
                ),
                &[],
                &[],
                &[],
            )),
        );
        tobj.insert(
            "activeTools".into(),
            json!(query_objects(
                conn,
                &format!(
                    "SELECT item_id, turn_id, started_at, started_event_id \
                     FROM projection_thread_active_tools WHERE thread_id = '{}'",
                    tid.replace('\'', "''")
                ),
                &[("item_id", "itemId"), ("started_event_id", "startedEventId")],
                &[],
                &[],
            )),
        );
    }
    threads
}

fn snapshot_sequence(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT COALESCE(MAX(last_applied_sequence), 0) FROM projection_state",
        [],
        |r| r.get(0),
    )
    .unwrap_or(0)
}

pub(crate) fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64 / 1000.0)
        .unwrap_or(0.0);
    // Millisecond-precision ISO without chrono.
    let ms = (secs * 1000.0) as i64;
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let (h, m, s, frac) = (
        rem / 3_600_000,
        (rem % 3_600_000) / 60_000,
        (rem % 60_000) / 1000,
        rem % 1000,
    );
    // Civil-from-days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let yy = if mo <= 2 { y + 1 } else { y };
    format!("{yy:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}.{frac:03}Z")
}

pub fn get_snapshot() -> Result<Value, WsErr> {
    let Some(copy) = open_state_copy() else {
        return Err(err("state.sqlite is unavailable"));
    };
    let conn = &copy.conn;
    let spaces = load_spaces(conn);
    let projects = load_projects(conn);
    let threads = load_threads(conn);
    let sequence = snapshot_sequence(conn);
    Ok(json!({
        "snapshotSequence": sequence,
        "spaces": spaces,
        "projects": projects,
        "threads": threads,
        "updatedAt": now_iso(),
    }))
}
