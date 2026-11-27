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
         WHERE thread_id = ?1 {} ORDER BY sequence DESC LIMIT ?2",
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

pub(crate) fn call_synara_tool(name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "synara_list_todos" => list_todos(args),
        "synara_create_todo" => create_todo(args),
        "synara_update_todo" => update_todo(args),
        "synara_list_threads" => list_threads(args),
        "synara_read_thread" => read_thread(args),
        "synara_read_thread_activity" => read_thread_activity(args),
        "synara_list_projects" => list_projects(args),
        "synara_list_automations" => list_automations(),
        _ => Err(format!("Unknown tool: {name}")),
    }
}
