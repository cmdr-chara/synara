// Orchestration engine: native orchestration.dispatchCommand.
//
// Intercepted only when SYNARA_NO_UPSTREAM is set: in that mode the front door
// owns state.sqlite directly (the original server is not started, so there is
// no EXCLUSIVE lock), and commands are decided, appended to
// orchestration_events, receipted and projected in one transaction.
// Without the flag every command keeps relaying to the original server.

use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

use crate::todo::{iso_from_ms, iso_now_ms, state_dir, WsErr};

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut bytes);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn handles(tag: &str) -> bool {
    tag == "orchestration.dispatchCommand" && std::env::var("SYNARA_NO_UPSTREAM").is_ok()
}

fn open_state_rw() -> Result<Connection, WsErr> {
    let db = PathBuf::from(state_dir()).join("state.sqlite");
    Connection::open(&db).map_err(|e| err(format!("could not open state.sqlite: {e}")))
}

fn s(command: &Map<String, Value>, key: &str) -> Option<String> {
    command.get(key).and_then(Value::as_str).map(str::to_string)
}

fn opt(command: &Map<String, Value>, key: &str) -> Value {
    command.get(key).cloned().unwrap_or(Value::Null)
}

fn is_group_container_kind(kind: &str) -> bool {
    kind == "group" || kind == "studio"
}

pub fn handle(_tag: &str, payload: &Value) -> Result<Value, WsErr> {
    let command = payload
        .as_object()
        .ok_or_else(|| err("orchestration.dispatchCommand expects a command object"))?;
    let command_type = command
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| err("orchestration.dispatchCommand payload is missing type"))?;
    match command_type {
        "thread.create" => thread_create(command),
        "thread.archive" => thread_archive(command, true),
        "thread.unarchive" => thread_archive(command, false),
        "thread.delete" => thread_delete(command),
        "thread.runtime-mode.set" => thread_mode_set(command, "runtimeMode", "runtime_mode"),
        "thread.interaction-mode.set" => {
            thread_mode_set(command, "interactionMode", "interaction_mode")
        }
        "thread.meta.update" => thread_meta_update(command),
        "thread.turn.start" => thread_turn_start(command),
        "thread.turn.interrupt" => thread_simple(command, "thread.turn-interrupt-requested"),
        "thread.task.stop" => thread_simple(command, "thread.task-stop-requested"),
        "thread.task.background" => thread_simple(command, "thread.task-background-requested"),
        "thread.approval.respond" => {
            thread_simple(command, "thread.approval-response-requested")
        }
        "thread.user-input.respond" => {
            thread_simple(command, "thread.user-input-response-requested")
        }
        "thread.pinned-message.add" => pinned_message(command, "add"),
        "thread.pinned-message.remove" => pinned_message(command, "remove"),
        "thread.pinned-message.done.set" => pinned_message(command, "done"),
        "thread.pinned-message.label.set" => pinned_message(command, "label"),
        "project.create" => project_create(command),
        "project.meta.update" => project_meta_update(command),
        "project.delete" => project_delete(command),
        "thread.checkpoint.revert" => checkpoint_revert(command),
        "thread.conversation.rollback" => conversation_rollback(command),
        "space.create" => space_create(command),
        "space.meta.update" => space_meta_update(command),
        "space.reorder" => space_reorder(command),
        "space.delete" => space_delete(command),
        "space.projects.assign" => space_projects_assign(command),
        "thread.message.edit-and-resend" => message_edit_resend(command),
        "thread.session.stop" => session_stop(command),
        "thread.session.set" => session_set(command),
        "thread.goal.continue" => goal_continue(command),
        "thread.turn.resume" => turn_resume(command),
        "thread.pull-request.link" | "thread.pull-request.unlink" | "thread.pull-request.sync" => {
            pull_request_command(command)
        }
        "thread.claude-cache.set" => claude_cache_set(command),
        "thread.claude-cache.compacted" => claude_cache_followup(command, true),
        "thread.claude-cache.respond" => claude_cache_followup(command, false),
        "thread.turn.dispatch-queued" => dispatch_queued(command),
        "thread.proposed-plan.upsert" => proposed_plan_upsert(command),
        "thread.turn.diff.complete" => turn_diff_complete(command),
        "thread.revert.complete" => revert_complete(command),
        "thread.conversation.rollback.complete" => conversation_rollback_complete(command),
        "thread.sidechat.activity.record" => sidechat_activity(command),
        "thread.sidechat.expire" => sidechat_expire(command),
        t if t.starts_with("thread.usage-limit.") => usage_limit_dispatch(command),
        "thread.messages.import" => messages_import(command),
        "thread.message.assistant.delta" => assistant_message_event(command, true),
        "thread.message.assistant.complete" => assistant_message_event(command, false),
        "thread.message.user.bind-turn" => user_bind_or_boundary(command, true),
        "thread.message.user.set-turn-boundary" => user_bind_or_boundary(command, false),
        other => Err(err(format!(
            "orchestration.dispatchCommand {other} is not ported yet (native engine slice)"
        ))),
    }
}

/// One event a decider emits; sequence and stream_version are assigned at commit.
struct OutEvent {
    event_id: String,
    event_type: &'static str,
    payload: Value,
    thread_id: String,
    agg_kind: &'static str,
    occurred_at: String,
    causation_event_id: Option<String>,
    metadata: Option<Value>,
}

fn out_event(
    event_type: &'static str,
    thread_id: &str,
    occurred_at: &str,
    payload: Value,
) -> OutEvent {
    OutEvent {
        event_id: uuid_v4(),
        event_type,
        payload,
        thread_id: thread_id.to_string(),
        agg_kind: "thread",
        occurred_at: occurred_at.to_string(),
        causation_event_id: None,
        metadata: None,
    }
}

struct ThreadRow {
    project_id: String,
    archived_at: Option<String>,
    deleted_at: Option<String>,
    interaction_mode: String,
    claude_cache_review: Option<String>,
    parent_thread_id: Option<String>,
    snoozed_until: Option<String>,
    snooze_reminder_at: Option<String>,
    latest_turn_id: Option<String>,
    latest_user_message_at: Option<String>,
    latest_human_message_at: Option<String>,
    updated_at: String,
}

fn require_thread(conn: &Connection, thread_id: &str) -> Result<ThreadRow, WsErr> {
    conn.query_row(
        "SELECT project_id, archived_at, deleted_at, interaction_mode, claude_cache_review_json, \
         parent_thread_id, snoozed_until, snooze_reminder_at, latest_turn_id, \
         latest_user_message_at, latest_human_message_at, updated_at \
         FROM projection_threads WHERE thread_id = ?1",
        params![thread_id],
        |r| {
            Ok(ThreadRow {
                project_id: r.get(0)?,
                archived_at: r.get(1)?,
                deleted_at: r.get(2)?,
                interaction_mode: r.get(3)?,
                claude_cache_review: r.get(4)?,
                parent_thread_id: r.get(5)?,
                snoozed_until: r.get(6)?,
                snooze_reminder_at: r.get(7)?,
                latest_turn_id: r.get(8)?,
                latest_user_message_at: r.get(9)?,
                latest_human_message_at: r.get(10)?,
                updated_at: r.get(11)?,
            })
        },
    )
    .map_err(|_| err(format!("thread {thread_id} not found")))
}

/// Appends every event (incrementing stream_version per stream), writes the
/// command receipt against the final event's sequence, returns that sequence.
fn commit_events(conn: &Connection, events: &[OutEvent], command_id: &str) -> Result<i64, WsErr> {
    let tx = conn
        .unchecked_transaction()
        .map_err(|e| err(e.to_string()))?;
    let mut sequence = 0i64;
    for ev in events {
        let stream_version: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(stream_version), 0) + 1 \
                 FROM orchestration_events WHERE aggregate_kind = ?2 AND stream_id = ?1",
                params![ev.thread_id, ev.agg_kind],
                |r| r.get(0),
            )
            .map_err(|e| err(e.to_string()))?;
        tx.execute(
            "INSERT INTO orchestration_events \
             (event_id, aggregate_kind, stream_id, stream_version, event_type, occurred_at, \
              command_id, causation_event_id, correlation_id, actor_kind, payload_json, metadata_json) \
             VALUES (?1, ?10, ?2, ?3, ?4, ?5, ?6, ?7, ?6, 'client', ?8, ?9)",
            params![
                ev.event_id,
                ev.thread_id,
                stream_version,
                ev.event_type,
                ev.occurred_at,
                command_id,
                ev.causation_event_id,
                ev.payload.to_string(),
                ev.metadata.as_ref().map_or("{}".to_string(), |m| m.to_string()),
                ev.agg_kind,
            ],
        )
        .map_err(|e| err(format!("append {} event: {e}", ev.event_type)))?;
        sequence = tx.last_insert_rowid();
    }
    let last = events
        .last()
        .ok_or_else(|| err("dispatch produced no events"))?;
    tx.execute(
        "INSERT INTO orchestration_command_receipts \
         (command_id, aggregate_kind, aggregate_id, accepted_at, result_sequence, status, error, \
          fingerprint_version, command_fingerprint) \
         VALUES (?1, ?5, ?2, ?3, ?4, 'accepted', NULL, NULL, NULL)",
        params![
            command_id,
            last.thread_id,
            iso_now_ms(),
            sequence,
            last.agg_kind
        ],
    )
    .map_err(|e| err(format!("write command receipt: {e}")))?;
    tx.commit().map_err(|e| err(e.to_string()))?;
    Ok(sequence)
}

/// UPDATE projection_threads SET col=?, ... WHERE thread_id=?
fn update_projection(
    conn: &Connection,
    thread_id: &str,
    sets: &[(&str, Value)],
) -> Result<(), WsErr> {
    if sets.is_empty() {
        return Ok(());
    }
    let assignments: Vec<String> = sets
        .iter()
        .enumerate()
        .map(|(i, (col, _))| format!("{col} = ?{}", i + 2))
        .collect();
    let sql = format!(
        "UPDATE projection_threads SET {} WHERE thread_id = ?1",
        assignments.join(", ")
    );
    let mut values: Vec<rusqlite::types::Value> = Vec::with_capacity(sets.len() + 1);
    values.push(rusqlite::types::Value::Text(thread_id.to_string()));
    for (_, v) in sets {
        values.push(json_to_sql(v));
    }
    let refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as _).collect();
    conn.execute(&sql, refs.as_slice())
        .map_err(|e| err(format!("update projection_threads: {e}")))?;
    Ok(())
}

fn json_to_sql(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Null => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(*b as i64),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else {
                rusqlite::types::Value::Real(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        other => rusqlite::types::Value::Text(other.to_string()),
    }
}

fn collect_subagent_descendants(conn: &Connection, root: &str) -> Vec<String> {
    let rows: Vec<(String, Option<String>)> = conn
        .prepare("SELECT thread_id, parent_thread_id FROM projection_threads WHERE deleted_at IS NULL")
        .and_then(|mut st| {
            st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)))
                .map(|it| it.filter_map(|x| x.ok()).collect())
        })
        .unwrap_or_default();
    let mut out = Vec::new();
    let mut queue = vec![root.to_string()];
    let mut seen = std::collections::HashSet::from([root.to_string()]);
    while let Some(parent) = queue.pop() {
        for (id, parent_id) in &rows {
            if parent_id.as_deref() == Some(parent.as_str()) && seen.insert(id.clone()) {
                queue.push(id.clone());
                out.push(id.clone());
            }
        }
    }
    out
}

fn thread_archive(command: &Map<String, Value>, archive: bool) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let root = require_thread(&conn, &thread_id)?;
    if archive && root.archived_at.is_some() {
        return Err(err(format!("thread {thread_id} is already archived")));
    }
    if !archive && root.archived_at.is_none() {
        return Err(err(format!("thread {thread_id} is not archived")));
    }
    let occurred_at = iso_now_ms();
    // Descendants first (active ones), the commanded thread last.
    let mut ids: Vec<(String, ThreadRow)> = Vec::new();
    for id in collect_subagent_descendants(&conn, &thread_id) {
        let row = require_thread(&conn, &id)?;
        let include = if archive {
            row.deleted_at.is_none() && row.archived_at.is_none()
        } else {
            row.deleted_at.is_none() && row.archived_at.is_some()
        };
        if include {
            ids.push((id, row));
        }
    }
    ids.push((thread_id.clone(), require_thread(&conn, &thread_id)?));
    let mut events: Vec<OutEvent> = Vec::new();
    for (id, row) in &ids {
        if row.claude_cache_review.is_some() {
            events.push(out_event("thread.claude-cache-set", id, &occurred_at, json!({"threadId": id, "review": null, "updatedAt": occurred_at})));
        }
        events.push(out_event(if archive { "thread.archived" } else { "thread.unarchived" }, id, &occurred_at, if archive { json!({"threadId": id, "archivedAt": occurred_at, "updatedAt": occurred_at}) } else { json!({"threadId": id, "updatedAt": occurred_at}) }));
    }
    let seq = commit_events(&conn, &events, &command_id)?;
    for (id, row) in &ids {
        let mut sets: Vec<(&str, Value)> = Vec::new();
        if row.claude_cache_review.is_some() {
            sets.push(("claude_cache_review_json", Value::Null));
        }
        sets.push(("archived_at", if archive { json!(occurred_at) } else { Value::Null }));
        sets.push(("updated_at", json!(occurred_at)));
        update_projection(&conn, id, &sets)?;
    }
    Ok(json!({ "sequence": seq }))
}

fn thread_delete(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let row = require_thread(&conn, &thread_id)?;
    let occurred_at = iso_now_ms();
    let mut events: Vec<OutEvent> = Vec::new();
    if row.claude_cache_review.is_some() {
        events.push(out_event("thread.claude-cache-set", &thread_id, &occurred_at, json!({"threadId": thread_id, "review": null, "updatedAt": occurred_at})));
    }
    events.push(out_event("thread.deleted", &thread_id, &occurred_at, json!({"threadId": thread_id, "deletedAt": occurred_at})));
    let seq = commit_events(&conn, &events, &command_id)?;
    let mut sets: Vec<(&str, Value)> = vec![("deleted_at", json!(occurred_at))];
    if row.claude_cache_review.is_some() {
        sets.push(("claude_cache_review_json", Value::Null));
    }
    update_projection(&conn, &thread_id, &sets)?;
    Ok(json!({ "sequence": seq }))
}

fn thread_mode_set(
    command: &Map<String, Value>,
    field: &str,
    column: &str,
) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let value = s(command, field).ok_or_else(|| err(format!("missing {field}")))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let row = require_thread(&conn, &thread_id)?;
    let occurred_at = iso_now_ms();
    let event_type = if column == "runtime_mode" {
        "thread.runtime-mode-set"
    } else {
        "thread.interaction-mode-set"
    };
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    if column == "interaction_mode" {
        payload.insert("previousInteractionMode".into(), json!(row.interaction_mode));
    }
    payload.insert(field.to_string(), json!(value));
    payload.insert("updatedAt".into(), json!(occurred_at));
    let events = vec![out_event(event_type, &thread_id, &occurred_at, Value::Object(payload))];
    let seq = commit_events(&conn, &events, &command_id)?;
    update_projection(
        &conn,
        &thread_id,
        &[(column, json!(value)), ("updated_at", json!(occurred_at))],
    )?;
    Ok(json!({ "sequence": seq }))
}

