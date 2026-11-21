//! The Rust port of `apps/web/src/terminalActivity.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if js::is_undefined(v) {
        return None;
    }
    v.as_object().and_then(|o| o.get(key))
}

/// Port of `terminalActivityFromEvent` (`apps/web/src/terminalActivity.ts:9`).
pub fn terminal_activity_from_event(event: Value) -> Result<Value, String> {
    if event.is_null() {
        return Err("Cannot read properties of null (reading 'type')".to_string());
    }
    if js::is_undefined(&event) {
        return Err("Cannot read properties of undefined (reading 'type')".to_string());
    }
    match prop(&event, "type").and_then(Value::as_str) {
        Some("activity") => Ok(json!({
            "hasRunningSubprocess": prop(&event, "hasRunningSubprocess")
                .cloned()
                .unwrap_or_else(js::undefined),
            "agentState": prop(&event, "agentState")
                .cloned()
                .unwrap_or_else(js::undefined),
        })),
        Some("started") | Some("restarted") | Some("exited") => Ok(json!({
            "hasRunningSubprocess": false,
            "agentState": Value::Null,
        })),
        _ => Ok(Value::Null),
    }
}
