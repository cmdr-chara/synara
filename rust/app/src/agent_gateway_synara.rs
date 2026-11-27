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
            "synara_create_kanban_task",
            "Create a Kanban task from a title and optional description: starts a new thread and immediately starts a turn, so the card renders In Progress. requestId is required; retries with the same requestId replay exactly-once.",
            schema(json!({
                "title": {"type": "string", "maxLength": 200},
                "description": {"type": "string", "description": "Used as the first-turn prompt."},
                "projectId": {"type": "string"},
                "requestId": {"type": "string", "maxLength": 256, "description": "Idempotency key."},
            }), &["title", "requestId"]),
        ),
        def(
            "synara_create_kanban_draft",
            "Create a Kanban draft card: a thread with no turn started, so the card renders in Draft. requestId is the idempotency key.",
            schema(json!({
                "title": {"type": "string", "maxLength": 200},
                "projectId": {"type": "string"},
                "requestId": {"type": "string", "maxLength": 256},
            }), &["title", "requestId"]),
        ),
        def(
            "synara_read_kanban_card",
            "Read one Kanban card: its thread metadata and derived column with attention flags.",
            schema(json!({
                "threadId": {"type": "string"},
            }), &["threadId"]),
        ),
        def(
            "synara_move_kanban_card",
            "Move a card between actionable columns. target inProgress starts or resumes work, optionally with a message — required when restarting a settled thread. target done requests the running turn to interrupt.",
            schema(json!({
                "threadId": {"type": "string"},
                "target": {"type": "string", "enum": ["inProgress", "done"]},
                "message": {"type": "string", "description": "Required when restarting a settled thread."},
            }), &["threadId", "target"]),
        ),
        def(
            "synara_update_kanban_card",
            "Edit a card's title and/or description (stored as the thread notes). At least one of title/description required.",
            schema(json!({
                "threadId": {"type": "string"},
                "title": {"type": "string", "maxLength": 200},
                "description": {"type": "string"},
            }), &["threadId"]),
        ),
        def(
            "synara_delete_kanban_card",
            "Delete a card: permanently deletes the thread behind it. Cannot be undone.",
            schema(json!({
                "threadId": {"type": "string"},
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
            "synara_project_list_tasks",
            "List project coordinator tasks, including review state and dependencies.",
            schema(json!({
                "projectId": {"type": "string"},
                "includeArchived": {"type": "boolean"},
            }), &["projectId"]),
        ),
        def(
            "synara_project_list_threads",
            "List every hub thread in the project with its live state, PR link, last update time, and task id.",
            schema(json!({
                "projectId": {"type": "string"},
            }), &["projectId"]),
        ),
        def(
            "synara_project_read_document",
            "Read a shared project document by relative path (e.g. memory/MEMORY.md or memory/<date>-<slug>.md).",
            schema(json!({
                "projectId": {"type": "string"},
                "logicalPath": {"type": "string"},
            }), &["projectId", "logicalPath"]),
        ),
        def(
            "synara_project_write_document",
            "Write a shared project document at a relative path, creating a new revision.",
            schema(json!({
                "projectId": {"type": "string"},
                "logicalPath": {"type": "string"},
                "content": {"type": "string"},
            }), &["projectId", "logicalPath", "content"]),
        ),
        def(
            "synara_project_remember",
            "Save a note to the hub's shared memory (memory/<date>-<slug>.md) and update the MEMORY.md index every hub thread reads. Near-identical notes are deduplicated onto the existing memory file.",
            schema(json!({
                "projectId": {"type": "string"},
                "note": {"type": "string"},
                "title": {"type": "string"},
            }), &["projectId", "note"]),
        ),
        def(
            "synara_project_forget",
            "Remove a hub memory note file (memory/<date>-<slug>.md) and its index line in MEMORY.md.",
            schema(json!({
                "projectId": {"type": "string"},
                "path": {"type": "string"},
            }), &["projectId", "path"]),
        ),
        def(
            "synara_hub_list_work",
            "Read durable Hub task cards, queue state and progress revisions for your Hub.",
            schema(json!({}), &[]),
        ),
        def(
            "synara_hub_update_progress",
            "Persist your own Hub task checklist. Pass the last progress revision (0 initially). Checklist completion does not finish the task or release its worker slot.",
            schema(json!({
                "workItemId": {"type": "string"},
                "expectedRevision": {"type": "integer", "minimum": 0},
                "summary": {"type": "string", "maxLength": 2000},
                "steps": {
                    "type": "array",
                    "maxItems": 32,
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {"type": "string"},
                            "text": {"type": "string"},
                            "status": {"type": "string", "enum": ["pending", "inProgress", "completed"]},
                        },
                        "required": ["id", "text", "status"],
                    },
                },
            }), &["workItemId", "expectedRevision", "steps"]),
        ),
        def(
            "synara_hub_cancel_work",
            "Cancel queued or idle Hub work. Stop active workers using the existing interrupt tool before cancelling.",
            schema(json!({
                "workItemId": {"type": "string"},
            }), &["workItemId"]),
        ),
        def(
            "synara_create_thread",
            "Create a new thread. Pass prompt to start it immediately, or omit it for a draft. requestId is the idempotency key: retries with the same requestId replay the original result exactly once.",
            schema(json!({
                "requestId": {"type": "string", "maxLength": 256},
                "prompt": {"type": "string"},
                "title": {"type": "string"},
                "projectId": {"type": "string"},
                "provider": {"type": "string"},
                "model": {"type": "string"},
                "environment": {"type": "string", "enum": ["local", "worktree"]},
                "baseRef": {"type": "string", "description": "Local git revision the worktree is pinned at. Defaults to HEAD."},
            }), &["requestId"]),
        ),
        def(
            "synara_send_message",
            "Send a follow-up message to a thread. mode queue (default) waits for the current turn; steer interrupts the active turn and redirects it.",
            schema(json!({
                "threadId": {"type": "string"},
                "message": {"type": "string"},
                "mode": {"type": "string", "enum": ["queue", "steer"]},
            }), &["threadId", "message"]),
        ),
        def(
            "synara_interrupt_thread",
            "Interrupt the active turn on a thread. The thread settles with its work so far.",
            schema(json!({
                "threadId": {"type": "string"},
            }), &["threadId"]),
        ),
        def(
            "synara_set_thread_title",
            "Rename a thread.",
            schema(json!({
                "threadId": {"type": "string"},
                "title": {"type": "string", "maxLength": 200},
            }), &["threadId", "title"]),
        ),
        def(
            "synara_set_thread_archived",
            "Archive or unarchive a thread.",
            schema(json!({
                "threadId": {"type": "string"},
                "archived": {"type": "boolean"},
            }), &["threadId", "archived"]),
        ),
        def(
            "synara_read_thread_runtime_events",
            "Read retained provider-runtime events for one thread. This source has a global retention cap; inspect coverage before treating absence as evidence.",
            schema(json!({
                "threadId": {"type": "string"},
                "limit": {"type": "number", "description": "Default 50, max 200."},
                "turnId": {"type": "string"},
            }), &["threadId"]),
        ),
        def(
            "synara_wait_for_threads",
            "Wait for the pinned turns of 1-20 Synara threads and return every outcome in input order. When results are required, keep calling until allTerminal is true, passing the returned runIds to each subsequent call. A timedOut response is progress, not completion.",
            schema(json!({
                "threadIds": {"type": "array", "minItems": 1, "maxItems": 20, "items": {"type": "string"}},
                "runIds": {"type": "array", "maxItems": 20, "items": {"type": ["string", "null"]}},
                "timeoutMs": {"type": "integer", "minimum": 0, "maximum": 60000},
            }), &["threadIds"]),
        ),
        def(
            "synara_list_connections",
            "Discover this computer and its paired execution connections. Use the returned environmentId on Synara project/thread tools.",
            schema(json!({}), &[]),
        ),
        def(
            "synara_project_library_list",
            "List files in the hub's Library (optionally under relativePath). Returns the library root path and entries.",
            schema(json!({
                "projectId": {"type": "string"},
                "relativePath": {"type": "string"},
            }), &["projectId"]),
        ),
        def(
            "synara_project_library_add",
            "Copy a file or folder from this thread's own workspace into the hub's Library. sourcePath must resolve inside your workspace; destinationPath defaults to the source name at the library root.",
            schema(json!({
                "projectId": {"type": "string"},
                "sourcePath": {"type": "string"},
                "destinationPath": {"type": "string"},
            }), &["projectId", "sourcePath"]),
        ),
        def(
            "synara_project_get_overview",
            "Read the current hub's coordinator overview: goal, focus, blockers, last summary, and linked repositories. Does not include document bodies.",
            schema(json!({
                "projectId": {"type": "string"},
            }), &["projectId"]),
        ),
        def(
            "synara_project_context",
            "Load the bounded project context packet (goal, instructions, decisions, tasks) capped at 32,000 characters.",
            schema(json!({
                "projectId": {"type": "string"},
            }), &["projectId"]),
        ),
        def(
            "synara_project_report_result",
            "Report worker findings into the project inbox. This records evidence and moves the task to review. It does not mark the task done.",
            schema(json!({
                "requestId": {"type": "string"},
                "projectId": {"type": "string"},
                "taskId": {"type": "string"},
                "summary": {"type": "string"},
            }), &["requestId", "projectId", "taskId", "summary"]),
        ),
        def(
            "synara_update_automation_memory",
            "Fully replace an automation's persistent memory. Maximum size: 32 KiB. Omit automationId only when the current user message is the automation run envelope.",
            schema(json!({
                "memory": {"type": "string", "maxLength": 32768},
                "automationId": {"type": "string"},
            }), &["memory"]),
        ),
        def(
            "synara_report_automation_result",
            "Report the structured result only when the current user message is the automation run envelope. Use decision silent only when a successful run needs no user attention.",
            schema(json!({
                "decision": {"type": "string", "enum": ["notify", "silent"]},
                "title": {"type": "string", "maxLength": 160},
                "summary": {"type": "string", "maxLength": 2000},
            }), &["decision"]),
        ),
        def(
            "synara_set_thread_pull_request",
            "Link a GitHub pull request to a thread.",
            schema(json!({
                "threadId": {"type": "string"},
                "pullRequestUrl": {"type": "string"},
            }), &["pullRequestUrl"]),
        ),
        def(
            "synara_unlink_thread_pull_request",
            "Unlink a pull request from a thread.",
            schema(json!({
                "threadId": {"type": "string"},
                "pullRequestUrl": {"type": "string"},
            }), &["pullRequestUrl"]),
        ),
        def(
            "synara_set_thread_goal",
            "Set or clear the persistent goal on a thread.",
            schema(json!({
                "threadId": {"type": "string"},
                "goal": {"type": ["string", "null"], "maxLength": 4000},
            }), &["threadId", "goal"]),
        ),
        def(
            "synara_set_kanban_goal",
            "Set the persistent goal on a card's thread in your own project, from any column including live cards. Pass null or empty string to clear.",
            schema(json!({
                "threadId": {"type": "string"},
                "goal": {"type": ["string", "null"], "maxLength": 4000},
            }), &["threadId", "goal"]),
        ),
        def(
            "synara_project_link_repository",
            "Link a repository project to the hub so the coordinator may create threads there.",
            schema(json!({
                "projectId": {"type": "string"},
                "linkedProjectId": {"type": "string"},
            }), &["projectId", "linkedProjectId"]),
        ),
        def(
            "synara_diagnose_thread",
            "Build one bounded forensic snapshot from projected status/messages/activity, durable events, and runtime incidents.",
            schema(json!({
                "threadId": {"type": "string"},
            }), &["threadId"]),
        ),
        def(
            "synara_list_mcp_connectors",
            "List configured MCP connectors on this server.",
            schema(json!({}), &[]),
        ),
        def(
            "synara_create_threads",
            "Create an exact batch of 1-20 standalone Synara threads. Each item needs prompt and optional target/projectId/title. Validation failures create nothing; durable retries with the same requestId replay the exact operation.",
            schema(json!({
                "requestId": {"type": "string", "maxLength": 256},
                "threads": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 20,
                    "items": {
                        "type": "object",
                        "properties": {
                            "prompt": {"type": "string"},
                            "title": {"type": "string"},
                            "projectId": {"type": "string"},
                            "target": {"type": "object"},
                        },
                        "required": ["prompt"],
                    },
                },
            }), &["requestId", "threads"]),
        ),
        def(
            "synara_update_plan",
            "Keep the checklist Synara pins under your latest message while you work. Start: the whole plan in order with the step you are starting in_progress. Progress: send the whole list again with exactly one in_progress. Finish: leave the last step in_progress; Synara checks it off when your turn ends.",
            schema(json!({
                "items": {
                    "type": "array",
                    "minItems": 1,
                    "maxItems": 12,
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {"type": "string"},
                            "status": {"type": "string", "enum": ["pending", "in_progress", "completed"]},
                        },
                        "required": ["content", "status"],
                    },
                },
            }), &["items"]),
        ),
        def(
            "synara_add_mcp_connector",
            "Propose adding an MCP connector. Always requires the user's approval card; never executes a command or connects before approval. Pass command and args separately, not a shell string.",
            schema(json!({
                "name": {"type": "string"},
                "url": {"type": "string"},
                "command": {"type": "string"},
                "args": {"type": "array", "items": {"type": "string"}},
                "env": {"type": "object"},
                "cwd": {"type": "string"},
            }), &[]),
        ),
        def(
            "synara_remove_mcp_connector",
            "Propose removing a connector. Removal happens only when the user accepts the confirmation card.",
            schema(json!({
                "connectorId": {"type": "string"},
            }), &["connectorId"]),
        ),
        def(
            "synara_import_mcp_servers",
            "Discover MCP servers in Claude Code user/project config, Codex, workspace .mcp.json and Claude Desktop. Shows a separate approval card for each candidate. Only entries the user accepts are added.",
            schema(json!({}), &[]),
        ),
        def(
            "synara_context",
            "Inspect the current Synara harness identity, caller thread/turn, and authorized coordination capabilities.",
            schema(json!({}), &[]),
        ),
        def(
            "synara_e2e_review",
            "Load Synara's E2E testing workflow for web apps and iOS simulator apps: delegate a test subagent, verify real journeys, and report screenshot proof. Call only for an explicit E2E request.",
            schema(json!({}), &[]),
        ),
        def(
            "synara_capabilities",
            "List the provider instances and models automations and threads can target, with each instance's enabled state. Use exact slugs from here when creating automations or threads.",
            schema(json!({}), &[]),
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
    // Cost telemetry rides along so a coordinator can see what a task cost
    // without a second round trip.
    let cost = state_conn()
        .ok()
        .map(|conn| thread_cost_summary(&conn, &thread_id))
        .unwrap_or(Value::Null);
    Ok(json!({"thread": meta, "messages": messages, "cost": cost}))
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
        .query_map(rusqlite::params![project_id], |r| {
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

fn create_kanban_card(args: &Value, start_turn: bool) -> Result<Value, String> {
    let title = arg_str(args, "title").ok_or("Missing required argument: title")?;
    if title.chars().count() > 200 {
        return Err("title must be at most 200 characters.".into());
    }
    let request_id = arg_str(args, "requestId").ok_or("Missing required argument: requestId")?;
    let conn = state_conn()?;
    let project_id = arg_str(args, "projectId").ok_or(
        "Missing required argument: projectId (the kanban board's project).",
    )?;
    // Idempotency: the engine dedupes commands by commandId through the
    // receipts table, so a retried call replays exactly-once.
    let key: String = request_id.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '-').take(200).collect();
    // The thread id derives from the requestId, so a retried call targets the
    // same thread; the command receipt dedup then replays exactly-once.
    let thread_id = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h1 = DefaultHasher::new();
        format!("kanban-mcp\0{request_id}\0a").hash(&mut h1);
        let mut h2 = DefaultHasher::new();
        format!("kanban-mcp\0{request_id}\0b").hash(&mut h2);
        let a = h1.finish();
        let b = h2.finish();
        format!(
            "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
            (a >> 32) as u32,
            (a >> 16) as u16,
            a as u16 & 0xfff,
            (b as u16 & 0x3fff) | 0x8000,
            b & 0xffffffffffff
        )
    };
    let now = crate::todo::iso_now_ms();
    let create = json!({
        "type": "thread.create",
        "commandId": format!("kanban-mcp-{key}-thread-create"),
        "threadId": thread_id,
        "projectId": project_id,
        "title": title,
        "createdAt": now,
    });
    let created = match crate::orchestration_engine::handle(
        "orchestration.dispatchCommand",
        &create,
    ) {
        Ok(_) => true,
        Err(e) if e.0.contains("UNIQUE constraint") || e.0.contains("already exists") => false, // retry replay
        Err(e) => return Err(format!("kanban thread create failed: {}", e.0)),
    };
    drop(conn);
    if !created {
        return Ok(json!({
            "threadId": thread_id,
            "title": title,
            "started": start_turn,
            "replayed": true,
        }));
    }

    if start_turn {
        let prompt = arg_str(args, "description").unwrap_or_else(|| title.clone());
        let turn = json!({
            "type": "thread.turn.start",
            "commandId": format!("kanban-mcp-{key}-turn-start"),
            "threadId": thread_id,
            "createdAt": now,
            "dispatchMode": "queue",
            "dispatchOrigin": "kanban",
            "message": {
                "messageId": format!("kanban-mcp-{key}-message"),
                "text": prompt,
                "attachments": [],
            },
        });
        crate::orchestration_engine::handle("orchestration.dispatchCommand", &turn)
            .map_err(|e| format!("kanban turn start failed: {}", e.0))?;
    }
    Ok(json!({
        "threadId": thread_id,
        "title": title,
        "started": start_turn,
    }))
}

fn kanban_card_state(
    conn: &rusqlite::Connection,
    thread_id: &str,
) -> Result<(String, Vec<&'static str>, bool), String> {
    let row: Option<(String, Option<String>, Option<String>, i64, i64, Option<String>)> = conn
        .query_row(
            "SELECT t.updated_at, t.archived_at, t.deleted_at, t.pending_approval_count, t.pending_user_input_count, \
             (SELECT status FROM projection_thread_sessions s WHERE s.thread_id = t.thread_id) \
             FROM projection_threads t WHERE t.thread_id = ?1",
            [thread_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            },
        )
        .ok();
    let Some((updated_at, archived, deleted, approvals, inputs, session_status)) = row else {
        return Err(format!("Thread \"{thread_id}\" was not found."));
    };
    if deleted.is_some() {
        return Err(format!("Thread \"{thread_id}\" was deleted."));
    }
    let latest_turn: Option<(String, bool, bool, Option<String>)> = conn
        .query_row(
            "SELECT state, started_at IS NOT NULL, completed_at IS NOT NULL, last_error \
             FROM projection_turns WHERE thread_id = ?1 \
             ORDER BY COALESCE(started_at, requested_at) DESC LIMIT 1",
            [thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();
    let heartbeat = crate::orchestration_engine::iso_ms(&updated_at);
    let (col, flags) = match &latest_turn {
        Some((state, started, completed, last_err)) => derive_kanban_card(
            Some(state.as_str()),
            *started,
            *completed,
            session_status.as_deref(),
            last_err.as_deref(),
            heartbeat,
            approvals > 0,
            inputs > 0,
            now_unix_ms(),
        ),
        None => derive_kanban_card(
            None,
            false,
            false,
            session_status.as_deref(),
            None,
            heartbeat,
            approvals > 0,
            inputs > 0,
            now_unix_ms(),
        ),
    };
    Ok((col.to_string(), flags, archived.is_some()))
}

fn read_kanban_card(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let conn = state_conn()?;
    let (column, flags, archived) = kanban_card_state(&conn, &thread_id)?;
    let meta: Option<(Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT title, branch FROM projection_threads WHERE thread_id = ?1",
            [&thread_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    Ok(json!({
        "threadId": thread_id,
        "title": meta.as_ref().and_then(|m| m.0.clone()),
        "branch": meta.and_then(|m| m.1),
        "column": column,
        "attention": flags,
        "archived": archived,
    }))
}

fn move_kanban_card(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let target = arg_str(args, "target").ok_or("Missing required argument: target")?;
    let conn = state_conn()?;
    let (column, flags, archived) = kanban_card_state(&conn, &thread_id)?;
    if archived {
        return Err("Archived threads cannot be moved on the board.".into());
    }
    let failed = flags.contains(&"failed");
    drop(conn);
    let now = crate::todo::iso_now_ms();
    match target.as_str() {
        "inProgress" => {
            if column == "awaitingYou" && !failed {
                return Ok(json!({"threadId": thread_id, "column": "awaitingYou", "attention": flags, "moved": false, "reason": "awaitingYou is a human-attention state"}));
            }
            if column == "inProgress" {
                return Ok(json!({"threadId": thread_id, "column": "inProgress", "attention": flags, "moved": false, "alreadyInProgress": true}));
            }
            let message = arg_str(args, "message").ok_or(
                "message is required when restarting a settled thread.",
            )?;
            let turn = json!({
                "type": "thread.turn.start",
                "commandId": format!("kanban-move-{}", crate::orchestration_engine::uuid_v4()),
                "threadId": thread_id,
                "createdAt": now,
                "dispatchMode": "queue",
                "dispatchOrigin": "kanban",
                "message": {"messageId": format!("kanban-move-msg-{}", crate::orchestration_engine::uuid_v4()), "text": message, "attachments": []},
            });
            crate::orchestration_engine::handle("orchestration.dispatchCommand", &turn)
                .map_err(|e| format!("turn start failed: {}", e.0))?;
            Ok(json!({"threadId": thread_id, "column": "inProgress", "moved": true}))
        }
        "done" => {
            if column == "done" {
                return Ok(json!({"threadId": thread_id, "column": "done", "moved": false, "alreadyDone": true}));
            }
            if column == "awaitingYou" && !failed {
                return Err("A card awaiting human attention cannot be moved to done.".into());
            }
            if column != "inProgress" {
                return Ok(json!({"threadId": thread_id, "column": column, "moved": false}));
            }
            let interrupt = json!({
                "type": "thread.turn.interrupt",
                "commandId": format!("kanban-done-{}", crate::orchestration_engine::uuid_v4()),
                "threadId": thread_id,
                "createdAt": now,
            });
            crate::orchestration_engine::handle("orchestration.dispatchCommand", &interrupt)
                .map_err(|e| format!("turn interrupt failed: {}", e.0))?;
            Ok(json!({"threadId": thread_id, "column": "done", "moved": true}))
        }
        _ => Err("Argument \"target\" must be inProgress or done.".into()),
    }
}

fn update_kanban_card(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let title = arg_str(args, "title");
    let description = arg_str(args, "description");
    if title.is_none() && description.is_none() {
        return Err("Provide at least one of title/description.".into());
    }
    let now = crate::todo::iso_now_ms();
    let mut cmd = serde_json::Map::new();
    cmd.insert("type".into(), json!("thread.meta.update"));
    cmd.insert("commandId".into(), json!(format!("kanban-edit-{}", crate::orchestration_engine::uuid_v4())));
    cmd.insert("threadId".into(), json!(thread_id));
    cmd.insert("createdAt".into(), json!(now));
    if let Some(t) = title {
        cmd.insert("title".into(), json!(t));
    }
    if let Some(d) = description {
        cmd.insert("notes".into(), json!(d));
    }
    crate::orchestration_engine::handle("orchestration.dispatchCommand", &Value::Object(cmd))
        .map_err(|e| format!("card update failed: {}", e.0))?;
    read_kanban_card(&json!({"threadId": thread_id}))
}

fn delete_kanban_card(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let now = crate::todo::iso_now_ms();
    let cmd = json!({
        "type": "thread.delete",
        "commandId": format!("kanban-delete-{}", crate::orchestration_engine::uuid_v4()),
        "threadId": thread_id,
        "createdAt": now,
    });
    crate::orchestration_engine::handle("orchestration.dispatchCommand", &cmd)
        .map_err(|e| format!("card delete failed: {}", e.0))?;
    Ok(json!({"threadId": thread_id, "deleted": true}))
}

fn normalize_memory_note(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut last_space = true;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_space = false;
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    out.trim().to_string()
}

fn memory_note_body(content: &str) -> String {
    let stripped = if content.starts_with('#') {
        content.splitn(2, '\n').nth(1).unwrap_or("")
    } else {
        content
    };
    normalize_memory_note(stripped)
}

fn memory_note_summary(note: &str) -> String {
    note.lines()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.trim().chars().take(140).collect())
        .unwrap_or_else(|| note.chars().take(140).collect())
}

fn sanitize_memory_title(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let mut out = String::with_capacity(cleaned.len());
    let mut last_space = true;
    for c in cleaned.chars() {
        if c == ' ' {
            if !last_space {
                out.push(c);
            }
            last_space = true;
        } else {
            out.push(c);
            last_space = false;
        }
    }
    out.trim().to_string()
}

fn slugify_memory_title(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = true;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug: String = slug.trim_matches('-').chars().take(48).collect::<String>().trim_end_matches('-').to_string();
    if slug.is_empty() { "note".into() } else { slug }
}

fn project_context_root(project_id: &str) -> std::path::PathBuf {
    std::path::Path::new(&crate::todo::state_dir())
        .join("project-context")
        .join(project_id)
}

fn mirror_document(project_id: &str, logical_path: &str, content: &str) {
    let root = project_context_root(&project_id);
    let target = root.join(logical_path);
    if target.strip_prefix(&root).is_err() || logical_path.contains("..") {
        return;
    }
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = target.with_extension("tmp");
    if std::fs::write(&tmp, content).is_ok() {
        let _ = std::fs::rename(&tmp, &target);
    }
}

fn doc_head(conn: &rusqlite::Connection, project_id: &str, logical_path: &str) -> Option<(String, i64)> {
    conn.query_row(
        "SELECT content, revision FROM project_agent_documents \
         WHERE project_id = ?1 AND logical_path = ?2 ORDER BY revision DESC LIMIT 1",
        rusqlite::params![project_id, logical_path],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
    )
    .ok()
}

fn upsert_document(conn: &rusqlite::Connection, project_id: &str, logical_path: &str, content: &str, author_thread: Option<&str>) -> Result<(), String> {
    let next_revision = conn
        .query_row(
            "SELECT COALESCE(MAX(revision), 0) + 1 FROM project_agent_documents \
             WHERE project_id = ?1 AND logical_path = ?2",
            rusqlite::params![project_id, logical_path],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(1);
    let hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        content.hash(&mut h);
        format!("{:x}", h.finish())
    };
    conn.execute(
        "INSERT INTO project_agent_documents \
         (revision_id, project_id, logical_path, revision, content, content_hash, \
          author_kind, author_thread_id, sources_json, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,'thread',?7,'[]',?8)",
        rusqlite::params![
            crate::orchestration_engine::uuid_v4(),
            project_id,
            logical_path,
            next_revision,
            content,
            hash,
            author_thread,
            crate::todo::iso_now_ms(),
        ],
    )
    .map_err(|e| e.to_string())?;
    mirror_document(&project_id, logical_path, &content);
    Ok(())
}

const MEMORY_INDEX_PATH: &str = "memory/MEMORY.md";

fn update_memory_index(
    conn: &rusqlite::Connection,
    project_id: &str,
    logical_path: &str,
    title: &str,
    summary: Option<&str>,
) -> Result<(), String> {
    let current = doc_head(conn, project_id, MEMORY_INDEX_PATH)
        .map(|(c, _)| c)
        .unwrap_or_else(|| "# Memory\n".into());
    let mut lines: Vec<String> = current.split('\n').map(str::to_string).collect();
    let target_suffix = format!("]({logical_path})");
    let matches_path = |line: &str| -> bool {
        if !line.starts_with("- [") {
            return false;
        }
        let link_segment = line.split(" \u{2014} ").next().unwrap_or(line);
        let link_segment = line.split('\u{2014}').next().unwrap_or(link_segment);
        link_segment.trim_end().ends_with(&target_suffix)
    };
    let index = lines.iter().position(|l| matches_path(l));
    match summary {
        None => {
            if let Some(i) = index {
                lines.remove(i);
            }
        }
        Some(summary) => {
            let entry = format!("- [{title}]({logical_path}) \u{2014} {summary}");
            match index {
                Some(i) => {
                    lines[i] = entry;
                }
                None => {
                    if !lines.last().map(|l| l.trim().is_empty()).unwrap_or(true) {
                        lines.push(String::new());
                    }
                    lines.push(entry);
                }
            }
            // Cap the index at 256 entries, dropping the oldest that is not
            // the one just written.
            while lines.iter().filter(|l| l.starts_with("- [")).count() > 256 {
                if let Some(oldest) = lines
                    .iter()
                    .position(|l| l.starts_with("- [") && !matches_path(l))
                {
                    lines.remove(oldest);
                } else {
                    break;
                }
            }
        }
    }
    let mut next = lines.join("\n");
    while next.contains("\n\n\n") {
        next = next.replace("\n\n\n", "\n\n");
    }
    if !next.ends_with('\n') {
        next.push('\n');
    }
    upsert_document(conn, project_id, MEMORY_INDEX_PATH, &next, None)
}

fn project_remember(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let note = arg_str(args, "note").ok_or("Missing required argument: note")?;
    let note = note.trim();
    if note.is_empty() {
        return Err("note must not be empty.".into());
    }
    let raw_title = arg_str(args, "title").unwrap_or_default();
    let mut title = sanitize_memory_title(&raw_title);
    if title.is_empty() {
        title = memory_note_summary(note).chars().take(60).collect();
    }
    if title.is_empty() {
        title = "Note".into();
    }
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let wanted = normalize_memory_note(note);
    // Scan existing dated memory notes for a near-duplicate.
    let mut stmt = conn
        .prepare(
            "SELECT logical_path, content FROM project_agent_documents d \
             WHERE project_id = ?1 AND logical_path LIKE 'memory/%.md' \
             AND revision = (SELECT MAX(revision) FROM project_agent_documents d2 \
               WHERE d2.project_id = d.project_id AND d2.logical_path = d.logical_path)",
        )
        .map_err(|e| e.to_string())?;
    let existing: Vec<(String, String)> = stmt
        .query_map(rusqlite::params![project_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let existing_paths: std::collections::HashSet<String> =
        existing.iter().map(|(p, _)| p.clone()).collect();
    if let Some((dup_path, dup_content)) = existing.iter().find(|(_, content)| {
        let have = memory_note_body(content);
        have == wanted
            || (have.len() >= 40 && wanted.len() >= 40 && (have.contains(&wanted) || wanted.contains(&have)))
    }) {
        let have = memory_note_body(dup_content);
        let extends_existing = wanted != have && wanted.contains(&have);
        if extends_existing {
            upsert_document(
                &conn,
                &project_id,
                dup_path,
                &format!("# {title}\n\n{note}\n"),
                caller_thread,
            )?;
        }
        update_memory_index(&conn, &project_id, dup_path, &title, Some(&memory_note_summary(note)))?;
        return Ok(json!({
            "path": dup_path,
            "updated": extends_existing,
            "deduplicated": true,
        }));
    }
    let today = crate::todo::iso_now_ms().get(..10).unwrap_or("1970-01-01").to_string();
    let slug = slugify_memory_title(&title);
    let mut logical_path = format!("memory/{today}-{slug}.md");
    let mut counter = 2;
    while existing_paths.contains(&logical_path) {
        logical_path = format!("memory/{today}-{slug}-{counter}.md");
        counter += 1;
    }
    upsert_document(&conn, &project_id, &logical_path, &format!("# {title}\n\n{note}\n"), caller_thread)?;
    update_memory_index(&conn, &project_id, &logical_path, &title, Some(&memory_note_summary(note)))?;
    Ok(json!({
        "path": logical_path,
        "updated": true,
        "deduplicated": false,
    }))
}

fn project_forget(args: &Value) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let path = arg_str(args, "path").ok_or("Missing required argument: path")?;
    static NOTE_PATTERN: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = NOTE_PATTERN.get_or_init(|| {
        regex::Regex::new(r"^memory/\d{4}-\d{2}-\d{2}-[a-z0-9][a-z0-9-]*\.md$").unwrap()
    });
    if !re.is_match(&path) {
        return Err("Only hub memory notes (memory/<date>-<slug>.md) can be forgotten.".into());
    }
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    if doc_head(&conn, &project_id, &path).is_none() {
        return Ok(json!({"deleted": false}));
    }
    conn.execute(
        "DELETE FROM project_agent_documents WHERE project_id = ?1 AND logical_path = ?2",
        rusqlite::params![project_id, path],
    )
    .map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(project_context_root(&project_id).join(&path));
    update_memory_index(&conn, &project_id, &path, "", None)?;
    Ok(json!({"deleted": true}))
}

fn hub_work_item_json(row: (&str, &str, i64, i64, &str, Option<String>, i64, i64, &str)) -> Value {
    let (work_item_id, project_id, _seq, task_index, state, worker_thread_id, slot_held, revision, record_json) = row;
    let mut item = serde_json::from_str::<Value>(record_json).unwrap_or_else(|_| json!({}));
    if !item.is_object() {
        item = json!({});
    }
    let obj = item.as_object_mut().unwrap();
    obj.insert("workItemId".into(), json!(work_item_id));
    obj.insert("projectId".into(), json!(project_id));
    obj.insert("taskIndex".into(), json!(task_index));
    obj.insert("state".into(), json!(state));
    obj.insert("workerThreadId".into(), json!(worker_thread_id));
    obj.insert("slotHeld".into(), json!(slot_held != 0));
    obj.insert("revision".into(), json!(revision));
    item
}

fn hub_list_work(caller_thread: Option<&str>) -> Result<Value, String> {
    let caller = caller_thread.ok_or("This tool requires a calling thread.")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let project_id: String = conn
        .query_row(
            "SELECT project_id FROM projection_threads WHERE thread_id = ?1",
            [caller],
            |r| r.get(0),
        )
        .map_err(|_| "This thread does not belong to a Hub.".to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT work_item_id, project_id, sequence, task_index, state, worker_thread_id, \
             slot_held, revision, record_json FROM project_agent_work_items \
             WHERE project_id = ?1 ORDER BY sequence LIMIT 200",
        )
        .map_err(|e| e.to_string())?;
    let items: Vec<Value> = stmt
        .query_map([&project_id], |r| {
            Ok(hub_work_item_json((
                &r.get::<_, String>(0)?,
                &r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                &r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
                &r.get::<_, String>(8)?,
            )))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(json!({"workItems": items}))
}

fn hub_load_work_row(
    conn: &rusqlite::Connection,
    work_item_id: &str,
) -> Result<(String, String, i64, i64, String, Option<String>, i64, i64, String), String> {
    conn.query_row(
        "SELECT work_item_id, project_id, sequence, task_index, state, worker_thread_id, \
         slot_held, revision, record_json FROM project_agent_work_items WHERE work_item_id = ?1",
        [work_item_id],
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Option<String>>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, i64>(7)?,
                r.get::<_, String>(8)?,
            ))
        },
    )
    .map_err(|_| format!("Hub work item \"{work_item_id}\" was not found."))
}

fn hub_update_progress(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let caller = caller_thread.ok_or("This tool requires a calling thread.")?;
    let work_item_id = arg_str(args, "workItemId").ok_or("Missing required argument: workItemId")?;
    let expected_revision = args
        .get("expectedRevision")
        .and_then(Value::as_i64)
        .ok_or("Missing required argument: expectedRevision")?;
    let steps = args
        .get("steps")
        .and_then(Value::as_array)
        .ok_or("Missing required argument: steps")?;
    if steps.len() > 32 {
        return Err("A progress checklist is limited to 32 steps.".into());
    }
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let row = hub_load_work_row(&conn, &work_item_id)?;
    let (_id, _pid, seq, _ti, state, worker_thread_id, slot_held, revision, record_json) = row;
    if worker_thread_id.as_deref() != Some(caller) {
        return Err("Only the assigned worker can update progress on this work item.".into());
    }
    if revision != expected_revision {
        return Err(format!(
            "Progress revision mismatch: expected {expected_revision}, current {revision}. Re-read with synara_hub_list_work."
        ));
    }
    let mut record = serde_json::from_str::<Value>(&record_json).unwrap_or_else(|_| json!({}));
    if !record.is_object() {
        record = json!({});
    }
    let mut progress = json!({"revision": expected_revision + 1, "steps": steps});
    if let Some(summary) = arg_str(args, "summary") {
        progress
            .as_object_mut()
            .unwrap()
            .insert("summary".into(), json!(summary));
    }
    record
        .as_object_mut()
        .unwrap()
        .insert("progress".into(), progress);
    let new_revision = revision + 1;
    conn.execute(
        "UPDATE project_agent_work_items SET revision = ?1, record_json = ?2 WHERE sequence = ?3",
        rusqlite::params![new_revision, record.to_string(), seq],
    )
    .map_err(|e| e.to_string())?;
    Ok(hub_work_item_json((
        &work_item_id,
        &_pid,
        seq,
        _ti,
        &state,
        worker_thread_id,
        slot_held,
        new_revision,
        &record.to_string(),
    )))
}

fn hub_cancel_work(args: &Value) -> Result<Value, String> {
    let work_item_id = arg_str(args, "workItemId").ok_or("Missing required argument: workItemId")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let row = hub_load_work_row(&conn, &work_item_id)?;
    let (_id, _pid, seq, _ti, state, worker_thread_id, slot_held, revision, record_json) = row;
    match state.as_str() {
        "queued" | "idle" | "pending" | "admitted" => {}
        _ => {
            return Err(format!(
                "Hub work item \"{work_item_id}\" is {state} and cannot be cancelled directly."
            ))
        }
    }
    let mut record = serde_json::from_str::<Value>(&record_json).unwrap_or_else(|_| json!({}));
    if record.is_object() {
        record
            .as_object_mut()
            .unwrap()
            .insert("cancelledAt".into(), json!(crate::todo::iso_now_ms()));
    }
    let new_revision = revision + 1;
    conn.execute(
        "UPDATE project_agent_work_items SET state = 'cancelled', revision = ?1, record_json = ?2, slot_held = 0 WHERE sequence = ?3",
        rusqlite::params![new_revision, record.to_string(), seq],
    )
    .map_err(|e| e.to_string())?;
    Ok(hub_work_item_json((
        &work_item_id,
        &_pid,
        seq,
        _ti,
        "cancelled",
        worker_thread_id,
        0,
        new_revision,
        &record.to_string(),
    )))
}

fn dispatch_command(command: Value) -> Result<Value, String> {
    crate::orchestration_engine::handle("orchestration.dispatchCommand", &command)
        .map_err(|e| e.0)
}


/// Cost telemetry for one thread: total USD, token splits and provider set,
/// read from `turn.completed` activities. Powers the gateway cost surface.
fn thread_cost_summary(conn: &rusqlite::Connection, thread_id: &str) -> Value {
    let mut stmt = match conn.prepare(
        "SELECT payload_json FROM projection_thread_activities WHERE thread_id = ?1 AND kind = 'turn.completed'",
    ) {
        Ok(s) => s,
        Err(_) => return Value::Null,
    };
    let rows = stmt
        .query_map([thread_id], |r| r.get::<_, String>(0))
        .map(|r| r.filter_map(|x| x.ok()).collect::<Vec<_>>())
        .unwrap_or_default();
    if rows.is_empty() {
        return Value::Null;
    }
    let mut cost = 0.0_f64;
    let mut input = 0.0_f64;
    let mut output = 0.0_f64;
    let mut providers: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut turns = 0_i64;
    for raw in rows {
        let Ok(p) = serde_json::from_str::<Value>(&raw) else { continue };
        turns += 1;
        if let Some(c) = p.get("totalCostUsd").and_then(Value::as_f64) {
            cost += c;
        }
        if let Some(pr) = p.get("provider").and_then(Value::as_str) {
            providers.insert(pr.to_string());
        }
        if let Some(usage) = p.get("modelUsage").and_then(Value::as_object) {
            for u in usage.values() {
                input += u.get("inputTokens").and_then(Value::as_f64).unwrap_or(0.0);
                output += u.get("outputTokens").and_then(Value::as_f64).unwrap_or(0.0);
            }
        }
    }
    json!({
        "costUsd": cost,
        "inputTokens": input,
        "outputTokens": output,
        "turns": turns,
        "providers": providers.into_iter().collect::<Vec<_>>(),
    })
}

fn deterministic_thread_id(request_id: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    request_id.hash(&mut h);
    let a = h.finish();
    let mut h2 = DefaultHasher::new();
    (request_id, "synara-thread").hash(&mut h2);
    let b = h2.finish();
    format!(
        "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        (a >> 32) as u32,
        (a >> 16) as u16,
        (a as u16) & 0x0fff,
        ((b >> 48) as u16 & 0x3fff) | 0x8000,
        b & 0xffffffffffff
    )
}

fn create_thread_tool(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let request_id = arg_str(args, "requestId").ok_or("Missing required argument: requestId")?;
    let thread_id = deterministic_thread_id(&format!("gateway:{request_id}"));
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    if conn
        .query_row(
            "SELECT 1 FROM projection_threads WHERE thread_id = ?1",
            [&thread_id],
            |_| Ok(()),
        )
        .is_ok()
    {
        return Ok(json!({"threadId": thread_id, "replayed": true}));
    }
    let now = crate::todo::iso_now_ms();
    let title = {
        let t = arg_str(args, "title").unwrap_or_default();
        let t = t.trim();
        if t.is_empty() {
            let prompt = arg_str(args, "prompt").unwrap_or_default();
            let first = prompt.lines().next().unwrap_or("New thread").trim();
            first.chars().take(80).collect::<String>()
        } else {
            t.chars().take(200).collect()
        }
    };
    let project_id = match arg_str(args, "projectId") {
        Some(p) if !p.trim().is_empty() => p,
        _ => {
            // Resolve from the calling thread, else fall back to the first project.
            let from_caller = caller_thread.and_then(|t| {
                conn.query_row(
                    "SELECT project_id FROM projection_threads WHERE thread_id = ?1",
                    [t],
                    |r| r.get::<_, String>(0),
                )
                .ok()
            });
            from_caller
                .or_else(|| {
                    conn.query_row("SELECT project_id FROM projection_projects LIMIT 1", [], |r| {
                        r.get::<_, String>(0)
                    })
                    .ok()
                })
                .ok_or("No project found: pass projectId explicitly.")?
        }
    };
    // Isolated worktree environment: the worktree is created before dispatch
    // so the thread projection is born pointing at it. Managed branch naming
    // matches the reclaim rule (`synara/*` branches are disposable).
    let mut worktree: Option<(String, String)> = None; // (path, branch)
    if arg_str(args, "environment").as_deref() == Some("worktree") {
        let repo_root: String = conn
            .query_row(
                "SELECT workspace_root FROM projection_projects WHERE project_id = ?1",
                [&project_id],
                |r| r.get(0),
            )
            .map_err(|_| "Project has no workspace to isolate.".to_string())?;
        let is_repo = std::process::Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(&repo_root)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !is_repo {
            return Err("Worktree isolation requires a git repository workspace.".into());
        }
        let base_ref = arg_str(args, "baseRef").unwrap_or_else(|| "HEAD".into());
        if base_ref.starts_with('#') || base_ref.starts_with("http") {
            return Err("Pull-request baseRef resolution is not supported yet; pass a local ref.".into());
        }
        if base_ref.starts_with('-') {
            return Err("Invalid baseRef.".into());
        }
        let branch = format!("synara/thread-{}", &thread_id[..13]);
        let repo_name = std::path::Path::new(&repo_root)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("repo")
            .to_string();
        let path = format!(
            "{}/{repo_name}/{thread_id}",
            crate::git_domain::worktrees_dir()
        );
        let out = std::process::Command::new("git")
            .args(["worktree", "add", "-b", &branch, &path, &base_ref])
            .current_dir(&repo_root)
            .output()
            .map_err(|e| format!("git worktree add failed: {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!(
                "git worktree add failed: {}",
                stderr.trim().lines().next().unwrap_or("git failed")
            ));
        }
        worktree = Some((path, branch));
    }
    let mut create = json!({
        "type": "thread.create",
        "commandId": format!("gateway-mcp-{request_id}-thread-create"),
        "threadId": thread_id,
        "projectId": project_id,
        "title": title,
        "createdAt": now,
    });
    if let Some((path, _branch)) = &worktree {
        let obj = create.as_object_mut().unwrap();
        obj.insert("envMode".into(), json!("worktree"));
        obj.insert("worktreePath".into(), json!(path));
        obj.insert("workingDirectory".into(), json!(path));
    }
    if let Some(provider) = arg_str(args, "provider") {
        create.as_object_mut().unwrap().insert("provider".into(), json!(provider));
    }
    if let Some(model) = arg_str(args, "model") {
        create.as_object_mut().unwrap().insert("model".into(), json!(model));
    }
    if let Err(e) = dispatch_command(create) {
        if e.contains("UNIQUE") || e.contains("already exists") {
            return Ok(json!({"threadId": thread_id, "replayed": true}));
        }
        return Err(format!("thread create failed: {e}"));
    }
    let started = if let Some(prompt) = arg_str(args, "prompt") {
        if !prompt.trim().is_empty() {
            dispatch_command(json!({
                "type": "thread.turn.start",
                "commandId": format!("gateway-mcp-{request_id}-turn-start"),
                "threadId": thread_id,
                "createdAt": crate::todo::iso_now_ms(),
                "dispatchMode": "queue",
                "dispatchOrigin": "agentGateway",
                "message": {
                    "messageId": format!("gateway-mcp-{request_id}-message"),
                    "text": prompt,
                    "attachments": [],
                },
            }))
            .map_err(|e| format!("turn start failed: {e}"))?;
            true
        } else {
            false
        }
    } else {
        false
    };
    Ok(json!({"threadId": thread_id, "started": started}))
}

fn send_message_tool(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let message = arg_str(args, "message").ok_or("Missing required argument: message")?;
    if message.trim().is_empty() {
        return Err("message must not be empty.".into());
    }
    let mode = arg_str(args, "mode").unwrap_or_else(|| "queue".into());
    if mode != "queue" && mode != "steer" {
        return Err(format!("Unknown mode \"{mode}\": use queue or steer."));
    }
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT 1 FROM projection_threads WHERE thread_id = ?1 AND deleted_at IS NULL",
        [&thread_id],
        |_| Ok(()),
    )
    .map_err(|_| format!("Thread \"{thread_id}\" was not found."))?;
    let key = crate::orchestration_engine::uuid_v4();
    dispatch_command(json!({
        "type": "thread.turn.start",
        "commandId": format!("gateway-mcp-{key}-turn-start"),
        "threadId": thread_id,
        "createdAt": crate::todo::iso_now_ms(),
        "dispatchMode": mode,
        "dispatchOrigin": "agentGateway",
        "message": {
            "messageId": format!("gateway-mcp-{key}-message"),
            "text": message,
            "attachments": [],
        },
    }))?;
    Ok(json!({"threadId": thread_id, "mode": mode, "sent": true}))
}

fn interrupt_thread_tool(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    dispatch_command(json!({
        "type": "thread.turn.interrupt",
        "commandId": format!("gateway-mcp-{}-interrupt", crate::orchestration_engine::uuid_v4()),
        "threadId": thread_id,
        "createdAt": crate::todo::iso_now_ms(),
    }))?;
    Ok(json!({"threadId": thread_id, "interrupted": true}))
}

fn set_thread_title_tool(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let title = arg_str(args, "title").ok_or("Missing required argument: title")?;
    let title: String = title.trim().chars().take(200).collect();
    if title.is_empty() {
        return Err("title must not be empty.".into());
    }
    dispatch_command(json!({
        "type": "thread.meta.update",
        "commandId": format!("gateway-mcp-{}-meta", crate::orchestration_engine::uuid_v4()),
        "threadId": thread_id,
        "createdAt": crate::todo::iso_now_ms(),
        "title": title,
    }))?;
    Ok(json!({"threadId": thread_id, "title": title}))
}

fn set_thread_archived_tool(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let archived = args
        .get("archived")
        .and_then(Value::as_bool)
        .ok_or("Missing required argument: archived")?;
    dispatch_command(json!({
        "type": if archived { "thread.archive" } else { "thread.unarchive" },
        "commandId": format!("gateway-mcp-{}-archive", crate::orchestration_engine::uuid_v4()),
        "threadId": thread_id,
        "createdAt": crate::todo::iso_now_ms(),
    }))?;
    Ok(json!({"threadId": thread_id, "archived": archived}))
}

fn read_thread_runtime_events(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let limit = args
        .get("limit")
        .and_then(Value::as_i64)
        .unwrap_or(50)
        .clamp(1, 200);
    let turn_id = arg_str(args, "turnId");
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT 1 FROM projection_threads WHERE thread_id = ?1",
        [&thread_id],
        |_| Ok(()),
    )
    .map_err(|_| format!("Thread \"{thread_id}\" was not found."))?;
    let (high_water, oldest, retained): (i64, i64, i64) = conn
        .query_row(
            "SELECT COALESCE(MAX(sequence),0), COALESCE(MIN(sequence),0), COUNT(*) \
             FROM provider_runtime_events WHERE thread_id = ?1",
            [&thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap_or((0, 0, 0));
    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match &turn_id {
        Some(t) => (
            "SELECT sequence, event_id, event_type, turn_id, event_json, persisted_at \
             FROM provider_runtime_events WHERE thread_id = ?1 AND turn_id = ?2 \
             ORDER BY sequence DESC LIMIT ?3"
                .to_string(),
            vec![
                Box::new(thread_id.clone()),
                Box::new(t.clone()),
                Box::new(limit),
            ],
        ),
        None => (
            "SELECT sequence, event_id, event_type, turn_id, event_json, persisted_at \
             FROM provider_runtime_events WHERE thread_id = ?1 \
             ORDER BY sequence DESC LIMIT ?2"
                .to_string(),
            vec![Box::new(thread_id.clone()), Box::new(limit)],
        ),
    };
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut events: Vec<Value> = stmt
        .query_map(param_refs.as_slice(), |r| {
            let raw: String = r.get(4)?;
            let event = serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null);
            Ok(json!({
                "sequence": r.get::<_, i64>(0)?,
                "eventId": r.get::<_, String>(1)?,
                "type": r.get::<_, String>(2)?,
                "turnId": r.get::<_, Option<String>>(3)?,
                "createdAt": r.get::<_, String>(5)?,
                "provider": event.get("provider").cloned().unwrap_or(Value::Null),
                "itemId": event.get("itemId").cloned().unwrap_or(Value::Null),
                "requestId": event.get("requestId").cloned().unwrap_or(Value::Null),
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    events.reverse();
    Ok(json!({
        "threadId": thread_id,
        "events": events,
        "coverage": {
            "source": "provider_runtime_events",
            "highWaterSequence": high_water,
            "oldestRetainedSequence": oldest,
            "retainedForThread": retained,
            "sourceComplete": false,
        },
    }))
}

fn list_connections() -> Result<Value, String> {
    let env = crate::server_env::get_environment(&crate::todo::state_dir()).map_err(|e| e)?;
    let environment_id = env.get("environmentId").cloned().unwrap_or(Value::Null);
    let label = env.pointer("/label").cloned().unwrap_or(Value::Null);
    let remote = env
        .pointer("/capabilities/remoteConnections")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(json!({
        "currentEnvironmentId": environment_id,
        "remoteAvailable": remote,
        "computers": [{
            "environmentId": environment_id,
            "label": label,
            "local": true,
        }],
    }))
}

fn wait_for_threads(args: &Value) -> Result<Value, String> {
    let thread_ids: Vec<String> = args
        .get("threadIds")
        .and_then(Value::as_array)
        .ok_or("Missing required argument: threadIds")?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    if thread_ids.is_empty() || thread_ids.len() > 20 {
        return Err("threadIds must contain 1-20 threads.".into());
    }
    let timeout_ms = args
        .get("timeoutMs")
        .and_then(Value::as_i64)
        .unwrap_or(30_000)
        .clamp(0, 60_000);
    let started = std::time::Instant::now();
    let deadline = started + std::time::Duration::from_millis(timeout_ms as u64);
    loop {
        let conn = state_conn()?;
        conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
        let mut results = Vec::new();
        let mut all_terminal = true;
        for tid in &thread_ids {
            let exists = conn
                .query_row(
                    "SELECT 1 FROM projection_threads WHERE thread_id = ?1",
                    [tid],
                    |_| Ok(()),
                )
                .is_ok();
            if !exists {
                results.push(json!({
                    "threadId": tid,
                    "runId": Value::Null,
                    "status": "missing",
                }));
                continue;
            }
            let turn: Option<(String, String, Option<String>)> = conn
                .query_row(
                    "SELECT turn_id, state, completed_at FROM projection_turns \
                     WHERE thread_id = ?1 ORDER BY COALESCE(started_at, requested_at) DESC LIMIT 1",
                    [tid],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .ok();
            let Some((turn_id, state, completed_at)) = turn else {
                results.push(json!({
                    "threadId": tid,
                    "runId": Value::Null,
                    "status": "idle",
                }));
                continue;
            };
            let terminal = matches!(
                state.as_str(),
                "completed" | "failed" | "interrupted" | "cancelled" | "error"
            );
            if !terminal {
                all_terminal = false;
            }
            let status = if terminal { state.clone() } else { "running".to_string() };
            let mut row = json!({
                "threadId": tid,
                "runId": turn_id,
                "status": status,
            });
            if terminal {
                if let Some(ca) = completed_at {
                    row.as_object_mut().unwrap().insert("completedAt".into(), json!(ca));
                }
                let summary: Option<String> = conn
                    .query_row(
                        "SELECT substr(content, 1, 500) FROM projection_thread_messages \
                         WHERE thread_id = ?1 AND role = 'assistant' ORDER BY created_at DESC LIMIT 1",
                        [tid],
                        |r| r.get(0),
                    )
                    .ok();
                if let Some(s) = summary.filter(|s| !s.trim().is_empty()) {
                    row.as_object_mut().unwrap().insert("summary".into(), json!(s));
                }
            }
            results.push(row);
        }
        let timed_out = !all_terminal && std::time::Instant::now() >= deadline;
        if all_terminal || timed_out || timeout_ms == 0 {
            return Ok(json!({
                "allTerminal": all_terminal,
                "timedOut": timed_out,
                "results": results,
            }));
        }
        drop(conn);
        std::thread::sleep(std::time::Duration::from_millis(400));
    }
}

fn library_root(project_id: &str) -> std::path::PathBuf {
    project_context_root(project_id).join("library")
}

fn safe_relative(path: &str) -> Result<std::path::PathBuf, String> {
    let rel = std::path::Path::new(path);
    if rel.is_absolute() || path.split(['/', '\\']).any(|s| s == "..") {
        return Err("Path must be relative without '..' segments.".into());
    }
    Ok(rel.to_path_buf())
}

fn library_list(args: &Value) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let root = library_root(&project_id);
    let rel = match arg_str(args, "relativePath") {
        Some(r) if !r.is_empty() => safe_relative(&r)?,
        _ => std::path::PathBuf::new(),
    };
    let dir = root.join(&rel);
    if dir.strip_prefix(&root).is_err() {
        return Err("Path escapes the library root.".into());
    }
    let mut entries = Vec::new();
    if dir.is_dir() {
        if let Ok(read) = std::fs::read_dir(&dir) {
            for entry in read.flatten().take(500) {
                let meta = entry.metadata().ok();
                entries.push(json!({
                    "name": entry.file_name().to_string_lossy(),
                    "kind": if meta.as_ref().map(|m| m.is_dir()).unwrap_or(false) { "directory" } else { "file" },
                    "sizeBytes": meta.map(|m| m.len()).unwrap_or(0),
                }));
            }
        }
    }
    entries.sort_by_key(|e| e.get("name").and_then(Value::as_str).unwrap_or("").to_string());
    Ok(json!({
        "libraryRoot": root.to_string_lossy(),
        "path": rel.to_string_lossy(),
        "entries": entries,
    }))
}

fn copy_tree_guarded(
    source: &std::path::Path,
    target: &std::path::Path,
    filter_name: bool,
    file_count: &mut usize,
    total_bytes: &mut u64,
) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(source).map_err(|e| e.to_string())?;
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if meta.is_dir() {
        if filter_name {
            let name = source.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name == ".git" || name == "node_modules" {
                return Ok(());
            }
        }
        std::fs::create_dir_all(target).map_err(|e| e.to_string())?;
        for entry in std::fs::read_dir(source).map_err(|e| e.to_string())?.flatten() {
            copy_tree_guarded(&entry.path(), &target.join(entry.file_name()), true, file_count, total_bytes)?;
        }
        return Ok(());
    }
    if !meta.is_file() {
        return Ok(());
    }
    *file_count += 1;
    *total_bytes += meta.len();
    if *file_count > 10_000 {
        return Err("A library add is limited to 10000 files.".into());
    }
    if *total_bytes > 256 * 1024 * 1024 {
        return Err("A library add is limited to 256 MB total.".into());
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::copy(source, target).map_err(|e| e.to_string())?;
    Ok(())
}

fn library_add(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let source_path = arg_str(args, "sourcePath").ok_or("Missing required argument: sourcePath")?;
    let caller = caller_thread.ok_or("This tool requires a calling thread.")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let workspace: String = conn
        .query_row(
            "SELECT COALESCE(t.worktree_path, t.working_directory, (SELECT p.workspace_root FROM projection_projects p WHERE p.project_id = t.project_id)) FROM projection_threads t WHERE t.thread_id = ?1",
            [caller],
            |r| r.get(0),
        )
        .map_err(|_| "This thread does not belong to a Hub.".to_string())?;
    let source_raw = std::path::Path::new(&source_path);
    let source = if source_raw.is_absolute() {
        source_raw.to_path_buf()
    } else {
        std::path::Path::new(&workspace).join(source_raw)
    };
    let canonical_source = source.canonicalize().map_err(|_| format!("Source \"{source_path}\" was not found."))?;
    let canonical_workspace = std::path::Path::new(&workspace)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(&workspace));
    if !canonical_source.starts_with(&canonical_workspace) {
        return Err("sourcePath must resolve inside your workspace.".into());
    }
    let default_name = canonical_source
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "library-item".into());
    let dest_rel = match arg_str(args, "destinationPath") {
        Some(d) if !d.is_empty() => safe_relative(&d)?,
        _ => std::path::PathBuf::from(&default_name),
    };
    let root = library_root(&project_id);
    let target = root.join(&dest_rel);
    if target.strip_prefix(&root).is_err() {
        return Err("destinationPath escapes the library root.".into());
    }
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;
    copy_tree_guarded(&canonical_source, &target, false, &mut file_count, &mut total_bytes)?;
    // Commit inside the library when it is a git repo, mirroring upstream.
    if root.join(".git").exists() {
        let root_str = root.to_string_lossy().to_string();
        std::thread::spawn(move || {
            let _ = std::process::Command::new("git")
                .args(["-C", &root_str, "add", "-A"])
                .output();
            let _ = std::process::Command::new("git")
                .args([
                    "-C",
                    &root_str,
                    "-c",
                    "user.name=Synara Hub",
                    "-c",
                    "user.email=hub@synara.local",
                    "commit",
                    "-m",
                    "library add",
                    "--no-verify",
                ])
                .output();
        });
    }
    Ok(json!({
        "path": dest_rel.to_string_lossy(),
        "filesCopied": file_count,
        "bytesCopied": total_bytes,
    }))
}

fn project_get_overview(args: &Value) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let goal: Option<Value> = conn
        .query_row(
            "SELECT goal_id, objective, status, revision FROM project_agent_goals \
             WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 1",
            [&project_id],
            |r| {
                Ok(json!({
                    "goalId": r.get::<_, String>(0)?,
                    "objective": r.get::<_, String>(1)?,
                    "status": r.get::<_, String>(2)?,
                    "revision": r.get::<_, i64>(3)?,
                }))
            },
        )
        .ok();
    let focus: Option<String> = conn
        .query_row(
            "SELECT content FROM project_agent_documents \
             WHERE project_id = ?1 AND logical_path = 'focus.md' ORDER BY revision DESC LIMIT 1",
            [&project_id],
            |r| r.get(0),
        )
        .ok();
    let mut blockers_stmt = conn
        .prepare(
            "SELECT task_id, title, status FROM project_agent_tasks \
             WHERE project_id = ?1 AND archived_at IS NULL AND status IN ('blocked','failed','review') \
             ORDER BY task_id LIMIT 50",
        )
        .map_err(|e| e.to_string())?;
    let blockers: Vec<Value> = blockers_stmt
        .query_map([&project_id], |r| {
            Ok(json!({
                "taskId": r.get::<_, String>(0)?,
                "title": r.get::<_, Option<String>>(1)?,
                "status": r.get::<_, Option<String>>(2)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    let last_summary: Option<String> = conn
        .query_row(
            "SELECT summary FROM project_agent_activity \
             WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 1",
            [&project_id],
            |r| r.get(0),
        )
        .ok();
    let mut repos_stmt = conn
        .prepare(
            "SELECT linked_project_id FROM project_agent_linked_projects WHERE project_id = ?1",
        )
        .map_err(|e| e.to_string())?;
    let linked: Vec<String> = repos_stmt
        .query_map([&project_id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(json!({
        "projectId": project_id,
        "goal": goal,
        "focus": focus,
        "blockers": blockers,
        "lastSummary": last_summary,
        "linkedRepositories": linked,
    }))
}

fn project_context(args: &Value) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    const CAP: usize = 32_000;
    let mut packet = String::new();
    let mut push_section = |title: &str, body: &str| {
        if packet.len() >= CAP {
            return;
        }
        let section = format!("## {title}\n\n{}\n\n", body.trim());
        let remaining = CAP - packet.len();
        if section.len() <= remaining {
            packet.push_str(&section);
        } else {
            packet.push_str(&section[..remaining]);
        }
    };
    if let Some((objective, status)) = conn
        .query_row(
            "SELECT objective, status FROM project_agent_goals \
             WHERE project_id = ?1 ORDER BY created_at DESC LIMIT 1",
            [&project_id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .ok()
    {
        push_section("Goal", &format!("{objective}\n\nStatus: {status}"));
    }
    for (title, path) in [
        ("Instructions", "instructions.md"),
        ("Memory index", "memory/MEMORY.md"),
        ("Decisions", "decisions.md"),
        ("Focus", "focus.md"),
    ] {
        if let Some((content, _rev)) = doc_head(&conn, &project_id, path) {
            if !content.trim().is_empty() {
                push_section(title, &content);
            }
        }
    }
    let mut tasks_stmt = conn
        .prepare(
            "SELECT task_id, title, status FROM project_agent_tasks \
             WHERE project_id = ?1 AND archived_at IS NULL ORDER BY task_id LIMIT 100",
        )
        .map_err(|e| e.to_string())?;
    let tasks: Vec<String> = tasks_stmt
        .query_map([&project_id], |r| {
            Ok(format!(
                "- {} ({})",
                r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                r.get::<_, Option<String>>(2)?.unwrap_or_default()
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    if !tasks.is_empty() {
        push_section("Tasks", &tasks.join("\n"));
    }
    Ok(json!({
        "projectId": project_id,
        "packet": packet,
        "truncated": packet.len() >= CAP,
    }))
}

fn project_report_result(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let request_id = arg_str(args, "requestId").ok_or("Missing required argument: requestId")?;
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let task_id = arg_str(args, "taskId").ok_or("Missing required argument: taskId")?;
    let summary = arg_str(args, "summary").ok_or("Missing required argument: summary")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    // Exactly-once replay on requestId via the receipts table.
    let receipt_key = format!("report-result:{request_id}");
    if let Ok(json) = conn.query_row(
        "SELECT result_json FROM project_agent_receipts WHERE request_id = ?1",
        [&receipt_key],
        |r| r.get::<_, String>(0),
    ) {
        return serde_json::from_str::<Value>(&json).map_err(|e| e.to_string());
    }
    let status: String = conn
        .query_row(
            "SELECT status FROM project_agent_tasks WHERE project_id = ?1 AND task_id = ?2",
            rusqlite::params![project_id, task_id],
            |r| r.get(0),
        )
        .map_err(|_| format!("Task \"{task_id}\" was not found in this project."))?;
    let now = crate::todo::iso_now_ms();
    conn.execute(
        "INSERT INTO project_agent_evidence \
         (evidence_id, project_id, task_id, attempt_id, kind, classification, author_kind, \
          author_thread_id, source_thread_id, summary, created_at) \
         VALUES (?1,?2,?3,NULL,'result','worker','worker',?4,?4,?5,?6)",
        rusqlite::params![
            crate::orchestration_engine::uuid_v4(),
            project_id,
            task_id,
            caller_thread,
            summary,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;
    if status != "review" {
        conn.execute(
            "UPDATE project_agent_tasks SET status = 'review', revision = revision + 1 \
             WHERE project_id = ?1 AND task_id = ?2",
            rusqlite::params![project_id, task_id],
        )
        .map_err(|e| e.to_string())?;
    }
    let result = json!({"taskId": task_id, "status": "review", "recorded": true});
    let _ = conn.execute(
        "INSERT OR IGNORE INTO project_agent_receipts (request_id, project_id, operation, result_json, created_at) \
         VALUES (?1,?2,'report_result',?3,?4)",
        rusqlite::params![receipt_key, project_id, result.to_string(), now],
    );
    Ok(result)
}

fn update_automation_memory(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let memory = arg_str(args, "memory").ok_or("Missing required argument: memory")?;
    if memory.len() > 32 * 1024 {
        return Err("Automation memory is limited to 32 KiB.".into());
    }
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let automation_id = match arg_str(args, "automationId") {
        Some(id) => id,
        None => {
            // Resolve from the caller thread's latest automation run.
            let caller = caller_thread.ok_or(
                "automationId is required outside an automation run envelope.",
            )?;
            conn.query_row(
                "SELECT automation_id FROM automation_runs WHERE thread_id = ?1 \
                 ORDER BY created_at DESC LIMIT 1",
                [caller],
                |r| r.get::<_, String>(0),
            )
            .map_err(|_| {
                "automationId is required: no automation run found for this thread.".to_string()
            })?
        }
    };
    conn.query_row(
        "SELECT 1 FROM automation_definitions WHERE automation_id = ?1",
        [&automation_id],
        |_| Ok(()),
    )
    .map_err(|_| format!("Automation \"{automation_id}\" was not found."))?;
    conn.execute(
        "INSERT INTO automation_memory (automation_id, content, updated_at) VALUES (?1,?2,?3) \
         ON CONFLICT(automation_id) DO UPDATE SET content = excluded.content, updated_at = excluded.updated_at",
        rusqlite::params![automation_id, memory, crate::todo::iso_now_ms()],
    )
    .map_err(|e| e.to_string())?;
    Ok(json!({"memory": memory, "automationId": automation_id}))
}

fn report_automation_result(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let decision = arg_str(args, "decision").ok_or("Missing required argument: decision")?;
    if decision != "notify" && decision != "silent" {
        return Err("Argument \"decision\" must be \"notify\" or \"silent\".".into());
    }
    let caller = caller_thread.ok_or(
        "This tool must be called from the automation run thread.",
    )?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let (run_id, automation_id, status): (String, String, String) = conn
        .query_row(
            "SELECT run_id, automation_id, status FROM automation_runs WHERE thread_id = ?1 \
             ORDER BY created_at DESC LIMIT 1",
            [caller],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| "No automation run found for this thread.".to_string())?;
    let mut result = json!({"decision": decision});
    if let Some(title) = arg_str(args, "title") {
        result.as_object_mut().unwrap().insert("title".into(), json!(title));
    }
    if let Some(summary) = arg_str(args, "summary") {
        result.as_object_mut().unwrap().insert("summary".into(), json!(summary));
    }
    conn.execute(
        "UPDATE automation_runs SET result_json = ?1, updated_at = ?2 WHERE run_id = ?3",
        rusqlite::params![result.to_string(), crate::todo::iso_now_ms(), run_id],
    )
    .map_err(|e| e.to_string())?;
    let _ = status;
    Ok(json!({
        "automationId": automation_id,
        "runId": run_id,
        "decision": decision,
    }))
}

fn resolve_thread_pr_url(input: &str) -> String {
    input.trim().to_string()
}

fn set_thread_pull_request(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId")
        .or_else(|| caller_thread.map(str::to_string))
        .ok_or("Missing required argument: threadId")?;
    let url = arg_str(args, "pullRequestUrl").ok_or("Missing required argument: pullRequestUrl")?;
    let url = resolve_thread_pr_url(&url);
    if !url.contains("github.com") && !url.chars().all(|c| c.is_ascii_digit()) {
        return Err("pullRequestUrl must be a GitHub pull request URL or number.".into());
    }
    dispatch_command(json!({
        "type": "thread.pull-request.link",
        "commandId": format!("gateway-mcp-{}-pr-link", crate::orchestration_engine::uuid_v4()),
        "threadId": thread_id,
        "createdAt": crate::todo::iso_now_ms(),
        "source": "user",
        "pullRequest": {"url": url},
    }))?;
    Ok(json!({"threadId": thread_id, "linked": true, "pullRequestUrl": url}))
}

fn unlink_thread_pull_request(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId")
        .or_else(|| caller_thread.map(str::to_string))
        .ok_or("Missing required argument: threadId")?;
    let url = arg_str(args, "pullRequestUrl").ok_or("Missing required argument: pullRequestUrl")?;
    dispatch_command(json!({
        "type": "thread.pull-request.unlink",
        "commandId": format!("gateway-mcp-{}-pr-unlink", crate::orchestration_engine::uuid_v4()),
        "threadId": thread_id,
        "createdAt": crate::todo::iso_now_ms(),
        "pullRequest": {"url": url.trim()},
    }))?;
    Ok(json!({"threadId": thread_id, "unlinked": true}))
}

fn set_thread_goal_impl(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let goal = match args.get("goal") {
        Some(Value::String(g)) if !g.trim().is_empty() => Some(g.trim().chars().take(4000).collect::<String>()),
        _ => None,
    };
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let archived: bool = conn
        .query_row(
            "SELECT archived_at IS NOT NULL FROM projection_threads WHERE thread_id = ?1 AND deleted_at IS NULL",
            [&thread_id],
            |r| r.get(0),
        )
        .map_err(|_| format!("Thread \"{thread_id}\" was not found."))?;
    if archived {
        return Err("Archived threads cannot take a new goal.".into());
    }
    let mut command = json!({
        "type": "thread.meta.update",
        "commandId": format!("gateway-mcp-{}-goal", crate::orchestration_engine::uuid_v4()),
        "threadId": thread_id,
        "createdAt": crate::todo::iso_now_ms(),
    });
    match &goal {
        Some(g) => {
            command.as_object_mut().unwrap().insert("goal".into(), json!(g));
        }
        None => {
            command.as_object_mut().unwrap().insert("goal".into(), json!(""));
        }
    }
    dispatch_command(command)?;
    Ok(json!({"threadId": thread_id, "goal": goal}))
}

fn link_repository(args: &Value) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let linked = arg_str(args, "linkedProjectId").ok_or("Missing required argument: linkedProjectId")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    for id in [&project_id, &linked] {
        conn.query_row(
            "SELECT 1 FROM projection_projects WHERE project_id = ?1 AND deleted_at IS NULL",
            [id],
            |_| Ok(()),
        )
        .map_err(|_| format!("Project \"{id}\" was not found."))?;
    }
    conn.execute(
        "INSERT OR IGNORE INTO project_agent_linked_projects (project_id, linked_project_id, created_at) \
         VALUES (?1,?2,?3)",
        rusqlite::params![project_id, linked, crate::todo::iso_now_ms()],
    )
    .map_err(|e| e.to_string())?;
    Ok(json!({"projectId": project_id, "linkedProjectId": linked, "linked": true}))
}

fn diagnose_thread(args: &Value) -> Result<Value, String> {
    let thread_id = arg_str(args, "threadId").ok_or("Missing required argument: threadId")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let shell: Option<(String, Option<String>, Option<String>, i64, i64)> = conn
        .query_row(
            "SELECT title, archived_at, deleted_at, pending_approval_count, pending_user_input_count \
             FROM projection_threads WHERE thread_id = ?1",
            [&thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .ok();
    let Some((title, archived_at, deleted_at, pending_approvals, pending_inputs)) = shell else {
        return Err(format!("Thread \"{thread_id}\" was not found."));
    };
    let session: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT status, last_error FROM projection_thread_sessions WHERE thread_id = ?1",
            [&thread_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let turn: Option<(String, String)> = conn
        .query_row(
            "SELECT turn_id, state FROM projection_turns WHERE thread_id = ?1 \
             ORDER BY COALESCE(started_at, requested_at) DESC LIMIT 1",
            [&thread_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .ok();
    let message_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM projection_thread_messages WHERE thread_id = ?1",
            [&thread_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let last_message_at: Option<String> = conn
        .query_row(
            "SELECT created_at FROM projection_thread_messages WHERE thread_id = ?1 \
             ORDER BY created_at DESC LIMIT 1",
            [&thread_id],
            |r| r.get(0),
        )
        .ok();
    let runtime_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provider_runtime_events WHERE thread_id = ?1",
            [&thread_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let runtime_errors: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT event_json FROM provider_runtime_events WHERE thread_id = ?1 \
             AND event_type LIKE '%error%' ORDER BY sequence DESC LIMIT 5",
        ).map_err(|e| e.to_string())?;
        stmt.query_map([&thread_id], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
    };
    let durable_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM orchestration_events WHERE stream_id = ?1",
            [&thread_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let mut findings: Vec<String> = Vec::new();
    if deleted_at.is_some() {
        findings.push("thread is deleted".into());
    }
    if let Some((status, last_error)) = &session {
        if status == "error" || status == "failed" {
            findings.push(format!(
                "session status is {status}{}",
                last_error.as_deref().map(|e| format!(": {e}")).unwrap_or_default()
            ));
        }
    }
    if let Some((_, state)) = &turn {
        if state == "failed" || state == "error" {
            findings.push(format!("latest turn is {state}"));
        }
        if state == "interrupted" {
            findings.push("latest turn was interrupted".into());
        }
    }
    if pending_approvals > 0 {
        findings.push(format!("{pending_approvals} pending approval(s) blocking progress"));
    }
    if pending_inputs > 0 {
        findings.push(format!("{pending_inputs} pending user input request(s)"));
    }
    if !runtime_errors.is_empty() {
        findings.push(format!("{} recent provider runtime error event(s)", runtime_errors.len()));
    }
    if findings.is_empty() {
        findings.push("no obvious blockers found in projections".into());
    }
    Ok(json!({
        "threadId": thread_id,
        "title": title,
        "archived": archived_at.is_some(),
        "session": session.map(|(s, e)| json!({"status": s, "lastError": e})),
        "latestTurn": turn.map(|(id, state)| json!({"turnId": id, "state": state})),
        "messageCount": message_count,
        "lastMessageAt": last_message_at,
        "durableEventCount": durable_events,
        "runtimeEventCount": runtime_events,
        "pendingApprovals": pending_approvals,
        "pendingUserInputs": pending_inputs,
        "findings": findings,
    }))
}

fn list_mcp_connectors_tool() -> Result<Value, String> {
    crate::mcp_connectors::handle("server.listMcpConnectors", &Value::Null)
        .map(|v| json!({"connectors": v}))
        .map_err(|e| e.0)
}

fn create_threads_batch(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let request_id = arg_str(args, "requestId").ok_or("Missing required argument: requestId")?;
    let threads = args
        .get("threads")
        .and_then(Value::as_array)
        .ok_or("Missing required argument: threads")?;
    if threads.is_empty() || threads.len() > 20 {
        return Err("threads must contain 1-20 items.".into());
    }
    // Pre-validate everything before creating anything.
    for (i, item) in threads.iter().enumerate() {
        match item.get("prompt").and_then(Value::as_str) {
            Some(p) if !p.trim().is_empty() => {}
            _ => return Err(format!("threads[{i}].prompt is required.")),
        }
    }
    // Hub coordinators never create threads directly: their tasks go into the
    // durable hub work queue, where keys freeze at first decision and the
    // admission loop spawns workers. `accepted` does not mean `started`.
    if let Some(caller) = caller_thread {
        let is_coordinator = state_conn()
            .ok()
            .and_then(|conn| {
                conn.query_row(
                    "SELECT project_id FROM project_agent_configs WHERE coordinator_thread_id = ?1 AND enabled = 1",
                    [caller],
                    |r| r.get::<_, String>(0),
                )
                .ok()
            })
            .is_some();
        if is_coordinator {
            let tasks: Vec<Value> = threads
                .iter()
                .map(|item| {
                    let mut spec = json!({
                        "prompt": item.get("prompt").cloned().unwrap_or(Value::Null),
                    });
                    for key in ["title", "projectId", "taskKey", "contextMessageIds"] {
                        if let Some(v) = item.get(key) {
                            spec.as_object_mut().unwrap().insert(key.into(), v.clone());
                        }
                    }
                    json!({"spec": spec, "title": item.get("title").cloned().unwrap_or(Value::Null)})
                })
                .collect();
            // The calling turn's latest user message freezes as the source request.
            let source_messages = state_conn()
                .ok()
                .and_then(|conn| {
                    conn.query_row(
                        "SELECT message_id FROM projection_thread_messages \
                         WHERE thread_id = ?1 AND role = 'user' ORDER BY created_at DESC LIMIT 1",
                        [caller],
                        |r| r.get::<_, String>(0),
                    )
                    .ok()
                })
                .map(|mid| vec![json!({"threadId": caller, "messageId": mid})])
                .unwrap_or_else(|| vec![json!({"threadId": caller, "messageId": format!("gateway-{request_id}")})]);
            let outcome = crate::hub_work::submit(
                caller,
                None,
                &request_id,
                &source_messages,
                &tasks,
            )
            .map_err(|e| e.0)?;
            let items: Vec<Value> = outcome
                .items
                .iter()
                .map(|record| {
                    json!({
                        "workItemId": record.get("workItemId"),
                        "state": record.get("state"),
                        "title": record.get("title"),
                        "accepted": true,
                    })
                })
                .collect();
            return Ok(json!({
                "hubWork": true,
                "replayed": outcome.replayed,
                "workItems": items,
            }));
        }
    }
    let mut results = Vec::new();
    for (i, item) in threads.iter().enumerate() {
        let mut item_args = json!({
            "requestId": format!("batch-{request_id}-{i}"),
            "prompt": item.get("prompt").cloned().unwrap_or(Value::Null),
        });
        for key in ["title", "projectId"] {
            if let Some(v) = item.get(key) {
                item_args.as_object_mut().unwrap().insert(key.into(), v.clone());
            }
        }
        if let Some(target) = item.get("target").and_then(Value::as_object) {
            for key in ["provider", "model"] {
                if let Some(v) = target.get(key) {
                    item_args.as_object_mut().unwrap().insert(key.into(), v.clone());
                }
            }
        }
        match create_thread_tool(&item_args, caller_thread) {
            Ok(mut result) => {
                let link = result
                    .get("threadId")
                    .and_then(Value::as_str)
                    .map(|tid| format!("thread://{tid}"));
                if let Some(link) = link {
                    result
                        .as_object_mut()
                        .unwrap()
                        .insert("link".into(), json!(link));
                }
                result.as_object_mut().unwrap().insert("index".into(), json!(i));
                results.push(result);
            }
            Err(e) => {
                return Err(format!("threads[{i}] creation failed: {e}"));
            }
        }
    }
    Ok(json!({"threads": results}))
}

fn update_plan(args: &Value) -> Result<Value, String> {
    let items = args
        .get("items")
        .and_then(Value::as_array)
        .ok_or("items needs at least one step: [{content, status}].")?;
    if items.is_empty() {
        return Err("items needs at least one step: [{content, status}].".into());
    }
    if items.len() > 12 {
        return Err(format!("items has {} steps; keep the plan to 12.", items.len()));
    }
    let mut in_progress = 0usize;
    let mut remaining = 0usize;
    let mut steps = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let content = item
            .get("content")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        if content.is_empty() {
            return Err(format!("items[{index}].content must be a non-empty string."));
        }
        let status = item.get("status").and_then(Value::as_str).unwrap_or("");
        if !matches!(status, "pending" | "in_progress" | "completed") {
            return Err(format!(
                "items[{index}].status must be one of pending, in_progress, completed."
            ));
        }
        if status == "in_progress" {
            in_progress += 1;
        }
        if status != "completed" {
            remaining += 1;
        }
        steps.push(json!({"content": content, "status": status}));
    }
    let mut notes: Vec<String> = Vec::new();
    if in_progress > 1 {
        notes.push("Keep only one step in_progress.".into());
    }
    if in_progress == 0 && remaining > 0 {
        notes.push("No step is in_progress; mark the one you are on.".into());
    }
    Ok(json!({
        "acknowledged": true,
        "steps": steps,
        "notes": notes,
    }))
}

fn add_mcp_connector(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let has_command = args.get("command").and_then(Value::as_str).is_some();
    let has_url = args.get("url").and_then(Value::as_str).is_some();
    if !has_command && !has_url {
        return Err("Pass either a stdio command or an HTTP url.".into());
    }
    let mut input = json!({});
    for key in ["name", "url", "command", "args", "env", "cwd"] {
        if let Some(v) = args.get(key) {
            input.as_object_mut().unwrap().insert(key.into(), v.clone());
        }
    }
    crate::mcp_connectors::propose_add(&input, caller_thread)
        .map_err(|e| e.0)
        .map(|mut v| {
            v.as_object_mut().map(|o| {
                o.insert(
                    "approvalRequired".into(),
                    json!("The connector activates only after the user accepts the proposal in Settings."),
                )
            });
            v
        })
}

fn remove_mcp_connector(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let connector_id = arg_str(args, "connectorId").ok_or("Missing required argument: connectorId")?;
    crate::mcp_connectors::propose_remove(&connector_id, caller_thread).map_err(|e| e.0)
}

fn scan_mcp_config_file(path: &std::path::Path, source: &str, out: &mut Vec<(Value, String)>) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    let Ok(doc) = serde_json::from_str::<Value>(&text) else { return };
    let Some(servers) = doc.get("mcpServers").and_then(Value::as_object) else { return };
    for (name, server) in servers {
        let Some(command) = server.get("command").and_then(Value::as_str) else { continue };
        let mut input = json!({
            "name": name,
            "command": command,
            "args": server.get("args").cloned().unwrap_or(json!([])),
        });
        if let Some(env) = server.get("env").and_then(Value::as_object) {
            // Never carry secret values into the proposal: keys only.
            let keys: serde_json::Map<String, Value> = env
                .keys()
                .map(|k| (k.clone(), json!("")))
                .collect();
            input.as_object_mut().unwrap().insert("env".into(), Value::Object(keys));
        }
        out.push((input, source.to_string()));
    }
}

fn import_mcp_servers(caller_thread: Option<&str>) -> Result<Value, String> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    let mut candidates: Vec<(Value, String)> = Vec::new();
    scan_mcp_config_file(
        std::path::Path::new(&home).join(".claude.json").as_path(),
        "claude-code-user",
        &mut candidates,
    );
    scan_mcp_config_file(
        std::path::Path::new(&home)
            .join("Library/Application Support/Claude/claude_desktop_config.json")
            .as_path(),
        "claude-desktop",
        &mut candidates,
    );
    // Workspace .mcp.json relative to the caller thread's working directory.
    if let Some(thread) = caller_thread {
        if let Ok(conn) = state_conn() {
            let cwd: Option<String> = conn
                .query_row(
                    "SELECT COALESCE(t.worktree_path, t.working_directory, \
                     (SELECT p.workspace_root FROM projection_projects p WHERE p.project_id = t.project_id)) \
                     FROM projection_threads t WHERE t.thread_id = ?1",
                    [thread],
                    |r| r.get(0),
                )
                .ok();
            if let Some(cwd) = cwd {
                scan_mcp_config_file(
                    std::path::Path::new(&cwd).join(".mcp.json").as_path(),
                    "workspace",
                    &mut candidates,
                );
            }
        }
    }
    let mut proposals = Vec::new();
    let mut errors = Vec::new();
    for (input, source) in &candidates {
        match crate::mcp_connectors::propose_add(input, caller_thread) {
            Ok(mut proposal) => {
                proposal
                    .as_object_mut()
                    .map(|o| o.insert("source".into(), json!(source)));
                proposals.push(proposal);
            }
            Err(e) => errors.push(format!("{}: {}", source, e.0)),
        }
    }
    Ok(json!({
        "candidatesFound": candidates.len(),
        "proposalsCreated": proposals.len(),
        "proposals": proposals,
        "errors": errors,
    }))
}

const E2E_REVIEW_GUIDANCE: &str = r#"When the user explicitly requests an E2E/end-to-end test or review, follow this workflow. A discussion about E2E testing is not a request to run it.
1. Read repository instructions; identify the exact feature, running target, test fixtures and allowed side effects. Build a short journey matrix with passed, failed, blocked or not-applicable outcomes. Do not seed/reset production or shared data.
2. Use the provider-native subagent/Task capability to delegate a bounded E2E verification lane. Give it exact URLs, fixtures, roles, steps, expected outcomes, prohibited side effects and required screenshot evidence. Use a subagent, not a new Synara task. If native delegation is unavailable, say so and test directly; never pretend a subagent ran.
3. One agent owns the shared embedded browser at a time. While the child tests, the parent can inspect code or run local tests, but must not drive that browser concurrently. All tabs share the user's login session; test different identities sequentially with test accounts, never assume tab isolation.
4. Use browser_* for real visible interactions, one focused action per call. Verify state after each important action, persistence after reload, negative/permission cases, loading/error recovery and desktop/mobile layout where relevant. Use the project's test runner for durable E2E tests and viewport matrices, not shell-launched replacements for the embedded browser. Respect repository restrictions on commands. A screenshot or passing unit suite alone is not an E2E pass.
5. Inspect final-state screenshots; use browser_screenshot kind:proof for important completed flows. Do not capture exposed secrets. Child returns scope, exact commands and counts, findings, proof paths and untested areas. Wait for the child, verify its evidence, fix authorized failures and rerun affected paths.
iOS apps (device_* tools; the same delegation, proof and reporting rules apply):
- Build the exact scheme with the project's own tooling, then device_install and device_launch with restart: true, so the run starts from a known process. Pass test flags through launch environment.
- Remove nondeterminism first: device_configure for appearance, Dynamic Type, location and privacy grants, so no permission prompt interrupts a journey. Answer prompts it cannot pre-grant (notifications, camera) with device_tap.
- One owner drives the simulator at a time, like the browser. Prefer identifier taps over labels, which change with locale and copy, and coordinates last.
- Every input returns the settled screen: read the asserted text, values and toggle states from it. screenChanged: false means the step did nothing; settled: false means it is still loading, so device_wait for the expected element rather than repeating the input.
- Check persistence with device_launch restart: true, and failures with device_logs (unified log, print output, crash reports) before concluding.
- Capture device_screenshot proof: true for completed journeys and embed its artifactPath.
6. Finish with a completion report in this chat: outcome, journeys tested, commands/results, bugs fixed or remaining, and explicit limitations. Embed inspected proof images using ![Result description](/absolute/artifactPath.png). Report failures and blocked steps honestly; never present simulated or untested flows as verified."#;

fn synara_context_tool(caller_thread: Option<&str>) -> Result<Value, String> {
    let (thread, turn_id) = match caller_thread {
        Some(t) => {
            let conn = state_conn()?;
            conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
            let title: Option<String> = conn
                .query_row(
                    "SELECT title FROM projection_threads WHERE thread_id = ?1",
                    [t],
                    |r| r.get(0),
                )
                .ok();
            let running_turn: Option<String> = conn
                .query_row(
                    "SELECT turn_id FROM projection_turns WHERE thread_id = ?1 AND state IN ('running','starting') \
                     ORDER BY COALESCE(started_at, requested_at) DESC LIMIT 1",
                    [t],
                    |r| r.get(0),
                )
                .ok();
            (
                json!({"threadId": t, "title": title}),
                running_turn,
            )
        }
        None => (Value::Null, None),
    };
    Ok(json!({
        "harness": "synara",
        "runtime": "rust",
        "callerThread": thread,
        "callerTurnId": turn_id,
        "capabilities": [
            "thread:read", "thread:write", "kanban", "automation",
            "project-memory", "hub-work", "mcp-connectors",
        ],
    }))
}

fn capabilities() -> Result<Value, String> {
    let settings = crate::server_settings::get_settings(&Value::Null).map_err(|e| e.0)?;
    let instances = crate::provider_instances::derive_instances(&settings);
    let mut providers: serde_json::Map<String, Value> = serde_json::Map::new();
    for inst in &instances {
        let entry = providers
            .entry(inst.driver.clone())
            .or_insert_with(|| json!({"provider": inst.driver, "instances": []}));
        if let Some(arr) = entry.get_mut("instances").and_then(Value::as_array_mut) {
            arr.push(json!({
                "instanceId": inst.instance_id,
                "driver": inst.driver,
                "enabled": inst.enabled,
                "isDefault": inst.is_default,
            }));
        }
    }
    // The model catalog is cached per provider by provider.listModels.
    for (driver, entry) in providers.iter_mut() {
        let catalog = crate::provider_cache::handle(
            "provider.listModels",
            &json!({"provider": driver}),
        );
        if let Ok(cat) = catalog {
            if let Some(models) = cat.get("models") {
                entry.as_object_mut().map(|m| m.insert("models".into(), models.clone()));
            }
        }
    }
    let mut list: Vec<Value> = providers.into_values().collect();
    list.sort_by_key(|p| p.get("provider").and_then(Value::as_str).unwrap_or("").to_string());
    Ok(json!({"providers": list}))
}

fn project_list_tasks(args: &Value) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let include_archived = args.get("includeArchived").and_then(Value::as_bool).unwrap_or(false);
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let sql = format!(
        "SELECT task_id, goal_id, title, status, assigned_thread_id, revision \
         FROM project_agent_tasks WHERE project_id = ?1 {} ORDER BY task_id LIMIT 200",
        if include_archived { "" } else { "AND archived_at IS NULL" }
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows: Vec<Value> = stmt
        .query_map(rusqlite::params![project_id], |r| {
            Ok(json!({
                "taskId": r.get::<_, String>(0)?,
                "goalId": r.get::<_, Option<String>>(1)?,
                "title": r.get::<_, Option<String>>(2)?,
                "status": r.get::<_, Option<String>>(3)?,
                "assignedThreadId": r.get::<_, Option<String>>(4)?,
                "revision": r.get::<_, i64>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(json!({"tasks": rows}))
}

fn project_list_threads(args: &Value) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(
            "SELECT t.thread_id, t.title, t.branch, t.updated_at, t.archived_at, t.pull_requests_json, \
             (SELECT status FROM projection_thread_sessions s WHERE s.thread_id = t.thread_id), \
             (SELECT state FROM projection_turns tu WHERE tu.thread_id = t.thread_id \
               ORDER BY COALESCE(tu.started_at, tu.requested_at) DESC LIMIT 1) \
             FROM projection_threads t \
             WHERE t.project_id = ?1 AND t.deleted_at IS NULL \
             ORDER BY t.updated_at DESC LIMIT 200",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<Value> = stmt
        .query_map(rusqlite::params![project_id], |r| {
            let prs_raw: Option<String> = r.get(5)?;
            let pr_url = prs_raw
                .and_then(|p| serde_json::from_str::<Value>(&p).ok())
                .and_then(|v| v.as_array().cloned())
                .and_then(|arr| {
                    arr.iter()
                        .find_map(|l| l.pointer("/pullRequest/url").and_then(Value::as_str).map(str::to_string))
                });
            Ok(json!({
                "threadId": r.get::<_, String>(0)?,
                "title": r.get::<_, Option<String>>(1)?,
                "branch": r.get::<_, Option<String>>(2)?,
                "updatedAt": r.get::<_, Option<String>>(3)?,
                "archived": r.get::<_, Option<String>>(4)?.is_some(),
                "pullRequestUrl": pr_url,
                "sessionStatus": r.get::<_, Option<String>>(6)?,
                "latestTurnState": r.get::<_, Option<String>>(7)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    Ok(json!({"threads": rows}))
}

fn project_read_document(args: &Value) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let logical_path = arg_str(args, "logicalPath").ok_or("Missing required argument: logicalPath")?;
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT content, revision, author_kind, created_at FROM project_agent_documents \
         WHERE project_id = ?1 AND logical_path = ?2 ORDER BY revision DESC LIMIT 1",
        rusqlite::params![project_id, logical_path],
        |r| {
            Ok(json!({
                "logicalPath": logical_path,
                "content": r.get::<_, String>(0)?,
                "revision": r.get::<_, i64>(1)?,
                "authorKind": r.get::<_, Option<String>>(2)?,
                "createdAt": r.get::<_, Option<String>>(3)?,
            }))
        },
    )
    .map(|doc| json!({"document": doc}))
    .map_err(|_| format!("Document \"{logical_path}\" was not found in this project."))
}

fn project_write_document(args: &Value, caller_thread: Option<&str>) -> Result<Value, String> {
    let project_id = arg_str(args, "projectId").ok_or("Missing required argument: projectId")?;
    let logical_path = arg_str(args, "logicalPath").ok_or("Missing required argument: logicalPath")?;
    let content = arg_str(args, "content").ok_or("Missing required argument: content")?;
    if logical_path.contains("..") || logical_path.starts_with('/') {
        return Err("logicalPath must be a project-relative path without traversal.".into());
    }
    let conn = state_conn()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;").map_err(|e| e.to_string())?;
    let next_revision: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(revision), 0) + 1 FROM project_agent_documents \
             WHERE project_id = ?1 AND logical_path = ?2",
            rusqlite::params![project_id, logical_path],
            |r| r.get(0),
        )
        .unwrap_or(1);
    let revision_id = crate::orchestration_engine::uuid_v4();
    let hash = {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        content.hash(&mut h);
        format!("{:x}", h.finish())
    };
    conn.execute(
        "INSERT INTO project_agent_documents \
         (revision_id, project_id, logical_path, revision, content, content_hash, \
          author_kind, author_thread_id, sources_json, created_at) \
         VALUES (?1,?2,?3,?4,?5,?6,'thread',?7,'[]',?8)",
        rusqlite::params![
            revision_id,
            project_id,
            logical_path,
            next_revision,
            content,
            hash,
            caller_thread,
            crate::todo::iso_now_ms(),
        ],
    )
    .map_err(|e| e.to_string())?;
    mirror_document(&project_id, &logical_path, &content);
    Ok(json!({
        "logicalPath": logical_path,
        "revision": next_revision,
        "written": true,
    }))
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
        "synara_capabilities" => capabilities(),
        "synara_project_list_tasks" => project_list_tasks(args),
        "synara_project_list_threads" => project_list_threads(args),
        "synara_project_read_document" => project_read_document(args),
        "synara_project_write_document" => project_write_document(args, caller_thread),
        "synara_project_remember" => project_remember(args, caller_thread),
        "synara_project_forget" => project_forget(args),
        "synara_hub_list_work" => hub_list_work(caller_thread),
        "synara_hub_update_progress" => hub_update_progress(args, caller_thread),
        "synara_hub_cancel_work" => hub_cancel_work(args),
        "synara_create_thread" => create_thread_tool(args, caller_thread),
        "synara_send_message" => send_message_tool(args),
        "synara_interrupt_thread" => interrupt_thread_tool(args),
        "synara_set_thread_title" => set_thread_title_tool(args),
        "synara_set_thread_archived" => set_thread_archived_tool(args),
        "synara_read_thread_runtime_events" => read_thread_runtime_events(args),
        "synara_wait_for_threads" => wait_for_threads(args),
        "synara_list_connections" => list_connections(),
        "synara_project_library_list" => library_list(args),
        "synara_project_library_add" => library_add(args, caller_thread),
        "synara_project_get_overview" => project_get_overview(args),
        "synara_project_context" => project_context(args),
        "synara_project_report_result" => project_report_result(args, caller_thread),
        "synara_update_automation_memory" => update_automation_memory(args, caller_thread),
        "synara_report_automation_result" => report_automation_result(args, caller_thread),
        "synara_set_thread_pull_request" => set_thread_pull_request(args, caller_thread),
        "synara_unlink_thread_pull_request" => unlink_thread_pull_request(args, caller_thread),
        "synara_set_thread_goal" | "synara_set_kanban_goal" => set_thread_goal_impl(args),
        "synara_project_link_repository" => link_repository(args),
        "synara_diagnose_thread" => diagnose_thread(args),
        "synara_list_mcp_connectors" => list_mcp_connectors_tool(),
        "synara_create_threads" => create_threads_batch(args, caller_thread),
        "synara_update_plan" => update_plan(args),
        "synara_add_mcp_connector" => add_mcp_connector(args, caller_thread),
        "synara_remove_mcp_connector" => remove_mcp_connector(args, caller_thread),
        "synara_import_mcp_servers" => import_mcp_servers(caller_thread),
        "synara_context" => synara_context_tool(caller_thread),
        "synara_e2e_review" => Ok(json!({"guidance": E2E_REVIEW_GUIDANCE})),
        "synara_view_automation" => view_automation(args),
        "synara_create_automation" => create_automation(args, caller_thread),
        "synara_update_automation" => update_automation(args),
        "synara_cancel_automation" => cancel_automation(args),
        "synara_create_kanban_task" => create_kanban_card(args, true),
        "synara_create_kanban_draft" => create_kanban_card(args, false),
        "synara_read_kanban_card" => read_kanban_card(args),
        "synara_move_kanban_card" => move_kanban_card(args),
        "synara_update_kanban_card" => update_kanban_card(args),
        "synara_delete_kanban_card" => delete_kanban_card(args),
        "synara_list_projects" => list_projects(args),
        "synara_list_automations" => list_automations(),
        _ => Err(format!("Unknown tool: {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_tree_skips_symlinks_and_git() {
        let base = std::env::temp_dir().join(format!("synara-libtest-{}", std::process::id()));
        let source = base.join("src");
        let target = base.join("dst");
        std::fs::create_dir_all(source.join(".git")).unwrap();
        std::fs::create_dir_all(source.join("node_modules")).unwrap();
        std::fs::write(source.join("keep.txt"), "kept").unwrap();
        std::fs::write(source.join(".git/config"), "gitdir").unwrap();
        std::fs::write(source.join("node_modules/pkg.js"), "module").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(source.join("keep.txt"), source.join("link.txt")).unwrap();
        let mut files = 0usize;
        let mut bytes = 0u64;
        copy_tree_guarded(&source, &target, false, &mut files, &mut bytes).unwrap();
        assert!(target.join("keep.txt").exists());
        assert!(!target.join(".git/config").exists());
        assert!(!target.join("node_modules/pkg.js").exists());
        assert!(!target.join("link.txt").exists());
        assert_eq!(files, 1);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn safe_relative_rejects_traversal() {
        assert!(safe_relative("../etc").is_err());
        assert!(safe_relative("/abs/path").is_err());
        assert!(safe_relative("a/b/c.txt").is_ok());
    }

    #[test]
    fn memory_note_helpers_match_upstream() {
        assert_eq!(normalize_memory_note("Hello, World!"), "hello world");
        assert_eq!(memory_note_body("# Title\n\nSome note here"), "some note here");
        assert_eq!(slugify_memory_title("Release Schedule!"), "release-schedule");
        assert_eq!(sanitize_memory_title("a\n\tb"), "a b");
    }
}