fn thread_meta_update(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    if command.contains_key("goal")
        || command.contains_key("goalStartBehavior")
        || command.contains_key("goalAchieved")
        || command.contains_key("goalAchievements")
        || command.contains_key("providerHandoff")
    {
        return Err(err(
            "thread.meta.update with goal/providerHandoff fields is not ported yet",
        ));
    }
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let row = require_thread(&conn, &thread_id)?;
    let occurred_at = iso_now_ms();
    let project_kind: String = conn
        .query_row(
            "SELECT kind FROM projection_projects WHERE project_id = ?1",
            params![row.project_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "project".to_string());
    let is_group = is_group_container_kind(&project_kind);

    let mut ep = Map::new();
    ep.insert("threadId".into(), json!(thread_id));
    let mut sets: Vec<(&'static str, Value)> = Vec::new();
    let mut copy_field = |ep: &mut Map<String, Value>,
                          sets: &mut Vec<(&'static str, Value)>,
                          key: &'static str,
                          column: &'static str| {
        if let Some(v) = command.get(key) {
            ep.insert(key.to_string(), v.clone());
            sets.push((column, v.clone()));
        }
    };
    copy_field(&mut ep, &mut sets, "title", "title");
    copy_field(&mut ep, &mut sets, "parentThreadId", "parent_thread_id");
    copy_field(&mut ep, &mut sets, "subagentAgentId", "subagent_agent_id");
    copy_field(&mut ep, &mut sets, "subagentNickname", "subagent_nickname");
    copy_field(&mut ep, &mut sets, "subagentRole", "subagent_role");
    copy_field(&mut ep, &mut sets, "notes", "notes");
    if let Some(v) = command.get("modelSelection") {
        ep.insert("modelSelection".into(), v.clone());
        sets.push(("model_selection_json", json!(v.to_string())));
    }
    if let Some(v) = command.get("isPinned") {
        ep.insert("isPinned".into(), v.clone());
        sets.push(("is_pinned", v.clone()));
    }
    if let Some(v) = command.get("isSettled") {
        let settled = if v.as_bool().unwrap_or(false) {
            json!(occurred_at)
        } else {
            Value::Null
        };
        ep.insert("settledAt".into(), settled.clone());
        sets.push(("settled_at", settled));
    }
    if let Some(v) = command.get("snoozedUntil") {
        ep.insert("snoozedUntil".into(), v.clone());
        ep.insert("snoozeReminderAt".into(), Value::Null);
        sets.push(("snoozed_until", v.clone()));
        sets.push(("snooze_reminder_at", Value::Null));
    }
    if command.contains_key("expectedSnoozedUntil") {
        ep.insert("snoozeReminderAt".into(), json!(occurred_at));
        ep.insert("settledAt".into(), Value::Null);
        sets.push(("snooze_reminder_at", json!(occurred_at)));
        sets.push(("settled_at", Value::Null));
    }
    if let Some(v) = command.get("handoff") {
        ep.insert("handoff".into(), v.clone());
        sets.push(("handoff_json", v.clone()));
    }
    if let Some(v) = command.get("pinnedMessages") {
        ep.insert("pinnedMessages".into(), v.clone());
        sets.push(("pinned_messages_json", json!(v.to_string())));
    }
    // Workspace metadata patch (group/studio projects ignore worktree fields).
    let workspace_keys = [
        "envMode",
        "branch",
        "worktreePath",
        "workingDirectory",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
    ];
    if workspace_keys.iter().any(|k| command.contains_key(*k)) {
        let branch = if is_group { Value::Null } else { opt(command, "branch") };
        let worktree = if is_group {
            Value::Null
        } else {
            opt(command, "worktreePath")
        };
        let working = if is_group {
            if command.contains_key("workingDirectory") {
                opt(command, "workingDirectory")
            } else {
                opt(command, "worktreePath")
            }
        } else {
            opt(command, "workingDirectory")
        };
        let env_mode = if is_group {
            json!("local")
        } else {
            opt(command, "envMode")
        };
        ep.insert("envMode".into(), env_mode.clone());
        ep.insert("branch".into(), branch.clone());
        ep.insert("worktreePath".into(), worktree.clone());
        ep.insert("workingDirectory".into(), working.clone());
        sets.push(("env_mode", env_mode));
        sets.push(("branch", branch.clone()));
        sets.push(("worktree_path", worktree.clone()));
        sets.push(("working_directory", working));
        if !is_group {
            let (ap, ab, ar) = derive_associated(command, &branch, &worktree);
            ep.insert("associatedWorktreePath".into(), ap.clone());
            ep.insert("associatedWorktreeBranch".into(), ab.clone());
            ep.insert("associatedWorktreeRef".into(), ar.clone());
            sets.push(("associated_worktree_path", ap));
            sets.push(("associated_worktree_branch", ab));
            sets.push(("associated_worktree_ref", ar));
        }
    }
    ep.insert("updatedAt".into(), json!(occurred_at));
    sets.push(("updated_at", json!(occurred_at)));

    let events = vec![out_event("thread.meta-updated", &thread_id, &occurred_at, Value::Object(ep))];
    let seq = commit_events(&conn, &events, &command_id)?;
    update_projection(&conn, &thread_id, &sets)?;
    Ok(json!({ "sequence": seq }))
}

fn derive_associated(
    command: &Map<String, Value>,
    branch: &Value,
    worktree: &Value,
) -> (Value, Value, Value) {
    let path = if command.contains_key("associatedWorktreePath") {
        opt(command, "associatedWorktreePath")
    } else {
        worktree.clone()
    };
    let branch_v = if command.contains_key("associatedWorktreeBranch") {
        opt(command, "associatedWorktreeBranch")
    } else if !worktree.is_null() {
        branch.clone()
    } else {
        Value::Null
    };
    let ref_v = if command.contains_key("associatedWorktreeRef") {
        opt(command, "associatedWorktreeRef")
    } else if command.contains_key("associatedWorktreeBranch") {
        opt(command, "associatedWorktreeBranch")
    } else if !worktree.is_null() {
        branch.clone()
    } else {
        Value::Null
    };
    (path, branch_v, ref_v)
}

fn thread_create(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("thread.create missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("thread.create missing threadId"))?;
    let project_id = s(command, "projectId").ok_or_else(|| err("thread.create missing projectId"))?;
    let title = s(command, "title").ok_or_else(|| err("thread.create missing title"))?;
    let created_at =
        s(command, "createdAt").ok_or_else(|| err("thread.create missing createdAt"))?;

    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    // requireProject: the project must exist and not be deleted.
    let project_kind: String = conn
        .query_row(
            "SELECT kind FROM projection_projects WHERE project_id = ?1 AND deleted_at IS NULL",
            params![project_id],
            |r| r.get(0),
        )
        .map_err(|_| err(format!("project {project_id} not found")))?;

    // requireThreadAbsent.
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if exists {
        return Err(err(format!("thread {thread_id} already exists")));
    }

    let is_group = is_group_container_kind(&project_kind);
    let interaction_mode = s(command, "interactionMode").unwrap_or_else(|| "default".into());
    let env_mode = if is_group {
        "local".to_string()
    } else {
        s(command, "envMode").unwrap_or_else(|| "local".into())
    };
    let branch = if is_group { Value::Null } else { opt(command, "branch") };
    let worktree_path = if is_group {
        Value::Null
    } else {
        opt(command, "worktreePath")
    };
    let working_directory = if is_group {
        if command.contains_key("workingDirectory") {
            opt(command, "workingDirectory")
        } else {
            opt(command, "worktreePath")
        }
    } else {
        opt(command, "workingDirectory")
    };
    // deriveAssociatedWorktreeMetadata.
    let (associated_path, associated_branch, associated_ref) = if is_group {
        (Value::Null, Value::Null, Value::Null)
    } else {
        let path = if command.contains_key("associatedWorktreePath") {
            opt(command, "associatedWorktreePath")
        } else {
            worktree_path.clone()
        };
        let branch_v = if command.contains_key("associatedWorktreeBranch") {
            opt(command, "associatedWorktreeBranch")
        } else if !worktree_path.is_null() {
            branch.clone()
        } else {
            Value::Null
        };
        let ref_v = if command.contains_key("associatedWorktreeRef") {
            opt(command, "associatedWorktreeRef")
        } else if command.contains_key("associatedWorktreeBranch") {
            opt(command, "associatedWorktreeBranch")
        } else if !worktree_path.is_null() {
            branch.clone()
        } else {
            Value::Null
        };
        (path, branch_v, ref_v)
    };
    let create_branch_flow_completed = if is_group {
        false
    } else {
        command
            .get("createBranchFlowCompleted")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    let is_pinned = command
        .get("isPinned")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let last_known_pr = opt(command, "lastKnownPr");
    let pull_requests = if last_known_pr.is_null() {
        json!([])
    } else {
        json!([{
            "pullRequest": last_known_pr,
            "source": "checkout",
            "linkedAt": created_at,
        }])
    };

    let mut ep = Map::new();
    let put = |ep: &mut Map<String, Value>, k: &str, v: Value| {
        ep.insert(k.to_string(), v);
    };
    put(&mut ep, "threadId", json!(thread_id));
    put(&mut ep, "projectId", json!(project_id));
    put(&mut ep, "title", json!(title));
    put(&mut ep, "modelSelection", opt(command, "modelSelection"));
    put(&mut ep, "runtimeMode", opt(command, "runtimeMode"));
    put(&mut ep, "interactionMode", json!(interaction_mode));
    put(&mut ep, "envMode", json!(env_mode));
    put(&mut ep, "branch", branch.clone());
    put(&mut ep, "worktreePath", worktree_path.clone());
    put(&mut ep, "workingDirectory", working_directory.clone());
    put(&mut ep, "associatedWorktreePath", associated_path.clone());
    put(&mut ep, "associatedWorktreeBranch", associated_branch.clone());
    put(&mut ep, "associatedWorktreeRef", associated_ref.clone());
    put(&mut ep, "createBranchFlowCompleted", json!(create_branch_flow_completed));
    put(&mut ep, "isPinned", json!(is_pinned));
    put(&mut ep, "parentThreadId", opt(command, "parentThreadId"));
    if let Some(cs) = s(command, "creationSource") {
        put(&mut ep, "creationSource", json!(cs));
        put(&mut ep, "sourceThreadId", opt(command, "sourceThreadId"));
        put(&mut ep, "sourceTurnId", opt(command, "sourceTurnId"));
        put(&mut ep, "gatewayOperationId", opt(command, "gatewayOperationId"));
        put(&mut ep, "gatewayOperationIndex", opt(command, "gatewayOperationIndex"));
    }
    put(&mut ep, "subagentAgentId", opt(command, "subagentAgentId"));
    put(&mut ep, "subagentNickname", opt(command, "subagentNickname"));
    put(&mut ep, "subagentRole", opt(command, "subagentRole"));
    put(&mut ep, "forkSourceThreadId", Value::Null);
    let sidechat = opt(command, "sidechatContext");
    if !sidechat.is_null() {
        put(&mut ep, "sidechatContext", sidechat.clone());
        put(&mut ep, "sidechatLastActivityAt", json!(created_at));
        put(&mut ep, "sidechatExpiredAt", Value::Null);
    }
    put(&mut ep, "lastKnownPr", last_known_pr.clone());
    put(&mut ep, "pullRequests", pull_requests.clone());
    put(&mut ep, "handoff", Value::Null);
    put(&mut ep, "createdAt", json!(created_at));
    put(&mut ep, "updatedAt", json!(created_at));

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| err(e.to_string()))?;

    let stream_version: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(stream_version), 0) + 1 \
             FROM orchestration_events WHERE aggregate_kind = 'thread' AND stream_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e.to_string()))?;

    tx.execute(
        "INSERT INTO orchestration_events \
         (event_id, aggregate_kind, stream_id, stream_version, event_type, occurred_at, \
          command_id, causation_event_id, correlation_id, actor_kind, payload_json, metadata_json) \
         VALUES (?1, 'thread', ?2, ?3, 'thread.created', ?4, ?5, NULL, ?5, 'client', ?6, '{}')",
        params![
            uuid_v4(),
            thread_id,
            stream_version,
            created_at,
            command_id,
            Value::Object(ep).to_string(),
        ],
    )
    .map_err(|e| err(format!("append thread.created event: {e}")))?;
    let sequence = tx.last_insert_rowid();

    tx.execute(
        "INSERT INTO orchestration_command_receipts \
         (command_id, aggregate_kind, aggregate_id, accepted_at, result_sequence, status, error, \
          fingerprint_version, command_fingerprint) \
         VALUES (?1, 'thread', ?2, ?3, ?4, 'accepted', NULL, NULL, NULL)",
        params![command_id, thread_id, iso_now_ms(), sequence],
    )
    .map_err(|e| err(format!("write command receipt: {e}")))?;

    let str_or_null = |v: &Value| -> Option<String> { v.as_str().map(str::to_string) };
    tx.execute(
        "INSERT INTO projection_threads (\
           thread_id, project_id, title, branch, worktree_path, latest_turn_id, \
           created_at, updated_at, deleted_at, runtime_mode, interaction_mode, \
           model_selection_json, handoff_json, env_mode, fork_source_thread_id, \
           associated_worktree_path, associated_worktree_branch, associated_worktree_ref, \
           archived_at, parent_thread_id, subagent_agent_id, subagent_nickname, subagent_role, \
           latest_user_message_at, pending_approval_count, pending_user_input_count, \
           has_actionable_proposed_plan, last_known_pr_json, create_branch_flow_completed, \
           sidechat_source_thread_id, is_pinned, pinned_messages_json, notes, \
           creation_source, source_thread_id, source_turn_id, gateway_operation_id, \
           gateway_operation_index, working_directory, settled_at, goal, goal_started_at, \
           goal_paused_at, goal_achievements_json, sidechat_last_activity_at, sidechat_expired_at, \
           claude_cache_review_json, latest_human_message_at, sidechat_context_json, \
           snoozed_until, snooze_reminder_at, fork_source_message_id, \
           usage_limit_recovery_source_json, usage_limit_recovery_json, \
           usage_limit_recovery_cancelled_at, usage_limit_recovery_cancelled_turn_id, \
           usage_limit_recovery_last_reset_at, pull_requests_json\
         ) VALUES (\
           ?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6, NULL, ?7, ?8, ?9, NULL, ?10, NULL, \
           ?11, ?12, ?13, NULL, ?14, ?15, ?16, ?17, NULL, 0, 0, 0, ?18, ?19, \
           NULL, ?20, NULL, NULL, ?21, ?22, ?23, ?24, ?25, ?26, NULL, NULL, NULL, \
           NULL, NULL, ?27, NULL, NULL, NULL, ?28, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?29)",
        params![
            thread_id,                                                             // 1
            project_id,                                                            // 2
            title,                                                                 // 3
            str_or_null(&branch),                                                  // 4
            str_or_null(&worktree_path),                                           // 5
            created_at,                                                            // 6
            s(command, "runtimeMode").unwrap_or_else(|| "full-access".into()),      // 7
            interaction_mode,                                                      // 8
            opt(command, "modelSelection").to_string(),                            // 9
            env_mode,                                                              // 10
            str_or_null(&associated_path),                                         // 11
            str_or_null(&associated_branch),                                       // 12
            str_or_null(&associated_ref),                                          // 13
            s(command, "parentThreadId"),                                          // 14
            s(command, "subagentAgentId"),                                         // 15
            s(command, "subagentNickname"),                                        // 16
            s(command, "subagentRole"),                                            // 17
            if last_known_pr.is_null() { None } else { Some(last_known_pr.to_string()) }, // 18
            create_branch_flow_completed as i64,                                   // 19
            is_pinned as i64,                                                      // 20
            s(command, "creationSource"),                                          // 21
            s(command, "sourceThreadId"),                                          // 22
            s(command, "sourceTurnId"),                                            // 23
            s(command, "gatewayOperationId"),                                      // 24
            command.get("gatewayOperationIndex").and_then(Value::as_i64),          // 25
            str_or_null(&working_directory),                                       // 26
            if sidechat.is_null() { None } else { Some(created_at.clone()) },       // 27 sidechat_last_activity_at
            if sidechat.is_null() { None } else { Some(sidechat.to_string()) },     // 28 sidechat_context_json
            pull_requests.to_string(),                                             // 29 pull_requests_json
        ],
    )
    .map_err(|e| err(format!("project thread: {e}")))?;

    tx.commit().map_err(|e| err(e.to_string()))?;
    Ok(json!({ "sequence": sequence }))
}

const PROVIDER_SEND_TURN_MAX_INPUT_CHARS: usize = 120_000;

