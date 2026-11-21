//! The Rust port of `apps/desktop/src/backendStartupReadiness.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined") == Some(&Value::Bool(true)))
}

/// `!response.ok` leads the function: member reads on primitives yield undefined
/// (falsy → early false); a truthy `ok` then calls `response.json()`, which a
/// bridged value can never provide as a function — `TypeError`, caught → false.
pub async fn is_backend_startup_ready_response(
    response: serde_json::Value,
) -> Result<bool, String> {
    if response.is_null() {
        return Err("Cannot read properties of null (reading 'ok')".to_string());
    }
    if is_undefined(&response) {
        return Err("Cannot read properties of undefined (reading 'ok')".to_string());
    }
    Ok(false)
}
