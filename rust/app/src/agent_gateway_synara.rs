//! `synara_*` MCP tool family for the agent gateway: thin read/write wrappers
//! over the tasks store, orchestration projections, and automations, ported
//! from upstream `agentGateway/{todoTools,automationTools,threadTools}.ts`.
//! These are the tools agents use to coordinate work inside threads.

use serde_json::{json, Value};

const MAX_LISTED_TODOS: usize = 200;
const LISTED_NOTES_PREVIEW_CHARS: usize = 500;
const MAX_LISTED_THREADS: usize = 100;
const MAX_THREAD_ACTIVITIES: usize = 100;

fn schema(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

fn def(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
    })
}

pub(crate) fn synara_tool_definitions() -> Vec<Value> {
    vec![
        def(
            "synara_list_todos",
            "List the user's to-dos from the Tasks list. Completed ones are hidden unless includeCompleted is true. Returns at most 200 and reports truncated when there are more; notes are cut to 500 characters unless todoId selects one to-do.",
            schema(json!({
                "includeCompleted": {"type": "boolean"},
                "projectId": {"type": "string", "description": "Only to-dos filed under this project."},
                "todoId": {"type": "string", "description": "Return only this to-do, completed or not, with its full notes."},
            }), &[]),
        ),
        def(
            "synara_create_todo",
            "Add a to-do to the user's Tasks list. Use this for work the user asked you to track, not for steps inside the current turn.",
            schema(json!({
                "title": {"type": "string", "maxLength": 200, "description": "What needs doing, as one line."},
                "notes": {"type": "string", "description": "Optional details. An agent the to-do is later handed to receives them."},
                "priority": {"type": "string", "enum": ["low", "medium", "high"]},
                "dueDate": {"type": "string", "description": "Due day in the user's calendar, as YYYY-MM-DD."},
                "projectId": {"type": "string", "description": "Project to file the to-do under."},
            }), &["title"]),
        ),
        def(
            "synara_update_todo",
            "Edit a to-do in the user's Tasks list, or mark it done or not done. Only the fields you send change. Get todoId from synara_list_todos.",
            schema(json!({
                "todoId": {"type": "string"},
                "title": {"type": "string", "maxLength": 200},
                "notes": {"type": "string"},
                "priority": {"type": "string", "enum": ["low", "medium", "high"]},
                "dueDate": {"type": ["string", "null"], "description": "Pass null to clear it."},
                "projectId": {"type": ["string", "null"], "description": "Pass null to file it under none."},
                "completed": {"type": "boolean", "description": "true marks the to-do done; false reopens it."},
            }), &["todoId"]),
        ),
        def(
            "synara_list_threads",
            "List threads (chats) in the workspace, newest first. Excludes archived and deleted threads unless includeArchived is true.",
            schema(json!({
                "projectId": {"type": "string", "description": "Only threads in this project."},
                "includeArchived": {"type": "boolean"},
                "limit": {"type": "number", "description": "Max threads to return (default 50, max 100)."},
            }), &[]),
        ),
        def(
            "synara_read_thread",
            "Read one thread's metadata and recent messages. Get threadId from synara_list_threads.",
            schema(json!({
                "threadId": {"type": "string"},
                "messageLimit": {"type": "number", "description": "Max recent messages (default 20, max 100)."},
            }), &["threadId"]),
        ),
        def(
            "synara_read_thread_activity",
            "Read the activity feed of one thread: tool calls, file edits, command runs and errors, newest last.",
            schema(json!({
                "threadId": {"type": "string"},
                "turnId": {"type": "string", "description": "Only activities of this turn."},
                "limit": {"type": "number", "description": "Max activities (default 50, max 100)."},
            }), &["threadId"]),
        ),
        def(
            "synara_read_thread_events",
            "Read the durable orchestration events of one thread (turn lifecycle, meta updates, checkpoint events), newest last. Use read_thread_activity for the human-facing feed; events are the raw record.",
            schema(json!({
                "threadId": {"type": "string"},
                "limit": {"type": "number", "description": "Max events (default 50, max 200)."},
                "type": {"type": "string", "description": "Only events of this type, e.g. thread.turn-started."},
            }), &["threadId"]),
        ),
        def(
            "synara_create_automation",
            "Create a heartbeat, standalone, or dedicated Synara automation. heartbeat appends turns to an existing thread; standalone opens a fresh thread per run; dedicated reuses one owned thread. Omitting mode/schedule creates a heartbeat on your thread using everyMinutes (default 5).",
            schema(json!({
                "name": {"type": "string"},
                "prompt": {"type": "string"},
                "mode": {"type": "string", "enum": ["heartbeat", "standalone", "dedicated"]},
                "everyMinutes": {"type": "number", "description": "Interval shorthand, default 5."},
                "targetThreadId": {"type": "string", "description": "Heartbeat only; defaults to the caller thread."},
                "projectId": {"type": "string", "description": "Standalone/dedicated; defaults to the caller's project."},
                "worktreeMode": {"type": "string", "enum": ["auto", "local", "worktree"]},
                "maxIterations": {"type": "number", "minimum": 1},
                "enabled": {"type": "boolean"},
            }), &["name", "prompt"]),
        ),
        def(
            "synara_update_automation",
            "Replace an automation's mutable configuration. Call synara_view_automation first and resend every field, including unchanged ones; partial updates are rejected.",
            schema(json!({
                "automationId": {"type": "string"},
                "name": {"type": "string"},
                "prompt": {"type": "string"},
                "everyMinutes": {"type": "number"},
                "enabled": {"type": "boolean"},
                "maxIterations": {"type": ["number", "null"]},
            }), &["automationId"]),
        ),
        def(
            "synara_cancel_automation",
            "Disable an automation so it stops scheduling runs. Existing runs finish.",
            schema(json!({
                "automationId": {"type": "string"},
            }), &["automationId"]),
        ),
        def(
            "synara_view_automation",
            "Read one automation's full configuration. Get automationId from synara_list_automations.",
            schema(json!({
                "automationId": {"type": "string"},
            }), &["automationId"]),
        ),
        def(
            "synara_read_kanban_board",
            "Read the kanban board of a project: every thread as a card in one of Draft / In Progress / Awaiting you / Done, with attention flags (failed, stuck, awaiting approval/input).",
            schema(json!({
                "projectId": {"type": "string"},
                "includeArchived": {"type": "boolean"},
            }), &["projectId"]),
        ),
        def(
            "synara_list_projects",
            "List the user's projects with their workspace roots.",
            schema(json!({
                "includeArchived": {"type": "boolean"},
            }), &[]),
        ),
        def(
            "synara_list_automations",
            "List the user's automations: scheduled or event-driven agent runs, with their status and schedule.",
            schema(json!({}), &[]),
        ),
    ]
}

