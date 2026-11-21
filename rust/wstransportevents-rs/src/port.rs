//! The Rust port of `apps/web/src/wsTransportEvents.ts` — renderer-local
//! transport state store. The browser `CustomEvent` fan-out is a no-op
//! off-DOM, matching the recorded behavior.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::sync::Mutex;

static LATEST_TRANSPORT_STATE: Mutex<Option<String>> = Mutex::new(None);
static LATEST_COMPATIBILITY_ISSUE: Mutex<Option<Value>> = Mutex::new(None);

/// Port of `readLatestWsTransportState` (`apps/web/src/wsTransportEvents.ts:16`).
pub fn read_latest_ws_transport_state() -> Result<Option<String>, String> {
    // INFALLIBLE: mutex is only poisoned by a panic while locked, which cannot
    // happen here since the guard is dropped before any user code runs.
    Ok(LATEST_TRANSPORT_STATE.lock().unwrap().clone())
}

/// Port of `getWsSettlingThreadIds` (`apps/web/src/wsTransportEvents.ts:24`).
/// `trackWsTurnSettlement` is not part of the bridged surface, so the set is
/// always empty from this side — encoded as the `$set` wire marker.
pub fn get_ws_settling_thread_ids() -> Result<serde_json::Value, String> {
    Ok(json!({"$set": []}))
}

/// Port of `emitWsTransportState` (`apps/web/src/wsTransportEvents.ts:64`).
/// Stores the latest state; `window.dispatchEvent` is absent under Node.
pub fn emit_ws_transport_state(state: String) -> Result<(), String> {
    // INFALLIBLE: see read_latest_ws_transport_state.
    *LATEST_TRANSPORT_STATE.lock().unwrap() = Some(state);
    Ok(())
}

/// Port of `readLatestWsCompatibilityIssue` (`apps/web/src/wsTransportEvents.ts:105`).
pub fn read_latest_ws_compatibility_issue() -> Result<Option<serde_json::Value>, String> {
    // INFALLIBLE: see read_latest_ws_transport_state.
    Ok(LATEST_COMPATIBILITY_ISSUE.lock().unwrap().clone())
}

/// Port of `emitWsCompatibilityIssue` (`apps/web/src/wsTransportEvents.ts:109`).
pub fn emit_ws_compatibility_issue(issue: Option<serde_json::Value>) -> Result<(), String> {
    // INFALLIBLE: see read_latest_ws_transport_state.
    *LATEST_COMPATIBILITY_ISSUE.lock().unwrap() = issue;
    Ok(())
}
