//! The Rust port of `packages/shared/src/automationMode.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined_value(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

fn undefined_marker() -> Value {
    serde_json::json!({ "$undefined": true })
}

fn mode_str(mode: &Value) -> Option<&str> {
    mode.as_str()
}

fn continues(mode: &Value) -> bool {
    matches!(mode_str(mode), Some("heartbeat") | Some("dedicated"))
}

/// Port of `automationContinuesThread` (`packages/shared/src/automationMode.ts:19`).
pub fn automation_continues_thread(mode: serde_json::Value) -> Result<bool, String> {
    Ok(continues(&mode))
}

/// Port of `automationOwnsItsThread` (`packages/shared/src/automationMode.ts:24`).
pub fn automation_owns_its_thread(mode: serde_json::Value) -> Result<bool, String> {
    Ok(mode_str(&mode) == Some("dedicated"))
}

/// Port of `automationRequiresTargetThread` (`packages/shared/src/automationMode.ts:29`).
pub fn automation_requires_target_thread(mode: serde_json::Value) -> Result<bool, String> {
    Ok(mode_str(&mode) == Some("heartbeat"))
}

/// Port of `automationContinuationThreadId` (`packages/shared/src/automationMode.ts:38`).
pub fn automation_continuation_thread_id(
    definition: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if definition.is_null() {
        return Err("Cannot read properties of null (reading 'mode')".to_string());
    }
    if is_undefined_value(&definition) {
        return Err("Cannot read properties of undefined (reading 'mode')".to_string());
    }
    if !continues(definition.get("mode").unwrap_or(&Value::Null)) {
        return Ok(Some(Value::Null));
    }
    Ok(match definition.get("targetThreadId") {
        Some(v) => Some(v.clone()),
        None => Some(undefined_marker()),
    })
}

/// Port of `isAutomationRunThread` (`packages/shared/src/automationMode.ts:50`).
pub fn is_automation_run_thread(thread: serde_json::Value) -> Result<bool, String> {
    if thread.is_null() {
        return Err("Cannot read properties of null (reading 'creationSource')".to_string());
    }
    if is_undefined_value(&thread) {
        return Err("Cannot read properties of undefined (reading 'creationSource')".to_string());
    }
    Ok(thread.get("creationSource") == Some(&Value::String("automation_run".to_string())))
}
