//! Claude task tracking, ported from upstream `claudeTaskTracker.ts` and
//! `runtimeTaskList.ts`. Claude streams TodoWrite inputs and the results of
//! its TaskCreate/TaskUpdate/TaskGet/TaskList tools; the tracker folds them
//! into a per-session task map and renders the shared `turn.tasks.updated`
//! payload ({tasks:[{task,status}]}, status pending/inProgress/completed).

use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Clone)]
pub struct TrackedTask {
    pub subject: String,
    pub description: Option<String>,
    pub active_form: Option<String>,
    pub status: String, // pending | in_progress | completed
    pub owner: Option<String>,
    pub blocked_by: Vec<String>,
}

fn read_task_string(input: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(v) = input.get(*key).and_then(Value::as_str) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn read_task_id(input: &Value) -> Option<String> {
    read_task_string(input, &["taskId", "id", "task_id"])
}

fn read_tracked_status(value: Option<&Value>) -> Option<String> {
    match value.and_then(Value::as_str) {
        Some("pending") => Some("pending".to_string()),
        Some("in_progress") => Some("in_progress".to_string()),
        Some("completed") => Some("completed".to_string()),
        Some("deleted") => Some("deleted".to_string()),
        _ => None,
    }
}

fn read_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|e| e.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Parse a tool-result `content` payload the way upstream does: a JSON string
/// is parsed, an array yields the first parseable entry, a `{type:"text"}`
/// block unwraps its text, other values pass through.
pub fn parse_tool_result_value(value: &Value) -> Option<Value> {
    if let Some(s) = value.as_str() {
        return serde_json::from_str(s).ok();
    }
    if let Some(arr) = value.as_array() {
        for entry in arr {
            if let Some(parsed) = parse_tool_result_value(entry) {
                return Some(parsed);
            }
        }
        return None;
    }
    if value.is_object() {
        if value.get("type").and_then(Value::as_str) == Some("text") {
            if let Some(text) = value.get("text").and_then(Value::as_str) {
                return serde_json::from_str(text).ok().or_else(|| Some(json!(text)));
            }
        }
        return Some(value.clone());
    }
    None
}

fn tracked_task_from_record(record: &Value, previous: Option<&TrackedTask>) -> Option<(String, TrackedTask)> {
    let id = read_task_id(record)?;
    let subject = read_task_string(record, &["subject"]).or_else(|| previous.map(|p| p.subject.clone()))?;
    let status = read_tracked_status(record.get("status"));
    if status.as_deref() == Some("deleted") {
        return None;
    }
    Some((
        id,
        TrackedTask {
            subject,
            description: read_task_string(record, &["description"]).or_else(|| previous.and_then(|p| p.description.clone())),
            active_form: read_task_string(record, &["activeForm", "active_form"]).or_else(|| previous.and_then(|p| p.active_form.clone())),
            status: status.or_else(|| previous.map(|p| p.status.clone())).unwrap_or_else(|| "pending".to_string()),
            owner: read_task_string(record, &["owner"]).or_else(|| previous.and_then(|p| p.owner.clone())),
            blocked_by: if record.get("blockedBy").is_some() || record.get("blocked_by").is_some() {
                read_string_array(record.get("blockedBy").or_else(|| record.get("blocked_by")))
            } else {
                previous.map(|p| p.blocked_by.clone()).unwrap_or_default()
            },
        },
    ))
}

fn merge_task_update(existing: &TrackedTask, input: &Value) -> TrackedTask {
    let status = read_tracked_status(input.get("status"));
    let added = read_string_array(input.get("addBlockedBy").or_else(|| input.get("add_blocked_by")));
    let mut blocked_by = existing.blocked_by.clone();
    for b in added {
        if !blocked_by.contains(&b) {
            blocked_by.push(b);
        }
    }
    TrackedTask {
        subject: read_task_string(input, &["subject"]).unwrap_or_else(|| existing.subject.clone()),
        description: read_task_string(input, &["description"]).or_else(|| existing.description.clone()),
        active_form: read_task_string(input, &["activeForm", "active_form"]).or_else(|| existing.active_form.clone()),
        status: status.filter(|s| s != "deleted").unwrap_or_else(|| existing.status.clone()),
        owner: read_task_string(input, &["owner"]).or_else(|| existing.owner.clone()),
        blocked_by,
    }
}

/// Apply one task-tool result to the tracked map. Returns true when the map
/// changed and the whole list should be re-emitted.
pub fn apply_tool_result(
    tasks: &mut HashMap<String, TrackedTask>,
    tool_name: &str,
    input: &Value,
    result_content: &Value,
    is_error: bool,
) -> bool {
    if is_error {
        return false;
    }
    let result = parse_tool_result_value(result_content);

    match tool_name {
        "TaskCreate" => {
            let result_task = result
                .as_ref()
                .and_then(|r| r.get("task"))
                .filter(|t| t.is_object());
            let Some(result_task) = result_task else { return false };
            let Some(id) = read_task_id(result_task) else { return false };
            let subject = read_task_string(result_task, &["subject"])
                .or_else(|| read_task_string(input, &["subject"]));
            let Some(subject) = subject else { return false };
            tasks.insert(
                id,
                TrackedTask {
                    subject,
                    description: read_task_string(input, &["description"]),
                    active_form: read_task_string(input, &["activeForm", "active_form"]),
                    status: "pending".to_string(),
                    owner: None,
                    blocked_by: vec![],
                },
            );
            true
        }
        "TaskUpdate" => {
            if result.as_ref().and_then(|r| r.get("success")).and_then(Value::as_bool) == Some(false) {
                return false;
            }
            let Some(task_id) = read_task_id(input) else { return false };
            let status = read_tracked_status(input.get("status"));
            if status.as_deref() == Some("deleted") {
                return tasks.remove(&task_id).is_some();
            }
            match tasks.get(&task_id) {
                Some(existing) => {
                    let merged = merge_task_update(existing, input);
                    tasks.insert(task_id, merged);
                    true
                }
                None => {
                    let Some(subject) = read_task_string(input, &["subject"]) else { return false };
                    tasks.insert(
                        task_id,
                        TrackedTask {
                            subject,
                            description: read_task_string(input, &["description"]),
                            active_form: read_task_string(input, &["activeForm", "active_form"]),
                            status: status.unwrap_or_else(|| "pending".to_string()),
                            owner: read_task_string(input, &["owner"]),
                            blocked_by: read_string_array(input.get("addBlockedBy").or_else(|| input.get("add_blocked_by"))),
                        },
                    );
                    true
                }
            }
        }
        "TaskGet" => {
            let Some(r) = result.as_ref() else { return false };
            if !r.get("task").map(|t| !t.is_null()).unwrap_or(false) {
                if r.get("task").is_some() {
                    // explicit null task: the task is gone
                    if let Some(id) = read_task_id(input) {
                        return tasks.remove(&id).is_some();
                    }
                }
                return false;
            }
            let record = r.get("task").unwrap();
            let prior = read_task_id(record).and_then(|id| tasks.get(&id).cloned());
            match tracked_task_from_record(record, prior.as_ref()) {
                Some((id, task)) => {
                    tasks.insert(id, task);
                    true
                }
                None => false,
            }
        }
        "TaskList" => {
            let list = result
                .as_ref()
                .and_then(|r| r.get("tasks"))
                .and_then(Value::as_array);
            let Some(list) = list else { return false };
            let mut snapshot: HashMap<String, TrackedTask> = HashMap::new();
            for entry in list {
                let prior = read_task_id(entry).and_then(|id| tasks.get(&id).cloned());
                if let Some((id, task)) = tracked_task_from_record(entry, prior.as_ref()) {
                    snapshot.insert(id, task);
                }
            }
            *tasks = snapshot;
            true
        }
        _ => false,
    }
}

fn normalize_status(value: &str) -> &'static str {
    match value {
        "completed" => "completed",
        "in_progress" | "inProgress" => "inProgress",
        _ => "pending",
    }
}