fn arg_str(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(Value::as_str).map(str::to_string)
}

fn todos_store() -> Result<std::sync::Arc<crate::todo::TodoStore>, String> {
    Ok(crate::todo::TodoStore::open(&crate::todo::state_dir()))
}

fn truncate_notes(notes: &str, limit: usize) -> String {
    if notes.chars().count() <= limit {
        return notes.to_string();
    }
    notes.chars().take(limit).collect()
}

fn todo_json_out(todo: &Value, preview_notes: bool) -> Value {
    let mut out = todo.clone();
    if preview_notes {
        if let Some(notes) = todo.get("notes").and_then(Value::as_str) {
            let cut = truncate_notes(notes, LISTED_NOTES_PREVIEW_CHARS);
            if cut.len() != notes.len() {
                out.as_object_mut().map(|m| m.insert("notes".into(), json!(cut)));
            }
        }
    }
    out
}

fn list_todos(args: &Value) -> Result<Value, String> {
    let store = todos_store()?;
    let result = store.handle("todo.list", Value::Null).map_err(|e| e.0)?;
    let empty = vec![];
    let all = result.get("todos").and_then(Value::as_array).unwrap_or(&empty);
    if let Some(todo_id) = arg_str(args, "todoId") {
        let Some(todo) = all.iter().find(|t| {
            t.get("id").and_then(Value::as_str) == Some(todo_id.as_str())
                || t.get("todoId").and_then(Value::as_str) == Some(todo_id.as_str())
        }) else {
            return Err(format!("To-do \"{todo_id}\" was not found."));
        };
        return Ok(json!({"todos": [todo_json_out(todo, false)], "truncated": false}));
    }
    let include_completed = args.get("includeCompleted").and_then(Value::as_bool).unwrap_or(false);
    let project_id = arg_str(args, "projectId");
    let matching: Vec<&Value> = all
        .iter()
        .filter(|t| {
            (include_completed || t.get("completedAt").is_none_or(Value::is_null))
                && project_id.as_ref().is_none_or(|p| {
                    t.get("projectId").and_then(Value::as_str) == Some(p.as_str())
                })
        })
        .collect();
    let truncated = matching.len() > MAX_LISTED_TODOS;
    let todos: Vec<Value> = matching
        .iter()
        .take(MAX_LISTED_TODOS)
        .map(|t| todo_json_out(t, true))
        .collect();
    Ok(json!({"todos": todos, "truncated": truncated}))
}

