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

pub(crate) fn uuid_v4() -> String {
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
    matches!(
        tag,
        "orchestration.dispatchCommand"
            | "orchestration.settleTurnDispatch"
            | "orchestration.prepareQuitResume"
            | "orchestration.reconcileProviderDelivery"
            | "orchestration.repairState"
    ) && std::env::var("SYNARA_NO_UPSTREAM").is_ok()
}

pub(crate) fn open_state_rw() -> Result<Connection, WsErr> {
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

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    if tag == "orchestration.prepareQuitResume" {
        return prepare_quit_resume(payload);
    }
    if tag == "orchestration.repairState" {
        return repair_state();
    }
    if tag == "orchestration.reconcileProviderDelivery" {
        return reconcile_provider_delivery(payload);
    }
    if tag == "orchestration.settleTurnDispatch" {
        let command = payload
            .get("command")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                err("orchestration.settleTurnDispatch expects a command object")
            })?;
        return settle_turn_dispatch(command);
    }
    let command = payload
        .as_object()
        .ok_or_else(|| err("orchestration.dispatchCommand expects a command object"))?;
    let command_type = command
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| err("orchestration.dispatchCommand payload is missing type"))?;
    match command_type {
        "thread.create" => thread_create(command),
        "thread.handoff.create" => lineage_create(command, true),
        "thread.fork.create" => lineage_create(command, false),
        "thread.archive" => thread_archive(command, true),
        "thread.unarchive" => thread_archive(command, false),
        "thread.delete" => thread_delete(command),
        "thread.runtime-mode.set" => thread_mode_set(command, "runtimeMode", "runtime_mode"),
        "thread.interaction-mode.set" => {
            thread_mode_set(command, "interactionMode", "interaction_mode")
        }
        "thread.meta.update" => thread_meta_update(command),
        "thread.turn.start" => thread_turn_start(command),
        "thread.turn.interrupt" => {
            if let Some(t) = s(command, "threadId") {
                cancel_any_provider_turn(&t);
            }
            thread_simple(command, "thread.turn-interrupt-requested")
        }
        "thread.task.stop" => thread_simple(command, "thread.task-stop-requested"),
        "thread.task.background" => thread_simple(command, "thread.task-background-requested"),
        "thread.approval.respond" => {
            // Record the decision on the row: the live provider runtime
            // (the ACP runner polls `decision` to answer the ACP
            // request_permission call) consumes it and then deletes the
            // row when it resolves the interaction. 'confirmed' rows no
            // consumer ever claims stay invisible — every UI surface
            // filters on status='pending'.
            if let (Some(tid), Some(rid)) = (s(command, "threadId"), s(command, "requestId")) {
                let conn = open_state_rw()?;
                conn.execute(
                    "UPDATE projection_pending_interactions \
                     SET status='confirmed', decision=?3, \
                         response_command_id=?4, resolved_at=?5 \
                     WHERE thread_id=?1 AND request_id=?2 \
                     AND interaction_kind='approval'",
                    params![
                        tid,
                        rid,
                        s(command, "decision").unwrap_or_default(),
                        s(command, "commandId").unwrap_or_default(),
                        s(command, "createdAt").unwrap_or_default()
                    ],
                )
                .map_err(|e| err(format!("resolve approval: {e}")))?;
            }
            thread_simple(command, "thread.approval-response-requested")
        }
        "thread.user-input.respond" => {
            // Like approvals, leave a confirmed row for the live provider
            // runtime to consume. Structured answers are stored as JSON in the
            // legacy decision column; rows are deleted only after forwarding.
            if let (Some(tid), Some(rid)) = (s(command, "threadId"), s(command, "requestId")) {
                let conn = open_state_rw()?;
                let answers = command
                    .get("answers")
                    .cloned()
                    .unwrap_or(json!({}))
                    .to_string();
                conn.execute(
                    "UPDATE projection_pending_interactions \
                     SET status='confirmed', decision=?3, response_command_id=?4, resolved_at=?5 \
                     WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='userInput'",
                    params![
                        tid,
                        rid,
                        answers,
                        s(command, "commandId").unwrap_or_default(),
                        s(command, "createdAt").unwrap_or_default()
                    ],
                )
                .map_err(|e| err(format!("resolve user-input: {e}")))?;
            }
            thread_simple(command, "thread.user-input-response-requested")
        }
        "thread.activity.append" => thread_activity_append(command),
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
        "thread.session.stop" => {
            if let Some(t) = s(command, "threadId") {
                cancel_any_provider_turn(&t);
            }
            session_stop(command)
        }
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
    cancel_any_provider_turn(&thread_id);
    crate::acp_runner::drop_thread_resources(&thread_id);
    crate::codex_runner::drop_thread_resources(&thread_id);
    crate::claude_runner::drop_thread_resources(&thread_id);
    crate::opencode_runner::drop_thread_resources(&thread_id);
    crate::pi_runner::drop_thread_resources(&thread_id);
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
    if let Some(v) = command.get("modelSelection").filter(|v| v.is_object()) {
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


#[allow(clippy::type_complexity)]
fn resolve_workspace_fields(
    command: &Map<String, Value>,
    is_group: bool,
) -> (String, Value, Value, Value, Value, Value, Value, bool) {
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
    (env_mode, branch, worktree_path, working_directory, associated_path, associated_branch, associated_ref, create_branch_flow_completed)
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
    let (env_mode, branch, worktree_path, working_directory, associated_path, associated_branch, associated_ref, create_branch_flow_completed) =
        resolve_workspace_fields(command, is_group);
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
    // Upstream ThreadCreatedPayload requires modelSelection as a real object
    // and runtimeMode as a RuntimeMode string; a null key fails schema decode
    // during replay on the upstream reader.
    let model_selection = command
        .get("modelSelection")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| json!({"provider": "codex", "model": "gpt-6-astra"}));
    let runtime_mode = command
        .get("runtimeMode")
        .and_then(|v| v.as_str())
        .unwrap_or("full-access");
    put(&mut ep, "modelSelection", model_selection);
    put(&mut ep, "runtimeMode", json!(runtime_mode));
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
            Value::Object(ep.clone()).to_string(),
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

    project_thread_created(&tx, &ep)?;

    tx.commit().map_err(|e| err(e.to_string()))?;
    Ok(json!({ "sequence": sequence }))
}

const PROVIDER_SEND_TURN_MAX_INPUT_CHARS: usize = 120_000;


fn project_thread_created(conn: &Connection, p: &Map<String, Value>) -> Result<(), WsErr> {
    let g = |k: &str| p.get(k).cloned().unwrap_or(Value::Null);
    let str_or_null = |v: &Value| -> Option<String> { v.as_str().map(str::to_string) };
    let thread_id = g("threadId").as_str().unwrap_or_default().to_string();
    let project_id = g("projectId").as_str().unwrap_or_default().to_string();
    let title = g("title").as_str().unwrap_or_default().to_string();
    let created_at = g("createdAt").as_str().unwrap_or_default().to_string();
    let last_known_pr = g("lastKnownPr");
    let pull_requests = g("pullRequests");
    let sidechat_context = g("sidechatContext");
    let sidechat_last_activity_at = g("sidechatLastActivityAt");
    let handoff = g("handoff");
    let sidechat_source = g("sidechatSourceThreadId");
    conn.execute(
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
           ?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6, NULL, ?7, ?8, ?9, ?30, ?10, ?31, \
           ?11, ?12, ?13, NULL, ?14, ?15, ?16, ?17, NULL, 0, 0, 0, ?18, ?19, \
           ?32, ?20, NULL, NULL, ?21, ?22, ?23, ?24, ?25, ?26, NULL, NULL, NULL, \
           NULL, NULL, ?27, NULL, NULL, NULL, ?28, NULL, NULL, ?33, NULL, NULL, NULL, NULL, NULL, ?29)",
        params![
            thread_id,
            project_id,
            title,
            str_or_null(&g("branch")),
            str_or_null(&g("worktreePath")),
            created_at,
            g("runtimeMode").as_str().unwrap_or("full-access"),
            g("interactionMode").as_str().unwrap_or("default"),
            g("modelSelection").to_string(),
            g("envMode").as_str().unwrap_or("local"),
            str_or_null(&g("associatedWorktreePath")),
            str_or_null(&g("associatedWorktreeBranch")),
            str_or_null(&g("associatedWorktreeRef")),
            g("parentThreadId").as_str(),
            g("subagentAgentId").as_str(),
            g("subagentNickname").as_str(),
            g("subagentRole").as_str(),
            if last_known_pr.is_null() { None } else { Some(last_known_pr.to_string()) },
            g("createBranchFlowCompleted").as_bool().unwrap_or(false) as i64,
            g("isPinned").as_bool().unwrap_or(false) as i64,
            g("creationSource").as_str(),
            g("sourceThreadId").as_str(),
            g("sourceTurnId").as_str(),
            g("gatewayOperationId").as_str(),
            g("gatewayOperationIndex").as_i64(),
            str_or_null(&g("workingDirectory")),
            if sidechat_last_activity_at.is_null() { None } else { Some(sidechat_last_activity_at.to_string().trim_matches('"').to_string()) },
            if sidechat_context.is_null() { None } else { Some(sidechat_context.to_string()) },
            if pull_requests.is_null() { "[]".to_string() } else { pull_requests.to_string() },
            if handoff.is_null() { None } else { Some(handoff.to_string()) },
            g("forkSourceThreadId").as_str(),
            str_or_null(&sidechat_source),
            g("forkSourceMessageId").as_str(),
        ],
    )
    .map_err(|e| err(format!("project thread: {e}")))?;
    Ok(())
}

