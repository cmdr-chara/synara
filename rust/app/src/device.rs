//! device.* — mirrors upstream when no device engine can run: every
//! request method fails with the unsupported message, getThreadState
//! answers a real snapshot naming the unsupported platform, and
//! subscribeEvents (handled in ws_relay) stays open and silent forever —
//! an empty stream completes instantly and the client treats that as a
//! zombie socket that forces reconnect loops.

use serde_json::{json, Value};

pub fn handles(tag: &str) -> bool {
    tag.starts_with("device.")
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, crate::todo::WsErr> {
    let unsupported = || {
        crate::todo::WsErr(
            "The device pane requires macOS with Xcode and an iOS simulator runtime.".to_string(),
            None,
        )
    };
    match tag {
        "device.getThreadState" => Ok(json!({
            "threadId": payload.get("threadId").cloned().unwrap_or(Value::Null),
            "version": 0,
            "attachedDeviceUdid": Value::Null,
            "devices": [],
            "agentActive": false,
            "availability": {"kind": "unsupported-platform", "platform": "darwin"},
            "lastError": Value::Null,
        })),
        _ => Err(unsupported()),
    }
}