fn create_todo(args: &Value) -> Result<Value, String> {
    let store = todos_store()?;
    let title = arg_str(args, "title").ok_or("Missing required argument: title")?;
    let mut payload = serde_json::Map::new();
    // Upstream derives the id from the caller thread and requestId so retries
    // land on the same to-do; MCP calls here carry no request id, so a fresh
    // id per call is the correct fallback.
    payload.insert(
        "id".into(),
        json!(format!("todo:{}", crate::orchestration_engine::uuid_v4().replace('-', ""))),
    );
    payload.insert("title".into(), json!(title));
    for key in ["notes", "priority", "dueDate", "projectId"] {
        if let Some(v) = args.get(key) {
            if !v.is_null() {
                payload.insert(key.into(), v.clone());
            }
        }
    }
    store.handle("todo.create", Value::Object(payload)).map_err(|e| e.0)
}

fn update_todo(args: &Value) -> Result<Value, String> {
    let store = todos_store()?;
    let todo_id = arg_str(args, "todoId").ok_or("Missing required argument: todoId")?;
    let mut payload = serde_json::Map::new();
    payload.insert("id".into(), json!(todo_id));
    for key in ["title", "notes", "priority", "dueDate", "projectId"] {
        if let Some(v) = args.get(key) {
            payload.insert(key.into(), v.clone());
        }
    }
    if let Some(completed) = args.get("completed").and_then(Value::as_bool) {
        payload.insert("completed".into(), json!(completed));
    }
    store.handle("todo.update", Value::Object(payload)).map_err(|e| e.0)
}

fn state_conn() -> Result<rusqlite::Connection, String> {
    crate::orchestration_engine::open_state_rw().map_err(|e| e.0)
}

