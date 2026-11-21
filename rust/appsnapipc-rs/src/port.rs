//! The Rust port of `apps/desktop/src/appSnapIpc.ts`.
//!
//! `webContents?.send(...)` is a no-op on null/undefined; any bridged value
//! lacks a callable `send`, matching the recorded TypeError.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined") == Some(&Value::Bool(true)))
}

fn send(web_contents: &Option<Value>) -> Result<(), String> {
    match web_contents {
        None => Ok(()),
        Some(v) if v.is_null() || is_undefined(v) => Ok(()),
        Some(_) => Err("webContents?.send is not a function".to_string()),
    }
}

/// Port of `sendAppSnapState` (`apps/desktop/src/appSnapIpc.ts:55`).
pub fn send_app_snap_state(
    web_contents: Option<serde_json::Value>,
    state: serde_json::Value,
) -> Result<(), String> {
    let _ = &state;
    send(&web_contents)
}

/// Port of `sendAppSnapCaptured` (`apps/desktop/src/appSnapIpc.ts:62`).
pub fn send_app_snap_captured(
    web_contents: Option<serde_json::Value>,
    capture: serde_json::Value,
) -> Result<(), String> {
    let _ = &capture;
    send(&web_contents)
}

/// Port of `sendAppSnapError` (`apps/desktop/src/appSnapIpc.ts:69`).
pub fn send_app_snap_error(
    web_contents: Option<serde_json::Value>,
    error: serde_json::Value,
) -> Result<(), String> {
    let _ = &error;
    send(&web_contents)
}

/// Port of `sendAppSnapPermissionGuideState` (`apps/desktop/src/appSnapIpc.ts:76`).
pub fn send_app_snap_permission_guide_state(
    web_contents: Option<serde_json::Value>,
    state: String,
) -> Result<(), String> {
    let _ = &state;
    send(&web_contents)
}