fn thread_turn_start(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    for k in [
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
    // asyncUserInputResponse: upstream loads the referenced question message to
    // validate the answers, then steers the turn. With no provider runtime the
    // recorded response is best-effort: mark the question answered so the card
    // settles, then continue as a normal steer.
    let async_response = command.get("asyncUserInputResponse").cloned();
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
    let active_provider: Option<String> = command
        .get("modelSelection")
        .and_then(|m| m.get("provider"))
        .and_then(Value::as_str)
        .map(String::from)
        .or_else(|| {
            conn.query_row(
                "SELECT provider_name FROM projection_thread_sessions                  WHERE thread_id = ?1",
                params![thread_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten()
        })
        .or_else(|| {
            conn.query_row(
                "SELECT json_extract(model_selection_json, '$.provider')                  FROM projection_threads WHERE thread_id = ?1",
                params![thread_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten()
        });
    let provider_turn_live = crate::acp_runner::has_running_turn(&thread_id)
        || crate::codex_runner::has_running_turn(&thread_id)
        || crate::claude_runner::has_running_turn(&thread_id)
        || crate::opencode_runner::has_running_turn(&thread_id)
        || crate::pi_runner::has_running_turn(&thread_id);
    // Codex app-server and Claude streaming-input both accept native mid-turn
    // input. Unsupported steers queue and interrupt the active turn instead
    // of spawning a parallel provider turn.
    let native_steer = dispatch_mode == "steer"
        && provider_turn_live
        && matches!(active_provider.as_deref(), Some("codex" | "claudeAgent" | "pi"));
    let should_queue = row.parent_thread_id.is_none()
        && (row.claude_cache_review.is_some()
            || (provider_turn_live && !native_steer));
    let starts_new_turn = !native_steer;

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
    mp.insert("startsNewTurn".into(), json!(starts_new_turn));
    mp.insert("turnId".into(), Value::Null);
    mp.insert("streaming".into(), json!(false));
    mp.insert("source".into(), json!("native"));
    mp.insert("createdAt".into(), json!(created_at));
    mp.insert("updatedAt".into(), json!(created_at));
    let mut async_answer_index: Option<usize> = None;
    if let Some(resp) = async_response.as_ref().and_then(Value::as_object) {
        if let Some(question_message_id) = s(resp, "messageId") {
            let answers = resp.get("answers").cloned().unwrap_or(json!([]));
            async_answer_index = Some(events.len());
            events.push(out_event(
                "thread.async-user-input-answered",
                &thread_id,
                &created_at,
                json!({
                    "threadId": thread_id,
                    "messageId": question_message_id,
                    "response": {
                        "messageId": message_id,
                        "answers": answers,
                    },
                }),
            ));
        }
    }
    let message_event = out_event(
        "thread.message-sent",
        &thread_id,
        &created_at,
        Value::Object(mp),
    );
    let message_event_id = message_event.event_id.clone();
    events.push(message_event);

    // A non-queued turn moves the session to 'starting' and applies the
    // turn's runtimeMode to the session row — upstream's starting
    // session-set. The ACP runner reads it back for session/set_mode and
    // re-emits it on its own session-set events; queued turns do the same
    // when dispatch-queued runs them.
    let start_provider: Option<String> = if should_queue || native_steer {
        None
    } else {
        active_provider.clone()
    };
    let starting_ev = if should_queue {
        None
    } else {
        let mut ev = out_event(
            "thread.session-set",
            &thread_id,
            &created_at,
            json!({
                "threadId": thread_id,
                "session": {
                    "threadId": thread_id,
                    "status": "starting",
                    "providerName": start_provider.as_deref(),
                    "runtimeMode": command.get("runtimeMode").cloned().unwrap_or(Value::Null),
                    "activeTurnId": null,
                    "lastError": null,
                    "updatedAt": created_at,
                }
            }),
        );
        ev.causation_event_id = Some(message_event_id.clone());
        Some(ev)
    };

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
    turn_event.causation_event_id = starting_ev
        .as_ref()
        .map(|e| e.event_id.clone())
        .or(Some(message_event_id));
    if let Some(ev) = starting_ev {
        events.push(ev);
    }
    let interrupt_after_queue = should_queue
        && dispatch_mode == "steer"
        && row.claude_cache_review.is_none()
        && provider_turn_live;
    let queued_turn_event_id = turn_event.event_id.clone();
    events.push(turn_event);
    if interrupt_after_queue {
        let mut interrupt_ev = out_event(
            "thread.turn-interrupt-requested",
            &thread_id,
            &created_at,
            json!({
                "threadId": thread_id,
                "createdAt": created_at,
            }),
        );
        interrupt_ev.causation_event_id = Some(queued_turn_event_id);
        events.push(interrupt_ev);
    }

    // Record the async-user-input answer on the referenced question message so
    // the card settles as responded; resolve any open pending interaction too.
    if let Some(resp) = async_response.as_ref().and_then(Value::as_object) {
        if let Some(mid) = s(resp, "messageId") {
            let answers = resp.get("answers").cloned().unwrap_or(json!([]));
            // The response row and sequence are written after commit, when the
            // async-user-input-answered event sequence is known.
            let _ = answers;
            conn.execute(
                "DELETE FROM projection_pending_interactions \
                 WHERE thread_id=?1 AND status='pending' AND interaction_kind='userInput'",
                params![thread_id],
            )
            .map_err(|e| err(format!("resolve pending interaction: {e}")))?;
        }
    }

    let seq = commit_events(&conn, &events, &command_id)?;
    if let (Some(resp), Some(index)) = (async_response.as_ref().and_then(Value::as_object), async_answer_index) {
        if let Some(question_message_id) = s(resp, "messageId") {
            let answers = resp.get("answers").cloned().unwrap_or(json!([]));
            let response = json!({ "messageId": message_id, "answers": answers });
            let response_sequence = seq - (events.len() as i64 - 1 - index as i64);
            conn.execute(
                "UPDATE projection_thread_messages \
                 SET async_user_input_json = json_set(async_user_input_json, '$.response', json(?3), '$.responseSequence', ?4) \
                 WHERE thread_id=?1 AND message_id=?2 AND async_user_input_json IS NOT NULL",
                params![thread_id, question_message_id, response.to_string(), response_sequence],
            )
            .map_err(|e| err(format!("record async-user-input response sequence: {e}")))?;
        }
    }

    // --- projection: message row + thread shell fields ---------------------
    let attachments = message.get("attachments").cloned().unwrap_or(json!([]));
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_messages \
         (message_id, thread_id, turn_id, role, text, is_streaming, created_at, updated_at, \
          attachments_json, source, skills_json, mentions_json, dispatch_mode, dispatch_origin, \
          starts_new_turn) \
         VALUES (?1, ?2, NULL, 'user', ?3, 0, ?4, ?4, ?5, 'native', ?6, ?7, ?8, ?9, ?10)",
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
            starts_new_turn as i64,
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
        if !runtime_mode.is_null() {
            sets.push(("runtime_mode", runtime_mode.clone()));
        }
        if !interaction_mode.is_null() {
            sets.push(("interaction_mode", interaction_mode.clone()));
        }
    }
    // Fresh thread (no turn, no session): adopt the requested model selection.
    if row.latest_turn_id.is_none()
        && let Some(ms) = command.get("modelSelection").filter(|v| v.is_object())
    {
        sets.push(("model_selection_json", json!(ms.to_string())));
    }
    if events.iter().any(|e| e.event_type == "thread.meta-updated") {
        sets.push(("snoozed_until", Value::Null));
        sets.push(("snooze_reminder_at", Value::Null));
    }
    update_projection(&conn, &thread_id, &sets)?;

    if interrupt_after_queue {
        cancel_any_provider_turn(&thread_id);
    }
    if native_steer {
        let delivered = match active_provider.as_deref() {
            Some("codex") => crate::codex_runner::steer_turn(&thread_id, &text),
            Some("pi") => crate::pi_runner::steer_turn(&thread_id, &text),
            _ => crate::claude_runner::steer_turn(&thread_id, &text),
        };
        if !delivered {
            return Err(err("failed to deliver steering message to the active provider turn"));
        }
    }

    // Drive a real provider turn unless the message queued or rode the live turn.
    if !should_queue && !native_steer {
        conn.execute(
            "INSERT OR REPLACE INTO projection_thread_sessions \
             (thread_id, status, provider_name, provider_session_id, provider_thread_id, \
              active_turn_id, last_error, updated_at, runtime_mode) \
             VALUES (?1, 'starting', ?2, NULL, NULL, NULL, NULL, ?3, ?4)",
            params![
                thread_id,
                start_provider,
                created_at,
                s(command, "runtimeMode").unwrap_or_else(|| "full-access".into())
            ],
        )
        .map_err(|e| err(e.to_string()))?;
        let provider = start_provider;
        if provider.as_deref().is_some_and(crate::codex_runner::supports_provider)
            || provider.as_deref().is_some_and(crate::claude_runner::supports_provider)
            || provider.as_deref().is_some_and(crate::opencode_runner::supports_provider)
            || provider.as_deref().is_some_and(crate::pi_runner::supports_provider)
            || provider.as_deref().is_some_and(crate::acp_runner::supports_provider)
        {
            let cwd: Option<String> = conn
                .query_row(
                    "SELECT p.workspace_root FROM projection_projects p \
                     JOIN projection_threads t ON t.project_id = p.project_id \
                     WHERE t.thread_id = ?1",
                    params![thread_id],
                    |r| r.get(0),
                )
                .ok();
            if let Some(cwd) = cwd {
                let pname = provider.clone().expect("supported provider");
                if crate::codex_runner::supports_provider(&pname) {
                    crate::codex_runner::start_turn(thread_id.clone(), message_id.clone(), text.clone(), cwd);
                } else if crate::claude_runner::supports_provider(&pname) {
                    crate::claude_runner::start_turn(thread_id.clone(), message_id.clone(), text.clone(), cwd);
                } else if crate::opencode_runner::supports_provider(&pname) {
                    crate::opencode_runner::start_turn(thread_id.clone(), message_id.clone(), text.clone(), cwd);
                } else if crate::pi_runner::supports_provider(&pname) {
                    crate::pi_runner::start_turn(thread_id.clone(), message_id.clone(), text.clone(), cwd);
                } else {
                    crate::acp_runner::start_turn(
                        pname,
                        thread_id.clone(),
                        message_id.clone(),
                        text.clone(),
                        cwd,
                    );
                }
            }
        }
    }

    Ok(json!({ "sequence": seq }))
}

fn cancel_any_provider_turn(thread_id: &str) {
    crate::acp_runner::cancel_turn(thread_id);
    crate::codex_runner::cancel_turn(thread_id);
    crate::claude_runner::cancel_turn(thread_id);
    crate::opencode_runner::cancel_turn(thread_id);
    crate::pi_runner::cancel_turn(thread_id);
}

/// After a provider turn completes, start the oldest queued user message
/// that no turn has consumed yet. Queued messages are recorded with
/// dispatch_mode 'queue'; consumption is proven by a projection_turns row
/// whose pending_message_id is the message. Recursive safety: the next
/// runner repeats the check when it completes, draining the queue one
/// turn at a time.
pub(crate) fn continue_queued_turn(thread_id: &str) {
    let Ok(conn) = open_state_rw() else {
        return;
    };
    let _ = conn.execute_batch("PRAGMA busy_timeout = 5000;");
    let next: Option<(String, String)> = conn
        .query_row(
            "SELECT message_id, text FROM projection_thread_messages              WHERE thread_id = ?1 AND role = 'user' AND dispatch_mode IN ('queue', 'steer') AND starts_new_turn = 1              AND message_id NOT IN (                SELECT COALESCE(pending_message_id, '') FROM projection_turns WHERE thread_id = ?1              )              ORDER BY created_at ASC LIMIT 1",
            params![thread_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .ok();
    let Some((message_id, text)) = next else {
        return;
    };
    let provider: Option<String> = conn
        .query_row(
            "SELECT model_selection_json FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| {
            v.get("provider")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        });
    let cwd: Option<String> = conn
        .query_row(
            "SELECT p.workspace_root FROM projection_projects p              JOIN projection_threads t ON t.project_id = p.project_id              WHERE t.thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok();
    let (Some(provider), Some(cwd)) = (provider, cwd) else {
        return;
    };
    let tid = thread_id.to_string();
    if crate::codex_runner::supports_provider(&provider) {
        crate::codex_runner::start_turn(tid, message_id, text, cwd);
    } else if crate::claude_runner::supports_provider(&provider) {
        crate::claude_runner::start_turn(tid, message_id, text, cwd);
    } else if crate::opencode_runner::supports_provider(&provider) {
        crate::opencode_runner::start_turn(tid, message_id, text, cwd);
    } else if crate::pi_runner::supports_provider(&provider) {
        crate::pi_runner::start_turn(tid, message_id, text, cwd);
    } else if crate::acp_runner::supports_provider(&provider) {
        crate::acp_runner::start_turn(provider, tid, message_id, text, cwd);
    }
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

/// `thread.activity.append`: append a caller-supplied activity exactly once,
/// carrying requestId metadata when the payload names the interaction it belongs to.
fn thread_activity_append(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let activity = command
        .get("activity")
        .filter(|v| v.is_object())
        .cloned()
        .ok_or_else(|| err("thread.activity.append missing activity"))?;
    for key in ["id", "tone", "kind", "summary", "createdAt"] {
        if activity.get(key).and_then(Value::as_str).is_none() {
            return Err(err(format!(
                "thread.activity.append activity missing {key}"
            )));
        }
    }
    if !matches!(
        activity.get("tone").and_then(Value::as_str),
        Some("info" | "tool" | "approval" | "error")
    ) {
        return Err(err("thread.activity.append activity has an invalid tone"));
    }
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let thread = require_thread(&conn, &thread_id)?;
    if thread.deleted_at.is_some() {
        return Err(err(format!("thread {thread_id} not found")));
    }
    if command
        .get("requireUnarchived")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && thread.archived_at.is_some()
    {
        return Err(err(format!(
            "Thread '{thread_id}' is already archived and cannot handle command 'thread.activity.append'."
        )));
    }
    let mut ev = out_event(
        "thread.activity-appended",
        &thread_id,
        &created_at,
        json!({
            "threadId": thread_id,
            "activity": activity,
        }),
    );
    if let Some(request_id) = activity
        .get("payload")
        .and_then(Value::as_object)
        .and_then(|payload| payload.get("requestId"))
        .and_then(Value::as_str)
    {
        ev.metadata = Some(json!({ "requestId": request_id }));
    }
    let events = vec![ev];
    let seq = commit_events(&conn, &events, &command_id)?;
    append_activity_projection(&conn, &thread_id, &activity, seq)?;
    update_projection(&conn, &thread_id, &[("updated_at", json!(created_at))])?;
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

pub(crate) fn project_create(command: &Map<String, Value>) -> Result<Value, WsErr> {
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
    // Same starting session-set the immediate path emits: it applies the
    // turn's runtimeMode to the session row (the ACP runner reads it back
    // for session/set_mode and re-emits it on its own session-set events).
    let provider_for_start = model_sel
        .get("provider")
        .and_then(Value::as_str)
        .map(String::from)
        .or_else(|| {
            conn.query_row(
                "SELECT provider_name FROM projection_thread_sessions \
                 WHERE thread_id = ?1",
                params![thread_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .ok()
            .flatten()
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
                "providerName": provider_for_start,
                "runtimeMode": command.get("runtimeMode").cloned().unwrap_or(Value::Null),
                "activeTurnId": null,
                "lastError": null,
                "updatedAt": created_at,
            }
        }),
    );
    let mut requested = out_event(
        "thread.turn-start-requested",
        &thread_id,
        &created_at,
        Value::Object(payload),
    );
    requested.causation_event_id = Some(starting.event_id.clone());
    let events = vec![starting, requested];
    let seq = commit_events(&conn, &events, &command_id)?;
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_sessions \
         (thread_id, status, provider_name, provider_session_id, provider_thread_id, \
          active_turn_id, last_error, updated_at, runtime_mode) \
         VALUES (?1, 'starting', ?2, NULL, NULL, NULL, NULL, ?3, ?4)",
        params![
            thread_id,
            provider_for_start,
            created_at,
            s(command, "runtimeMode").unwrap_or_else(|| "full-access".into())
        ],
    )
    .map_err(|e| err(e.to_string()))?;

    // A queued provider turn starts the same runner once dispatched.
    let provider = provider_for_start;
    if provider.as_deref().is_some_and(crate::codex_runner::supports_provider)
        || provider.as_deref().is_some_and(crate::claude_runner::supports_provider)
        || provider.as_deref().is_some_and(crate::opencode_runner::supports_provider)
        || provider.as_deref().is_some_and(crate::pi_runner::supports_provider)
        || provider.as_deref().is_some_and(crate::acp_runner::supports_provider)
    {
        let message = conn
            .query_row(
                "SELECT message_id, text FROM projection_thread_messages \
                 WHERE thread_id = ?1 AND role = 'user' \
                 ORDER BY created_at DESC LIMIT 1",
                params![thread_id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )
            .ok();
        let cwd: Option<String> = conn
            .query_row(
                "SELECT p.workspace_root FROM projection_projects p \
                 JOIN projection_threads t ON t.project_id = p.project_id \
                 WHERE t.thread_id = ?1",
                params![thread_id],
                |r| r.get(0),
            )
            .ok();
        if let (Some((message_id, text)), Some(cwd)) = (message, cwd) {
            let pname = provider.clone().expect("supported provider");
            if crate::codex_runner::supports_provider(&pname) {
                crate::codex_runner::start_turn(thread_id.clone(), message_id, text, cwd);
            } else if crate::claude_runner::supports_provider(&pname) {
                crate::claude_runner::start_turn(thread_id.clone(), message_id, text, cwd);
            } else if crate::opencode_runner::supports_provider(&pname) {
                crate::opencode_runner::start_turn(thread_id.clone(), message_id, text, cwd);
            } else if crate::pi_runner::supports_provider(&pname) {
                crate::pi_runner::start_turn(thread_id.clone(), message_id, text, cwd);
            } else {
                crate::acp_runner::start_turn(pname, thread_id.clone(), message_id, text, cwd);
            }
        }
    }

    Ok(json!({ "sequence": seq }))
}


pub fn upsert_proposed_plan_projection(thread_id: &str, plan: &Value) -> Result<(), WsErr> {
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    let plan_id = plan.get("id").and_then(Value::as_str).unwrap_or("");
    let markdown = plan.get("planMarkdown").and_then(Value::as_str).unwrap_or("");
    if plan_id.is_empty() || markdown.is_empty() {
        return Ok(());
    }
    let now = iso_now_ms();
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_proposed_plans \
         (plan_id, thread_id, turn_id, plan_markdown, implemented_at, implementation_thread_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, COALESCE((SELECT created_at FROM projection_thread_proposed_plans WHERE plan_id=?1), ?7), ?7)",
        params![
            plan_id,
            thread_id,
            plan.get("turnId").and_then(Value::as_str),
            markdown,
            plan.get("implementedAt").and_then(Value::as_str),
            plan.get("implementationThreadId").and_then(Value::as_str),
            now,
        ],
    )
    .map_err(|e| err(format!("upsert proposed plan: {e}")))?;
    let actionable: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projection_thread_proposed_plans WHERE thread_id=?1 AND implemented_at IS NULL",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "UPDATE projection_threads SET has_actionable_proposed_plan=?2, updated_at=?3 WHERE thread_id=?1",
        params![thread_id, if actionable > 0 { 1 } else { 0 }, now],
    )
    .map_err(|e| err(format!("update proposed plan summary: {e}")))?;
    Ok(())
}

fn proposed_plan_upsert(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    require_thread(&conn, &thread_id)?;
    let proposed_plan = command.get("proposedPlan").cloned().unwrap_or(Value::Null);
    let events = vec![out_event(
        "thread.proposed-plan-upserted",
        &thread_id,
        &created_at,
        json!({
            "threadId": thread_id,
            "proposedPlan": proposed_plan,
        }),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    upsert_proposed_plan_projection(&thread_id, &proposed_plan)?;
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


fn apply_conversation_rollback_projection(
    conn: &Connection,
    thread_id: &str,
    message_id: &str,
    removed_turn_ids: &[String],
    event_sequence: i64,
    occurred_at: &str,
) -> Result<(), WsErr> {
    let target: Option<(String, String)> = conn
        .query_row(
            "SELECT created_at, message_id FROM projection_thread_messages \
             WHERE thread_id=?1 AND message_id=?2 AND role='user'",
            params![thread_id, message_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    if let Some((target_created, target_id)) = target {
        conn.execute(
            "DELETE FROM projection_thread_messages \
             WHERE thread_id=?1 AND (created_at > ?2 OR (created_at = ?2 AND message_id >= ?3))",
            params![thread_id, target_created, target_id],
        )
        .map_err(|e| err(format!("rollback messages: {e}")))?;
        conn.execute(
            "DELETE FROM message_text_chunks \
             WHERE thread_id=?1 AND message_id NOT IN (SELECT message_id FROM projection_thread_messages WHERE thread_id=?1)",
            params![thread_id],
        )
        .ok();
        conn.execute(
            "DELETE FROM message_text_segments \
             WHERE thread_id=?1 AND message_id NOT IN (SELECT message_id FROM projection_thread_messages WHERE thread_id=?1)",
            params![thread_id],
        )
        .ok();
    }
    for turn_id in removed_turn_ids {
        conn.execute(
            "DELETE FROM projection_turns WHERE thread_id=?1 AND turn_id=?2",
            params![thread_id, turn_id],
        )
        .map_err(|e| err(format!("rollback turns: {e}")))?;
        conn.execute(
            "DELETE FROM projection_thread_activities WHERE thread_id=?1 AND turn_id=?2",
            params![thread_id, turn_id],
        )
        .map_err(|e| err(format!("rollback activities: {e}")))?;
        conn.execute(
            "DELETE FROM projection_thread_proposed_plans WHERE thread_id=?1 AND turn_id=?2",
            params![thread_id, turn_id],
        )
        .map_err(|e| err(format!("rollback plans: {e}")))?;
        conn.execute(
            "DELETE FROM projection_pending_interactions WHERE thread_id=?1 AND turn_id=?2",
            params![thread_id, turn_id],
        )
        .map_err(|e| err(format!("rollback pending interactions: {e}")))?;
    }
    conn.execute(
        "UPDATE projection_thread_messages SET text_event_sequence=?2 WHERE thread_id=?1",
        params![thread_id, event_sequence],
    )
    .ok();
    let latest_user: Option<String> = conn
        .query_row(
            "SELECT MAX(created_at) FROM projection_thread_messages WHERE thread_id=?1 AND role='user'",
            params![thread_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let latest_turn: Option<String> = conn
        .query_row(
            "SELECT turn_id FROM projection_turns WHERE thread_id=?1 ORDER BY requested_at DESC LIMIT 1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let actionable: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projection_thread_proposed_plans WHERE thread_id=?1 AND implemented_at IS NULL",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let pending_approvals: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projection_pending_interactions WHERE thread_id=?1 AND status='pending' AND interaction_kind='approval'",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let pending_inputs: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projection_pending_interactions WHERE thread_id=?1 AND status='pending' AND interaction_kind='userInput'",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "UPDATE projection_threads SET latest_turn_id=?2, latest_user_message_at=?3, latest_human_message_at=?3, \
         has_actionable_proposed_plan=?4, pending_approval_count=?5, pending_user_input_count=?6, updated_at=?7 \
         WHERE thread_id=?1",
        params![thread_id, latest_turn, latest_user, actionable > 0, pending_approvals, pending_inputs, occurred_at],
    )
    .map_err(|e| err(format!("rollback thread summary: {e}")))?;
    Ok(())
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
    let rollback_payload = payload.clone();
    let events = vec![out_event(
        "thread.conversation-rolled-back",
        &thread_id,
        &created_at,
        Value::Object(payload),
    )];
    let seq = commit_events(&conn, &events, &command_id)?;
    let removed_turn_ids: Vec<String> = rollback_payload
        .get("removedTurnIds")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();
    if let Some(message_id) = rollback_payload.get("messageId").and_then(Value::as_str) {
        apply_conversation_rollback_projection(
            &conn,
            &thread_id,
            message_id,
            &removed_turn_ids,
            seq,
            &created_at,
        )?;
    }
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

struct LineageThread {
    id: String,
    project_id: String,
    title: String,
    fork_source_thread_id: Option<String>,
    is_sidechat: bool,
}

fn parse_fork_version(title: &str) -> (String, i64) {
    if let Some(end) = title.strip_suffix(')') {
        if let Some(pos) = end.rfind(" (") {
            if let Ok(n) = end[pos + 2..].parse::<i64>() {
                if n >= 2 {
                    return (end[..pos].to_string(), n);
                }
            }
        }
    }
    (title.to_string(), 1)
}

fn find_lineage_root<'a>(
    source: &'a LineageThread,
    by_id: &'a std::collections::HashMap<String, LineageThread>,
) -> (&'a LineageThread, bool) {
    let mut visited = std::collections::HashSet::new();
    visited.insert(source.id.clone());
    let mut current = source;
    while let Some(pid) = &current.fork_source_thread_id {
        match by_id.get(pid) {
            Some(p) if p.project_id == source.project_id && !visited.contains(&p.id) => {
                visited.insert(p.id.clone());
                current = p;
            }
            _ => return (current, false),
        }
    }
    (current, true)
}

fn build_fork_thread_title(
    conn: &Connection,
    source: &LineageThread,
    project_id: &str,
) -> Result<String, WsErr> {
    let mut stmt = conn
        .prepare(
            "SELECT thread_id, project_id, title, fork_source_thread_id, \
             sidechat_source_thread_id IS NOT NULL OR sidechat_context_json IS NOT NULL \
             FROM projection_threads WHERE project_id = ?1 AND deleted_at IS NULL",
        )
        .map_err(|e| err(e.to_string()))?;
    let threads: Vec<LineageThread> = stmt
        .query_map(params![project_id], |r| {
            Ok(LineageThread {
                id: r.get(0)?,
                project_id: r.get(1)?,
                title: r.get(2)?,
                fork_source_thread_id: r.get(3)?,
                is_sidechat: r.get::<_, bool>(4)?,
            })
        })
        .map_err(|e| err(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();
    let by_id: std::collections::HashMap<String, LineageThread> =
        threads.into_iter().map(|t| (t.id.clone(), t)).collect();
    let (root_thread, root_complete) = find_lineage_root(source, &by_id);
    let (lineage_base, lineage_version) = if root_complete {
        parse_fork_version(&root_thread.title)
    } else {
        parse_fork_version(&source.title)
    };
    let source_root_id = root_thread.id.clone();
    let family: Vec<&LineageThread> = by_id
        .values()
        .filter(|t| {
            if t.project_id != project_id || t.is_sidechat || !root_complete {
                return false;
            }
            let (root, complete) = find_lineage_root(t, &by_id);
            complete && root.id == source_root_id
        })
        .collect();
    let latest_named_version = family.iter().fold(lineage_version, |latest, t| {
        let (base, v) = parse_fork_version(&t.title);
        if base == lineage_base {
            latest.max(v)
        } else {
            latest
        }
    });
    let next_version = (2)
        .max(family.len() as i64 + 1)
        .max(latest_named_version + 1);
    Ok(format!("{lineage_base} ({next_version})"))
}

fn lineage_create(command: &Map<String, Value>, is_handoff: bool) -> Result<Value, WsErr> {
    let ctype = if is_handoff {
        "thread.handoff.create"
    } else {
        "thread.fork.create"
    };
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("missing threadId"))?;
    let project_id = s(command, "projectId").ok_or_else(|| err("missing projectId"))?;
    let source_thread_id =
        s(command, "sourceThreadId").ok_or_else(|| err("missing sourceThreadId"))?;
    let created_at = s(command, "createdAt").ok_or_else(|| err("missing createdAt"))?;

    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    let project_kind: String = conn
        .query_row(
            "SELECT kind FROM projection_projects WHERE project_id = ?1 AND deleted_at IS NULL",
            params![project_id],
            |r| r.get(0),
        )
        .map_err(|_| {
            err(format!(
                "Project '{project_id}' does not exist for command '{ctype}'."
            ))
        })?;

    // requireThread(sourceThreadId)
    let src = conn
        .query_row(
            "SELECT project_id, title, handoff_json, model_selection_json, deleted_at, \
             fork_source_thread_id, sidechat_source_thread_id, sidechat_context_json \
             FROM projection_threads WHERE thread_id = ?1",
            params![source_thread_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<String>>(7)?,
                ))
            },
        )
        .map_err(|_| {
            err(format!(
                "Thread '{source_thread_id}' does not exist for command '{ctype}'."
            ))
        })?;
    let (
        src_project_id,
        src_title,
        src_handoff,
        src_model_sel,
        src_deleted,
        src_fork,
        src_sc_tid,
        src_sc_ctx,
    ) = src;
    if src_deleted.is_some() {
        return Err(err(format!(
            "Thread '{source_thread_id}' was deleted and cannot handle command '{ctype}'."
        )));
    }

    // requireThreadAbsent(threadId)
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if exists {
        return Err(err(format!(
            "Thread '{thread_id}' already exists and cannot be created twice."
        )));
    }

    // validateAutoRuntimeMode
    let runtime_mode = s(command, "runtimeMode").unwrap_or_default();
    let model_sel = opt(command, "modelSelection");
    if let Some(issue) = auto_runtime_issue(&runtime_mode, &model_sel) {
        return Err(err(issue));
    }

    if src_project_id != project_id {
        return Err(err(format!(
            "Source thread '{source_thread_id}' belongs to a different project."
        )));
    }

    if is_handoff && src_handoff.is_some() {
        let has_native: bool = conn
            .query_row(
                "SELECT 1 FROM projection_thread_messages WHERE thread_id = ?1 \
                 AND role IN ('user','assistant') AND source IN ('native','async-user-input') \
                 AND is_streaming = 0 LIMIT 1",
                params![source_thread_id],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if !has_native {
            return Err(err(format!(
                "Source thread '{source_thread_id}' must contain at least one native chat message after handoff before it can be handed off again."
            )));
        }
    }

    let sidechat_source = opt(command, "sidechatSourceThreadId");
    let title = if is_handoff || !sidechat_source.is_null() {
        s(command, "title").ok_or_else(|| err("missing title"))?
    } else {
        build_fork_thread_title(
            &conn,
            &LineageThread {
                id: source_thread_id.clone(),
                project_id: src_project_id.clone(),
                title: src_title.clone(),
                fork_source_thread_id: src_fork.clone(),
                is_sidechat: src_sc_tid.is_some() || src_sc_ctx.is_some(),
            },
            &project_id,
        )?
    };

    let is_group = is_group_container_kind(&project_kind);
    let (
        env_mode,
        branch,
        worktree_path,
        working_directory,
        associated_path,
        associated_branch,
        associated_ref,
        create_branch_flow_completed,
    ) = resolve_workspace_fields(command, is_group);

    let source_provider: Option<String> = src_model_sel
        .as_deref()
        .and_then(|j| serde_json::from_str::<Value>(j).ok())
        .and_then(|v| v.get("provider").and_then(Value::as_str).map(str::to_string));

    let mut ep = Map::new();
    let put = |ep: &mut Map<String, Value>, k: &str, v: Value| {
        ep.insert(k.to_string(), v);
    };
    put(&mut ep, "threadId", json!(thread_id));
    put(&mut ep, "projectId", json!(project_id));
    put(&mut ep, "title", json!(title));
    // Upstream ThreadCreatedPayload requires modelSelection as a real object
    // and runtimeMode as a RuntimeMode string; a null key fails schema decode
    // during replay on the upstream reader.
    let model_selection = command
        .get("modelSelection")
        .filter(|v| v.is_object())
        .cloned()
        .unwrap_or_else(|| json!({"provider": "codex", "model": "gpt-6-astra"}));
    let runtime_mode = command
        .get("runtimeMode")
        .and_then(|v| v.as_str())
        .unwrap_or("full-access");
    put(&mut ep, "modelSelection", model_selection);
    put(&mut ep, "runtimeMode", json!(runtime_mode));
    put(
        &mut ep,
        "interactionMode",
        json!(s(command, "interactionMode").unwrap_or_else(|| "default".into())),
    );
    put(&mut ep, "envMode", json!(env_mode));
    put(&mut ep, "branch", branch);
    put(&mut ep, "worktreePath", worktree_path);
    put(&mut ep, "workingDirectory", working_directory);
    put(&mut ep, "associatedWorktreePath", associated_path);
    put(&mut ep, "associatedWorktreeBranch", associated_branch);
    put(&mut ep, "associatedWorktreeRef", associated_ref);
    put(
        &mut ep,
        "createBranchFlowCompleted",
        json!(create_branch_flow_completed),
    );
    put(&mut ep, "isPinned", json!(false));
    put(&mut ep, "parentThreadId", Value::Null);
    put(&mut ep, "subagentAgentId", Value::Null);
    put(&mut ep, "subagentNickname", Value::Null);
    put(&mut ep, "subagentRole", Value::Null);
    if is_handoff {
        put(&mut ep, "forkSourceThreadId", Value::Null);
        put(
            &mut ep,
            "handoff",
            json!({
                "sourceThreadId": source_thread_id,
                "sourceProvider": source_provider,
                "importedAt": created_at,
                "bootstrapStatus": "pending",
            }),
        );
    } else {
        put(&mut ep, "forkSourceThreadId", json!(source_thread_id));
        if let Some(tm) = command.get("throughMessageId") {
            put(&mut ep, "forkSourceMessageId", tm.clone());
        }
        put(&mut ep, "sidechatSourceThreadId", sidechat_source.clone());
        put(
            &mut ep,
            "sidechatLastActivityAt",
            if sidechat_source.is_null() {
                Value::Null
            } else {
                json!(created_at)
            },
        );
        put(&mut ep, "sidechatExpiredAt", Value::Null);
        put(&mut ep, "handoff", Value::Null);
    }
    put(&mut ep, "createdAt", json!(created_at));
    put(&mut ep, "updatedAt", json!(created_at));

    let mut events: Vec<OutEvent> = vec![out_event(
        "thread.created",
        &thread_id,
        &created_at,
        Value::Object(ep.clone()),
    )];
    let import_source = if is_handoff {
        "handoff-import"
    } else {
        "fork-import"
    };
    if let Some(msgs) = command.get("importedMessages").and_then(Value::as_array) {
        for m in msgs {
            let mut mp = Map::new();
            mp.insert("threadId".into(), json!(thread_id));
            mp.insert("messageId".into(), m.get("messageId").cloned().unwrap_or(Value::Null));
            mp.insert("role".into(), m.get("role").cloned().unwrap_or(json!("user")));
            mp.insert("text".into(), m.get("text").cloned().unwrap_or(json!("")));
            if let Some(a) = m.get("attachments") {
                mp.insert("attachments".into(), a.clone());
            }
            mp.insert("turnId".into(), Value::Null);
            mp.insert("streaming".into(), json!(false));
            mp.insert("source".into(), json!(import_source));
            mp.insert(
                "createdAt".into(),
                m.get("createdAt").cloned().unwrap_or(json!(created_at)),
            );
            mp.insert(
                "updatedAt".into(),
                m.get("updatedAt").cloned().unwrap_or(json!(created_at)),
            );
            events.push(out_event(
                "thread.message-sent",
                &thread_id,
                &created_at,
                Value::Object(mp),
            ));
        }
    }

    // Projections (upstream projector semantics).
    project_thread_created(&conn, &ep)?;
    for ev in events.iter().skip(1) {
        project_message_sent(&conn, &thread_id, &ev.payload)?;
    }
    let seq = commit_events(&conn, &events, &command_id)?;
    Ok(json!({ "sequence": seq }))
}

fn settle_turn_dispatch(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("missing commandId"))?;
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    let receipt: Option<(String, i64, Option<String>)> = conn
        .query_row(
            "SELECT status, result_sequence, error FROM orchestration_command_receipts \
             WHERE command_id = ?1",
            params![command_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| err(e.to_string()))?;

    if let Some((status, result_sequence, error)) = receipt {
        if status == "accepted" {
            return Ok(json!({ "status": "accepted", "sequence": result_sequence }));
        }
        return Err(err(error.unwrap_or_else(|| "Previously rejected.".into())));
    }

    let detail =
        "The connection was interrupted before this message was accepted. Please send it again.";
    let ctype = s(command, "type").unwrap_or_default();
    let (agg_kind, agg_id) = if ctype.starts_with("space.") {
        ("space", s(command, "spaceId").unwrap_or_default())
    } else if ctype.starts_with("project.") {
        ("project", s(command, "projectId").unwrap_or_default())
    } else {
        ("thread", s(command, "threadId").unwrap_or_default())
    };
    let snapshot_sequence: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) FROM orchestration_events",
            [],
            |r| r.get(0),
        )
        .map_err(|e| err(e.to_string()))?;
    conn.execute(
        "INSERT INTO orchestration_command_receipts \
         (command_id, aggregate_kind, aggregate_id, accepted_at, result_sequence, status, error, \
          fingerprint_version, command_fingerprint) \
         VALUES (?1, ?2, ?3, ?4, ?5, 'rejected', ?6, NULL, NULL)",
        params![command_id, agg_kind, agg_id, iso_now_ms(), snapshot_sequence, detail],
    )
    .map_err(|_| err("Failed to settle the original message."))?;
    Err(err(detail))
}

fn prepare_quit_resume(payload: &Value) -> Result<Value, WsErr> {
    let thread_ids: Vec<String> = payload
        .get("threadIds")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let continuation_prompt = payload
        .get("continuationPrompt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let now = iso_now_ms();
    let record_id = uuid_v4();
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    let mut seen = std::collections::HashSet::new();
    let mut recorded: Vec<(String, Option<String>)> = Vec::new();
    for thread_id in thread_ids {
        if !seen.insert(thread_id.clone()) {
            continue;
        }
        let row: Option<(Option<String>,)> = conn
            .query_row(
                "SELECT deleted_at FROM projection_threads WHERE thread_id = ?1",
                params![thread_id],
                |r| Ok((r.get(0)?,)),
            )
            .optional()
            .map_err(|e| err(e.to_string()))?;
        let Some((deleted_at,)) = row else { continue };
        if deleted_at.is_some() {
            continue;
        }
        let session: Option<(String, Option<String>)> = conn
            .query_row(
                "SELECT status, active_turn_id FROM projection_thread_sessions \
                 WHERE thread_id = ?1",
                params![thread_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|e| err(e.to_string()))?;
        let latest = latest_turn(&conn, &thread_id);
        let in_flight = session
            .as_ref()
            .map(|(st, at)| {
                (st != "error" && at.is_some()) || st == "starting" || st == "running"
            })
            .unwrap_or(false)
            || latest.as_ref().map(|(_, st, _)| st == "running").unwrap_or(false);
        if !in_flight {
            continue;
        }
        let turn_id = session
            .and_then(|(_, at)| at)
            .or_else(|| {
                latest.and_then(|(tid, st, _)| (st == "running").then_some(tid))
            });
        recorded.push((thread_id, turn_id));
    }

    let record = json!({
        "version": 1,
        "recordId": record_id,
        "recordedAt": now,
        "continuationPrompt": continuation_prompt,
        "threads": recorded.iter().map(|(tid, turn)| json!({"threadId": tid, "turnId": turn})).collect::<Vec<_>>(),
    });
    let record_path = PathBuf::from(state_dir()).join("quit-resume.json");
    let tmp_path = record_path.with_extension("json.tmp");
    std::fs::write(&tmp_path, format!("{}\n", record.to_string()))
        .and_then(|_| std::fs::rename(&tmp_path, &record_path))
        .map_err(|e| err(format!("persist quit-resume record: {e}")))?;

    // Best-effort interrupts (upstream: detached, errors only logged).
    for (thread_id, turn_id) in &recorded {
        let mut cmd = Map::new();
        cmd.insert("type".into(), json!("thread.turn.interrupt"));
        cmd.insert(
            "commandId".into(),
            json!(format!("quit-resume-interrupt:{record_id}:{thread_id}")),
        );
        cmd.insert("threadId".into(), json!(thread_id));
        if let Some(t) = turn_id {
            cmd.insert("turnId".into(), json!(t));
        }
        cmd.insert("createdAt".into(), json!(now));
        let _ = handle("orchestration.dispatchCommand", &Value::Object(cmd));
    }

    // 30s abandon sweep: if the quit never completed, drop our own record.
    {
        let path = record_path.clone();
        let rid = record_id.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(30));
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if serde_json::from_str::<Value>(raw.trim())
                    .ok()
                    .and_then(|v| v.get("recordId").and_then(Value::as_str).map(str::to_string))
                    .as_deref()
                    == Some(rid.as_str())
                {
                    let _ = std::fs::remove_file(&path);
                }
            }
        });
    }

    Ok(json!({
        "recordedThreadIds": recorded.iter().map(|(tid, _)| tid).collect::<Vec<_>>(),
        "recordedAt": now,
    }))
}

// ---- orchestration.repairState ----
//
// Rebuilds the derived projection state from the event log, matching upstream
// runProjectionRepair: capture the durable event fence, stage spaces/projects/
// projection_state into temp tables, clear them, replay every stored event
// through the projection writers, stamp every required projector cursor at the
// fence, then verify and return the refreshed read model. On any failure the
// staged backup is restored.

const REPAIR_METADATA_PROJECTORS: &[&str] = &[
    "projection.hot",
    "projection.projects",
    "projection.threads",
    "projection.thread-messages",
    "projection.thread-proposed-plans",
    "projection.thread-activities",
    "projection.thread-sessions",
    "projection.checkpoints",
];

const REPAIR_REQUIRED_PROJECTORS: &[&str] = &[
    "projection.hot",
    "projection.projects",
    "projection.threads",
    "projection.thread-shell-summaries",
    "projection.thread-messages",
    "projection.thread-proposed-plans",
    "projection.thread-activities",
    "projection.thread-sessions",
    "projection.thread-turns",
    "projection.checkpoints",
    "projection.pending-approvals",
];

/// thread.meta-updated payload key -> projection_threads column.
const META_PATCH_COLS: &[(&str, &str)] = &[
    ("title", "title"),
    ("parentThreadId", "parent_thread_id"),
    ("subagentAgentId", "subagent_agent_id"),
    ("subagentNickname", "subagent_nickname"),
    ("subagentRole", "subagent_role"),
    ("notes", "notes"),
    ("isPinned", "is_pinned"),
    ("settledAt", "settled_at"),
    ("snoozedUntil", "snoozed_until"),
    ("snoozeReminderAt", "snooze_reminder_at"),
    ("envMode", "env_mode"),
    ("branch", "branch"),
    ("worktreePath", "worktree_path"),
    ("workingDirectory", "working_directory"),
    ("associatedWorktreePath", "associated_worktree_path"),
    ("associatedWorktreeBranch", "associated_worktree_branch"),
    ("associatedWorktreeRef", "associated_worktree_ref"),
    ("goal", "goal"),
    ("goalStartedAt", "goal_started_at"),
    ("goalPausedAt", "goal_paused_at"),
    ("usageLimitRecoveryCancelledAt", "usage_limit_recovery_cancelled_at"),
    ("usageLimitRecoveryCancelledTurnId", "usage_limit_recovery_cancelled_turn_id"),
    ("usageLimitRecoveryLastResetAt", "usage_limit_recovery_last_reset_at"),
    ("updatedAt", "updated_at"),
];

/// thread.meta-updated payload keys stored as JSON text columns.
const META_JSON_COLS: &[(&str, &str)] = &[
    ("modelSelection", "model_selection_json"),
    ("handoff", "handoff_json"),
    ("pinnedMessages", "pinned_messages_json"),
    ("pullRequests", "pull_requests_json"),
    ("lastKnownPr", "last_known_pr_json"),
    ("goalAchievements", "goal_achievements_json"),
    ("usageLimitRecovery", "usage_limit_recovery_json"),
    ("usageLimitRecoverySource", "usage_limit_recovery_source_json"),
];

fn replay_session_projection(
    conn: &Connection,
    thread_id: &str,
    session: &Value,
) -> Result<(), WsErr> {
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
    Ok(())
}

fn replay_pinned_projection(
    conn: &Connection,
    event_type: &str,
    thread_id: &str,
    p: &Value,
    updated_at: &str,
) -> Result<(), WsErr> {
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
    match event_type {
        "thread.pinned-message-added" => {
            if let Some(pin) = p.get("pin") {
                let mid = pin.get("messageId").and_then(Value::as_str).unwrap_or("");
                if !pins
                    .iter()
                    .any(|x| x.get("messageId").and_then(Value::as_str) == Some(mid))
                {
                    pins.push(pin.clone());
                }
            }
        }
        "thread.pinned-message-removed" => {
            let mid = p.get("messageId").and_then(Value::as_str).unwrap_or("");
            pins.retain(|x| x.get("messageId").and_then(Value::as_str) != Some(mid));
        }
        "thread.pinned-message-done-set" | "thread.pinned-message-label-set" => {
            let mid = p.get("messageId").and_then(Value::as_str).unwrap_or("");
            if let Some(pin) = pins
                .iter_mut()
                .find(|x| x.get("messageId").and_then(Value::as_str) == Some(mid))
            {
                if event_type == "thread.pinned-message-done-set" {
                    pin["done"] = p.get("done").cloned().unwrap_or(Value::Null);
                } else {
                    pin["label"] = p.get("label").cloned().unwrap_or(Value::Null);
                }
            }
        }
        _ => {}
    }
    update_projection(
        conn,
        thread_id,
        &[
            ("pinned_messages_json", json!(Value::Array(pins).to_string())),
            ("updated_at", json!(updated_at)),
        ],
    )
}

/// Re-apply one stored event's projection writes. Mirrors exactly what the
/// dispatch-time writers emit so a replay rebuilds the same read model.
fn replay_projection_event(
    conn: &Connection,
    event_type: &str,
    stream_id: &str,
    payload: &Value,
    sequence: i64,
    occurred_at: &str,
) -> Result<(), WsErr> {
    let pg = |k: &str| payload.get(k).cloned().unwrap_or(Value::Null);
    let ps = |k: &str| payload.get(k).and_then(Value::as_str).unwrap_or("");
    match event_type {
        "space.created" => {
            conn.execute(
                "INSERT OR REPLACE INTO projection_spaces \
                 (space_id, name, icon, sort_order, created_at, updated_at, deleted_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL)",
                params![
                    ps("spaceId"),
                    ps("name"),
                    ps("icon"),
                    payload.get("sortOrder").and_then(Value::as_i64).unwrap_or(0),
                    json_to_sql(&pg("createdAt")),
                    json_to_sql(&pg("updatedAt")),
                ],
            )
            .map_err(|e| err(format!("space projection: {e}")))?;
        }
        "space.meta-updated" => {
            let mut sets: Vec<(&str, Value)> = Vec::new();
            for (k, col) in [("name", "name"), ("icon", "icon"), ("updatedAt", "updated_at")] {
                if let Some(v) = payload.get(k) {
                    sets.push((col, v.clone()));
                }
            }
            let assignments: Vec<String> = sets
                .iter()
                .enumerate()
                .map(|(i, (c, _))| format!("{c} = ?{}", i + 2))
                .collect();
            if !assignments.is_empty() {
                let mut params_vec: Vec<rusqlite::types::Value> =
                    vec![rusqlite::types::Value::Text(stream_id.to_string())];
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
            }
        }
        "space.order-updated" => {
            let ordered: Vec<String> = payload
                .get("orderedSpaceIds")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            for (i, sid) in ordered.iter().enumerate() {
                conn.execute(
                    "UPDATE projection_spaces SET sort_order = ?2, updated_at = ?3 \
                     WHERE space_id = ?1 AND sort_order != ?2",
                    params![sid, i as i64, json_to_sql(&pg("updatedAt"))],
                )
                .map_err(|e| err(e.to_string()))?;
            }
        }
        "space.deleted" => {
            conn.execute(
                "UPDATE projection_spaces SET deleted_at = ?2, updated_at = ?2 WHERE space_id = ?1",
                params![stream_id, json_to_sql(&pg("deletedAt"))],
            )
            .map_err(|e| err(e.to_string()))?;
            conn.execute(
                "UPDATE projection_projects SET space_id = NULL WHERE space_id = ?1",
                params![stream_id],
            )
            .map_err(|e| err(e.to_string()))?;
        }
        "project.created" => {
            conn.execute(
                "INSERT OR REPLACE INTO projection_projects \
                 (project_id, kind, title, workspace_root, scripts_json, created_at, updated_at, \
                  deleted_at, default_model_selection_json, is_pinned, space_id, additional_folders_json) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?9, ?10, ?11)",
                params![
                    ps("projectId"),
                    if payload.get("kind").is_some() { ps("kind").to_string() } else { "project".to_string() },
                    ps("title"),
                    ps("workspaceRoot"),
                    payload.get("scripts").map(|v| v.to_string()).unwrap_or_else(|| "[]".into()),
                    json_to_sql(&pg("createdAt")),
                    json_to_sql(&pg("updatedAt")),
                    payload.get("defaultModelSelection").map(|v| v.to_string()),
                    payload.get("isPinned").and_then(Value::as_bool).unwrap_or(false) as i64,
                    json_to_sql(&pg("spaceId")),
                    payload.get("additionalFolders").map(|v| v.to_string()).unwrap_or_else(|| "[]".into()),
                ],
            )
            .map_err(|e| err(format!("project projection: {e}")))?;
        }
        "project.meta-updated" => {
            let mut sets: Vec<(&str, Value)> = Vec::new();
            for (k, col) in [
                ("kind", "kind"),
                ("title", "title"),
                ("workspaceRoot", "workspace_root"),
                ("spaceId", "space_id"),
                ("updatedAt", "updated_at"),
            ] {
                if let Some(v) = payload.get(k) {
                    sets.push((col, v.clone()));
                }
            }
            for (k, col) in [
                ("defaultModelSelection", "default_model_selection_json"),
                ("scripts", "scripts_json"),
                ("additionalFolders", "additional_folders_json"),
            ] {
                if let Some(v) = payload.get(k) {
                    sets.push((
                        col,
                        if v.is_null() { Value::Null } else { json!(v.to_string()) },
                    ));
                }
            }
            if let Some(v) = payload.get("isPinned") {
                sets.push(("is_pinned", json!(v.as_bool().unwrap_or(false) as i64)));
            }
            let assignments: Vec<String> = sets
                .iter()
                .enumerate()
                .map(|(i, (c, _))| format!("{c} = ?{}", i + 2))
                .collect();
            if !assignments.is_empty() {
                let mut params_vec: Vec<rusqlite::types::Value> =
                    vec![rusqlite::types::Value::Text(stream_id.to_string())];
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
                .map_err(|e| err(e.to_string()))?;
            }
        }
        "project.deleted" => {
            conn.execute(
                "UPDATE projection_projects SET deleted_at = ?2, updated_at = ?2 \
                 WHERE project_id = ?1",
                params![stream_id, json_to_sql(&pg("deletedAt"))],
            )
            .map_err(|e| err(e.to_string()))?;
        }
        "thread.created" => {
            // The repair keeps existing thread rows: replay only heals missing
            // ones, mirroring the idempotent bootstrap apply upstream.
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM projection_threads WHERE thread_id = ?1",
                    params![stream_id],
                    |r| r.get(0),
                )
                .unwrap_or(false);
            if !exists
                && let Some(obj) = payload.as_object()
            {
                project_thread_created(conn, obj)?;
            }
        }
        "thread.deleted" => {
            update_projection(
                conn,
                stream_id,
                &[("deleted_at", pg("deletedAt"))],
            )?;
        }
        "thread.archived" => {
            update_projection(
                conn,
                stream_id,
                &[("archived_at", pg("archivedAt")), ("updated_at", pg("updatedAt"))],
            )?;
        }
        "thread.unarchived" => {
            update_projection(
                conn,
                stream_id,
                &[("archived_at", Value::Null), ("updated_at", pg("updatedAt"))],
            )?;
        }
        "thread.meta-updated" => {
            let mut sets: Vec<(&str, Value)> = Vec::new();
            for (k, col) in META_PATCH_COLS {
                if let Some(v) = payload.get(*k) {
                    sets.push((col, v.clone()));
                }
            }
            for (k, col) in META_JSON_COLS {
                if let Some(v) = payload.get(*k) {
                    sets.push((
                        col,
                        if v.is_null() { Value::Null } else { json!(v.to_string()) },
                    ));
                }
            }
            update_projection(conn, stream_id, &sets)?;
        }
        "thread.runtime-mode-set" => {
            update_projection(
                conn,
                stream_id,
                &[("runtime_mode", pg("runtimeMode")), ("updated_at", pg("updatedAt"))],
            )?;
        }
        "thread.interaction-mode-set" => {
            update_projection(
                conn,
                stream_id,
                &[("interaction_mode", pg("interactionMode")), ("updated_at", pg("updatedAt"))],
            )?;
        }
        "thread.claude-cache-set" => {
            update_projection(
                conn,
                stream_id,
                &[
                    ("claude_cache_review_json", pg("review")),
                    ("updated_at", pg("updatedAt")),
                ],
            )?;
        }
        "thread.sidechat-activity-recorded" => {
            update_projection(
                conn,
                stream_id,
                &[
                    ("sidechat_last_activity_at", pg("lastActivityAt")),
                    ("updated_at", json!(occurred_at)),
                ],
            )?;
        }
        "thread.sidechat-expired" => {
            update_projection(
                conn,
                stream_id,
                &[
                    ("sidechat_expired_at", pg("expiredAt")),
                    ("updated_at", json!(occurred_at)),
                ],
            )?;
        }
        "thread.pinned-message-added"
        | "thread.pinned-message-removed"
        | "thread.pinned-message-done-set"
        | "thread.pinned-message-label-set" => {
            replay_pinned_projection(conn, event_type, stream_id, payload, occurred_at)?;
        }
        "thread.session-set" => {
            replay_session_projection(conn, stream_id, &pg("session"))?;
        }
        "thread.message-sent" => {
            project_message_sent(conn, stream_id, payload)?;
        }
        "thread.activity-appended" => {
            let activity = pg("activity");
            if !activity.is_null() {
                append_activity_projection(conn, stream_id, &activity, sequence)?;
            }
        }
        "thread.conversation-rolled-back" => {
            let removed_turn_ids: Vec<String> = payload
                .get("removedTurnIds")
                .and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
                .unwrap_or_default();
            if !ps("messageId").is_empty() {
                apply_conversation_rollback_projection(
                    conn,
                    stream_id,
                    ps("messageId"),
                    &removed_turn_ids,
                    sequence,
                    occurred_at,
                )?;
            }
        }
        "thread.proposed-plan-upserted" => {
            upsert_proposed_plan_projection(stream_id, &pg("proposedPlan"))?;
        }
        // Request/record events and lifecycle markers with no projection writes.
        _ => {}
    }
    Ok(())
}

fn repair_state() -> Result<Value, WsErr> {
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    let repair_fence: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) FROM orchestration_events",
            [],
            |r| r.get(0),
        )
        .map_err(|e| {
            err(format!(
                "Failed to capture the durable event fence before repair: {e}"
            ))
        })?;

    // Stage the derived projection state for rollback.
    conn.execute_batch(
        "DROP TABLE IF EXISTS temp_repair_projection_spaces;\
         DROP TABLE IF EXISTS temp_repair_projection_projects;\
         DROP TABLE IF EXISTS temp_repair_projection_state;\
         CREATE TEMP TABLE temp_repair_projection_spaces AS SELECT * FROM projection_spaces;\
         CREATE TEMP TABLE temp_repair_projection_projects AS SELECT * FROM projection_projects;\
         CREATE TEMP TABLE temp_repair_projection_state AS SELECT * FROM projection_state;",
    )
    .map_err(|_| err("Failed to stage the current local state before rebuilding it."))?;

    let restore = |conn: &Connection| {
        let _ = conn.execute_batch(
            "DELETE FROM projection_spaces;\
             INSERT INTO projection_spaces SELECT * FROM temp_repair_projection_spaces;\
             DELETE FROM projection_projects;\
             INSERT INTO projection_projects SELECT * FROM temp_repair_projection_projects;\
             DELETE FROM projection_state;\
             INSERT INTO projection_state SELECT * FROM temp_repair_projection_state;",
        );
    };

    // Clear the derived rows the replay rebuilds.
    let metadata_list = REPAIR_METADATA_PROJECTORS
        .iter()
        .map(|p| format!("'{p}'"))
        .collect::<Vec<_>>()
        .join(",");
    if let Err(e) = conn.execute_batch(&format!(
        "DELETE FROM projection_spaces;\
         DELETE FROM projection_projects;\
         DELETE FROM projection_state WHERE projector IN ({metadata_list});"
    )) {
        restore(&conn);
        return Err(err(format!(
            "Failed to clear the local projection cache before rebuilding it: {e}"
        )));
    }

    // Replay the whole event log through the projection writers.
    let replay_result = (|| -> Result<(), WsErr> {
        let mut st = conn
            .prepare(
                "SELECT event_type, stream_id, payload_json, sequence, occurred_at \
                 FROM orchestration_events ORDER BY sequence",
            )
            .map_err(|e| err(e.to_string()))?;
        let mut rows = st
            .query([])
            .map_err(|e| err(e.to_string()))?;
        while let Some(row) = rows.next().map_err(|e| err(e.to_string()))? {
            let event_type: String = row.get(0).map_err(|e| err(e.to_string()))?;
            let stream_id: String = row.get(1).map_err(|e| err(e.to_string()))?;
            let payload_json: String = row.get(2).map_err(|e| err(e.to_string()))?;
            let sequence: i64 = row.get(3).map_err(|e| err(e.to_string()))?;
            let occurred_at: String = row.get(4).map_err(|e| err(e.to_string()))?;
            let payload: Value = serde_json::from_str(&payload_json).unwrap_or(Value::Null);
            replay_projection_event(
                &conn, &event_type, &stream_id, &payload, sequence, &occurred_at,
            )?;
        }
        Ok(())
    })();

    if let Err(e) = replay_result {
        restore(&conn);
        return Err(e);
    }

    // Stamp every required projector cursor at the captured fence.
    let now = iso_now_ms();
    for name in REPAIR_REQUIRED_PROJECTORS {
        if let Err(e) = conn.execute(
            "INSERT OR REPLACE INTO projection_state \
             (projector, last_applied_sequence, updated_at) VALUES (?1, ?2, ?3)",
            params![name, repair_fence, now],
        ) {
            restore(&conn);
            return Err(err(format!(
                "Failed to rebuild local projections from the saved event history: {e}"
            )));
        }
    }

    // Verify the rebuilt projections reached the fence.
    let mut lagging: Vec<String> = Vec::new();
    for name in REPAIR_REQUIRED_PROJECTORS {
        let cursor: i64 = conn
            .query_row(
                "SELECT last_applied_sequence FROM projection_state WHERE projector = ?1",
                params![name],
                |r| r.get(0),
            )
            .unwrap_or(-1);
        if cursor < repair_fence {
            lagging.push(name.to_string());
        }
    }
    if !lagging.is_empty() {
        restore(&conn);
        return Err(err(format!(
            "Rebuilt local projections did not reach captured event fence {repair_fence}. \
             Lagging projectors: {}.",
            lagging.join(", ")
        )));
    }

    let _ = conn.execute_batch(
        "DROP TABLE IF EXISTS temp_repair_projection_spaces;\
         DROP TABLE IF EXISTS temp_repair_projection_projects;\
         DROP TABLE IF EXISTS temp_repair_projection_state;",
    );

    // Return the refreshed read model (same shape as orchestration.getSnapshot).
    crate::orchestration_snapshot::get_snapshot()
}

/// `orchestration.reconcileProviderDelivery` (upstream
/// providerCommandReactor.reconcileDelivery, simplified): settle a dead or
/// uncertain provider-command-reactor delivery. `safe_retry` returns the
/// delivery to 'retry'; `accepted`/`abandon` mark it 'succeeded' and lift the
/// thread's quarantined error by clearing session.last_error and appending a
/// `thread.session-set` event with lastError null.
fn reconcile_provider_delivery(payload: &Value) -> Result<Value, WsErr> {
    let event_sequence = payload
        .get("eventSequence")
        .and_then(Value::as_i64)
        .ok_or_else(|| err("reconcileProviderDelivery expects eventSequence"))?;
    let thread_id = payload
        .get("threadId")
        .and_then(Value::as_str)
        .ok_or_else(|| err("reconcileProviderDelivery expects threadId"))?;
    let expected_state = payload
        .get("expectedState")
        .and_then(Value::as_str)
        .unwrap_or("dead");
    let outcome = payload
        .get("outcome")
        .and_then(Value::as_str)
        .unwrap_or("abandon");
    let note = payload.get("note").and_then(Value::as_str);
    if !matches!(expected_state, "dead" | "uncertain") {
        return Err(err("expectedState must be 'dead' or 'uncertain'"));
    }
    if !matches!(outcome, "accepted" | "safe_retry" | "abandon") {
        return Err(err("outcome must be accepted, safe_retry or abandon"));
    }

    let conn = open_state_rw()?;
    let (delivery_thread, delivery_state): (String, String) = conn
        .query_row(
            "SELECT thread_id, state FROM orchestration_event_deliveries \
             WHERE consumer_name = 'provider-command-reactor.v1' AND event_sequence = ?1",
            params![event_sequence],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| WsErr(
            "Provider delivery no longer matches the requested thread and blocking state."
                .to_string(),
            Some("PROVIDER_DELIVERY_RECONCILIATION_CONFLICT"),
        ))?;
    if delivery_thread != thread_id || delivery_state != expected_state {
        return Err(WsErr(
            "Provider delivery no longer matches the requested thread and blocking state."
                .to_string(),
            Some("PROVIDER_DELIVERY_RECONCILIATION_CONFLICT"),
        ));
    }

    let reconciled_at = iso_now_ms();
    let new_state = if outcome == "safe_retry" { "retry" } else { "succeeded" };
    conn.execute(
        "INSERT INTO provider_delivery_reconciliations \
         (reconciliation_id, consumer_name, event_sequence, thread_id, previous_state, \
          outcome, reconciled_by, note, reconciled_at) \
         VALUES (?1, 'provider-command-reactor.v1', ?2, ?3, ?4, ?5, 'native:shell', ?6, ?7)",
        params![
            uuid_v4(),
            event_sequence,
            thread_id,
            expected_state,
            outcome,
            note,
            reconciled_at,
        ],
    )
    .map_err(|e| err(format!("write reconciliation: {e}")))?;
    conn.execute(
        "UPDATE orchestration_event_deliveries SET state = ?1, updated_at = ?2, \
         completed_at = CASE WHEN ?1 = 'succeeded' THEN ?2 ELSE completed_at END \
         WHERE consumer_name = 'provider-command-reactor.v1' AND event_sequence = ?3",
        params![new_state, reconciled_at, event_sequence],
    )
    .map_err(|e| err(format!("update delivery: {e}")))?;

    if outcome != "safe_retry" {
        // Lift the quarantine: clear the session error in the projection and
        // append a session-set event so replays and live snapshots agree.
        conn.execute(
            "UPDATE projection_thread_sessions SET last_error = NULL, updated_at = ?1 \
             WHERE thread_id = ?2",
            params![reconciled_at, thread_id],
        )
        .map_err(|e| err(format!("clear session error: {e}")))?;
        let session_row: Option<(String, Option<String>, Option<String>, String)> = conn
            .query_row(
                "SELECT status, active_turn_id, provider_name, runtime_mode \
                 FROM projection_thread_sessions WHERE thread_id = ?1",
                params![thread_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .ok();
        if let Some((status, active_turn_id, provider_name, runtime_mode)) = session_row {
            let event_payload = json!({
                "threadId": thread_id,
                "session": {
                    "status": status,
                    "activeTurnId": active_turn_id,
                    "providerName": provider_name,
                    "runtimeMode": runtime_mode,
                    "updatedAt": reconciled_at,
                    "threadId": thread_id,
                    "lastError": null,
                },
            });
            let stream_version: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(stream_version), 0) + 1 \
                     FROM orchestration_events WHERE aggregate_kind = 'thread' AND stream_id = ?1",
                    params![thread_id],
                    |r| r.get(0),
                )
                .map_err(|e| err(e.to_string()))?;
            conn.execute(
                "INSERT INTO orchestration_events \
                 (event_id, aggregate_kind, stream_id, stream_version, event_type, occurred_at, \
                  command_id, causation_event_id, correlation_id, actor_kind, payload_json, metadata_json) \
                 VALUES (?1, 'thread', ?2, ?3, 'thread.session-set', ?4, NULL, NULL, NULL, 'client', ?5, '{}')",
                params![uuid_v4(), thread_id, stream_version, reconciled_at, event_payload.to_string()],
            )
            .map_err(|e| err(format!("append session-set event: {e}")))?;
        }
    }

    Ok(json!({
        "eventSequence": event_sequence,
        "threadId": thread_id,
        "outcome": outcome,
        "state": new_state,
        "reconciledAt": reconciled_at,
    }))
}