fn list_threads(args: &Value) -> Result<Value, String> {
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let include_archived = args.get("includeArchived").and_then(Value::as_bool).unwrap_or(false);
    let limit = args
        .get("limit")
        .and_then(Value::as_i64)
        .unwrap_or(50)
        .clamp(1, MAX_LISTED_THREADS as i64);
    let project_filter = arg_str(args, "projectId");
    let sql = format!(
        "SELECT thread_id, project_id, title, branch, created_at, updated_at, archived_at \
         FROM projection_threads \
         WHERE deleted_at IS NULL {} {} \
         ORDER BY updated_at DESC LIMIT ?1",
        if include_archived { "" } else { "AND archived_at IS NULL" },
        if project_filter.is_some() { "AND project_id = ?2" } else { "" }
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<Value> {
        Ok(json!({
            "threadId": r.get::<_, String>(0)?,
            "projectId": r.get::<_, String>(1)?,
            "title": r.get::<_, Option<String>>(2)?,
            "branch": r.get::<_, Option<String>>(3)?,
            "createdAt": r.get::<_, Option<String>>(4)?,
            "updatedAt": r.get::<_, Option<String>>(5)?,
            "archived": r.get::<_, Option<String>>(6)?.is_some(),
        }))
    };
    let rows: Vec<Value> = if let Some(p) = project_filter {
        stmt.query_map(rusqlite::params![limit, p], map_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
    } else {
        stmt.query_map(rusqlite::params![limit], map_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
    };
    Ok(json!({"threads": rows}))
}

fn read_thread(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let limit = args
        .get("messageLimit")
        .and_then(Value::as_i64)
        .unwrap_or(20)
        .clamp(1, 100);
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let meta: Option<Value> = conn
        .query_row(
            "SELECT thread_id, project_id, title, branch, created_at, updated_at \
             FROM projection_threads WHERE thread_id = ?1",
            [&thread_id],
            |r| {
                Ok(json!({
                    "threadId": r.get::<_, String>(0)?,
                    "projectId": r.get::<_, String>(1)?,
                    "title": r.get::<_, Option<String>>(2)?,
                    "branch": r.get::<_, Option<String>>(3)?,
                    "createdAt": r.get::<_, Option<String>>(4)?,
                    "updatedAt": r.get::<_, Option<String>>(5)?,
                }))
            },
        )
        .ok();
    let Some(meta) = meta else {
        return Err(format!("Thread \"{thread_id}\" was not found."));
    };
    let messages: Vec<Value> = conn
        .prepare(
            "SELECT role, text, created_at FROM projection_thread_messages \
             WHERE thread_id = ?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .and_then(|mut stmt| {
            let rows: Vec<Value> = stmt
                .query_map(rusqlite::params![thread_id, limit], |r| {
                    Ok(json!({
                        "role": r.get::<_, String>(0)?,
                        "text": r.get::<_, Option<String>>(1)?,
                        "createdAt": r.get::<_, Option<String>>(2)?,
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect();
            Ok(rows)
        })
        .unwrap_or_default();
    let mut messages = messages;
    messages.reverse(); // chronological
    Ok(json!({"thread": meta, "messages": messages}))
}

fn read_thread_activity(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let limit = args
        .get("limit")
        .and_then(Value::as_i64)
        .unwrap_or(50)
        .clamp(1, MAX_THREAD_ACTIVITIES as i64);
    let turn_filter = arg_str(args, "turnId");
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let sql = format!(
        "SELECT activity_id, turn_id, tone, kind, summary, payload_json, created_at \
         FROM projection_thread_activities \
         WHERE stream_id = ?1 AND aggregate_kind = 'thread' {} ORDER BY sequence DESC LIMIT ?2",
        if turn_filter.is_some() { "AND turn_id = ?3" } else { "" }
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<Value> {
        let payload_raw: Option<String> = r.get(5)?;
        Ok(json!({
            "activityId": r.get::<_, String>(0)?,
            "turnId": r.get::<_, Option<String>>(1)?,
            "tone": r.get::<_, Option<String>>(2)?,
            "kind": r.get::<_, Option<String>>(3)?,
            "summary": r.get::<_, Option<String>>(4)?,
            "payload": payload_raw.and_then(|p| serde_json::from_str::<Value>(&p).ok()).unwrap_or(Value::Null),
            "createdAt": r.get::<_, Option<String>>(6)?,
        }))
    };
    let mut rows: Vec<Value> = if let Some(t) = turn_filter {
        stmt.query_map(rusqlite::params![thread_id, limit, t], map_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
    } else {
        stmt.query_map(rusqlite::params![thread_id, limit], map_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
    };
    rows.reverse(); // chronological, newest last
    Ok(json!({"activities": rows}))
}

fn list_projects(_args: &Value) -> Result<Value, String> {
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let sql = format!(
        "SELECT project_id, title, kind, workspace_root, created_at, updated_at \
         FROM projection_projects WHERE deleted_at IS NULL ORDER BY updated_at DESC LIMIT 200",
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Vec<Value> = stmt
        .query_map([], |r| {
            Ok(json!({
                "projectId": r.get::<_, String>(0)?,
                "name": r.get::<_, Option<String>>(1)?,
                "kind": r.get::<_, Option<String>>(2)?,
                "workspaceRoot": r.get::<_, Option<String>>(3)?,
                "createdAt": r.get::<_, Option<String>>(4)?,
                "updatedAt": r.get::<_, Option<String>>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(json!({"projects": rows}))
}

fn list_automations() -> Result<Value, String> {
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT automation_id, name, prompt, schedule_json, enabled, created_at, updated_at \
             FROM automation_definitions ORDER BY updated_at DESC LIMIT 200",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<Value> = stmt
        .query_map([], |r| {
            let schedule_raw: Option<String> = r.get(3)?;
            Ok(json!({
                "automationId": r.get::<_, String>(0)?,
                "name": r.get::<_, Option<String>>(1)?,
                "prompt": r.get::<_, Option<String>>(2)?,
                "schedule": schedule_raw.and_then(|p| serde_json::from_str::<Value>(&p).ok()).unwrap_or(Value::Null),
                "enabled": r.get::<_, bool>(4)?,
                "createdAt": r.get::<_, Option<String>>(5)?,
                "updatedAt": r.get::<_, Option<String>>(6)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(json!({"automations": rows}))
}

fn read_thread_events(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let limit = args
        .get("limit")
        .and_then(Value::as_i64)
        .unwrap_or(50)
        .clamp(1, 200);
    let type_filter = arg_str(args, "type");
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let sql = format!(
        "SELECT event_type, payload_json, occurred_at, sequence FROM orchestration_events \
         WHERE stream_id = ?1 AND aggregate_kind = 'thread' {} ORDER BY sequence DESC LIMIT ?2",
        if type_filter.is_some() { "AND event_type = ?3" } else { "" }
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<Value> {
        let payload_raw: Option<String> = r.get(1)?;
        Ok(json!({
            "type": r.get::<_, String>(0)?,
            "payload": payload_raw.and_then(|p| serde_json::from_str::<Value>(&p).ok()).unwrap_or(Value::Null),
            "occurredAt": r.get::<_, Option<String>>(2)?,
            "sequence": r.get::<_, i64>(3)?,
        }))
    };
    let mut rows: Vec<Value> = if let Some(t) = type_filter {
        stmt.query_map(rusqlite::params![thread_id, limit, t], map_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
    } else {
        stmt.query_map(rusqlite::params![thread_id, limit], map_row)
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
    };
    rows.reverse();
    Ok(json!({"events": rows}))
}

const KANBAN_STUCK_WARN_MS: i64 = 20 * 60_000;
const KANBAN_STUCK_HARD_MS: i64 = 40 * 60_000;

fn now_unix_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Port of the shared `kanban.ts` v2 derivation: awaitingYou (failed >
/// pending approval/input > stuck) wins over inProgress, then draft, then done.
fn derive_kanban_card(
    turn_state: Option<&str>,
    turn_started: bool,
    turn_completed: bool,
    session_status: Option<&str>,
    session_last_error: Option<&str>,
    heartbeat_ms: Option<i64>,
    has_pending_approvals: bool,
    has_pending_input: bool,
    now: i64,
) -> (&'static str, Vec<&'static str>) {
    let can_answer = !matches!(session_status, Some("stopped") | Some("error"));
    let settled = turn_started
        && turn_completed
        && (matches!(turn_state, Some("interrupted") | Some("error"))
            || session_status != Some("running"));
    let mut live = false;
    if (has_pending_approvals || has_pending_input) && can_answer {
        live = true;
    }
    if turn_state == Some("running") || (turn_started && !settled) {
        live = true;
    }
    if matches!(session_status, Some("starting") | Some("running")) {
        live = true;
    }

    let hard_stuck = heartbeat_ms.map_or(false, |hb| now - hb >= KANBAN_STUCK_HARD_MS);
    let warn_stuck = heartbeat_ms.map_or(false, |hb| now - hb >= KANBAN_STUCK_WARN_MS);

    // awaitingYou reason precedence: failed > pending > stuck
    let mut reason: Option<&str> = None;
    if turn_state == Some("error")
        || session_status == Some("error")
        || session_last_error.is_some()
    {
        reason = Some("failed");
    } else if can_answer && has_pending_approvals {
        reason = Some("pending-approval");
    } else if can_answer && has_pending_input {
        reason = Some("pending-input");
    } else if live && hard_stuck {
        reason = Some("stuck");
    }

    let column = if reason.is_some() {
        "awaitingYou"
    } else if live {
        "inProgress"
    } else if turn_state.is_none() {
        "draft"
    } else {
        "done"
    };

    let mut flags = Vec::new();
    match reason {
        Some("failed") => flags.push("failed"),
        Some("pending-approval") => flags.push("awaiting-approval"),
        Some("pending-input") => flags.push("awaiting-input"),
        Some("stuck") => flags.push("stuck"),
        _ => {}
    }
    if live && warn_stuck && reason != Some("stuck") {
        flags.push("stuck");
    }
    (column, flags)
}

fn read_kanban_board(args: &Value) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let include_archived = args.get("includeArchived").and_then(Value::as_bool).unwrap_or(false);
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let sql = format!(
        "SELECT t.thread_id, t.title, t.branch, t.updated_at, \
         t.pending_approval_count, t.pending_user_input_count, \
         (SELECT state FROM projection_turns tu WHERE tu.thread_id = t.thread_id \
           ORDER BY COALESCE(tu.started_at, tu.requested_at) DESC LIMIT 1), \
         (SELECT started_at IS NOT NULL FROM projection_turns tu WHERE tu.thread_id = t.thread_id \
           ORDER BY COALESCE(tu.started_at, tu.requested_at) DESC LIMIT 1), \
         (SELECT completed_at IS NOT NULL FROM projection_turns tu WHERE tu.thread_id = t.thread_id \
           ORDER BY COALESCE(tu.started_at, tu.requested_at) DESC LIMIT 1), \
         (SELECT status FROM projection_thread_sessions s WHERE s.thread_id = t.thread_id), \
         (SELECT last_error FROM projection_thread_sessions s WHERE s.thread_id = t.thread_id), \
         (SELECT updated_at FROM projection_thread_sessions s WHERE s.thread_id = t.thread_id) \
         FROM projection_threads t \
         WHERE t.project_id = ?1 AND t.deleted_at IS NULL {} \
         ORDER BY t.updated_at DESC LIMIT 300",
        if include_archived { "" } else { "AND t.archived_at IS NULL" }
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let now = now_unix_ms();
    let rows: Vec<Value> = stmt
        .query_map([&project_id], |r| {
            let thread_updated: Option<String> = r.get(3)?;
            let session_updated: Option<String> = r.get(11)?;
            let heartbeat = [thread_updated, session_updated]
                .into_iter()
                .flatten()
                .filter_map(|s| crate::orchestration_engine::iso_ms(&s))
                .max();
            let turn_state: Option<String> = r.get(6)?;
            let (column, flags) = derive_kanban_card(
                turn_state.as_deref(),
                r.get::<_, bool>(7)?,
                r.get::<_, bool>(8)?,
                r.get::<_, Option<String>>(9)?.as_deref(),
                r.get::<_, Option<String>>(10)?.as_deref(),
                heartbeat,
                r.get::<_, i64>(4)? > 0,
                r.get::<_, i64>(5)? > 0,
                now,
            );
            Ok(json!({
                "threadId": r.get::<_, String>(0)?,
                "title": r.get::<_, Option<String>>(1)?,
                "branch": r.get::<_, Option<String>>(2)?,
                "column": column,
                "attention": flags,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let mut columns: serde_json::Map<String, Value> = serde_json::Map::new();
    for key in ["draft", "inProgress", "awaitingYou", "done"] {
        columns.insert(key.into(), json!([]));
    }
    for card in &rows {
        let col = card.get("column").and_then(Value::as_str).unwrap_or("done");
        if let Some(arr) = columns.get_mut(col).and_then(Value::as_array_mut) {
            arr.push(card.clone());
        }
    }
    Ok(json!({"projectId": project_id, "columns": Value::Object(columns), "cardCount": rows.len()}))
}

fn view_automation(args: &Value) -> Result<Value, String> {
    let automation_id = arg_str(args, "automationId").ok_or("Missing required argument: automationId")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT automation_id, project_id, name, prompt, schedule_json, enabled, next_run_at, \
         model_selection_json, runtime_mode, interaction_mode, mode, target_thread_id, max_iterations, \
         stop_on_error, minimum_interval_seconds, created_at, updated_at \
         FROM automation_definitions WHERE automation_id = ?1",
        [&automation_id],
        |r| {
            let parse = |raw: Option<String>| raw.and_then(|p| serde_json::from_str::<Value>(&p).ok()).unwrap_or(Value::Null);
            Ok(json!({
                "automationId": r.get::<_, String>(0)?,
                "projectId": r.get::<_, String>(1)?,
                "name": r.get::<_, Option<String>>(2)?,
                "prompt": r.get::<_, Option<String>>(3)?,
                "schedule": parse(r.get::<_, Option<String>>(4)?),
                "enabled": r.get::<_, bool>(5)?,
                "nextRunAt": r.get::<_, Option<String>>(6)?,
                "modelSelection": parse(r.get::<_, Option<String>>(7)?),
                "runtimeMode": r.get::<_, Option<String>>(8)?,
                "interactionMode": r.get::<_, Option<String>>(9)?,
                "mode": r.get::<_, Option<String>>(10)?,
                "targetThreadId": r.get::<_, Option<String>>(11)?,
                "maxIterations": r.get::<_, Option<i64>>(12)?,
                "stopOnError": r.get::<_, bool>(13)?,
                "minimumIntervalSeconds": r.get::<_, i64>(14)?,
                "createdAt": r.get::<_, Option<String>>(15)?,
                "updatedAt": r.get::<_, Option<String>>(16)?,
            }))
        },
    )
    .map(|v| json!({"automation": v}))
    .map_err(|_| format!("Automation \"{automation_id}\" was not found."))
}

fn caller_project_id(conn: &rusqlite::Connection, thread_id: &str) -> Option<String> {
    conn.query_row(
        "SELECT project_id FROM projection_threads WHERE thread_id = ?1",
        [thread_id],
        |r| r.get(0),
    )
    .ok()
}

fn create_automation(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let name = arg_str(args, "name").ok_or("Missing required argument: name")?;
    let prompt = arg_str(args, "prompt").ok_or("Missing required argument: prompt")?;
    let mode = arg_str(args, "mode").unwrap_or_else(|| "heartbeat".into());
    if mode == "heartbeat" && args.get("target").is_some() {
        return Err("Heartbeat automations continue their target thread's session and reject target.".into());
    }
    let conn = state_conn()?;
    let every_minutes = args.get("everyMinutes").and_then(Value::as_f64).unwrap_or(5.0).max(1.0);
    let schedule = json!({"type": "interval", "everySeconds": (every_minutes * 60.0).round() as i64});
    let target_thread_id = arg_str(args, "targetThreadId").or_else(|| {
        if mode == "heartbeat" {
            caller_thread.map(str::to_string)
        } else {
            None
        }
    });
    let project_id = arg_str(args, "projectId")
        .or_else(|| caller_thread.and_then(|t| caller_project_id(&conn, t)))
        .ok_or("Could not resolve a project: pass projectId explicitly.")?;
    let mut payload = serde_json::Map::new();
    payload.insert("projectId".into(), json!(project_id));
    payload.insert("name".into(), json!(name));
    payload.insert("prompt".into(), json!(prompt));
    payload.insert("schedule".into(), schedule);
    payload.insert("mode".into(), json!(mode));
    payload.insert("enabled".into(), json!(args.get("enabled").and_then(Value::as_bool).unwrap_or(true)));
    if let Some(t) = target_thread_id {
        payload.insert("targetThreadId".into(), json!(t));
    }
    if let Some(t) = caller_thread {
        payload.insert("sourceThreadId".into(), json!(t));
    }
    if let Some(w) = arg_str(args, "worktreeMode") {
        payload.insert("worktreeMode".into(), json!(w));
    }
    if let Some(m) = args.get("maxIterations").and_then(Value::as_i64) {
        payload.insert("maxIterations".into(), json!(m));
    }
    crate::automation::handle("automation.create", &Value::Object(payload)).map_err(|e| e.0)
}

fn update_automation(args: &Value) -> Result<Value, String> {
    let automation_id = arg_str(args, "automationId").ok_or("Missing required argument: automationId")?;
    // Upstream requires a full replacement payload; read the stored definition
    // and overlay the sent fields so partial input cannot blank the rest.
    let current = view_automation(args)?;
    let current = current.get("automation").cloned().unwrap_or(Value::Null);
    let mut payload = serde_json::Map::new();
    payload.insert("automationId".into(), json!(automation_id));
    payload.insert(
        "name".into(),
        arg_str(args, "name").map(|s| json!(s)).unwrap_or_else(|| current.get("name").cloned().unwrap_or(Value::Null)),
    );
    payload.insert(
        "prompt".into(),
        arg_str(args, "prompt").map(|s| json!(s)).unwrap_or_else(|| current.get("prompt").cloned().unwrap_or(Value::Null)),
    );
    let schedule = if let Some(mins) = args.get("everyMinutes").and_then(Value::as_f64) {
        json!({"type": "interval", "everySeconds": (mins.max(1.0) * 60.0).round() as i64})
    } else {
        current.get("schedule").cloned().unwrap_or(Value::Null)
    };
    payload.insert("schedule".into(), schedule);
    payload.insert(
        "enabled".into(),
        args.get("enabled").cloned().unwrap_or_else(|| current.get("enabled").cloned().unwrap_or(json!(true))),
    );
    for key in ["maxIterations"] {
        if let Some(v) = args.get(key) {
            payload.insert(key.into(), v.clone());
        } else if let Some(v) = current.get(key) {
            payload.insert(key.into(), v.clone());
        }
    }
    crate::automation::handle("automation.update", &Value::Object(payload)).map_err(|e| e.0)
}

fn cancel_automation(args: &Value) -> Result<Value, String> {
    let automation_id = arg_str(args, "automationId").ok_or("Missing required argument: automationId")?;
    let mut payload = serde_json::Map::new();
    payload.insert("automationId".into(), json!(automation_id));
    payload.insert("enabled".into(), json!(false));
    crate::automation::handle("automation.update", &Value::Object(payload)).map_err(|e| e.0)
}

pub(crate) fn call_synara_tool(name: &str, args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    match name {
        "synara_list_todos" => list_todos(args),
        "synara_create_todo" => create_todo(args),
        "synara_update_todo" => update_todo(args),
        "synara_list_threads" => list_threads(args),
        "synara_read_thread" => read_thread(args),
        "synara_read_thread_activity" => read_thread_activity(args),
        "synara_read_thread_events" => read_thread_events(args),
        "synara_read_kanban_board" => read_kanban_board(args),
        "synara_view_automation" => view_automation(args),
        "synara_create_automation" => create_automation(args, caller_thread),
        "synara_update_automation" => update_automation(args),
        "synara_cancel_automation" => cancel_automation(args),
        "synara_list_projects" => list_projects(args),
        "synara_list_automations" => list_automations(),
        _ => Err(format!("Unknown tool: {name}")),
    }
}