fn thread_turn_start(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    for k in [
        "asyncUserInputResponse",
        "sourceProposedPlan",
        "resumePrecondition",
        "reviewTarget",
    ] {
        if command.contains_key(k) {
            return Err(err(format!(
                "thread.turn.start with {k} is not ported yet (native engine slice)"
            )));
        }
    }
    let message = command
        .get("message")
        .and_then(Value::as_object)
        .ok_or_else(|| err("thread.turn.start missing message"))?;
    let message_id = s(message, "messageId").ok_or_else(|| err("missing message.messageId"))?;
    let text = s(message, "text").ok_or_else(|| err("missing message.text"))?;
    if text.len() > PROVIDER_SEND_TURN_MAX_INPUT_CHARS {
        return Err(err("the message exceeds the maximum message length"));
    }
    let created_at =
        s(command, "createdAt").ok_or_else(|| err("thread.turn.start missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let row = require_thread(&conn, &thread_id)?;

    let dispatch_mode = s(command, "dispatchMode").unwrap_or_else(|| "queue".into());
    let dispatch_origin = s(command, "dispatchOrigin").unwrap_or_else(|| "user".into());
    let runtime_mode = opt(command, "runtimeMode");
    let interaction_mode = opt(command, "interactionMode");
    // With no native provider sessions yet no turn can be running, so messages
    // queue only for a claude-cache review gate, exactly like upstream.
    let should_queue = row.parent_thread_id.is_none() && row.claude_cache_review.is_some();

    let mut events: Vec<OutEvent> = Vec::new();
    if dispatch_origin == "user"
        && (row.snoozed_until.is_some() || row.snooze_reminder_at.is_some())
    {
        events.push(out_event(
            "thread.meta-updated",
            &thread_id,
            &created_at,
            json!({
                "threadId": thread_id,
                "snoozedUntil": null,
                "snoozeReminderAt": null,
                "updatedAt": created_at,
            }),
        ));
    }

    let mut mp = Map::new();
    mp.insert("threadId".into(), json!(thread_id));
    mp.insert("messageId".into(), json!(message_id));
    mp.insert("role".into(), json!("user"));
    mp.insert("text".into(), json!(text));
    mp.insert(
        "attachments".into(),
        message.get("attachments").cloned().unwrap_or(json!([])),
    );
    if let Some(v) = message.get("skills") {
        mp.insert("skills".into(), v.clone());
    }
    if let Some(v) = message.get("mentions") {
        mp.insert("mentions".into(), v.clone());
    }
    mp.insert("dispatchMode".into(), json!(dispatch_mode));
    mp.insert("dispatchOrigin".into(), json!(dispatch_origin));
    // isThreadRunning is always false without native sessions: startsNewTurn is
    // `dispatchMode !== "steer" || !isThreadRunning || shouldQueue` => true.
    mp.insert("startsNewTurn".into(), json!(true));
    mp.insert("turnId".into(), Value::Null);
    mp.insert("streaming".into(), json!(false));
    mp.insert("source".into(), json!("native"));
    mp.insert("createdAt".into(), json!(created_at));
    mp.insert("updatedAt".into(), json!(created_at));
    let message_event = out_event(
        "thread.message-sent",
        &thread_id,
        &created_at,
        Value::Object(mp),
    );
    let message_event_id = message_event.event_id.clone();
    events.push(message_event);

    let mut tp = Map::new();
    tp.insert("threadId".into(), json!(thread_id));
    tp.insert("messageId".into(), json!(message_id));
    if let Some(v) = command.get("modelSelection") {
        tp.insert("modelSelection".into(), v.clone());
    }
    if let Some(v) = command.get("providerOptions") {
        tp.insert("providerOptions".into(), v.clone());
    }
    if let Some(v) = command.get("computerControlMode") {
        tp.insert("computerControlMode".into(), v.clone());
    }
    tp.insert(
        "assistantDeliveryMode".into(),
        command
            .get("assistantDeliveryMode")
            .cloned()
            .unwrap_or(json!("streaming")),
    );
    tp.insert("dispatchMode".into(), json!(dispatch_mode));
    tp.insert("dispatchOrigin".into(), json!(dispatch_origin));
    tp.insert("runtimeMode".into(), runtime_mode.clone());
    tp.insert("interactionMode".into(), interaction_mode.clone());
    tp.insert("createdAt".into(), json!(created_at));
    let mut turn_event = out_event(
        if should_queue {
            "thread.turn-queued"
        } else {
            "thread.turn-start-requested"
        },
        &thread_id,
        &created_at,
        Value::Object(tp),
    );
    turn_event.causation_event_id = Some(message_event_id);
    events.push(turn_event);

    let seq = commit_events(&conn, &events, &command_id)?;

    // --- projection: message row + thread shell fields ---------------------
    let attachments = message.get("attachments").cloned().unwrap_or(json!([]));
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_messages \
         (message_id, thread_id, turn_id, role, text, is_streaming, created_at, updated_at, \
          attachments_json, source, skills_json, mentions_json, dispatch_mode, dispatch_origin, \
          starts_new_turn) \
         VALUES (?1, ?2, NULL, 'user', ?3, 0, ?4, ?4, ?5, 'native', ?6, ?7, ?8, ?9, 1)",
        params![
            message_id,
            thread_id,
            text,
            created_at,
            attachments.to_string(),
            message.get("skills").map(|v| v.to_string()),
            message.get("mentions").map(|v| v.to_string()),
            dispatch_mode,
            dispatch_origin,
        ],
    )
    .map_err(|e| err(format!("insert message: {e}")))?;

    let mut sets: Vec<(&'static str, Value)> = Vec::new();
    let created_v = json!(created_at);
    if row
        .latest_user_message_at
        .as_deref()
        .map_or(true, |v| v < created_at.as_str())
    {
        sets.push(("latest_user_message_at", created_v.clone()));
    }
    if dispatch_origin == "user"
        && row
            .latest_human_message_at
            .as_deref()
            .map_or(true, |v| v < created_at.as_str())
    {
        sets.push(("latest_human_message_at", created_v.clone()));
    }
    if row.updated_at.as_str() < created_at.as_str() {
        sets.push(("updated_at", created_v.clone()));
    }
    if dispatch_origin != "automation" {
        sets.push(("runtime_mode", runtime_mode.clone()));
        sets.push(("interaction_mode", interaction_mode.clone()));
    }
    // Fresh thread (no turn, no session): adopt the requested model selection.
    if row.latest_turn_id.is_none()
        && let Some(ms) = command.get("modelSelection")
    {
        sets.push(("model_selection_json", json!(ms.to_string())));
    }
    if events.iter().any(|e| e.event_type == "thread.meta-updated") {
        sets.push(("snoozed_until", Value::Null));
        sets.push(("snooze_reminder_at", Value::Null));
    }
    update_projection(&conn, &thread_id, &sets)?;
    Ok(json!({ "sequence": seq }))
}

/// Event-only commands: validate the thread exists, append one event whose
/// payload is the command's fields plus createdAt; no projection side effects.
fn thread_simple(command: &Map<String, Value>, event_type: &'static str) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    for (k, v) in command {
        if matches!(
            k.as_str(),
            "type" | "commandId" | "threadId" | "createdAt"
        ) {
            continue;
        }
        payload.insert(k.clone(), v.clone());
    }
    payload.insert("createdAt".into(), json!(created_at));
    let mut ev = out_event(
        event_type,
        &thread_id,
        &created_at,
        Value::Object(payload),
    );
    if event_type == "thread.approval-response-requested"
        || event_type == "thread.user-input-response-requested"
    {
        if let Some(rid) = s(command, "requestId") {
            ev.metadata = Some(json!({ "requestId": rid }));
        }
    }
    // thread.turn.interrupt pauses an active goal before the interrupt request.
    let mut events: Vec<OutEvent> = Vec::new();
    if event_type == "thread.turn-interrupt-requested" {
        let (goal, paused): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT goal, goal_paused_at FROM projection_threads WHERE thread_id = ?1",
                params![thread_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| err(e.to_string()))?;
        if goal.as_deref().map_or(false, |g| !g.trim().is_empty()) && paused.is_none() {
            let pause_at = iso_now_ms();
            let mut pause = out_event(
                "thread.meta-updated",
                &thread_id,
                &pause_at,
                json!({"threadId": thread_id, "goalPausedAt": pause_at, "updatedAt": pause_at}),
            );
            events.push(pause.clone_shallow());
            ev.causation_event_id = Some(pause.event_id);
            update_projection(
                &conn,
                &thread_id,
                &[
                    ("goal_paused_at", json!(pause_at)),
                    ("updated_at", json!(pause_at)),
                ],
            )?;
        }
    }
    events.push(ev);
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

impl OutEvent {
    fn clone_shallow(&self) -> OutEvent {
        OutEvent {
            event_id: self.event_id.clone(),
            event_type: self.event_type,
            payload: self.payload.clone(),
            thread_id: self.thread_id.clone(),
            agg_kind: self.agg_kind,
            occurred_at: self.occurred_at.clone(),
            causation_event_id: None,
            metadata: None,
        }
    }
}

const PINNED_MESSAGES_MAX_COUNT: usize = 100;

fn pinned_message(command: &Map<String, Value>, op: &str) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let message_id = s(command, "messageId").ok_or_else(|| err("missing messageId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let pins_json: Option<String> = conn
        .query_row(
            "SELECT pinned_messages_json FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or(None);
    let mut pins: Vec<Value> = pins_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();
    let occurred_at = iso_now_ms();
    let existing = pins
        .iter()
        .position(|p| p.get("messageId").and_then(Value::as_str) == Some(message_id.as_str()));

    let (event_type, payload) = match op {
        "add" => {
            if existing.is_none() && pins.len() >= PINNED_MESSAGES_MAX_COUNT {
                return Err(err(format!(
                    "thread {thread_id} already has the maximum of {PINNED_MESSAGES_MAX_COUNT} pinned messages"
                )));
            }
            let pin = existing
                .map(|i| pins[i].clone())
                .unwrap_or_else(|| {
                    json!({"messageId": message_id, "label": null, "done": false, "pinnedAt": occurred_at})
                });
            if existing.is_none() {
                pins.push(pin.clone());
            }
            (
                "thread.pinned-message-added",
                json!({"threadId": thread_id, "pin": pin, "updatedAt": occurred_at}),
            )
        }
        "remove" => {
            if let Some(i) = existing {
                pins.remove(i);
            }
            (
                "thread.pinned-message-removed",
                json!({"threadId": thread_id, "messageId": message_id, "updatedAt": occurred_at}),
            )
        }
        "done" => {
            let done = command.get("done").cloned().unwrap_or(Value::Null);
            if let Some(i) = existing {
                pins[i]["done"] = done.clone();
            }
            (
                "thread.pinned-message-done-set",
                json!({"threadId": thread_id, "messageId": message_id, "done": done, "updatedAt": occurred_at}),
            )
        }
        "label" => {
            let label = command.get("label").cloned().unwrap_or(Value::Null);
            if let Some(i) = existing {
                pins[i]["label"] = label.clone();
            }
            (
                "thread.pinned-message-label-set",
                json!({"threadId": thread_id, "messageId": message_id, "label": label, "updatedAt": occurred_at}),
            )
        }
        _ => unreachable!(),
    };
    let events = vec![out_event(event_type, &thread_id, &occurred_at, payload)];
    let seq = commit_events(&conn, &events, &command_id)?;
    update_projection(
        &conn,
        &thread_id,
        &[
            ("pinned_messages_json", json!(Value::Array(pins).to_string())),
            ("updated_at", json!(occurred_at)),
        ],
    )?;
    Ok(json!({ "sequence": seq }))
}

// ---- project.create ----

const PLATFORM: &str = std::env::consts::OS;
const MAX_PINNED_PROJECTS: i64 = 3;

/// normalizeWorkspaceRootForComparison: separator + slash-run canonicalization,
/// trailing-slash strip, macOS /private{,/var,/tmp} alias folding, Windows lowercasing.
fn normalize_root(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let fwd = trimmed.replace('\\', "/");
    let is_windows_drive_root = {
        let b = fwd.as_bytes();
        b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && fwd[2..].chars().all(|c| c == '/')
    };
    let has_unc = fwd.starts_with("//");
    let prefix: &str = if has_unc {
        "//"
    } else if fwd.starts_with('/') {
        "/"
    } else {
        ""
    };
    let mut body = fwd[prefix.len()..].to_string();
    while body.contains("//") {
        body = body.replace("//", "/");
    }
    while body.ends_with('/') {
        body.pop();
    }
    let normalized = if !prefix.is_empty() {
        format!("{prefix}{body}")
    } else {
        body
    };
    let mut final_value = if is_windows_drive_root {
        format!("{}/", &fwd[..2])
    } else if normalized.is_empty() {
        prefix.to_string()
    } else {
        normalized
    };
    if PLATFORM == "macos"
        && (final_value.starts_with("/private/var/") || final_value.starts_with("/private/tmp/"))
    {
        final_value = final_value["/private".len()..].to_string();
    }
    // isLikelyWindowsWorkspaceRoot: on non-win32 platforms only true when the path
    // literally looks like a Windows path (drive letter or UNC).
    let looks_windows = {
        let t = trimmed;
        (t.len() >= 3 && t.as_bytes()[1] == b':' && (t.as_bytes()[2] == b'\\' || t.as_bytes()[2] == b'/')
            && t.as_bytes()[0].is_ascii_alphabetic())
            || t.starts_with("\\\\")
            || t.starts_with("//")
    };
    if PLATFORM == "windows" || looks_windows {
        return final_value.to_lowercase();
    }
    final_value
}

fn root_within(candidate: &str, ancestor: &str) -> bool {
    let c = normalize_root(candidate);
    let a = normalize_root(ancestor);
    if c.is_empty() || a.is_empty() {
        return false;
    }
    if c == a {
        return true;
    }
    let prefix = if a.ends_with('/') { a } else { format!("{a}/") };
    c.starts_with(&prefix)
}

fn is_absolute_folder_path(path: &str) -> bool {
    let t = path.trim();
    t.starts_with('/')
        || (t.len() >= 3
            && t.as_bytes()[0].is_ascii_alphabetic()
            && t.as_bytes()[1] == b':'
            && (t.as_bytes()[2] == b'\\' || t.as_bytes()[2] == b'/'))
        || t.starts_with("\\\\")
}

fn folder_name(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|p| !p.is_empty())
        .last()
        .unwrap_or(path)
        .to_string()
}

/// findProjectFolderProblem: absolute, distinct, and non-nested folder list.
fn find_project_folder_problem(paths: &[String]) -> Option<String> {
    if paths.is_empty() {
        return Some("Add at least one folder.".to_string());
    }
    for p in paths {
        if !is_absolute_folder_path(p) {
            return Some(format!("Use an absolute path: {p}"));
        }
    }
    for outer in 0..paths.len() {
        for inner in (outer + 1)..paths.len() {
            let left = &paths[outer];
            let right = &paths[inner];
            if normalize_root(left) == normalize_root(right) {
                return Some(format!("{} is already in this project.", folder_name(right)));
            }
            if root_within(right, left) {
                return Some(format!(
                    "{} is inside {}. Add only one of them.",
                    folder_name(right),
                    folder_name(left)
                ));
            }
            if root_within(left, right) {
                return Some(format!(
                    "{} is inside {}. Add only one of them.",
                    folder_name(left),
                    folder_name(right)
                ));
            }
        }
    }
    None
}

/// realpathNearestExisting: canonicalize the deepest existing ancestor and
/// re-append the not-yet-created remainder.
fn realpath_nearest_existing(p: &std::path::Path) -> std::path::PathBuf {
    if let Ok(c) = std::fs::canonicalize(p) {
        return c;
    }
    let mut missing: Vec<std::ffi::OsString> = Vec::new();
    let mut cur = p.to_path_buf();
    loop {
        if let Ok(c) = std::fs::canonicalize(&cur) {
            let mut out = c;
            for part in missing.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match cur.parent() {
            Some(parent) => {
                missing.push(cur.file_name().unwrap_or_default().to_os_string());
                cur = parent.to_path_buf();
            }
            None => return p.to_path_buf(),
        }
    }
}

fn workspace_paths() -> (String, String) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    let home_dir = realpath_nearest_existing(std::path::Path::new(&home))
        .to_string_lossy()
        .to_string();
    let chat_root = realpath_nearest_existing(
        &std::path::Path::new(&home_dir).join("Documents").join("Synara"),
    )
    .to_string_lossy()
    .to_string();
    (home_dir, chat_root)
}

fn is_legacy_home_row(title: &str, root: &str) -> bool {
    if title != "Home" {
        return false;
    }
    let (home_dir, chat_root) = workspace_paths();
    if home_dir.trim().is_empty() || chat_root.trim().is_empty() {
        return false;
    }
    normalize_root(root) == normalize_root(&chat_root) || normalize_root(root) == normalize_root(&home_dir)
}

struct ProjectRow {
    id: String,
    kind: Option<String>,
    title: String,
    workspace_root: String,
    is_pinned: bool,
    space_id: Option<String>,
}

fn active_projects_by_root(
    conn: &Connection,
    root: &str,
    kinds: &[&str],
) -> Result<Vec<ProjectRow>, WsErr> {
    let normalized = normalize_root(root);
    let mut st = conn
        .prepare(
            "SELECT project_id, kind, title, workspace_root, is_pinned, space_id \
             FROM projection_projects WHERE deleted_at IS NULL",
        )
        .map_err(|e| err(e.to_string()))?;
    let rows = st
        .query_map([], |r| {
            Ok(ProjectRow {
                id: r.get(0)?,
                kind: r.get(1)?,
                title: r.get(2)?,
                workspace_root: r.get(3)?,
                is_pinned: r.get::<_, i64>(4)? != 0,
                space_id: r.get(5)?,
            })
        })
        .map_err(|e| err(e.to_string()))?;
    let mut out = Vec::new();
    for row in rows {
        let row = row.map_err(|e| err(e.to_string()))?;
        let kind = row.kind.clone().unwrap_or_else(|| "project".to_string());
        if !kinds.contains(&kind.as_str()) {
            continue;
        }
        if normalize_root(&row.workspace_root) == normalized {
            out.push(row);
        }
    }
    Ok(out)
}

fn project_event(agg_id: &str, event_type: &'static str, occurred_at: &str, payload: Value) -> OutEvent {
    let mut ev = out_event(event_type, agg_id, occurred_at, payload);
    ev.agg_kind = "project";
    ev
}

fn project_create(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let project_id = s(command, "projectId").ok_or_else(|| err("missing projectId"))?;
    let title = s(command, "title").ok_or_else(|| err("missing title"))?;
    let workspace_root =
        s(command, "workspaceRoot").ok_or_else(|| err("missing workspaceRoot"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let next_kind = s(command, "kind").unwrap_or_else(|| "project".to_string());
    let additional_folders: Vec<String> = command
        .get("additionalFolders")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let is_pinned = command.get("isPinned").and_then(Value::as_bool).unwrap_or(false);
    let preserve = command
        .get("preserveExistingProject")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let space_id: Option<String> = command
        .get("spaceId")
        .and_then(Value::as_str)
        .map(String::from);
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    // requireProjectAbsent
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projection_projects WHERE project_id = ?1",
            params![project_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e.to_string()))?;
    if exists > 0 {
        return Err(err(format!(
            "Project '{project_id}' already exists and cannot be created twice."
        )));
    }

    // requireValidProjectFolders
    if !additional_folders.is_empty() {
        if next_kind != "project" {
            return Err(err("Only ordinary projects can have additional folders."));
        }
        let mut paths = vec![workspace_root.clone()];
        paths.extend(additional_folders.iter().cloned());
        if let Some(problem) = find_project_folder_problem(&paths) {
            return Err(err(problem));
        }
    }

    let mut events: Vec<OutEvent> = Vec::new();
    let mut stale_ids: Vec<String> = Vec::new();

    if next_kind == "project" {
        // Group containers own their root exclusively; never retired here.
        if let Some(existing) =
            active_projects_by_root(&conn, &workspace_root, &["studio", "group"])?.into_iter().next()
        {
            return Err(err(format!(
                "Project '{}' already uses workspace root '{}'.",
                existing.id, existing.workspace_root
            )));
        }
        for existing in active_projects_by_root(&conn, &workspace_root, &["project"])? {
            let live_threads: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM projection_threads \
                     WHERE project_id = ?1 AND deleted_at IS NULL",
                    params![existing.id],
                    |r| r.get(0),
                )
                .map_err(|e| err(e.to_string()))?;
            if live_threads > 0 || preserve {
                return Err(err(format!(
                    "Project '{}' already uses workspace root '{}'.",
                    existing.id, existing.workspace_root
                )));
            }
            stale_ids.push(existing.id.clone());
        }
        for stale_id in &stale_ids {
            events.push(project_event(
                stale_id,
                "project.deleted",
                &created_at,
                json!({"projectId": stale_id, "deletedAt": created_at}),
            ));
        }
    }
    if next_kind == "group" || next_kind == "studio" {
        // Cross-kind: a regular project already owning this root would break
        // workspace-root-to-project uniqueness.
        if let Some(existing) =
            active_projects_by_root(&conn, &workspace_root, &["project", "studio", "group"])?
                .into_iter()
                .next()
        {
            return Err(err(format!(
                "Project '{}' already uses workspace root '{}'.",
                existing.id, existing.workspace_root
            )));
        }
    }

    // validateProjectPinLimit
    if is_pinned && next_kind != "project" {
        return Err(err("Only projects can be pinned."));
    }
    if is_pinned {
        let mut st = conn
            .prepare(
                "SELECT project_id FROM projection_projects \
                 WHERE deleted_at IS NULL AND kind = 'project' AND is_pinned = 1",
            )
            .map_err(|e| err(e.to_string()))?;
        let pinned: Vec<String> = st
            .query_map([], |r| r.get(0))
            .map_err(|e| err(e.to_string()))?
            .collect::<Result<_, _>>()
            .map_err(|e| err(e.to_string()))?;
        let count = pinned
            .iter()
            .filter(|id| *id != &project_id && !stale_ids.contains(id))
            .count() as i64;
        if count >= MAX_PINNED_PROJECTS {
            return Err(err(format!(
                "Only {MAX_PINNED_PROJECTS} projects can be pinned at once."
            )));
        }
    }

    // Best-effort space filing: an unusable target degrades to Void.
    let creation_space_id: Option<String> = if next_kind == "project" && space_id.is_some() {
        let sid = space_id.clone().unwrap();
        let space_ok: bool = conn
            .query_row(
                "SELECT deleted_at IS NULL FROM projection_spaces WHERE space_id = ?1",
                params![sid],
                |r| r.get::<_, bool>(0),
            )
            .unwrap_or(false);
        if space_ok && !is_legacy_home_row(&title, &workspace_root) {
            Some(sid)
        } else {
            None
        }
    } else {
        None
    };

    let default_model = command
        .get("defaultModelSelection")
        .cloned()
        .unwrap_or(Value::Null);
    events.push(project_event(
        &project_id,
        "project.created",
        &created_at,
        json!({
            "projectId": project_id,
            "kind": next_kind,
            "title": title,
            "workspaceRoot": workspace_root,
            "defaultModelSelection": default_model,
            "scripts": [],
            "isPinned": is_pinned,
            "spaceId": creation_space_id,
            "additionalFolders": additional_folders,
            "createdAt": created_at,
            "updatedAt": created_at,
        }),
    ));

    let seq = commit_events(&conn, &events, &command_id)?;

    // Projection writes: retire stale shells, insert the new row.
    for stale_id in &stale_ids {
        conn.execute(
            "UPDATE projection_projects SET deleted_at = ?2, updated_at = ?2 \
             WHERE project_id = ?1",
            params![stale_id, created_at],
        )
        .map_err(|e| err(e.to_string()))?;
    }
    conn.execute(
        "INSERT OR REPLACE INTO projection_projects \
         (project_id, kind, title, workspace_root, scripts_json, created_at, updated_at, \
          deleted_at, default_model_selection_json, is_pinned, space_id, additional_folders_json) \
         VALUES (?1, ?2, ?3, ?4, '[]', ?5, ?5, NULL, ?6, ?7, ?8, ?9)",
        params![
            project_id,
            next_kind,
            title,
            workspace_root,
            created_at,
            if default_model.is_null() { None } else { Some(default_model.to_string()) },
            is_pinned as i64,
            creation_space_id,
            serde_json::to_string(&additional_folders).unwrap_or_else(|_| "[]".to_string()),
        ],
    )
    .map_err(|e| err(format!("project projection insert: {e}")))?;
    Ok(json!({ "sequence": seq }))
}

// ---- project.meta.update / project.delete ----

/// relocateProjectPath: rebase `value` from previousRoot onto nextRoot when it
/// points at or inside the old root; returns the input unchanged otherwise.
fn relocate_project_path(value: &str, previous_root: &str, next_root: &str) -> String {
    if !is_absolute_folder_path(value)
        || !is_absolute_folder_path(previous_root)
        || !is_absolute_folder_path(next_root)
    {
        return value.to_string();
    }
    let prev = normalize_root(previous_root);
    let cand = normalize_root(value);
    let relative = if cand == prev {
        String::new()
    } else {
        let prefix = format!("{prev}/");
        match cand.strip_prefix(&prefix) {
            Some(r) => r.to_string(),
            None => return value.to_string(),
        }
    };
    if relative.is_empty() {
        next_root.to_string()
    } else {
        format!("{}/{}", next_root.trim_end_matches('/'), relative)
    }
}

/// threadHasInFlightTurn on projection data: a live session or running turn.
fn thread_has_in_flight(conn: &Connection, thread_id: &str) -> Result<bool, WsErr> {
    let session: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT status, active_turn_id FROM projection_thread_sessions \
             WHERE thread_id = ?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    if let Some((status, active_turn)) = session {
        let status = status.unwrap_or_default();
        if status == "starting" || status == "running" {
            return Ok(true);
        }
        if status != "error" && active_turn.is_some() {
            return Ok(true);
        }
    }
    let running_turn: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM projection_turns \
             WHERE thread_id = ?1 AND state = 'running'",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or(false);
    Ok(running_turn)
}

/// threadHasCheckpointRevertInProgress: latest checkpoint-revert lifecycle
/// activity is the started marker.
fn thread_has_revert_in_progress(conn: &Connection, thread_id: &str) -> Result<bool, WsErr> {
    let kind: Option<String> = conn
        .query_row(
            "SELECT kind FROM projection_thread_activities \
             WHERE thread_id = ?1 \
               AND kind IN ('checkpoint.revert.started','checkpoint.revert.succeeded','checkpoint.revert.failed') \
             ORDER BY sequence DESC, created_at DESC, activity_id DESC LIMIT 1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok();
    Ok(kind.as_deref() == Some("checkpoint.revert.started"))
}

fn project_meta_update(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let project_id = s(command, "projectId").ok_or_else(|| err("missing projectId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let existing: Option<ProjectRow> = conn
        .query_row(
            "SELECT project_id, kind, title, workspace_root, is_pinned, space_id \
             FROM projection_projects WHERE project_id = ?1",
            params![project_id],
            |r| {
                Ok(ProjectRow {
                    id: r.get(0)?,
                    kind: r.get(1)?,
                    title: r.get(2)?,
                    workspace_root: r.get(3)?,
                    is_pinned: r.get::<_, i64>(4)? != 0,
                    space_id: r.get(5)?,
                })
            },
        )
        .ok();
    let existing = existing.ok_or_else(|| {
        err(format!(
            "Project '{project_id}' does not exist for command 'project.meta.update'."
        ))
    })?;
    let deleted_at: Option<String> = conn
        .query_row(
            "SELECT deleted_at FROM projection_projects WHERE project_id = ?1",
            params![project_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e.to_string()))?;
    let existing_kind = existing.kind.clone().unwrap_or_else(|| "project".to_string());
    let next_kind = s(command, "kind").unwrap_or_else(|| existing_kind.clone());
    let has = |k: &str| command.contains_key(k);
    let cmd_space_present = has("spaceId");
    let cmd_space: Option<String> = command
        .get("spaceId")
        .and_then(|v| v.as_str().map(String::from));
    let requested_space: Option<Option<String>> = if cmd_space_present {
        Some(cmd_space.clone())
    } else if next_kind != "project" && existing.space_id.is_some() {
        Some(None)
    } else {
        None
    };
    let effective_space = requested_space
        .clone()
        .unwrap_or_else(|| existing.space_id.clone());
    let changed_space: Option<Option<String>> = match &requested_space {
        Some(rs) if rs != &existing.space_id => Some(rs.clone()),
        _ => None,
    };
    let has_other_metadata = has("kind")
        || has("title")
        || has("workspaceRoot")
        || has("defaultModelSelection")
        || has("scripts")
        || has("isPinned")
        || has("additionalFolders");

    let legacy = is_legacy_home_row(&existing.title, &existing.workspace_root);
    if has("title") && s(command, "title") != Some(existing.title.clone()) && legacy {
        return Err(err("The legacy Chats container cannot be renamed."));
    }
    if has("workspaceRoot")
        && s(command, "workspaceRoot")
            .map(|r| normalize_root(&r) != normalize_root(&existing.workspace_root))
            .unwrap_or(false)
        && legacy
    {
        return Err(err(
            "The legacy Chats container workspace root cannot be changed.",
        ));
    }
    if has("workspaceRoot") || has("additionalFolders") || has("kind") {
        let root = s(command, "workspaceRoot").unwrap_or_else(|| existing.workspace_root.clone());
        let folders: Vec<String> = command
            .get("additionalFolders")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| {
                conn.query_row(
                    "SELECT additional_folders_json FROM projection_projects WHERE project_id = ?1",
                    params![project_id],
                    |r| r.get::<_, String>(0),
                )
                .ok()
                .and_then(|j| serde_json::from_str::<Vec<String>>(&j).ok())
                .unwrap_or_default()
            });
        if !folders.is_empty() {
            if next_kind != "project" {
                return Err(err("Only ordinary projects can have additional folders."));
            }
            let mut paths = vec![root];
            paths.extend(folders);
            if let Some(problem) = find_project_folder_problem(&paths) {
                return Err(err(problem));
            }
        }
    }
    if effective_space.is_some() {
        let t = s(command, "title").unwrap_or_else(|| existing.title.clone());
        let r = s(command, "workspaceRoot").unwrap_or_else(|| existing.workspace_root.clone());
        if is_legacy_home_row(&t, &r) {
            return Err(err(
                "The Chats container is reachable from every space and cannot be assigned to one.",
            ));
        }
    }
    if cmd_space_present && cmd_space.is_some() {
        if deleted_at.is_some() {
            return Err(err("Deleted projects cannot be assigned to a space."));
        }
        if next_kind != "project" {
            return Err(err("Only ordinary projects can be assigned to a space."));
        }
        let sid = cmd_space.clone().unwrap();
        let space_ok: bool = conn
            .query_row(
                "SELECT deleted_at IS NULL FROM projection_spaces WHERE space_id = ?1",
                params![sid],
                |r| r.get(0),
            )
            .unwrap_or(false);
        if !space_ok {
            return Err(err(format!(
                "Space '{sid}' does not exist for command 'project.meta.update'."
            )));
        }
    }
    if requested_space.is_some() && changed_space.is_none() && !has_other_metadata {
        return Err(err("Project is already assigned to this space."));
    }
    let ownership_may_change = has("workspaceRoot")
        || (has("kind") && next_kind != existing_kind);
    if ownership_may_change && next_kind != "chat" {
        let root = s(command, "workspaceRoot").unwrap_or_else(|| existing.workspace_root.clone());
        if let Some(other) =
            active_projects_by_root(&conn, &root, &["project", "studio", "group"])?
                .into_iter()
                .find(|p| p.id != project_id)
        {
            return Err(err(format!(
                "Project '{}' already uses workspace root '{}'.",
                other.id, other.workspace_root
            )));
        }
    }
    // validateProjectPinLimit
    let cmd_pinned = command.get("isPinned").and_then(Value::as_bool);
    let next_pinned = cmd_pinned.unwrap_or(existing.is_pinned);
    if next_pinned && next_kind != "project" {
        return Err(err("Only projects can be pinned."));
    }
    if cmd_pinned == Some(true) {
        if deleted_at.is_some() {
            return Err(err(format!(
                "Deleted project '{project_id}' cannot be pinned."
            )));
        }
        if !existing.is_pinned {
            let count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM projection_projects \
                     WHERE deleted_at IS NULL AND kind = 'project' AND is_pinned = 1 \
                       AND project_id != ?1",
                    params![project_id],
                    |r| r.get(0),
                )
                .map_err(|e| err(e.to_string()))?;
            if count >= MAX_PINNED_PROJECTS {
                return Err(err(format!(
                    "Only {MAX_PINNED_PROJECTS} projects can be pinned at once."
                )));
            }
        }
    }

    let occurred_at = iso_now_ms();
    let mut payload = Map::new();
    payload.insert("projectId".into(), json!(project_id));
    for (k, ev_key) in [
        ("kind", "kind"),
        ("title", "title"),
        ("workspaceRoot", "workspaceRoot"),
        ("defaultModelSelection", "defaultModelSelection"),
        ("scripts", "scripts"),
        ("isPinned", "isPinned"),
        ("additionalFolders", "additionalFolders"),
    ] {
        if let Some(v) = command.get(k) {
            payload.insert(ev_key.to_string(), v.clone());
        }
    }
    if let Some(cs) = &changed_space {
        payload.insert("spaceId".into(), json!(cs));
    }
    payload.insert("updatedAt".into(), json!(occurred_at));

    let mut events: Vec<OutEvent> = vec![project_event(
        &project_id,
        "project.meta-updated",
        &occurred_at,
        Value::Object(payload.clone()),
    )];

    // withProjectRelocationEvents: moving a real project relocates live threads'
    // working directories in the same committed batch.
    let mut relocations: Vec<(String, String)> = Vec::new();
    if let Some(new_root) = payload.get("workspaceRoot").and_then(Value::as_str) {
        let roots_differ = normalize_root(new_root) != normalize_root(&existing.workspace_root);
        let prev_is_project = existing_kind == "project";
        let next_is_project =
            payload.get("kind").and_then(Value::as_str).unwrap_or(&existing_kind) == "project";
        if roots_differ && prev_is_project && next_is_project {
            let mut st = conn
                .prepare(
                    "SELECT thread_id, working_directory, worktree_path, associated_worktree_path \
                     FROM projection_threads \
                     WHERE project_id = ?1 AND deleted_at IS NULL",
                )
                .map_err(|e| err(e.to_string()))?;
            let threads: Vec<(String, Option<String>, Option<String>, Option<String>)> = st
                .query_map(params![project_id], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                })
                .map_err(|e| err(e.to_string()))?
                .collect::<Result<_, _>>()
                .map_err(|e| err(e.to_string()))?;
            for (tid, wd, wt, awt) in threads {
                if thread_has_in_flight(&conn, &tid)? || thread_has_revert_in_progress(&conn, &tid)?
                {
                    return Err(err(
                        "Stop active turns and wait for checkpoint restores before changing the project path.",
                    ));
                }
                for worktree in [wt.as_deref(), awt.as_deref()].into_iter().flatten() {
                    if relocate_project_path(worktree, &existing.workspace_root, new_root)
                        != worktree
                    {
                        return Err(err(
                            "This project has linked Git worktrees inside its old folder. Move and repair those worktrees separately before changing the project path.",
                        ));
                    }
                }
                if let Some(wd) = wd.as_deref() {
                    let relocated =
                        relocate_project_path(wd, &existing.workspace_root, new_root);
                    if relocated != wd {
                        relocations.push((tid.clone(), relocated.clone()));
                        let mut ev = out_event(
                            "thread.meta-updated",
                            &tid,
                            &occurred_at,
                            json!({"threadId": tid, "workingDirectory": relocated, "updatedAt": occurred_at}),
                        );
                        ev.causation_event_id = Some(events[0].event_id.clone());
                        events.push(ev);
                    }
                }
            }
        }
    }

    let seq = commit_events(&conn, &events, &command_id)?;

    // Projection write: apply the changed fields.
    let mut sets: Vec<(&str, Value)> = Vec::new();
    if let Some(v) = payload.get("kind") {
        sets.push(("kind", v.clone()));
    }
    if let Some(v) = payload.get("title") {
        sets.push(("title", v.clone()));
    }
    if let Some(v) = payload.get("workspaceRoot") {
        sets.push(("workspace_root", v.clone()));
    }
    if let Some(v) = payload.get("defaultModelSelection") {
        sets.push((
            "default_model_selection_json",
            if v.is_null() { Value::Null } else { json!(v.to_string()) },
        ));
    }
    if let Some(v) = payload.get("scripts") {
        sets.push(("scripts_json", json!(v.to_string())));
    }
    if let Some(v) = payload.get("isPinned") {
        sets.push(("is_pinned", json!(v.as_bool().unwrap_or(false) as i64)));
    }
    if let Some(cs) = &changed_space {
        sets.push(("space_id", json!(cs)));
    }
    if let Some(v) = payload.get("additionalFolders") {
        sets.push(("additional_folders_json", json!(v.to_string())));
    }
    sets.push(("updated_at", json!(occurred_at)));
    let cols: Vec<String> = sets.iter().map(|(c, _)| c.to_string()).collect();
    let assignments: Vec<String> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{c} = ?{}", i + 2))
        .collect();
    let mut params_vec: Vec<rusqlite::types::Value> = vec![rusqlite::types::Value::Text(project_id.clone())];
    for (_, v) in &sets {
        params_vec.push(json_to_sql(v));
    }
    conn.execute(
        &format!(
            "UPDATE projection_projects SET {} WHERE project_id = ?1",
            assignments.join(", ")
        ),
        rusqlite::params_from_iter(params_vec),
    )
    .map_err(|e| err(format!("project meta projection: {e}")))?;
    for (tid, wd) in relocations {
        update_projection(
            &conn,
            &tid,
            &[("working_directory", json!(wd)), ("updated_at", json!(occurred_at))],
        )?;
    }
    Ok(json!({ "sequence": seq }))
}

fn project_delete(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let project_id = s(command, "projectId").ok_or_else(|| err("missing projectId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM projection_projects WHERE project_id = ?1",
            params![project_id],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if !exists {
        return Err(err(format!(
            "Project '{project_id}' does not exist for command 'project.delete'."
        )));
    }
    let remaining: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projection_threads \
             WHERE project_id = ?1 AND deleted_at IS NULL",
            params![project_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e.to_string()))?;
    if remaining > 0 {
        return Err(err(format!(
            "Project '{project_id}' still has {remaining} thread{} and cannot be deleted.",
            if remaining == 1 { "" } else { "s" }
        )));
    }
    let occurred_at = iso_now_ms();
    let events = vec![project_event(
        &project_id,
        "project.deleted",
        &occurred_at,
        json!({"projectId": project_id, "deletedAt": occurred_at}),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    conn.execute(
        "UPDATE projection_projects SET deleted_at = ?2, updated_at = ?2 \
         WHERE project_id = ?1",
        params![project_id, occurred_at],
    )
    .map_err(|e| err(e.to_string()))?;
    Ok(json!({ "sequence": seq }))
}

// ---- thread.checkpoint.revert / thread.conversation.rollback ----

fn checkpoint_revert(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let turn_count = command
        .get("turnCount")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let scope = s(command, "scope").unwrap_or_else(|| "thread".to_string());
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    if thread_has_in_flight(&conn, &thread_id)? {
        return Err(err(format!(
            "Thread '{thread_id}' has an active turn. Interrupt the current turn before reverting checkpoints."
        )));
    }
    if thread_has_revert_in_progress(&conn, &thread_id)? {
        return Err(err(format!(
            "Thread '{thread_id}' already has a checkpoint revert in progress."
        )));
    }
    let activity_id = uuid_v4();
    let started = out_event(
        "thread.activity-appended",
        &thread_id,
        &created_at,
        json!({
            "threadId": thread_id,
            "activity": {
                "id": activity_id,
                "tone": "info",
                "kind": "checkpoint.revert.started",
                "summary": "Checkpoint revert started",
                "payload": {"turnCount": turn_count, "scope": scope},
                "turnId": null,
                "createdAt": created_at,
            }
        }),
    );
    let mut requested = out_event(
        "thread.checkpoint-revert-requested",
        &thread_id,
        &created_at,
        json!({"threadId": thread_id, "turnCount": turn_count, "scope": scope, "createdAt": created_at}),
    );
    requested.causation_event_id = Some(started.event_id.clone());
    let events = vec![started, requested];
    let seq = commit_events(&conn, &events, &command_id)?;
    // Projection: append the started activity with the thread's next sequence.
    let next_seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM projection_thread_activities \
             WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e.to_string()))?;
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_activities \
         (activity_id, thread_id, turn_id, tone, kind, summary, payload_json, created_at, sequence) \
         VALUES (?1, ?2, NULL, 'info', 'checkpoint.revert.started', 'Checkpoint revert started', ?3, ?4, ?5)",
        params![
            activity_id,
            thread_id,
            json!({"turnCount": turn_count, "scope": scope}).to_string(),
            created_at,
            next_seq,
        ],
    )
    .map_err(|e| err(format!("activity projection: {e}")))?;
    Ok(json!({ "sequence": seq }))
}

fn conversation_rollback(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let message_id = s(command, "messageId").ok_or_else(|| err("missing messageId"))?;
    let num_turns = command
        .get("numTurns")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    if thread_has_revert_in_progress(&conn, &thread_id)? {
        return Err(err(format!(
            "Thread '{thread_id}' already has a checkpoint revert in progress."
        )));
    }
    // Read-model message order: insertion order (created_at, then id).
    let mut st = conn
        .prepare(
            "SELECT message_id, role, turn_id FROM projection_thread_messages \
             WHERE thread_id = ?1 ORDER BY created_at, message_id",
        )
        .map_err(|e| err(e.to_string()))?;
    let messages: Vec<(String, String, Option<String>)> = st
        .query_map(params![thread_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(|e| err(e.to_string()))?
        .collect::<Result<_, _>>()
        .map_err(|e| err(e.to_string()))?;
    let target_index = messages.iter().position(|(id, _, _)| id == &message_id);
    let target_index = match target_index {
        Some(i) if messages[i].1 == "user" => i,
        _ => {
            return Err(err(
                "Conversation rollback must target an existing user message.",
            ))
        }
    };
    let mut removed_turns: Vec<String> = Vec::new();
    for (_, _, turn_id) in &messages[target_index..] {
        if let Some(t) = turn_id {
            if !removed_turns.contains(t) {
                removed_turns.push(t.clone());
            }
        }
    }
    if num_turns <= 0 || removed_turns.len() as i64 != num_turns {
        return Err(err(format!(
            "Conversation rollback requested {num_turns} turn(s), but target message '{message_id}' would remove {} turn(s).",
            removed_turns.len()
        )));
    }
    let events = vec![out_event(
        "thread.conversation-rollback-requested",
        &thread_id,
        &created_at,
        json!({"threadId": thread_id, "messageId": message_id, "numTurns": num_turns, "createdAt": created_at}),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

// ---- space.* ----

const SPACES_MAX_COUNT: i64 = 50;
const RESERVED_VOID_SPACE_ID: &str = "void";

fn space_event(agg_id: &str, event_type: &'static str, occurred_at: &str, payload: Value) -> OutEvent {
    let mut ev = out_event(event_type, agg_id, occurred_at, payload);
    ev.agg_kind = "space";
    ev
}

fn require_space(conn: &Connection, space_id: &str, command_type: &str) -> Result<(), WsErr> {
    let row: Option<Option<String>> = conn
        .query_row(
            "SELECT deleted_at FROM projection_spaces WHERE space_id = ?1",
            params![space_id],
            |r| r.get(0),
        )
        .ok();
    match row {
        Some(None) => Ok(()),
        Some(Some(_)) => Err(err(format!(
            "Space '{space_id}' was deleted and cannot handle command '{command_type}'."
        ))),
        None => Err(err(format!(
            "Space '{space_id}' does not exist for command '{command_type}'."
        ))),
    }
}

fn require_space_name_available(
    conn: &Connection,
    name: &str,
    exclude: Option<&str>,
    command_type: &str,
) -> Result<(), WsErr> {
    let normalized = name.trim().to_lowercase();
    if normalized == "void" {
        return Err(err("'Void' is reserved for unassigned projects."));
    }
    let mut st = conn
        .prepare(
            "SELECT space_id, name FROM projection_spaces WHERE deleted_at IS NULL",
        )
        .map_err(|e| err(e.to_string()))?;
    let rows: Vec<(String, String)> = st
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|e| err(e.to_string()))?
        .collect::<Result<_, _>>()
        .map_err(|e| err(e.to_string()))?;
    for (sid, sname) in rows {
        if Some(sid.as_str()) != exclude && sname.trim().to_lowercase() == normalized {
            let _ = command_type;
            return Err(err(format!("A space named '{name}' already exists.")));
        }
    }
    Ok(())
}

fn space_create(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let space_id = s(command, "spaceId").ok_or_else(|| err("missing spaceId"))?;
    let name = s(command, "name").ok_or_else(|| err("missing name"))?;
    let icon = s(command, "icon").ok_or_else(|| err("missing icon"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM projection_spaces WHERE space_id = ?1",
            params![space_id],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if exists {
        return Err(err(format!(
            "Space '{space_id}' already exists and cannot be created twice."
        )));
    }
    if space_id == RESERVED_VOID_SPACE_ID {
        return Err(err(
            "The reserved Void identity cannot be used for a custom space.",
        ));
    }
    require_space_name_available(&conn, &name, None, "space.create")?;
    let (count, max_order): (i64, Option<i64>) = conn
        .query_row(
            "SELECT COUNT(*), MAX(sort_order) FROM projection_spaces WHERE deleted_at IS NULL",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| err(e.to_string()))?;
    if count >= SPACES_MAX_COUNT {
        return Err(err(format!(
            "A maximum of {SPACES_MAX_COUNT} custom spaces is supported."
        )));
    }
    let sort_order = max_order.unwrap_or(-1) + 1;
    let events = vec![space_event(
        &space_id,
        "space.created",
        &created_at,
        json!({"spaceId": space_id, "name": name, "icon": icon, "sortOrder": sort_order, "createdAt": created_at, "updatedAt": created_at}),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    conn.execute(
        "INSERT INTO projection_spaces \
         (space_id, name, icon, sort_order, created_at, updated_at, deleted_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?5, NULL)",
        params![space_id, name, icon, sort_order, created_at],
    )
    .map_err(|e| err(format!("space projection insert: {e}")))?;
    Ok(json!({ "sequence": seq }))
}

fn space_meta_update(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let space_id = s(command, "spaceId").ok_or_else(|| err("missing spaceId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_space(&conn, &space_id, "space.meta.update")?;
    let (cur_name, cur_icon): (String, String) = conn
        .query_row(
            "SELECT name, icon FROM projection_spaces WHERE space_id = ?1",
            params![space_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|e| err(e.to_string()))?;
    let next_name = s(command, "name").filter(|n| n != &cur_name);
    let next_icon = s(command, "icon").filter(|i| i != &cur_icon);
    if next_name.is_none() && next_icon.is_none() {
        return Err(err("Space metadata update must change a name or icon."));
    }
    if let Some(n) = &next_name {
        require_space_name_available(&conn, n, Some(&space_id), "space.meta.update")?;
    }
    let occurred_at = iso_now_ms();
    let mut payload = Map::new();
    payload.insert("spaceId".into(), json!(space_id));
    if let Some(n) = &next_name {
        payload.insert("name".into(), json!(n));
    }
    if let Some(i) = &next_icon {
        payload.insert("icon".into(), json!(i));
    }
    payload.insert("updatedAt".into(), json!(occurred_at));
    let events = vec![space_event(
        &space_id,
        "space.meta-updated",
        &occurred_at,
        Value::Object(payload),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    let mut sets: Vec<(&str, Value)> = vec![("updated_at", json!(occurred_at))];
    if let Some(n) = next_name {
        sets.push(("name", json!(n)));
    }
    if let Some(i) = next_icon {
        sets.push(("icon", json!(i)));
    }
    let assignments: Vec<String> = sets
        .iter()
        .enumerate()
        .map(|(i, (c, _))| format!("{c} = ?{}", i + 2))
        .collect();
    let mut params_vec: Vec<rusqlite::types::Value> =
        vec![rusqlite::types::Value::Text(space_id.clone())];
    for (_, v) in &sets {
        params_vec.push(json_to_sql(v));
    }
    conn.execute(
        &format!(
            "UPDATE projection_spaces SET {} WHERE space_id = ?1",
            assignments.join(", ")
        ),
        rusqlite::params_from_iter(params_vec),
    )
    .map_err(|e| err(e.to_string()))?;
    Ok(json!({ "sequence": seq }))
}

fn space_reorder(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let space_id = s(command, "spaceId").ok_or_else(|| err("missing spaceId"))?;
    let ordered: Vec<String> = command
        .get("orderedSpaceIds")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_space(&conn, &space_id, "space.reorder")?;
    let mut st = conn
        .prepare("SELECT space_id FROM projection_spaces WHERE deleted_at IS NULL")
        .map_err(|e| err(e.to_string()))?;
    let active: Vec<String> = st
        .query_map([], |r| r.get(0))
        .map_err(|e| err(e.to_string()))?
        .collect::<Result<_, _>>()
        .map_err(|e| err(e.to_string()))?;
    let unique: std::collections::HashSet<&String> = ordered.iter().collect();
    let exact = ordered.len() == active.len()
        && unique.len() == active.len()
        && active.iter().all(|id| unique.contains(id));
    if !exact {
        return Err(err(
            "Space order must contain every active custom space exactly once.",
        ));
    }
    let occurred_at = iso_now_ms();
    let events = vec![space_event(
        &space_id,
        "space.order-updated",
        &occurred_at,
        json!({"spaceId": space_id, "orderedSpaceIds": ordered, "updatedAt": occurred_at}),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    for (i, sid) in ordered.iter().enumerate() {
        conn.execute(
            "UPDATE projection_spaces SET sort_order = ?2, updated_at = ?3 \
             WHERE space_id = ?1 AND sort_order != ?2",
            params![sid, i as i64, occurred_at],
        )
        .map_err(|e| err(e.to_string()))?;
    }
    Ok(json!({ "sequence": seq }))
}

fn space_delete(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let space_id = s(command, "spaceId").ok_or_else(|| err("missing spaceId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_space(&conn, &space_id, "space.delete")?;
    let occurred_at = iso_now_ms();
    let events = vec![space_event(
        &space_id,
        "space.deleted",
        &occurred_at,
        json!({"spaceId": space_id, "deletedAt": occurred_at}),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    conn.execute(
        "UPDATE projection_spaces SET deleted_at = ?2, updated_at = ?2 WHERE space_id = ?1",
        params![space_id, occurred_at],
    )
    .map_err(|e| err(e.to_string()))?;
    // The deletion event owns the re-filing invariant: clear every assignment in
    // one pass, including soft-deleted projects.
    conn.execute(
        "UPDATE projection_projects SET space_id = NULL WHERE space_id = ?1",
        params![space_id],
    )
    .map_err(|e| err(e.to_string()))?;
    Ok(json!({ "sequence": seq }))
}

fn space_projects_assign(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let space_id = s(command, "spaceId").ok_or_else(|| err("missing spaceId"))?;
    let project_ids: Vec<String> = command
        .get("projectIds")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_space(&conn, &space_id, "space.projects.assign")?;
    let occurred_at = iso_now_ms();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut events: Vec<OutEvent> = Vec::new();
    for pid in project_ids {
        if !seen.insert(pid.clone()) {
            continue;
        }
        let row: Option<(Option<String>, String, String, Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT kind, title, workspace_root, space_id, deleted_at \
                 FROM projection_projects WHERE project_id = ?1",
                params![pid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            )
            .ok();
        let (kind, title, root, cur_space, deleted_at) = row.ok_or_else(|| {
            err(format!(
                "Project '{pid}' does not exist for command 'space.projects.assign'."
            ))
        })?;
        if deleted_at.is_some() || cur_space.as_deref() == Some(space_id.as_str()) {
            continue;
        }
        if kind.as_deref().unwrap_or("project") != "project" {
            return Err(err("Only ordinary projects can be assigned to a space."));
        }
        if is_legacy_home_row(&title, &root) {
            return Err(err(
                "The Chats container is reachable from every space and cannot be assigned to one.",
            ));
        }
        events.push(project_event(
            &pid,
            "project.meta-updated",
            &occurred_at,
            json!({"projectId": pid, "spaceId": space_id, "updatedAt": occurred_at}),
        ));
    }
    if events.is_empty() {
        return Err(err(
            "None of the selected projects need to be assigned to this space.",
        ));
    }
    let assigned: Vec<String> = events.iter().map(|e| e.thread_id.clone()).collect();
    let seq = commit_events(&conn, &events, &command_id)?;
    for pid in assigned {
        conn.execute(
            "UPDATE projection_projects SET space_id = ?2, updated_at = ?3 WHERE project_id = ?1",
            params![pid, space_id, occurred_at],
        )
        .map_err(|e| err(e.to_string()))?;
    }
    Ok(json!({ "sequence": seq }))
}

// ---- edit-and-resend / session.* / goal.continue / turn.resume ----

fn require_thread_not_archived(
    conn: &Connection,
    thread_id: &str,
    command_type: &str,
) -> Result<(), WsErr> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| err(e.to_string()))?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(err(format!(
            "Thread '{thread_id}' does not exist for command '{command_type}'."
        )))
    }
}

fn sidechat_expired(conn: &Connection, thread_id: &str) -> bool {
    conn.query_row(
        "SELECT sidechat_expired_at IS NOT NULL FROM projection_threads WHERE thread_id = ?1",
        params![thread_id],
        |r| r.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

fn require_sidechat_live(conn: &Connection, thread_id: &str) -> Result<(), WsErr> {
    if sidechat_expired(conn, thread_id) {
        return Err(err(
            "This side chat expired after a period of inactivity. Start a new side chat.",
        ));
    }
    Ok(())
}

fn auto_runtime_issue(runtime_mode: &str, model_sel: &Value) -> Option<String> {
    if runtime_mode != "auto" {
        return None;
    }
    let provider = model_sel
        .get("provider")
        .and_then(Value::as_str)
        .unwrap_or("");
    let supported = provider == "codex" || provider == "claudeAgent";
    if !supported {
        return Some(format!(
            "Provider \"{provider}\" does not support Auto runtime mode. Auto is available only for Codex and Claude Code."
        ));
    }
    if provider == "claudeAgent"
        && model_sel.get("supportsAutoMode").and_then(Value::as_bool) != Some(true)
    {
        let model = model_sel.get("model").and_then(Value::as_str).unwrap_or("");
        return Some(
            if model_sel.get("supportsAutoMode").and_then(Value::as_bool) == Some(false) {
                format!("Claude model \"{model}\" does not support Auto mode.")
            } else {
                format!(
                    "Claude model \"{model}\" has not been verified to support Auto mode."
                )
            },
        );
    }
    None
}

fn thread_model_selection(conn: &Connection, thread_id: &str) -> Value {
    conn.query_row(
        "SELECT model_selection_json FROM projection_threads WHERE thread_id = ?1",
        params![thread_id],
        |r| r.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
    .and_then(|j| serde_json::from_str(&j).ok())
    .unwrap_or(Value::Null)
}

fn session_row(conn: &Connection, thread_id: &str) -> Option<(String, Option<String>, Option<String>)> {
    conn.query_row(
        "SELECT status, active_turn_id, provider_name FROM projection_thread_sessions \
         WHERE thread_id = ?1",
        params![thread_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .ok()
}

fn message_edit_resend(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let message_id = s(command, "messageId").ok_or_else(|| err("missing messageId"))?;
    let text = s(command, "text").ok_or_else(|| err("missing text"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    require_sidechat_live(&conn, &thread_id)?;
    if thread_has_revert_in_progress(&conn, &thread_id)? {
        return Err(err(format!(
            "Thread '{thread_id}' already has a checkpoint revert in progress."
        )));
    }
    let runtime_mode = s(command, "runtimeMode").unwrap_or_else(|| "full-access".into());
    let model_sel = command
        .get("modelSelection")
        .cloned()
        .filter(|v| !v.is_null())
        .unwrap_or_else(|| thread_model_selection(&conn, &thread_id));
    if let Some(issue) = auto_runtime_issue(&runtime_mode, &model_sel) {
        return Err(err(issue));
    }
    let mut st = conn
        .prepare(
            "SELECT message_id, role, turn_id, source, async_user_input_json \
             FROM projection_thread_messages WHERE thread_id = ?1 \
             ORDER BY created_at, message_id",
        )
        .map_err(|e| err(e.to_string()))?;
    let messages: Vec<(String, String, Option<String>, String, Option<String>)> = st
        .query_map(params![thread_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .map_err(|e| err(e.to_string()))?
        .collect::<Result<_, _>>()
        .map_err(|e| err(e.to_string()))?;
    let session = session_row(&conn, &thread_id);
    let active_turn_id: Option<String> = session
        .as_ref()
        .filter(|(status, _, _)| status == "running")
        .and_then(|(_, t, _)| t.clone());

    // resolveTailUserMessageEditTarget
    let idx = messages.iter().position(|(id, ..)| id == &message_id);
    let reject = |reason: &str| -> WsErr {
        err(format!(
            "Only the latest rollbackable user message can be edited and resent ({reason})."
        ))
    };
    let i = match idx {
        None => return Err(reject("missing-message")),
        Some(i) => i,
    };
    let (_, role, _, source, _) = &messages[i];
    if role != "user" {
        return Err(reject("not-user-message"));
    }
    if source != "native" {
        return Err(reject("non-native-message"));
    }
    let answered = messages.iter().any(|(_, _, _, _, aui)| {
        aui.as_deref()
            .and_then(|j| serde_json::from_str::<Value>(j).ok())
            .and_then(|v| {
                v.get("response")
                    .and_then(|r| r.get("messageId"))
                    .and_then(Value::as_str)
                    .map(String::from)
            })
            .as_deref()
            == Some(message_id.as_str())
    });
    if answered {
        return Err(reject("structured-answer"));
    }
    let latest_native = messages.iter().rposition(|(_, role, _, src, _)| {
        role == "user" && (src == "native" || src.is_empty())
    });
    if latest_native != Some(i) {
        return Err(reject("not-latest-native-user-message"));
    }
    let mut removed_turns: Vec<String> = Vec::new();
    for (_, _, tid, _, _) in &messages[i..] {
        if let Some(t) = tid {
            if !removed_turns.contains(t) {
                removed_turns.push(t.clone());
            }
        }
    }
    if removed_turns.len() > 1 {
        return Err(reject("spans-multiple-turns"));
    }
    let rollback_count = removed_turns.len() as i64;
    if removed_turns.is_empty() && active_turn_id.is_none() {
        return Err(reject("missing-turn-metadata"));
    }
    let removed: Vec<String> = if removed_turns.is_empty() {
        Vec::new()
    } else {
        removed_turns
    };

    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    payload.insert("messageId".into(), json!(message_id));
    payload.insert("text".into(), json!(text));
    payload.insert("rollbackTurnCount".into(), json!(rollback_count));
    payload.insert("removedTurnIds".into(), json!(removed));
    for k in ["modelSelection", "providerOptions", "assistantDeliveryMode"] {
        if let Some(v) = command.get(k) {
            payload.insert(k.to_string(), v.clone());
        }
    }
    // computerActivationMetadata passthrough fields
    for k in ["enableComputerControl", "computerControlMode", "computerControlGeneration"] {
        if let Some(v) = command.get(k) {
            payload.insert(k.to_string(), v.clone());
        }
    }
    payload.insert(
        "runtimeMode".into(),
        command
            .get("runtimeMode")
            .cloned()
            .unwrap_or(Value::Null),
    );
    payload.insert(
        "interactionMode".into(),
        command
            .get("interactionMode")
            .cloned()
            .unwrap_or(Value::Null),
    );
    payload.insert("createdAt".into(), json!(created_at));
    let mut requested = out_event(
        "thread.message-edit-resend-requested",
        &thread_id,
        &created_at,
        Value::Object(payload),
    );
    let session_status = session.as_ref().map(|(st, _, _)| st.as_str()).unwrap_or("");
    let mut events: Vec<OutEvent> = Vec::new();
    if session_status == "starting" || session_status == "running" {
        events.push(requested);
    } else {
        let provider = session
            .as_ref()
            .and_then(|(_, _, p)| p.clone())
            .or_else(|| {
                model_sel
                    .get("provider")
                    .and_then(Value::as_str)
                    .map(String::from)
            });
        let starting = out_event(
            "thread.session-set",
            &thread_id,
            &created_at,
            json!({
                "threadId": thread_id,
                "session": {
                    "threadId": thread_id,
                    "status": "starting",
                    "providerName": provider,
                    "runtimeMode": command.get("runtimeMode").cloned().unwrap_or(Value::Null),
                    "activeTurnId": null,
                    "lastError": null,
                    "updatedAt": created_at,
                }
            }),
        );
        requested.causation_event_id = Some(starting.event_id.clone());
        events.push(starting.clone_shallow());
        events.push(requested);
    }
    let seq = commit_events(&conn, &events, &command_id)?;
    // Session projection when we wrote the starting event.
    if events.len() == 2 {
        let provider = events[0]
            .payload
            .get("session")
            .and_then(|s| s.get("providerName"))
            .cloned()
            .unwrap_or(Value::Null);
        conn.execute(
            "INSERT OR REPLACE INTO projection_thread_sessions \
             (thread_id, status, provider_name, provider_session_id, provider_thread_id, \
              active_turn_id, last_error, updated_at, runtime_mode) \
             VALUES (?1, 'starting', ?2, NULL, NULL, NULL, NULL, ?3, ?4)",
            params![
                thread_id,
                if provider.is_null() {
                    None
                } else {
                    provider.as_str().map(String::from)
                },
                created_at,
                s(command, "runtimeMode").unwrap_or_else(|| "full-access".into())
            ],
        )
        .map_err(|e| err(e.to_string()))?;
    }
    Ok(json!({ "sequence": seq }))
}

fn session_stop(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let thread = require_thread(&conn, &thread_id)?;
    let stop = out_event(
        "thread.session-stop-requested",
        &thread_id,
        &created_at,
        json!({"threadId": thread_id, "createdAt": created_at}),
    );
    let mut events: Vec<OutEvent> = Vec::new();
    if thread.claude_cache_review.is_some() {
        let mut cache = out_event(
            "thread.claude-cache-set",
            &thread_id,
            &created_at,
            json!({"threadId": thread_id, "review": null, "updatedAt": created_at}),
        );
        cache.causation_event_id = None;
        events.push(cache);
    }
    let mut stop_ev = stop;
    if let Some(first) = events.last() {
        stop_ev.causation_event_id = Some(first.event_id.clone());
    }
    events.push(stop_ev);
    let had_review = events.len() == 2;
    let seq = commit_events(&conn, &events, &command_id)?;
    if had_review {
        update_projection(
            &conn,
            &thread_id,
            &[("claude_cache_review_json", Value::Null), ("updated_at", json!(created_at))],
        )?;
    }
    Ok(json!({ "sequence": seq }))
}

fn session_set(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    if command.get("expectedSessionStatus").is_some()
        || command.get("expectedSessionUpdatedAt").is_some()
    {
        let (cur_status, cur_updated): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT status, updated_at FROM projection_thread_sessions WHERE thread_id = ?1",
                params![thread_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap_or((None, None));
        let changed = (command.get("expectedSessionStatus").is_some()
            && command.get("expectedSessionStatus").and_then(Value::as_str)
                != cur_status.as_deref())
            || (command.get("expectedSessionUpdatedAt").is_some()
                && command.get("expectedSessionUpdatedAt").and_then(Value::as_str)
                    != cur_updated.as_deref());
        if changed {
            return Err(err(format!(
                "Thread '{thread_id}' session changed before the conditional update."
            )));
        }
    }
    let session = command
        .get("session")
        .cloned()
        .unwrap_or(Value::Null);
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    payload.insert("session".into(), session.clone());
    if session.get("status").and_then(Value::as_str) == Some("starting") {
        if let Some(v) = command.get("pendingResumeMessageId") {
            payload.insert("pendingResumeMessageId".into(), v.clone());
        }
    }
    let events = vec![out_event(
        "thread.session-set",
        &thread_id,
        &created_at,
        Value::Object(payload),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    // Projection: upsert the session row.
    let g = |k: &str| session.get(k).cloned().unwrap_or(Value::Null);
    conn.execute(
        "INSERT INTO projection_thread_sessions \
         (thread_id, status, provider_name, provider_session_id, provider_thread_id, \
          active_turn_id, last_error, updated_at, runtime_mode, provider_instance_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
         ON CONFLICT(thread_id) DO UPDATE SET \
          status=excluded.status, provider_name=excluded.provider_name, \
          provider_session_id=excluded.provider_session_id, \
          provider_thread_id=excluded.provider_thread_id, \
          active_turn_id=excluded.active_turn_id, last_error=excluded.last_error, \
          updated_at=excluded.updated_at, runtime_mode=excluded.runtime_mode, \
          provider_instance_id=excluded.provider_instance_id",
        params![
            thread_id,
            json_to_sql(&g("status")),
            json_to_sql(&g("providerName")),
            json_to_sql(&g("providerSessionId")),
            json_to_sql(&g("providerThreadId")),
            json_to_sql(&g("activeTurnId")),
            json_to_sql(&g("lastError")),
            json_to_sql(&g("updatedAt")),
            json_to_sql(&g("runtimeMode")),
            json_to_sql(&g("providerInstanceId")),
        ],
    )
    .map_err(|e| err(format!("session projection: {e}")))?;
    Ok(json!({ "sequence": seq }))
}

fn goal_continue(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    require_sidechat_live(&conn, &thread_id)?;
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    payload.insert(
        "goalStartedAt".into(),
        command.get("goalStartedAt").cloned().unwrap_or(Value::Null),
    );
    payload.insert(
        "trigger".into(),
        command.get("trigger").cloned().unwrap_or(Value::Null),
    );
    if let Some(v) = command.get("sourceTurnId") {
        payload.insert("sourceTurnId".into(), v.clone());
    }
    payload.insert("createdAt".into(), json!(created_at));
    let events = vec![out_event(
        "thread.goal-continuation-requested",
        &thread_id,
        &created_at,
        Value::Object(payload),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

fn turn_resume(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread_not_archived(&conn, &thread_id, "thread.turn.resume")?;
    require_sidechat_live(&conn, &thread_id)?;

    // Latest turn (by requested_at desc) for precondition + sourceTurnId.
    let latest_turn: Option<(String, String, String, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT turn_id, state, requested_at, started_at, completed_at \
             FROM projection_turns WHERE thread_id = ?1 \
             ORDER BY requested_at DESC LIMIT 1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .ok();
    if let Some(pre) = command.get("resumePrecondition") {
        let recorded_turn = pre.get("recordedTurnId").and_then(Value::as_str);
        let recorded_at = pre.get("recordedAt").and_then(Value::as_str).unwrap_or("");
        let violation: Option<&str> = if thread_has_in_flight(&conn, &thread_id)? {
            Some("turn-in-flight")
        } else if let Some((tid, state, req, started, completed)) = &latest_turn {
            if state == "completed"
                && (Some(tid.as_str()) == recorded_turn
                    || completed.is_none()
                    || completed.as_deref().unwrap_or("") >= recorded_at)
            {
                Some("turn-completed")
            } else if state != "completed"
                && Some(tid.as_str()) != recorded_turn
                && (req.as_str() >= recorded_at
                    || (recorded_turn.is_some()
                        && started.as_deref().unwrap_or("") >= recorded_at))
            {
                Some("turn-replaced")
            } else {
                None
            }
        } else {
            None
        };
        if let Some(v) = violation {
            return Err(err(format!(
                "Thread '{thread_id}' resume precondition failed: {v}."
            )));
        }
    }
    let refusal = if thread_has_in_flight(&conn, &thread_id)? {
        Some(format!("Thread '{thread_id}' already has a turn in flight."))
    } else if thread_has_revert_in_progress(&conn, &thread_id)? {
        Some(format!(
            "Thread '{thread_id}' already has a checkpoint revert in progress."
        ))
    } else {
        let (approvals, inputs, recovery, creation_source): (i64, i64, Option<String>, Option<String>) =
            conn.query_row(
                "SELECT pending_approval_count, pending_user_input_count, \
                        usage_limit_recovery_json, creation_source \
                 FROM projection_threads WHERE thread_id = ?1",
                params![thread_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(|e| err(e.to_string()))?;
        if approvals > 0 || inputs > 0 {
            Some(format!("Thread '{thread_id}' is waiting for an answer."))
        } else if recovery.is_some() && recovery.as_deref() != Some("null") {
            Some(format!(
                "Thread '{thread_id}' resumes on its own after the usage limit."
            ))
        } else if creation_source.as_deref() == Some("provider_native")
            || thread_id.starts_with("subagent:")
        {
            Some(format!("Subagent '{thread_id}' cannot be resumed."))
        } else {
            None
        }
    };
    if let Some(r) = refusal {
        return Err(err(r));
    }
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    payload.insert("goalStartedAt".into(), Value::Null);
    payload.insert("trigger".into(), json!("manual-resume"));
    if let Some((tid, ..)) = &latest_turn {
        payload.insert("sourceTurnId".into(), json!(tid));
    }
    if let Some(v) = command.get("continuationPrompt") {
        payload.insert("resumePrompt".into(), v.clone());
    }
    if let Some(v) = command.get("computerControlGeneration") {
        payload.insert("resumeComputerControlGeneration".into(), v.clone());
    }
    payload.insert("createdAt".into(), json!(created_at));
    let events = vec![out_event(
        "thread.goal-continuation-requested",
        &thread_id,
        &created_at,
        Value::Object(payload),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

// ---- thread.pull-request.* ----

const PR_LINK_LIMIT: usize = 50;

fn pr_link_key(url: &str) -> String {
    let trimmed = url.trim();
    // normalizeGitHubPullRequestUrl
    if let Some(rest) = trimmed
        .strip_prefix("https://github.com/")
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
    {
        let mut parts = rest.split('/');
        if let (Some(owner), Some(repo), Some(pull_tag), Some(num)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        {
            let num: String = num
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if pull_tag == "pull"
                && !num.is_empty()
                && num.len() == num.chars().take_while(|c| c.is_ascii_digit()).count()
                && !owner.is_empty()
                && !repo.is_empty()
                && !owner.contains(char::is_whitespace)
                && !repo.contains(char::is_whitespace)
            {
                // valid only when nothing but optional ?/#/path suffix follows the number
                let tail = &rest[rest.find(num.as_str()).map(|i| i + num.len()).unwrap_or(rest.len())..];
                if tail.is_empty()
                    || tail.starts_with('/')
                    || tail.starts_with('?')
                    || tail.starts_with('#')
                {
                    return format!(
                        "https://github.com/{}/{}/pull/{}",
                        owner.to_lowercase(),
                        repo.to_lowercase(),
                        num
                    );
                }
            }
        }
    }
    trimmed
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .trim_end_matches('/')
        .to_lowercase()
}

fn pr_is_visible(link: &Value) -> bool {
    link.get("dismissedAt").is_none() || link.get("dismissedAt").map_or(true, Value::is_null)
}

fn pr_is_explicit_source(source: &str) -> bool {
    matches!(source, "created" | "agent" | "user" | "checkout")
}

fn pr_source(link: &Value) -> &str {
    link.get("source").and_then(Value::as_str).unwrap_or("")
}

fn pr_state(link: &Value) -> &str {
    link.pointer("/pullRequest/state")
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn pr_url(link: &Value) -> &str {
    link.pointer("/pullRequest/url")
        .and_then(Value::as_str)
        .unwrap_or("")
}

fn pr_linked_at(link: &Value) -> &str {
    link.get("linkedAt").and_then(Value::as_str).unwrap_or("")
}

fn resolve_current_pr_link(links: &[Value]) -> Option<Value> {
    let mut current: Option<&Value> = None;
    for link in links {
        if !pr_is_visible(link) {
            continue;
        }
        match current {
            None => current = Some(link),
            Some(cur) => {
                let is_open = pr_state(link) == "open";
                let cur_open = pr_state(cur) == "open";
                if is_open != cur_open {
                    if is_open {
                        current = Some(link);
                    }
                } else if pr_linked_at(link) >= pr_linked_at(cur) {
                    current = Some(link);
                }
            }
        }
    }
    current.cloned()
}

fn cap_pr_links(links: Vec<Value>) -> Vec<Value> {
    if links.len() <= PR_LINK_LIMIT {
        return links;
    }
    let current = resolve_current_pr_link(&links);
    let rank = |l: &Value| {
        if !pr_is_visible(l) {
            0
        } else if pr_state(l) != "open" {
            1
        } else {
            2
        }
    };
    let current_pos = current
        .as_ref()
        .and_then(|cur| links.iter().position(|l| l == cur));
    let mut removable: Vec<usize> = (0..links.len())
        .filter(|i| Some(*i) != current_pos)
        .collect();
    removable.sort_by(|a, b| {
        rank(&links[*a])
            .cmp(&rank(&links[*b]))
            .then(pr_linked_at(&links[*a]).cmp(pr_linked_at(&links[*b])))
    });
    let drop_n = links.len() - PR_LINK_LIMIT;
    let dropped: std::collections::HashSet<usize> =
        removable.iter().take(drop_n).copied().collect();
    links
        .into_iter()
        .enumerate()
        .filter(|(i, _)| !dropped.contains(i))
        .map(|(_, l)| l)
        .collect()
}

fn pr_index_of(links: &[Value], url: &str) -> Option<usize> {
    let key = pr_link_key(url);
    links.iter().position(|l| pr_link_key(pr_url(l)) == key)
}

fn pr_find<'a>(links: &'a [Value], url: &str) -> Option<&'a Value> {
    pr_index_of(links, url).map(|i| &links[i])
}

fn link_pull_request(links: Vec<Value>, pr: Value, source: &str, linked_at: &str) -> Vec<Value> {
    let fresh = json!({"pullRequest": pr, "source": source, "linkedAt": linked_at});
    match pr_index_of(&links, pr.get("url").and_then(Value::as_str).unwrap_or("")) {
        None => cap_pr_links(
            links.iter().cloned().chain([fresh]).collect::<Vec<_>>(),
        ),
        Some(i) => {
            let mut next = links;
            let existing = &next[i];
            next[i] = if !pr_is_visible(existing) {
                fresh
            } else {
                json!({
                    "pullRequest": pr,
                    "source": if pr_is_explicit_source(pr_source(existing)) {
                        pr_source(existing)
                    } else {
                        source
                    },
                    "linkedAt": pr_linked_at(existing),
                })
            };
            next
        }
    }
}

fn unlink_pull_request(links: &[Value], url: &str, dismissed_at: &str) -> Option<Vec<Value>> {
    let i = pr_index_of(links, url)?;
    let existing = &links[i];
    if !pr_is_visible(existing) {
        return None;
    }
    let mut next = links.to_vec();
    let mut tombstone = existing.clone();
    tombstone["dismissedAt"] = json!(dismissed_at);
    next[i] = tombstone;
    Some(next)
}

fn sync_discovered_pr(links: &[Value], pr: &Value, observed_at: &str) -> Vec<Value> {
    let key = if pr.is_null() {
        None
    } else {
        Some(pr_link_key(pr.get("url").and_then(Value::as_str).unwrap_or("")))
    };
    let mut kept: Vec<Value> = links
        .iter()
        .filter(|l| {
            !pr_is_visible(l)
                || pr_is_explicit_source(pr_source(l))
                || Some(pr_link_key(pr_url(l))) == key
        })
        .cloned()
        .collect();
    if pr.is_null() {
        return kept;
    }
    match pr_index_of(&kept, pr.get("url").and_then(Value::as_str).unwrap_or("")) {
        None => {
            kept.push(json!({"pullRequest": pr, "source": "discovered", "linkedAt": observed_at}));
            cap_pr_links(kept)
        }
        Some(i) => {
            if !pr_is_visible(&kept[i]) {
                kept
            } else {
                kept[i]["pullRequest"] = pr.clone();
                kept
            }
        }
    }
}

fn refresh_pr_snapshots(links: &[Value], prs: &[Value]) -> Vec<Value> {
    links
        .iter()
        .map(|l| {
            if !pr_is_visible(l) {
                return l.clone();
            }
            let key = pr_link_key(pr_url(l));
            match prs
                .iter()
                .find(|p| pr_link_key(p.get("url").and_then(Value::as_str).unwrap_or("")) == key)
            {
                Some(fresh) => {
                    let mut next = l.clone();
                    next["pullRequest"] = fresh.clone();
                    next
                }
                None => l.clone(),
            }
        })
        .collect()
}

fn pr_pull_requests_equal(a: &Value, b: &Value) -> bool {
    if a == b {
        return true;
    }
    let g = |v: &Value, k: &str| v.get(k).cloned().unwrap_or(Value::Null);
    for k in [
        "number",
        "title",
        "url",
        "baseBranch",
        "headBranch",
        "state",
    ] {
        if g(a, k) != g(b, k) {
            return false;
        }
    }
    if g(a, "isDraft").as_bool().unwrap_or(false) != g(b, "isDraft").as_bool().unwrap_or(false)
        || g(a, "mergeability").as_str().unwrap_or("unknown")
            != g(b, "mergeability").as_str().unwrap_or("unknown")
    {
        return false;
    }
    for k in ["additions", "deletions", "changedFiles"] {
        if g(a, k) != g(b, k) {
            return false;
        }
    }
    true
}

fn pr_links_equal(left: &[Value], right: &[Value]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter().zip(right.iter()).all(|(a, b)| {
        pr_source(a) == pr_source(b)
            && pr_linked_at(a) == pr_linked_at(b)
            && a.get("dismissedAt").cloned().unwrap_or(Value::Null)
                == b.get("dismissedAt").cloned().unwrap_or(Value::Null)
            && pr_pull_requests_equal(
                a.get("pullRequest").unwrap_or(&Value::Null),
                b.get("pullRequest").unwrap_or(&Value::Null),
            )
    })
}

fn thread_pr_links(conn: &Connection, thread_id: &str, occurred_at: &str) -> Vec<Value> {
    let (links_json, last_pr): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT pull_requests_json, last_known_pr_json FROM projection_threads \
             WHERE thread_id = ?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((None, None));
    if let Some(j) = links_json {
        if !j.is_empty() && j != "null" {
            return serde_json::from_str(&j).unwrap_or_default();
        }
    }
    match last_pr.and_then(|j| serde_json::from_str::<Value>(&j).ok()) {
        Some(pr) if !pr.is_null() => {
            vec![json!({"pullRequest": pr, "source": "legacy", "linkedAt": occurred_at})]
        }
        _ => Vec::new(),
    }
}

fn pr_links_dispatch(
    conn: &Connection,
    command: &Map<String, Value>,
    next_links: Vec<Value>,
    occurred_at: &str,
) -> Result<Vec<OutEvent>, WsErr> {
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let current = thread_pr_links(conn, &thread_id, occurred_at);
    if pr_links_equal(&current, &next_links) {
        return Ok(Vec::new());
    }
    let last_known = resolve_current_pr_link(&next_links)
        .map(|l| l.get("pullRequest").cloned().unwrap_or(Value::Null))
        .unwrap_or(Value::Null);
    update_projection(
        conn,
        &thread_id,
        &[
            ("pull_requests_json", json!(next_links)),
            ("last_known_pr_json", last_known.clone()),
            ("updated_at", json!(occurred_at)),
        ],
    )?;
    Ok(vec![out_event(
        "thread.meta-updated",
        &thread_id,
        occurred_at,
        json!({
            "threadId": thread_id,
            "pullRequests": next_links,
            "lastKnownPr": last_known,
            "updatedAt": occurred_at,
        }),
    )])
}

fn pull_request_command(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let command_type = s(command, "type").unwrap_or_default();
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let occurred_at = iso_now_ms();
    let links = thread_pr_links(&conn, &thread_id, &occurred_at);
    let next_links = match command_type.as_str() {
        "thread.pull-request.link" => {
            let pr = command.get("pullRequest").cloned().unwrap_or(Value::Null);
            let source = s(command, "source").unwrap_or_else(|| "user".into());
            if source == "created" {
                if let Some(existing) = pr_find(
                    &links,
                    pr.get("url").and_then(Value::as_str).unwrap_or(""),
                ) {
                    if !pr_is_visible(existing) {
                        return Err(err("Command produced no events."));
                    }
                }
            }
            link_pull_request(links, pr, &source, &occurred_at)
        }
        "thread.pull-request.unlink" => {
            let url = s(command, "url").ok_or_else(|| err("missing url"))?;
            match unlink_pull_request(&links, &url, &occurred_at) {
                None => return Err(err("Command produced no events.")),
                Some(next) => next,
            }
        }
        "thread.pull-request.sync" => {
            let mut next = links;
            if let Some(discovered) = command.get("discovered") {
                next = sync_discovered_pr(&next, discovered, &occurred_at);
            }
            if let Some(snapshots) = command.get("snapshots").and_then(Value::as_array) {
                next = refresh_pr_snapshots(&next, snapshots);
            }
            next
        }
        _ => return Err(err(format!("unsupported pull-request command {command_type}"))),
    };
    let events = pr_links_dispatch(&conn, command, next_links, &occurred_at)?;
    if events.is_empty() {
        return Err(err("Command produced no events."));
    }
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

// ---- claude-cache.* / dispatch-queued / proposed-plan / diff+revert complete ----

fn append_activity_projection(
    conn: &Connection,
    thread_id: &str,
    activity: &Value,
    event_sequence: i64,
) -> Result<(), WsErr> {
    let activity_id = activity
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let sequence = match activity.get("sequence").and_then(Value::as_i64) {
        Some(n) => n,
        None => event_sequence,
    };
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_activities \
         (activity_id, thread_id, turn_id, tone, kind, summary, payload_json, created_at, sequence) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            activity_id,
            thread_id,
            json_to_sql(&activity.get("turnId").cloned().unwrap_or(Value::Null)),
            json_to_sql(&activity.get("tone").cloned().unwrap_or(json!("info"))),
            json_to_sql(&activity.get("kind").cloned().unwrap_or(Value::Null)),
            json_to_sql(&activity.get("summary").cloned().unwrap_or(Value::Null)),
            activity.get("payload").cloned().unwrap_or(Value::Null).to_string(),
            json_to_sql(&activity.get("createdAt").cloned().unwrap_or(Value::Null)),
            sequence,
        ],
    )
    .map_err(|e| err(format!("activity projection: {e}")))?;
    Ok(())
}

fn revert_succeeded_activity(command_id: &str, thread_id: &str, turn_count: i64, created_at: &str, causation: &str) -> OutEvent {
    let mut ev = out_event(
        "thread.activity-appended",
        thread_id,
        created_at,
        json!({
            "threadId": thread_id,
            "activity": {
                "id": uuid_v4(),
                "tone": "info",
                "kind": "checkpoint.revert.succeeded",
                "summary": "Checkpoint revert completed",
                "payload": {"turnCount": turn_count},
                "turnId": null,
                "createdAt": created_at,
            }
        }),
    );
    ev.causation_event_id = Some(causation.to_string());
    let _ = command_id;
    ev
}

fn claude_cache_set(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let thread = require_thread(&conn, &thread_id)?;
    let hold = command.get("hold").cloned().unwrap_or(Value::Null);
    if !hold.is_null() {
        let hold_session = hold.get("session").cloned().unwrap_or(Value::Null);
        let review = command.get("review").cloned().unwrap_or(Value::Null);
        let review_valid = if review.is_null() {
            true
        } else {
            let status_ok = review.get("status").and_then(Value::as_str) == Some("pending");
            let msg_id = review.get("messageId").and_then(Value::as_str).unwrap_or("");
            let msg_exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) FROM projection_thread_messages \
                     WHERE thread_id = ?1 AND message_id = ?2 AND role = 'user'",
                    params![thread_id, msg_id],
                    |r| r.get::<_, i64>(0),
                )
                .unwrap_or(0)
                    > 0;
            status_ok && msg_exists
        };
        let rejected = thread.deleted_at.is_some()
            || thread.archived_at.is_some()
            || sidechat_expired(&conn, &thread_id)
            || hold_session.get("threadId").and_then(Value::as_str) != Some(thread_id.as_str())
            || hold_session.get("status").and_then(Value::as_str) != Some("ready")
            || !review_valid;
        if rejected {
            return Err(err("Command produced no events."));
        }
    }
    if let Some(expected) = command.get("expectedReviewId") {
        let current = thread
            .claude_cache_review
            .as_ref()
            .and_then(|r| serde_json::from_str::<Value>(r).ok())
            .and_then(|v| v.get("reviewId").cloned())
            .unwrap_or(Value::Null);
        if &current != expected {
            return Err(err("Command produced no events."));
        }
    }
    let review = command.get("review").cloned().unwrap_or(Value::Null);
    let review_ev = out_event(
        "thread.claude-cache-set",
        &thread_id,
        &created_at,
        json!({"threadId": thread_id, "review": review, "updatedAt": created_at}),
    );
    let mut events = vec![review_ev];
    if !hold.is_null() {
        let mut sess_ev = out_event(
            "thread.session-set",
            &thread_id,
            &created_at,
            json!({"threadId": thread_id, "session": hold.get("session").cloned().unwrap_or(Value::Null)}),
        );
        sess_ev.causation_event_id = Some(events[0].event_id.clone());
        events.push(sess_ev);
    }
    let seq = commit_events(&conn, &events, &command_id)?;
    update_projection(
        &conn,
        &thread_id,
        &[
            ("claude_cache_review_json", review),
            ("updated_at", json!(created_at)),
        ],
    )?;
    if events.len() == 2 {
        let session = events[1]
            .payload
            .get("session")
            .cloned()
            .unwrap_or(Value::Null);
        let g = |k: &str| json_to_sql(&session.get(k).cloned().unwrap_or(Value::Null));
        conn.execute(
            "INSERT INTO projection_thread_sessions \
             (thread_id, status, provider_name, provider_session_id, provider_thread_id, \
              active_turn_id, last_error, updated_at, runtime_mode, provider_instance_id) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
             ON CONFLICT(thread_id) DO UPDATE SET \
              status=excluded.status, provider_name=excluded.provider_name, \
              provider_session_id=excluded.provider_session_id, \
              provider_thread_id=excluded.provider_thread_id, \
              active_turn_id=excluded.active_turn_id, last_error=excluded.last_error, \
              updated_at=excluded.updated_at, runtime_mode=excluded.runtime_mode, \
              provider_instance_id=excluded.provider_instance_id",
            params![
                thread_id,
                g("status"),
                g("providerName"),
                g("providerSessionId"),
                g("providerThreadId"),
                g("activeTurnId"),
                g("lastError"),
                g("updatedAt"),
                g("runtimeMode"),
                g("providerInstanceId"),
            ],
        )
        .map_err(|e| err(format!("session projection: {e}")))?;
    }
    Ok(json!({ "sequence": seq }))
}

fn claude_cache_review_thread(conn: &Connection, thread_id: &str) -> Option<Value> {
    conn.query_row(
        "SELECT claude_cache_review_json FROM projection_threads WHERE thread_id = ?1",
        params![thread_id],
        |r| r.get::<_, Option<String>>(0),
    )
    .ok()
    .flatten()
    .and_then(|j| serde_json::from_str(&j).ok())
    .filter(|v: &Value| !v.is_null())
}

fn claude_cache_followup(command: &Map<String, Value>, compacted: bool) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let thread = require_thread(&conn, &thread_id)?;
    let review = claude_cache_review_thread(&conn, &thread_id);
    let command_type = s(command, "type").unwrap_or_default();
    let ok = match (&review, compacted) {
        (Some(r), true) => {
            r.get("reviewId").and_then(Value::as_str)
                == command.get("reviewId").and_then(Value::as_str)
                && matches!(
                    r.get("status").and_then(Value::as_str),
                    Some("compacting") | Some("uncertain")
                )
                && r.get("compactionTurnId").and_then(Value::as_str)
                    == command.get("turnId").and_then(Value::as_str)
                && thread.archived_at.is_none()
        }
        (Some(r), false) => {
            r.get("reviewId").and_then(Value::as_str)
                == command.get("reviewId").and_then(Value::as_str)
                && r.get("messageId").and_then(Value::as_str)
                    == command.get("messageId").and_then(Value::as_str)
                && matches!(
                    r.get("status").and_then(Value::as_str),
                    Some("pending") | Some("failed")
                )
        }
        _ => false,
    };
    if !ok {
        return Err(err("Command produced no events."));
    }
    let review = review.unwrap();
    let mut next_review = review.clone();
    next_review["status"] = json!("responding");
    if !compacted {
        if let Some(obj) = next_review.as_object_mut() {
            obj.remove("error");
        }
    }
    let decision = if compacted {
        json!("continue")
    } else {
        command.get("decision").cloned().unwrap_or(Value::Null)
    };
    let set_ev = out_event(
        "thread.claude-cache-set",
        &thread_id,
        &created_at,
        json!({"threadId": thread_id, "review": next_review, "updatedAt": created_at}),
    );
    let mut respond_ev = out_event(
        "thread.claude-cache-response-requested",
        &thread_id,
        &created_at,
        json!({
            "threadId": thread_id,
            "review": review,
            "decision": decision,
            "createdAt": created_at,
        }),
    );
    respond_ev.causation_event_id = Some(set_ev.event_id.clone());
    let events = vec![set_ev, respond_ev];
    let seq = commit_events(&conn, &events, &command_id)?;
    update_projection(
        &conn,
        &thread_id,
        &[
            ("claude_cache_review_json", next_review),
            ("updated_at", json!(created_at)),
        ],
    )?;
    let _ = command_type;
    Ok(json!({ "sequence": seq }))
}

fn dispatch_queued(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    require_sidechat_live(&conn, &thread_id)?;
    if thread_has_revert_in_progress(&conn, &thread_id)? {
        return Err(err(format!(
            "Thread '{thread_id}' already has a checkpoint revert in progress."
        )));
    }
    let runtime_mode = s(command, "runtimeMode").unwrap_or_else(|| "full-access".into());
    let model_sel = command
        .get("modelSelection")
        .cloned()
        .filter(|v| !v.is_null())
        .unwrap_or_else(|| thread_model_selection(&conn, &thread_id));
    if let Some(issue) = auto_runtime_issue(&runtime_mode, &model_sel) {
        return Err(err(issue));
    }
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    if let Some(v) = command.get("messageId") {
        payload.insert("messageId".into(), v.clone());
    }
    for k in [
        "initialGoalStartedAt",
        "modelSelection",
        "providerOptions",
        "reviewTarget",
        "enableComputerControl",
        "computerControlMode",
        "computerControlGeneration",
        "sourceProposedPlan",
    ] {
        if let Some(v) = command.get(k) {
            payload.insert(k.to_string(), v.clone());
        }
    }
    payload.insert(
        "assistantDeliveryMode".into(),
        command
            .get("assistantDeliveryMode")
            .cloned()
            .unwrap_or(json!("queue")),
    );
    payload.insert(
        "dispatchMode".into(),
        command.get("dispatchMode").cloned().unwrap_or(json!("queue")),
    );
    payload.insert(
        "dispatchOrigin".into(),
        command
            .get("dispatchOrigin")
            .cloned()
            .unwrap_or(json!("user")),
    );
    payload.insert(
        "runtimeMode".into(),
        command.get("runtimeMode").cloned().unwrap_or(Value::Null),
    );
    payload.insert(
        "interactionMode".into(),
        command
            .get("interactionMode")
            .cloned()
            .unwrap_or(Value::Null),
    );
    payload.insert("createdAt".into(), json!(created_at));
    let events = vec![out_event(
        "thread.turn-start-requested",
        &thread_id,
        &created_at,
        Value::Object(payload),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

fn proposed_plan_upsert(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let events = vec![out_event(
        "thread.proposed-plan-upserted",
        &thread_id,
        &created_at,
        json!({
            "threadId": thread_id,
            "proposedPlan": command.get("proposedPlan").cloned().unwrap_or(Value::Null),
        }),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

fn turn_diff_complete(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    for k in [
        "turnId",
        "checkpointTurnCount",
        "checkpointRef",
        "status",
        "files",
        "completedAt",
    ] {
        payload.insert(k.to_string(), command.get(k).cloned().unwrap_or(Value::Null));
    }
    payload.insert(
        "assistantMessageId".into(),
        command
            .get("assistantMessageId")
            .cloned()
            .unwrap_or(Value::Null),
    );
    if command.get("preserveLatestTurn").and_then(Value::as_bool) == Some(true) {
        payload.insert("preserveLatestTurn".into(), json!(true));
    }
    let diff_ev = out_event(
        "thread.turn-diff-completed",
        &thread_id,
        &created_at,
        Value::Object(payload),
    );
    let mut events = vec![diff_ev];
    if let Some(turn_count) = command.get("checkpointRevertTurnCount").and_then(Value::as_i64) {
        let succ = revert_succeeded_activity(
            &command_id,
            &thread_id,
            turn_count,
            &created_at,
            &events[0].event_id.clone(),
        );
        events.push(succ);
    }
    let seq = commit_events(&conn, &events, &command_id)?;
    if events.len() == 2 {
        let activity = events[1]
            .payload
            .get("activity")
            .cloned()
            .unwrap_or(Value::Null);
        append_activity_projection(&conn, &thread_id, &activity, seq)?;
    }
    Ok(json!({ "sequence": seq }))
}

fn revert_complete(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let turn_count = command
        .get("turnCount")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let reverted = out_event(
        "thread.reverted",
        &thread_id,
        &created_at,
        json!({"threadId": thread_id, "turnCount": turn_count}),
    );
    let succ = revert_succeeded_activity(
        &command_id,
        &thread_id,
        turn_count,
        &created_at,
        &reverted.event_id,
    );
    let events = vec![reverted, succ];
    let seq = commit_events(&conn, &events, &command_id)?;
    let activity = events[1]
        .payload
        .get("activity")
        .cloned()
        .unwrap_or(Value::Null);
    append_activity_projection(&conn, &thread_id, &activity, seq)?;
    Ok(json!({ "sequence": seq }))
}

fn conversation_rollback_complete(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    for k in ["messageId", "numTurns", "removedTurnIds"] {
        if let Some(v) = command.get(k) {
            payload.insert(k.to_string(), v.clone());
        }
    }
    let events = vec![out_event(
        "thread.conversation-rolled-back",
        &thread_id,
        &created_at,
        Value::Object(payload),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

// ---- thread.usage-limit.* ----

fn iso_ms(s: &str) -> Option<i64> {
    // "YYYY-MM-DDTHH:MM:SS.mmm(Z|+hh:mm)" → epoch ms; enough for comparisons.
    let b = s.as_bytes();
    if b.len() < 20 {
        return None;
    }
    let n = |a: usize, b: usize| -> Option<i64> { s.get(a..b)?.parse().ok() };
    let (y, mo, d) = (n(0, 4)?, n(5, 7)?, n(8, 10)?);
    let (h, mi, sec) = (n(11, 13)?, n(14, 16)?, n(17, 19)?);
    let mut ms = n(20, 23).unwrap_or(0);
    if !(19..b.len()).contains(&20) || b.get(19) != Some(&b'.') {
        ms = 0;
    }
    // days since epoch (civil)
    let days = {
        let y = if mo <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let mp = (mo + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146097 + doe - 719468
    };
    let mut total = days * 86_400_000 + h * 3_600_000 + mi * 60_000 + sec * 1_000 + ms;
    if let Some(off) = s.rfind('+').or_else(|| s[19..].rfind('-').map(|i| i + 19)) {
        if let (Some(oh), Some(om)) = (s.get(off + 1..off + 3), s.get(off + 4..off + 6)) {
            if let (Ok(oh), Ok(om)) = (oh.parse::<i64>(), om.parse::<i64>()) {
                total -= (oh * 60 + om) * 60_000;
            }
        }
    }
    Some(total)
}

#[derive(Default)]
struct UsageCtx {
    recovery: Value,
    recovery_source: Value,
    cancelled_at: Option<String>,
    cancelled_turn_id: Option<String>,
    last_reset_at: Option<String>,
    goal: Option<String>,
    goal_started_at: Option<String>,
    goal_paused_at: Option<String>,
    archived_at: Option<String>,
    settled_at: Option<String>,
    parent_thread_id: Option<String>,
    interaction_mode: String,
    model_selection: Value,
    latest_turn_id: Option<String>,
    session_active_turn_id: Option<String>,
    session_status: Option<String>,
    snoozed_until: Option<String>,
    sidechat_expired: bool,
    claude_cache_review: Option<String>,
    pending_approvals: i64,
    pending_inputs: i64,
}

fn usage_ctx(conn: &Connection, thread_id: &str) -> Result<UsageCtx, WsErr> {
    let row = conn
        .query_row(
            "SELECT usage_limit_recovery_json, usage_limit_recovery_source_json, \
                    usage_limit_recovery_cancelled_at, usage_limit_recovery_cancelled_turn_id, \
                    usage_limit_recovery_last_reset_at, goal, goal_started_at, goal_paused_at, \
                    archived_at, settled_at, parent_thread_id, interaction_mode, \
                    model_selection_json, latest_turn_id, sidechat_expired_at, \
                    claude_cache_review_json, pending_approval_count, pending_user_input_count, \
                    snoozed_until \
             FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| {
                Ok(UsageCtx {
                    recovery: r
                        .get::<_, Option<String>>(0)?
                        .and_then(|j| serde_json::from_str(&j).ok())
                        .unwrap_or(Value::Null),
                    recovery_source: r
                        .get::<_, Option<String>>(1)?
                        .and_then(|j| serde_json::from_str(&j).ok())
                        .unwrap_or(Value::Null),
                    cancelled_at: r.get(2)?,
                    cancelled_turn_id: r.get(3)?,
                    last_reset_at: r.get(4)?,
                    goal: r.get(5)?,
                    goal_started_at: r.get(6)?,
                    goal_paused_at: r.get(7)?,
                    archived_at: r.get(8)?,
                    settled_at: r.get(9)?,
                    parent_thread_id: r.get(10)?,
                    interaction_mode: r.get::<_, Option<String>>(11)?.unwrap_or_default(),
                    model_selection: r
                        .get::<_, Option<String>>(12)?
                        .and_then(|j| serde_json::from_str(&j).ok())
                        .unwrap_or(Value::Null),
                    latest_turn_id: r.get(13)?,
                    session_active_turn_id: None,
                    session_status: None,
                    snoozed_until: r.get(18)?,
                    sidechat_expired: r.get::<_, Option<String>>(14)?.is_some(),
                    claude_cache_review: r.get(15)?,
                    pending_approvals: r.get(16)?,
                    pending_inputs: r.get(17)?,
                })
            },
        )
        .map_err(|e| err(format!("Thread '{thread_id}' does not exist: {e}")))?;
    Ok(row)
}

fn usage_ctx_session(conn: &Connection, thread_id: &str, ctx: &mut UsageCtx) {
    if let Some((status, active)) = conn
        .query_row(
            "SELECT status, active_turn_id FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?)),
        )
        .ok()
    {
        ctx.session_status = Some(status);
        ctx.session_active_turn_id = active;
    }
}

fn latest_turn(conn: &Connection, thread_id: &str) -> Option<(String, String, String)> {
    conn.query_row(
        "SELECT turn_id, state, requested_at FROM projection_turns WHERE thread_id = ?1 \
         ORDER BY requested_at DESC LIMIT 1",
        params![thread_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .ok()
}

fn usage_limit_meta_event(thread_id: &str, occurred_at: &str, patch: &Map<String, Value>) -> OutEvent {
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    payload.insert("updatedAt".into(), json!(occurred_at));
    for (k, v) in patch {
        payload.insert(k.clone(), v.clone());
    }
    out_event("thread.meta-updated", thread_id, occurred_at, Value::Object(payload))
}

const USAGE_PATCH_COLS: &[(&str, &str)] = &[
    ("usageLimitRecovery", "usage_limit_recovery_json"),
    ("usageLimitRecoverySource", "usage_limit_recovery_source_json"),
    ("usageLimitRecoveryCancelledAt", "usage_limit_recovery_cancelled_at"),
    ("usageLimitRecoveryCancelledTurnId", "usage_limit_recovery_cancelled_turn_id"),
    ("usageLimitRecoveryLastResetAt", "usage_limit_recovery_last_reset_at"),
    ("goalPausedAt", "goal_paused_at"),
    ("goalStartedAt", "goal_started_at"),
    ("goal", "goal"),
    ("goalAchievements", "goal_achievements_json"),
];

fn usage_limit_dispatch(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").unwrap_or_else(iso_now_ms);
    let command_type = s(command, "type").unwrap_or_default();
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let mut ctx = usage_ctx(&conn, &thread_id)?;
    usage_ctx_session(&conn, &thread_id, &mut ctx);
    let occurred_at = iso_now_ms();
    let gv = |v: &Value, k: &str| v.get(k).cloned().unwrap_or(Value::Null);
    let gc = |k: &str| command.get(k).cloned().unwrap_or(Value::Null);
    let gs = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(String::from);
    let mut patch = Map::new();
    let mut extra_events: Vec<OutEvent> = Vec::new();

    match command_type.as_str() {
        "thread.usage-limit.admit" => {
            let recovery_id = command.get("recoveryId").and_then(Value::as_str);
            let bad_recovery = recovery_id.is_some_and(|rid| {
                gs(&ctx.recovery, "id").as_deref() != Some(rid)
                    || gs(&ctx.recovery, "state").as_deref() != Some("resuming")
                    || gv(&ctx.recovery, "policyEpoch") != gc("policyEpoch")
            });
            if !bad_recovery {
                let current = gv(&ctx.recovery_source, "admissionSequence").as_i64();
                let incoming = gc("admissionSequence").as_i64().unwrap_or(0);
                if current.map_or(true, |c| c <= incoming) {
                    patch.insert(
                        "usageLimitRecoverySource".into(),
                        json!({
                            "turnId": format!("pending:{incoming}"),
                            "bindingKey": "admission",
                            "admissionSequence": incoming,
                            "policyEpoch": gc("policyEpoch"),
                        }),
                    );
                }
            }
        }
        "thread.usage-limit.source" => {
            let source = gc("source");
            let binding_ok = command
                .get("expectedBindingKey")
                .map_or(true, |e| gv(&ctx.recovery_source, "bindingKey") == *e);
            if ctx.session_active_turn_id.as_deref()
                == source.get("turnId").and_then(Value::as_str)
                && binding_ok
            {
                patch.insert("usageLimitRecoverySource".into(), source);
            }
        }
        "thread.usage-limit.defer" => {
            let paused_at = ctx
                .goal
                .as_deref()
                .map(|g| g.trim())
                .filter(|g| !g.is_empty())
                .map(|_| occurred_at.clone());
            if gs(&ctx.recovery, "id") == s(command, "recoveryId")
                && gs(&ctx.recovery, "state").as_deref() == Some("resuming")
                && ctx.recovery.get("resumedTurnId").is_none()
            {
                let mut next = ctx.recovery.clone();
                next["state"] = json!("waiting");
                if let Some(pa) = &paused_at {
                    next["goalStartedAt"] =
                        ctx.goal_started_at.clone().map_or(Value::Null, |v| json!(v));
                    next["goalPausedAt"] = json!(pa);
                }
                patch.insert("usageLimitRecovery".into(), next);
                if let Some(pa) = paused_at {
                    patch.insert("goalPausedAt".into(), json!(pa));
                }
            }
        }
        "thread.usage-limit.started" | "thread.usage-limit.finish" => {
            let valid = !ctx.recovery.is_null()
                && gs(&ctx.recovery, "id") == s(command, "recoveryId")
                && gs(&ctx.recovery, "state").as_deref() == Some("resuming")
                && (ctx.recovery.get("resumedTurnId").is_none()
                    || gs(&ctx.recovery, "resumedTurnId") == s(command, "turnId"));
            if valid {
                if command_type.ends_with("finish") {
                    patch.insert("usageLimitRecovery".into(), Value::Null);
                } else {
                    let mut next = ctx.recovery.clone();
                    next["resumedTurnId"] = gc("turnId");
                    patch.insert("usageLimitRecovery".into(), next);
                }
            }
        }
        "thread.usage-limit.cancel-turn" => {
            let lt = latest_turn(&conn, &thread_id);
            let latest_id = lt.as_ref().map(|(id, ..)| id.as_str());
            let turn_id = s(command, "turnId").unwrap_or_default();
            let source_turn = s(command, "sourceTurnId").unwrap_or_default();
            let matches_turn = latest_id == Some(turn_id.as_str())
                || latest_id == Some(source_turn.as_str())
                || ctx.session_active_turn_id.as_deref() == Some(turn_id.as_str());
            if matches_turn {
                patch.insert("usageLimitRecoveryCancelledAt".into(), json!(iso_now_ms()));
                patch.insert(
                    "usageLimitRecoveryCancelledTurnId".into(),
                    json!(turn_id),
                );
                let rec_match = !ctx.recovery.is_null()
                    && (gs(&ctx.recovery, "id") == s(command, "recoveryId")
                        || gs(&ctx.recovery, "turnId").as_deref() == Some(turn_id.as_str())
                        || gs(&ctx.recovery, "resumedTurnId").as_deref()
                            == Some(turn_id.as_str()));
                if rec_match {
                    patch.insert("usageLimitRecovery".into(), Value::Null);
                    let goal_active = ctx
                        .goal
                        .as_deref()
                        .map(|g| !g.trim().is_empty())
                        .unwrap_or(false);
                    if ctx.recovery.get("goalPausedAt").is_some()
                        && ctx.goal_paused_at.is_none()
                        && goal_active
                    {
                        patch.insert("goalPausedAt".into(), json!(iso_now_ms()));
                    }
                }
            }
        }
        "thread.usage-limit.cancel" => {
            let matches = command
                .get("recoveryId")
                .map_or(true, |rid| gv(&ctx.recovery, "id") == *rid);
            if matches {
                patch.insert("usageLimitRecovery".into(), Value::Null);
                patch.insert("usageLimitRecoverySource".into(), Value::Null);
                patch.insert("usageLimitRecoveryCancelledAt".into(), json!(iso_now_ms()));
                let lt = latest_turn(&conn, &thread_id);
                let cancelled_turn = ctx
                    .session_active_turn_id
                    .clone()
                    .or_else(|| lt.as_ref().map(|(id, ..)| id.clone()));
                patch.insert(
                    "usageLimitRecoveryCancelledTurnId".into(),
                    cancelled_turn.map_or(Value::Null, |v| json!(v)),
                );
                let goal_active = ctx
                    .goal
                    .as_deref()
                    .map(|g| !g.trim().is_empty())
                    .unwrap_or(false);
                if ctx.recovery.get("goalPausedAt").is_some()
                    && ctx.goal_paused_at.is_none()
                    && goal_active
                {
                    patch.insert("goalPausedAt".into(), json!(iso_now_ms()));
                }
            }
        }
        "thread.usage-limit.schedule" => {
            let recovery = gc("recovery");
            let lt = latest_turn(&conn, &thread_id);
            let reset_ms = recovery
                .get("resetAt")
                .and_then(Value::as_str)
                .and_then(iso_ms);
            let cancelled = ctx.cancelled_turn_id.as_deref()
                == recovery.get("turnId").and_then(Value::as_str)
                || (ctx.cancelled_turn_id.is_none()
                    && ctx.cancelled_at.is_some()
                    && lt.as_ref().map_or(false, |(_, _, req)| {
                        req.as_str() <= ctx.cancelled_at.as_deref().unwrap_or("")
                    }));
            let source = &ctx.recovery_source;
            let model = &ctx.model_selection;
            let goal_active = ctx
                .goal
                .as_deref()
                .map(|g| !g.trim().is_empty())
                .unwrap_or(false);
            let reject = source.is_null()
                || gs(source, "bindingKey").as_deref() == Some("pending")
                || gv(source, "turnId") != gv(&recovery, "turnId")
                || gv(source, "bindingKey") != gv(&recovery, "bindingKey")
                || gv(source, "policyEpoch") != gv(&recovery, "policyEpoch")
                || lt.is_none()
                || lt.as_ref().map(|(id, ..)| id.as_str())
                    != recovery.get("turnId").and_then(Value::as_str)
                || lt.as_ref().map(|(_, st, _)| st.as_str()) != Some("error")
                || cancelled
                || ctx.archived_at.is_some()
                || ctx.settled_at.is_some()
                || ctx.parent_thread_id.is_some()
                || ctx.interaction_mode == "plan"
                || ctx.sidechat_expired
                || ctx.claude_cache_review.is_some()
                || reset_ms.is_none()
                || reset_ms.unwrap() <= iso_ms(&created_at).unwrap_or(0)
                || (ctx.last_reset_at.is_some()
                    && reset_ms.unwrap()
                        <= iso_ms(ctx.last_reset_at.as_deref().unwrap_or("")).unwrap_or(0))
                || gs(model, "provider") != gs(&recovery, "provider")
                || gs(model, "instanceId").unwrap_or_else(|| gs(model, "provider").unwrap_or_default())
                    != gs(&recovery, "providerInstanceId").unwrap_or_default()
                || gs(model, "model") != gs(&recovery, "model")
                || (goal_active && ctx.goal_paused_at.is_some());
            if !reject {
                let paused_at = goal_active.then(|| occurred_at.clone());
                let mut next = recovery.clone();
                next["state"] = json!("waiting");
                if let Some(pa) = &paused_at {
                    next["goalStartedAt"] =
                        ctx.goal_started_at.clone().map_or(Value::Null, |v| json!(v));
                    next["goalPausedAt"] = json!(pa);
                }
                patch.insert("usageLimitRecovery".into(), next);
                if let Some(pa) = paused_at {
                    patch.insert("goalPausedAt".into(), json!(pa));
                }
            }
        }
        "thread.usage-limit.resume" => {
            let recovery = ctx.recovery.clone();
            let rid_ok = !recovery.is_null()
                && gs(&recovery, "id") == s(command, "recoveryId")
                && gs(&recovery, "state").as_deref() != Some("resuming");
            if !rid_ok {
                // emit meta-updated with empty patch
            } else {
                let lt = latest_turn(&conn, &thread_id);
                let model = &ctx.model_selection;
                let rec_paused = recovery.get("goalPausedAt").and_then(Value::as_str);
                let goal_mismatch = rec_paused.is_some()
                    && (ctx.goal_paused_at.as_deref() != rec_paused
                        || ctx.goal_started_at != gs(&recovery, "goalStartedAt"));
                let stale = gv(&recovery, "policyEpoch") != gc("policyEpoch")
                    || ctx.archived_at.is_some()
                    || ctx.settled_at.is_some()
                    || ctx.parent_thread_id.is_some()
                    || ctx.sidechat_expired
                    || lt.as_ref().map(|(id, ..)| id.as_str())
                        != recovery.get("turnId").and_then(Value::as_str)
                    || lt.as_ref().map(|(_, st, _)| st.as_str()) != Some("error")
                    || gs(model, "provider") != gs(&recovery, "provider")
                    || gs(model, "instanceId")
                        .unwrap_or_else(|| gs(model, "provider").unwrap_or_default())
                        != gs(&recovery, "providerInstanceId").unwrap_or_default()
                    || gs(model, "model") != gs(&recovery, "model")
                    || goal_mismatch;
                if stale {
                    patch.insert("usageLimitRecovery".into(), Value::Null);
                } else {
                    let now_ms = iso_ms(&iso_now_ms()).unwrap_or(0);
                    let not_ready = recovery
                        .get("resetAt")
                        .and_then(Value::as_str)
                        .and_then(iso_ms)
                        .map_or(false, |r| r > now_ms)
                        || thread_has_in_flight(&conn, &thread_id)?
                        || ctx.pending_approvals > 0
                        || ctx.pending_inputs > 0
                        || ctx.claude_cache_review.is_some()
                        || thread_has_revert_in_progress(&conn, &thread_id)?
                        || ctx.interaction_mode == "plan"
                        || ctx
                            .snoozed_until
                            .as_deref()
                            .and_then(iso_ms)
                            .map_or(false, |s| s > now_ms);
                    if not_ready {
                        return Err(err("Usage-limit recovery is not ready."));
                    }
                    // goalPatch for goalPaused=false
                    let mut goal_patch = Map::new();
                    if rec_paused.is_some() {
                        let goal_active = ctx
                            .goal
                            .as_deref()
                            .map(|g| !g.trim().is_empty())
                            .unwrap_or(false);
                        if goal_active && ctx.goal_paused_at.is_some() {
                            let started_ms = ctx
                                .goal_started_at
                                .as_deref()
                                .and_then(iso_ms)
                                .unwrap_or(0);
                            let paused_ms =
                                ctx.goal_paused_at.as_deref().and_then(iso_ms).unwrap_or(0);
                            let occurred_ms = iso_ms(&occurred_at).unwrap_or(0);
                            let rebased = if started_ms > 0 && paused_ms > 0 {
                                occurred_ms - (paused_ms - started_ms).max(0)
                            } else {
                                occurred_ms
                            };
                            goal_patch.insert(
                                "goalStartedAt".into(),
                                json!(iso_from_ms(rebased)),
                            );
                            goal_patch.insert("goalPausedAt".into(), Value::Null);
                        }
                    }
                    let mut next_recovery = recovery.clone();
                    next_recovery["state"] = json!("resuming");
                    patch.insert("usageLimitRecovery".into(), next_recovery.clone());
                    patch.insert(
                        "usageLimitRecoveryLastResetAt".into(),
                        gv(&recovery, "resetAt"),
                    );
                    for (k, v) in &goal_patch {
                        patch.insert(k.clone(), v.clone());
                    }
                    let goal_started = goal_patch
                        .get("goalStartedAt")
                        .cloned()
                        .unwrap_or_else(|| {
                            ctx.goal_started_at.clone().map_or(Value::Null, |v| json!(v))
                        });
                    let mut cont = out_event(
                        "thread.goal-continuation-requested",
                        &thread_id,
                        &created_at,
                        json!({
                            "threadId": thread_id,
                            "goalStartedAt": goal_started,
                            "trigger": "turn-completed",
                            "sourceTurnId": gv(&recovery, "turnId"),
                            "usageLimitRecovery": next_recovery,
                            "createdAt": created_at,
                        }),
                    );
                    cont.causation_event_id = None;
                    extra_events.push(cont);
                }
            }
        }
        _ => return Err(err(format!("unsupported usage-limit command {command_type}"))),
    }

    let meta = usage_limit_meta_event(&thread_id, &occurred_at, &patch);
    let causation = meta.event_id.clone();
    for ev in &mut extra_events {
        if ev.causation_event_id.is_none() {
            ev.causation_event_id = Some(causation.clone());
        }
    }
    let mut events = vec![meta];
    events.extend(extra_events);
    let seq = commit_events(&conn, &events, &command_id)?;
    // Projection: apply patched columns.
    for (key, col) in USAGE_PATCH_COLS {
        if let Some(v) = patch.get(*key) {
            let sql_val = if matches!(*key, "usageLimitRecovery" | "usageLimitRecoverySource" | "goalAchievements") {
                json_to_sql(&json!(v.to_string()))
            } else {
                json_to_sql(v)
            };
            conn.execute(
                &format!("UPDATE projection_threads SET {col} = ?1, updated_at = ?2 WHERE thread_id = ?3"),
                params![sql_val, occurred_at, thread_id],
            )
            .map_err(|e| err(format!("usage-limit projection: {e}")))?;
        }
    }
    if !patch.is_empty() {
        conn.execute(
            "UPDATE projection_threads SET updated_at = ?1 WHERE thread_id = ?2",
            params![occurred_at, thread_id],
        )
        .ok();
    }
    Ok(json!({ "sequence": seq }))
}

// ---- sidechat.* ----

fn thread_is_sidechat(conn: &Connection, thread_id: &str) -> bool {
    conn.query_row(
        "SELECT sidechat_source_thread_id IS NOT NULL OR sidechat_context_json IS NOT NULL \
         FROM projection_threads WHERE thread_id = ?1",
        params![thread_id],
        |r| r.get::<_, bool>(0),
    )
    .unwrap_or(false)
}

fn sidechat_activity(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    if !thread_is_sidechat(&conn, &thread_id) {
        return Err(err(format!("Thread '{thread_id}' is not a side chat.")));
    }
    if sidechat_expired(&conn, &thread_id) {
        return Err(err(format!("Side chat '{thread_id}' already expired.")));
    }
    let activity_at = s(command, "activityAt").unwrap_or_else(iso_now_ms);
    let current: Option<String> = conn
        .query_row(
            "SELECT sidechat_last_activity_at FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let last = match (&current, iso_ms(&activity_at)) {
        (Some(c), Some(a)) => {
            if iso_ms(c).map_or(true, |cm| a > cm) {
                activity_at.clone()
            } else {
                c.clone()
            }
        }
        (Some(c), None) => c.clone(),
        (None, _) => activity_at.clone(),
    };
    let events = vec![out_event(
        "thread.sidechat-activity-recorded",
        &thread_id,
        &last,
        json!({"threadId": thread_id, "lastActivityAt": last}),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    update_projection(
        &conn,
        &thread_id,
        &[
            ("sidechat_last_activity_at", json!(last)),
            ("updated_at", json!(iso_now_ms())),
        ],
    )?;
    Ok(json!({ "sequence": seq }))
}

fn sidechat_expire(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let thread = require_thread(&conn, &thread_id)?;
    if !thread_is_sidechat(&conn, &thread_id) {
        return Err(err(format!("Thread '{thread_id}' is not a side chat.")));
    }
    if sidechat_expired(&conn, &thread_id) {
        return Err(err(format!("Side chat '{thread_id}' already expired.")));
    }
    let last_activity: Option<String> = conn
        .query_row(
            "SELECT sidechat_last_activity_at FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let last = last_activity.unwrap_or_else(|| thread.updated_at.clone());
    let expected = s(command, "expectedLastActivityAt").unwrap_or_default();
    let instants_equal = match (iso_ms(&last), iso_ms(&expected)) {
        (Some(a), Some(b)) => a == b,
        _ => last == expected,
    };
    if !instants_equal {
        return Err(err(format!(
            "Side chat '{thread_id}' became active before expiry."
        )));
    }
    let latest_running = latest_turn(&conn, &thread_id)
        .map(|(_, st, _)| st == "running")
        .unwrap_or(false);
    let session = session_row(&conn, &thread_id);
    let session_live = session
        .as_ref()
        .map(|(st, _, _)| st == "starting" || st == "running")
        .unwrap_or(false);
    if latest_running || session_live {
        return Err(err(format!(
            "Side chat '{thread_id}' still has a running turn."
        )));
    }
    let (approvals, inputs): (i64, i64) = conn
        .query_row(
            "SELECT pending_approval_count, pending_user_input_count \
             FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((0, 0));
    if approvals > 0 || inputs > 0 {
        return Err(err(format!(
            "Side chat '{thread_id}' still has a pending interaction."
        )));
    }
    let expired_at = s(command, "expiredAt").unwrap_or_else(iso_now_ms);
    let events = vec![out_event(
        "thread.sidechat-expired",
        &thread_id,
        &expired_at,
        json!({
            "threadId": thread_id,
            "expectedLastActivityAt": expected,
            "expiredAt": expired_at,
        }),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    update_projection(
        &conn,
        &thread_id,
        &[
            ("sidechat_expired_at", json!(expired_at)),
            ("updated_at", json!(iso_now_ms())),
        ],
    )?;
    Ok(json!({ "sequence": seq }))
}

// ---- thread.messages.import / message.* ----

fn project_message_sent(
    conn: &Connection,
    thread_id: &str,
    p: &Value,
) -> Result<(), WsErr> {
    let g = |k: &str| p.get(k).cloned().unwrap_or(Value::Null);
    let message_id = g("messageId");
    let role = g("role");
    let streaming = g("streaming").as_bool().unwrap_or(false);
    let delta_mode = role.as_str() == Some("assistant") && streaming;
    let existing: Option<(String, Option<String>, i64)> = conn
        .query_row(
            "SELECT text, turn_id, is_streaming FROM projection_thread_messages \
             WHERE thread_id = ?1 AND message_id = ?2",
            params![thread_id, json_to_sql(&message_id)],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();
    let text = if delta_mode {
        match &existing {
            Some((t, _, _)) => {
                json!(format!("{}{}", t, g("text").as_str().unwrap_or("")))
            }
            _ => g("text"),
        }
    } else {
        match (&existing, g("text").as_str().unwrap_or("").is_empty()) {
            (Some((t, _, _)), true) => json!(t),
            _ => g("text"),
        }
    };
    let turn_id = match &existing {
        Some((_, Some(t), _)) => json!(t),
        _ => g("turnId"),
    };
    conn.execute(
        "INSERT INTO projection_thread_messages \
         (message_id, thread_id, turn_id, role, text, is_streaming, created_at, updated_at, \
          attachments_json, source, skills_json, mentions_json, dispatch_mode, dispatch_origin, \
          starts_new_turn, async_user_input_json) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16) \
         ON CONFLICT(thread_id, message_id) DO UPDATE SET \
          turn_id=excluded.turn_id, text=excluded.text, is_streaming=excluded.is_streaming, \
          updated_at=excluded.updated_at, \
          attachments_json=COALESCE(excluded.attachments_json, projection_thread_messages.attachments_json), \
          skills_json=COALESCE(excluded.skills_json, projection_thread_messages.skills_json), \
          mentions_json=COALESCE(excluded.mentions_json, projection_thread_messages.mentions_json), \
          dispatch_mode=COALESCE(excluded.dispatch_mode, projection_thread_messages.dispatch_mode), \
          dispatch_origin=COALESCE(excluded.dispatch_origin, projection_thread_messages.dispatch_origin), \
          starts_new_turn=COALESCE(excluded.starts_new_turn, projection_thread_messages.starts_new_turn), \
          async_user_input_json=COALESCE(excluded.async_user_input_json, projection_thread_messages.async_user_input_json)",
        params![
            json_to_sql(&message_id),
            thread_id,
            json_to_sql(&turn_id),
            json_to_sql(&role),
            json_to_sql(&text),
            if streaming { 1 } else { 0 },
            json_to_sql(&g("createdAt")),
            json_to_sql(&g("updatedAt")),
            g("attachments").to_string_opt(),
            g("source").as_str().unwrap_or("native").to_string(),
            g("skills").to_string_opt(),
            g("mentions").to_string_opt(),
            g("dispatchMode").to_string_opt(),
            g("dispatchOrigin").to_string_opt(),
            g("startsNewTurn").to_string_opt(),
            g("asyncUserInput").to_string_opt(),
        ],
    )
    .map_err(|e| err(format!("message projection: {e}")))?;
    Ok(())
}

trait ToStringOpt {
    fn to_string_opt(&self) -> Option<String>;
}
impl ToStringOpt for Value {
    fn to_string_opt(&self) -> Option<String> {
        if self.is_null() {
            None
        } else if let Some(s) = self.as_str() {
            Some(s.to_string())
        } else {
            Some(self.to_string())
        }
    }
}

fn messages_import(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let messages = command
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    if messages.is_empty() {
        return Err(err("Command produced no events."));
    }
    let mut events = Vec::new();
    for m in &messages {
        let mut payload = Map::new();
        payload.insert("threadId".into(), json!(thread_id));
        payload.insert("messageId".into(), m.get("messageId").cloned().unwrap_or(Value::Null));
        payload.insert("role".into(), m.get("role").cloned().unwrap_or(Value::Null));
        payload.insert("text".into(), m.get("text").cloned().unwrap_or(Value::Null));
        if let Some(v) = m.get("attachments") {
            payload.insert("attachments".into(), v.clone());
        }
        if let Some(v) = m.get("dispatchOrigin") {
            payload.insert("dispatchOrigin".into(), v.clone());
        }
        payload.insert("turnId".into(), Value::Null);
        payload.insert("streaming".into(), json!(false));
        payload.insert("source".into(), json!("native"));
        payload.insert("createdAt".into(), m.get("createdAt").cloned().unwrap_or(Value::Null));
        payload.insert("updatedAt".into(), m.get("updatedAt").cloned().unwrap_or(Value::Null));
        events.push(out_event(
            "thread.message-sent",
            &thread_id,
            &created_at,
            Value::Object(payload),
        ));
    }
    for ev in &events {
        project_message_sent(&conn, &thread_id, &ev.payload)?;
    }
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

fn assistant_message_event(command: &Map<String, Value>, delta: bool) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let message_id = s(command, "messageId").ok_or_else(|| err("missing messageId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let existing: Option<(String, i64, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT role, is_streaming, turn_id, async_user_input_json \
             FROM projection_thread_messages WHERE thread_id = ?1 AND message_id = ?2",
            params![thread_id, message_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();
    if delta
        && command.get("attachments").is_some()
        && existing
            .as_ref()
            .map(|(role, streaming, _, _)| role != "assistant" || *streaming == 1)
            .unwrap_or(true)
    {
        return Err(err(
            "Late image snapshots require an already finalized assistant message.",
        ));
    }
    let existing_turn = existing.as_ref().and_then(|(_, _, t, _)| t.clone());
    let turn_id = existing_turn
        .or_else(|| s(command, "turnId"))
        .map_or(Value::Null, |v| json!(v));
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    payload.insert("messageId".into(), json!(message_id));
    payload.insert("role".into(), json!("assistant"));
    if delta {
        payload.insert("text".into(), command.get("delta").cloned().unwrap_or(json!("")));
    } else {
        let existing_text: Option<String> = conn
            .query_row(
                "SELECT text FROM projection_thread_messages \
                 WHERE thread_id = ?1 AND message_id = ?2",
                params![thread_id, message_id],
                |r| r.get(0),
            )
            .ok();
        payload.insert("text".into(), existing_text.map_or(json!(""), |t| json!(t)));
        if let Some(v) = command.get("asyncQuestions") {
            let aui = existing
                .as_ref()
                .and_then(|(_, _, _, a)| a.clone())
                .and_then(|j| serde_json::from_str::<Value>(&j).ok())
                .unwrap_or_else(|| json!({"questions": v.clone()}));
            payload.insert("asyncUserInput".into(), aui);
        }
    }
    if let Some(v) = command.get("attachments") {
        payload.insert("attachments".into(), v.clone());
    }
    if delta {
        for k in ["segmentStartedAt", "segmentSequence"] {
            if let Some(v) = command.get(k) {
                payload.insert(k.to_string(), v.clone());
            }
        }
    }
    payload.insert("turnId".into(), turn_id);
    payload.insert("streaming".into(), json!(delta));
    payload.insert("createdAt".into(), json!(created_at));
    payload.insert("updatedAt".into(), json!(created_at));
    let first = out_event("thread.message-sent", &thread_id, &created_at, Value::Object(payload));
    let mut events = vec![first.clone_shallow()];
    let finalized_before = existing
        .as_ref()
        .map(|(role, streaming, _, _)| role == "assistant" && *streaming != 1)
        .unwrap_or(false);
    if delta && finalized_before {
        // Late delta on a settled message: re-settle in the same command.
        let mut settle = first.clone_shallow();
        settle.event_id = uuid_v4();
        settle.payload["streaming"] = json!(false);
        settle.payload["text"] = json!("");
        events.push(settle);
    }
    for ev in &events {
        project_message_sent(&conn, &thread_id, &ev.payload)?;
    }
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

fn user_bind_or_boundary(command: &Map<String, Value>, bind: bool) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let message_id = s(command, "messageId").ok_or_else(|| err("missing messageId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let msg: Option<(String, Option<String>, String, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<i64>)> = conn
        .query_row(
            "SELECT role, turn_id, text, source, attachments_json, skills_json, mentions_json, dispatch_origin, starts_new_turn \
             FROM projection_thread_messages WHERE thread_id = ?1 AND message_id = ?2",
            params![thread_id, message_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
        )
        .ok();
    let (role, cur_turn, text, source, attachments, skills, mentions, dispatch_origin, starts_new) =
        match msg {
            Some(m) => m,
            None => {
                return Err(err(format!(
                    "User message '{message_id}' does not exist on thread '{thread_id}'."
                )))
            }
        };
    if role != "user" {
        return Err(err(format!(
            "User message '{message_id}' does not exist on thread '{thread_id}'."
        )));
    }
    if bind {
        let cmd_turn = s(command, "turnId");
        if let (Some(cur), Some(new)) = (&cur_turn, &cmd_turn) {
            if cur != new {
                return Err(err(format!(
                    "User message '{message_id}' is already bound to turn '{cur}'."
                )));
            }
        }
    }
    let turn_id = if bind {
        s(command, "turnId").map_or(cur_turn.clone().map_or(Value::Null, |v| json!(v)), |v| json!(v))
    } else {
        cur_turn.clone().map_or(Value::Null, |v| json!(v))
    };
    let starts = if bind {
        starts_new.map(|v| v != 0)
    } else {
        command.get("startsNewTurn").and_then(Value::as_bool)
    };
    let mut payload = Map::new();
    payload.insert("threadId".into(), json!(thread_id));
    payload.insert("messageId".into(), json!(message_id));
    payload.insert("role".into(), json!("user"));
    payload.insert("text".into(), json!(text));
    if let Some(a) = attachments {
        if let Ok(v) = serde_json::from_str::<Value>(&a) {
            payload.insert("attachments".into(), v);
        }
    }
    for (k, v) in [("skills", skills), ("mentions", mentions)] {
        if let Some(j) = v {
            if let Ok(val) = serde_json::from_str::<Value>(&j) {
                payload.insert(k.to_string(), val);
            }
        }
    }
    if let Some(d) = dispatch_origin {
        payload.insert("dispatchOrigin".into(), json!(d));
    }
    if let Some(st) = starts {
        payload.insert("startsNewTurn".into(), json!(st));
    }
    payload.insert("turnId".into(), turn_id);
    payload.insert("streaming".into(), json!(false));
    payload.insert("source".into(), json!(source));
    let (created, updated): (Option<String>, Option<String>) = conn
        .query_row(
            "SELECT created_at, updated_at FROM projection_thread_messages \
             WHERE thread_id = ?1 AND message_id = ?2",
            params![thread_id, message_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap_or((None, None));
    payload.insert("createdAt".into(), created.map_or(Value::Null, |v| json!(v)));
    payload.insert("updatedAt".into(), updated.map_or(Value::Null, |v| json!(v)));
    let events = vec![out_event(
        "thread.message-sent",
        &thread_id,
        &created_at,
        Value::Object(payload),
    )];
    for ev in &events {
        project_message_sent(&conn, &thread_id, &ev.payload)?;
    }
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}
