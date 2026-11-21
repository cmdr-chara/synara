//! The Rust port of `apps/web/src/chatRouteRestore.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js;

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => js::is_undefined(v),
        _ => js::as_number(v).is_some_and(|n| n == 0.0 || n.is_nan()),
    }
}

fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Null => Value::Null,
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

fn same_value_zero(a: &Value, b: &Value) -> bool {
    match (js::as_number(a), js::as_number(b)) {
        (Some(x), Some(y)) => x == y || (x.is_nan() && y.is_nan()),
        _ => a == b,
    }
}

/// `set.has(v)`: a JS Set arrives as `{"$set": [...]}`.
fn set_has(set: &Value, v: &Value, name: &str) -> Result<bool, String> {
    if set.is_null() {
        return Err("Cannot read properties of null (reading 'has')".to_string());
    }
    if js::is_undefined(set) {
        return Err("Cannot read properties of undefined (reading 'has')".to_string());
    }
    match set.get("$set").and_then(|x| x.as_array()) {
        Some(items) => Ok(items.iter().any(|x| same_value_zero(x, v))),
        None => Err(format!("{name}.has is not a function")),
    }
}

/// Port of `resolveRestorableThreadRoute` (`apps/web/src/chatRouteRestore.ts:16`).
pub fn resolve_restorable_thread_route(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if input.is_null() {
        return Err(
            "Cannot destructure property 'lastThreadRoute' of 'input' as it is null.".to_string(),
        );
    }
    if js::is_undefined(&input) {
        return Err(
            "Cannot destructure property 'lastThreadRoute' of 'input' as it is undefined."
                .to_string(),
        );
    }
    let last = member(&input, "lastThreadRoute");
    if falsy(&last) {
        return Ok(None);
    }
    let ids = member(&input, "availableThreadIds");
    if !set_has(&ids, &member(&last, "threadId"), "availableThreadIds")? {
        return Ok(None);
    }
    let split_view_id = member(&last, "splitViewId");
    let split_ids = member(&input, "availableSplitViewIds");
    if !falsy(&split_view_id)
        && !falsy(&split_ids)
        && !set_has(&split_ids, &split_view_id, "availableSplitViewIds")?
    {
        return Ok(Some(json!({ "threadId": member(&last, "threadId") })));
    }
    Ok(Some(last))
}

fn should_remembered(input: &Value, done_ok: bool) -> Result<bool, String> {
    if input.is_null() || js::is_undefined(input) {
        let kind = if input.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "Cannot read properties of {kind} (reading 'lastThreadRoute')"
        ));
    }
    let state = member(input, "recoveryState");
    let state_ok = if done_ok {
        !(state.as_str() == Some("done"))
    } else {
        state.as_str() == Some("idle")
    };
    Ok(!falsy(&member(input, "lastThreadRoute"))
        && js::as_number(&member(input, "availableThreadCount")) == Some(0.0)
        && state_ok)
}

/// Port of `shouldStartRememberedRouteRecovery` (`apps/web/src/chatRouteRestore.ts:41`).
pub fn should_start_remembered_route_recovery(input: serde_json::Value) -> Result<bool, String> {
    should_remembered(&input, false)
}

/// Port of `shouldHoldRememberedRouteFallback` (`apps/web/src/chatRouteRestore.ts:50`).
pub fn should_hold_remembered_route_fallback(input: serde_json::Value) -> Result<bool, String> {
    should_remembered(&input, true)
}

fn should_missing(input: &Value, done_ok: bool) -> Result<bool, String> {
    if input.is_null() || js::is_undefined(input) {
        let kind = if input.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "Cannot read properties of {kind} (reading 'routeThreadExists')"
        ));
    }
    let state = member(input, "recoveryState");
    let state_ok = if done_ok {
        !(state.as_str() == Some("done"))
    } else {
        state.as_str() == Some("idle")
    };
    Ok(!truthy(&member(input, "routeThreadExists"))
        && !truthy(&member(input, "hasKnownServerThreads"))
        && state_ok)
}

fn truthy(v: &Value) -> bool {
    !falsy(v)
}

/// Port of `shouldStartMissingThreadRouteRecovery` (`apps/web/src/chatRouteRestore.ts:59`).
pub fn should_start_missing_thread_route_recovery(
    input: serde_json::Value,
) -> Result<bool, String> {
    should_missing(&input, false)
}

/// Port of `shouldHoldMissingThreadRouteFallback` (`apps/web/src/chatRouteRestore.ts:67`).
pub fn should_hold_missing_thread_route_fallback(input: serde_json::Value) -> Result<bool, String> {
    should_missing(&input, true)
}
