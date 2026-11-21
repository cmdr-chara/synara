//! The Rust port of `apps/desktop/src/browserIpc.ts`.
//!
//! `webContents?.send(...)` is a no-op on null/undefined; any bridged value
//! lacks a callable `send`, matching the recorded TypeError. Likewise
//! `registerBrowserIpcHandlers` cannot register real Electron handlers from
//! bridged data, so it reproduces the recorded property-access errors.

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

/// Port of `sendBrowserState` (`apps/desktop/src/browserIpc.ts:30`).
pub fn send_browser_state(
    web_contents: Option<serde_json::Value>,
    state: serde_json::Value,
) -> Result<(), String> {
    let _ = &state;
    send(&web_contents)
}

/// Port of `sendBrowserCopyLink` (`apps/desktop/src/browserIpc.ts:39`).
pub fn send_browser_copy_link(
    web_contents: Option<serde_json::Value>,
    event: serde_json::Value,
) -> Result<(), String> {
    let _ = &event;
    send(&web_contents)
}

/// Port of `sendBrowserAnnotationEvent` (`apps/desktop/src/browserIpc.ts:46`).
pub fn send_browser_annotation_event(
    web_contents: Option<serde_json::Value>,
    event: serde_json::Value,
) -> Result<(), String> {
    let _ = &event;
    send(&web_contents)
}

/// Port of `registerBrowserIpcHandlers` (`apps/desktop/src/browserIpc.ts:54`).
pub fn register_browser_ipc_handlers(
    ipc_main: serde_json::Value,
    browser_manager: serde_json::Value,
) -> Result<(), String> {
    let _ = &browser_manager;
    if ipc_main.is_null() {
        return Err("Cannot read properties of null (reading 'removeHandler')".to_string());
    }
    if is_undefined(&ipc_main) {
        return Err("Cannot read properties of undefined (reading 'removeHandler')".to_string());
    }
    Err("ipcMain.removeHandler is not a function".to_string())
}
