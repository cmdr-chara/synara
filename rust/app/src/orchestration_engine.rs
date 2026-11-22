// Orchestration engine: native orchestration.dispatchCommand.
//
// Intercepted only when SYNARA_NO_UPSTREAM is set: in that mode the front door
// owns state.sqlite directly (the original server is not started, so there is
// no EXCLUSIVE lock), and commands are decided, appended to
// orchestration_events, receipted and projected in one transaction.
// Without the flag every command keeps relaying to the original server.

use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

use crate::todo::{iso_now_ms, state_dir, WsErr};

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
