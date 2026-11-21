//! The Rust port of `packages/shared/src/turnStopActivity.ts`.
//!
//! Behavior inferred clean-room from recorded cases: `event.payload` is read
//! (member-read errors on null/undefined payload), then `requestedBy` — only
//! `"user"` produces an activity, everything else returns null. The activity's
//! `turnId` is `payload.turnId ?? activeTurnId` (nullish coalescing: `0`,
//! `false`, `{}` are kept); `id`/`sequence` stay `undefined` and
//! `sequenceSource` is `"orchestration"`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

/// Port of `deriveTurnStopActivity` (`packages/shared/src/turnStopActivity.ts`).
pub fn derive_turn_stop_activity(event: Value, active_turn_id: Value) -> Result<Value, String> {
    let payload = match &event {
        Value::Null => return Err("Cannot read properties of null (reading 'payload')".to_string()),
        v if crate::js::is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'payload')".to_string());
        }
        Value::Object(o) => o
            .get("payload")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    };
    let requested_by = match &payload {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'requestedBy')".to_string());
        }
        v if crate::js::is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'requestedBy')".to_string());
        }
        Value::Object(o) => o
            .get("requestedBy")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    };
    if requested_by != Value::String("user".to_string()) {
        return Ok(Value::Null);
    }
    let payload_turn_id = match &payload {
        Value::Object(o) => o
            .get("turnId")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    };
    let turn_id =
        if matches!(payload_turn_id, Value::Null) || crate::js::is_undefined(&payload_turn_id) {
            active_turn_id
        } else {
            payload_turn_id
        };
    if matches!(turn_id, Value::Null) {
        return Ok(Value::Null);
    }
    let created_at = match &payload {
        Value::Object(o) => o
            .get("createdAt")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    };
    Ok(json!({
        "id": crate::js::undefined(),
        "turnId": turn_id,
        "createdAt": created_at,
        "tone": "info",
        "kind": "turn.stop-requested",
        "summary": "Stop requested",
        "payload": { "requestedBy": "user" },
        "sequence": crate::js::undefined(),
        "sequenceSource": "orchestration",
    }))
}
