//! The Rust port of `packages/shared/src/turnResume.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn err(name: &str, prop: &str) -> String {
    format!("Cannot read properties of {name} (reading '{prop}')")
}

/// Port of `resolveTurnResumeTarget` (`packages/shared/src/turnResume.ts:35`).
pub fn resolve_turn_resume_target(thread: Value) -> Result<Option<Value>, String> {
    if thread.is_null() {
        return Err(err("null", "latestTurn"));
    }
    if crate::js::is_undefined(&thread) {
        return Err(err("undefined", "latestTurn"));
    }
    let latest_turn = thread.get("latestTurn");
    let messages = match thread.get("messages") {
        None => return Err(err("undefined", "length")),
        Some(Value::Null) => return Err(err("null", "length")),
        Some(m) => m,
    };
    let last = match messages {
        Value::Array(a) => a.last().cloned().unwrap_or_else(crate::js::undefined),
        Value::String(s) => s
            .chars()
            .last()
            .map(|c| Value::String(c.to_string()))
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    };
    let state = latest_turn.and_then(|t| t.get("state"));
    let running = state.is_some_and(|s| s.as_str() == Some("running"));
    if let Value::Object(m) = &last {
        let unsent_user = m.get("role").and_then(Value::as_str) == Some("user")
            && match m.get("turnId") {
                None => true,
                Some(v) => v.is_null() || crate::js::is_undefined(v),
            }
            && m.get("startsNewTurn") != Some(&Value::Bool(false));
        if unsent_user && !running {
            let message_id = m.get("id").cloned().unwrap_or_else(crate::js::undefined);
            return Ok(Some(json!({ "kind": "resend", "messageId": message_id })));
        }
    }
    if state.is_some_and(|s| s.as_str() == Some("interrupted") || s.as_str() == Some("error")) {
        let lt = latest_turn.unwrap_or(&Value::Null);
        let turn_id = lt
            .get("turnId")
            .cloned()
            .unwrap_or_else(crate::js::undefined);
        return Ok(Some(
            json!({ "kind": "continue", "turnId": turn_id, "state": lt.get("state").cloned().unwrap_or(Value::Null) }),
        ));
    }
    Ok(None)
}
