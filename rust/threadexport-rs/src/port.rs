//! The Rust port of `packages/shared/src/threadExport.ts`.
//!
//! Behavior inferred clean-room from recorded cases and black-box probes:
//! `latestTurn.state === "running"` blocks export with the turn reason; then
//! `messages.some(m => m.streaming)` blocks with the streaming reason — called
//! unconditionally, so a missing/null/non-array `messages` throws like JS.

#![forbid(unsafe_code)]

use serde_json::Value;

const RUNNING: &str =
    "Thread is still running. Wait for the current turn to finish before exporting.";
const STREAMING: &str =
    "Thread has a streaming message. Wait for the current response to finish before exporting.";

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !crate::js::is_undefined(v),
    }
}

/// Port of `threadExportBlockedReason` (`packages/shared/src/threadExport.ts:14`).
pub fn thread_export_blocked_reason(thread: Value) -> Result<Option<String>, String> {
    let latest_turn = match &thread {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'latestTurn')".to_string());
        }
        v if crate::js::is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'latestTurn')".to_string());
        }
        _ => thread.get("latestTurn"),
    };
    if latest_turn
        .and_then(|t| t.get("state"))
        .is_some_and(|s| s == "running")
    {
        return Ok(Some(RUNNING.to_string()));
    }
    match thread.get("messages") {
        Some(Value::Array(messages)) => {
            if messages
                .iter()
                .any(|m| m.get("streaming").is_some_and(truthy))
            {
                Ok(Some(STREAMING.to_string()))
            } else {
                Ok(None)
            }
        }
        None => Err("Cannot read properties of undefined (reading 'some')".to_string()),
        Some(Value::Null) => Err("Cannot read properties of null (reading 'some')".to_string()),
        Some(_) => Err("thread.messages.some is not a function".to_string()),
    }
}