fn make_item(task: &str, status: &str) -> Option<Value> {
    let t = task.trim();
    if t.is_empty() {
        return None;
    }
    Some(json!({ "task": t, "status": normalize_status(status) }))
}

/// Whole-list payload for the tracked map, upstream `claudeTrackedTasksPayload`.
pub fn tracked_payload(tasks: &HashMap<String, TrackedTask>) -> Value {
    let items: Vec<Value> = tasks
        .values()
        .filter_map(|t| {
            let label = if t.status == "in_progress" {
                t.active_form.as_deref().unwrap_or(&t.subject)
            } else {
                &t.subject
            };
            make_item(label, &t.status)
        })
        .collect();
    json!({ "tasks": items })
}

/// TodoWrite whole-list payload, upstream `normalizeClaudeTodoTasks`.
pub fn todo_payload(input: &Value) -> Option<Value> {
    let todos = input.get("todos")?.as_array()?;
    let items: Vec<Value> = todos
        .iter()
        .filter_map(|todo| {
            let status = normalize_status(todo.get("status").and_then(Value::as_str).unwrap_or("pending"));
            let content = read_task_string(todo, &["content"]);
            let active_form = read_task_string(todo, &["activeForm"]);
            let task = if status == "inProgress" {
                active_form.or(content)
            } else {
                content.or(active_form)
            }?;
            make_item(&task, status)
        })
        .collect();
    if items.is_empty() {
        None
    } else {
        Some(json!({ "tasks": items }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_write_payload_maps_statuses() {
        let input = json!({ "todos": [
            { "content": "write tests", "status": "pending" },
            { "content": "port tracker", "activeForm": "porting tracker", "status": "in_progress" },
            { "content": "done item", "status": "completed" },
        ]});
        let payload = todo_payload(&input).unwrap();
        let tasks = payload.get("tasks").unwrap().as_array().unwrap();
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[1].get("task").unwrap(), "porting tracker");
        assert_eq!(tasks[1].get("status").unwrap(), "inProgress");
    }

    #[test]
    fn task_create_result_tracks_task() {
        let mut tasks = HashMap::new();
        let input = json!({ "subject": "implement parser", "description": "recursive descent" });
        let result = json!(r#"{"task":{"taskId":"1","subject":"implement parser"}}"#);
        assert!(apply_tool_result(&mut tasks, "TaskCreate", &input, &result, false));
        assert_eq!(tasks.get("1").unwrap().status, "pending");
    }

    #[test]
    fn task_update_delete_removes() {
        let mut tasks = HashMap::new();
        tasks.insert("1".to_string(), TrackedTask {
            subject: "x".into(), description: None, active_form: None,
            status: "pending".into(), owner: None, blocked_by: vec![],
        });
        let input = json!({ "taskId": "1", "status": "deleted" });
        let result = json!({"success": true});
        assert!(apply_tool_result(&mut tasks, "TaskUpdate", &input, &result, false));
        assert!(tasks.is_empty());
    }

    #[test]
    fn task_list_result_replaces_snapshot() {
        let mut tasks = HashMap::new();
        tasks.insert("stale".to_string(), TrackedTask {
            subject: "old".into(), description: None, active_form: None,
            status: "pending".into(), owner: None, blocked_by: vec![],
        });
        let result = json!({ "tasks": [{ "taskId": "2", "subject": "fresh", "status": "in_progress", "activeForm": "working" }] });
        assert!(apply_tool_result(&mut tasks, "TaskList", &json!({}), &result, false));
        assert!(!tasks.contains_key("stale"));
        assert_eq!(tasks.get("2").unwrap().status, "in_progress");
    }
}
