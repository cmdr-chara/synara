//! The Rust port of `apps/web/src/workflowRunUiStore.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::collections::HashSet;

const MAX_ENTRIES_PER_LIST: usize = 50;

/// Port of `createDefaultWorkflowRunUiThreadState` (`apps/web/src/workflowRunUiStore.ts:37`).
pub fn create_default_workflow_run_ui_thread_state() -> Result<Value, String> {
    Ok(json!({ "pausedByUser": [], "dismissed": [] }))
}

fn unsafe_key(key: &str) -> bool {
    matches!(key, "__proto__" | "constructor" | "prototype")
}

fn sanitize_id_list(value: &Value) -> Vec<Value> {
    let Some(entries) = value.as_array() else {
        return Vec::new();
    };
    let mut seen: HashSet<&str> = HashSet::new();
    let mut ids: Vec<Value> = Vec::new();
    for entry in entries {
        let Some(s) = entry.as_str() else {
            continue;
        };
        if s.is_empty() || seen.contains(s) {
            continue;
        }
        seen.insert(s);
        ids.push(Value::String(s.to_string()));
        if ids.len() >= MAX_ENTRIES_PER_LIST {
            break;
        }
    }
    ids
}

fn sanitize_thread_state(raw: &Value) -> Option<Value> {
    let obj = raw.as_object()?;
    let paused = sanitize_id_list(obj.get("pausedByUser").unwrap_or(&Value::Null));
    let dismissed = sanitize_id_list(obj.get("dismissed").unwrap_or(&Value::Null));
    if paused.is_empty() && dismissed.is_empty() {
        return None;
    }
    Some(json!({ "pausedByUser": paused, "dismissed": dismissed }))
}

/// Port of `sanitizeWorkflowRunUiStateByThreadId` (`apps/web/src/workflowRunUiStore.ts:95`).
pub fn sanitize_workflow_run_ui_state_by_thread_id(value: Value) -> Result<Value, String> {
    let Some(obj) = value.as_object() else {
        return Ok(json!({}));
    };
    let mut result = serde_json::Map::new();
    for (key, raw) in obj {
        if unsafe_key(key) {
            continue;
        }
        if let Some(state) = sanitize_thread_state(raw) {
            result.insert(key.clone(), state);
        }
    }
    Ok(Value::Object(result))
}

/// Port of `selectWorkflowRunUiThreadState` (`apps/web/src/workflowRunUiStore.ts:173`).
pub fn select_workflow_run_ui_thread_state(_thread_id: Option<Value>) -> Result<Value, String> {
    Ok(json!({ "$function": "" }))
}

/// Port of `useWorkflowRunUiThreadState` (`apps/web/src/workflowRunUiStore.ts:181`).
pub fn use_workflow_run_ui_thread_state(_thread_id: Option<Value>) -> Result<Value, String> {
    Err("Cannot read properties of null (reading 'useCallback')".to_string())
}
