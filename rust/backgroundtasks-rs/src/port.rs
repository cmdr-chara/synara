//! The Rust port of `packages/shared/src/backgroundTasks.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.get("$undefined") == Some(&Value::Bool(true)))
}

fn member_error(subject: &str, member: &str) -> String {
    format!("Cannot read properties of {subject} (reading '{member}')")
}

const TERMINAL_STATUSES: [&str; 4] = ["completed", "failed", "killed", "paused"];

/// Port of `isBackgroundTaskActivity` (`packages/shared/src/backgroundTasks.ts:26`).
pub fn is_background_task_activity(activity: serde_json::Value) -> Result<bool, String> {
    let kind = match &activity {
        Value::Null => return Err(member_error("null", "kind")),
        v if is_undefined(v) => return Err(member_error("undefined", "kind")),
        v => v.get("kind"),
    };
    Ok(matches!(kind, Some(Value::String(k)) if k == "task.updated" || k == "task.completed"))
}

/// Port of `foldOutstandingBackgroundTaskIds` (`packages/shared/src/backgroundTasks.ts:45`).
pub fn fold_outstanding_background_task_ids(
    activities: serde_json::Value,
) -> Result<Vec<String>, String> {
    let items: Vec<Value> = match &activities {
        Value::Array(a) => a.clone(),
        // A JS string iterates as characters; none is an activity, so the result is empty.
        Value::String(_) => Vec::new(),
        Value::Object(map) if map.get("$set").is_some() => match map.get("$set") {
            Some(Value::Array(a)) => a.clone(),
            _ => Vec::new(),
        },
        // {$utf16: [...]} decodes to a JS string: iterable as characters.
        Value::Object(map) if map.get("$utf16").is_some() => Vec::new(),
        // A JS Map iterates [key, value] pairs; none is an activity with a taskId.
        Value::Object(map) if map.get("$map").is_some() => Vec::new(),
        _ => return Err("activities is not iterable".into()),
    };
    let mut outstanding: Vec<String> = Vec::new();
    for activity in &items {
        let payload = match activity {
            Value::Null => return Err(member_error("null", "payload")),
            v if is_undefined(v) => return Err(member_error("undefined", "payload")),
            v => v.get("payload"),
        };
        let task_id = payload
            .and_then(Value::as_object)
            .and_then(|p| p.get("taskId"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty());
        let Some(task_id) = task_id.map(str::to_owned) else {
            continue;
        };
        let kind = activity.get("kind").and_then(Value::as_str);
        match kind {
            Some("task.updated") => {
                let p = payload.and_then(Value::as_object);
                let terminal = p
                    .and_then(|m| m.get("status"))
                    .and_then(Value::as_str)
                    .is_some_and(|s| TERMINAL_STATUSES.contains(&s));
                let is_backgrounded = p.and_then(|m| m.get("isBackgrounded"));
                // An explicit `isBackgrounded: false` (back to foreground) or a terminal
                // status clears the task; only `=== true` marks it outstanding.
                if terminal || is_backgrounded == Some(&Value::Bool(false)) {
                    outstanding.retain(|id| id != &task_id);
                } else if is_backgrounded == Some(&Value::Bool(true))
                    && !outstanding.iter().any(|id| id == &task_id)
                {
                    outstanding.push(task_id);
                }
            }
            Some("task.completed") => outstanding.retain(|id| id != &task_id),
            _ => {}
        }
    }
    Ok(outstanding)
}

/// Port of `canSessionFinishBackgroundTasks` (`packages/shared/src/backgroundTasks.ts:88`).
pub fn can_session_finish_background_tasks(status: serde_json::Value) -> Result<bool, String> {
    Ok(!status.is_null() && !is_undefined(&status))
}
