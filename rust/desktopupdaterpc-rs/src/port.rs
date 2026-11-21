//! Port of `apps/server/src/desktopUpdateRpc.ts` (exported predicates).

use serde_json::{Value, json};

fn is_undefined(v: &Value) -> bool {
    v.is_null()
        || (v.is_object()
            && v.get("$undefined")
                .map(|m| m.is_boolean() && m == true)
                .unwrap_or(false))
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) => o.get(key),
        _ => None,
    }
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Some(Value::String(s)) => !s.is_empty(),
        Some(o) => !is_undefined(o),
    }
}

/// `!thread.deletedAt && (...)` from `hasDesktopUpdateThreadWork`.
pub fn has_desktop_update_thread_work(thread: serde_json::Value) -> Result<bool, String> {
    if thread.is_null() {
        return Err("Cannot read properties of null (reading 'deletedAt')".to_string());
    }
    if truthy(prop(&thread, "deletedAt")) {
        return Ok(false);
    }
    let session = prop(&thread, "session").cloned().unwrap_or(Value::Null);
    let latest_turn = prop(&thread, "latestTurn").cloned().unwrap_or(Value::Null);
    if prop(&session, "status").and_then(|s| s.as_str()) == Some("starting") {
        return Ok(true);
    }
    if let Some(at) = prop(&session, "activeTurnId")
        && !at.is_null()
        && !is_undefined(at)
    {
        return Ok(true);
    }
    if prop(&latest_turn, "state").and_then(|s| s.as_str()) == Some("running") {
        return Ok(true);
    }
    if prop(&thread, "hasPendingApprovals") == Some(&Value::Bool(true)) {
        return Ok(true);
    }
    if prop(&thread, "hasPendingUserInput") == Some(&Value::Bool(true)) {
        return Ok(true);
    }
    if let Some(Value::Array(rows)) = prop(&thread, "pendingInteractions")
        && rows
            .iter()
            .any(|row| prop(row, "status").and_then(|s| s.as_str()) != Some("confirmed"))
    {
        return Ok(true);
    }
    if let Some(review) = prop(&thread, "claudeCacheReview")
        && !review.is_null()
        && !is_undefined(review)
        && prop(review, "status").and_then(|s| s.as_str()) != Some("failed")
    {
        return Ok(true);
    }
    Ok(false)
}

/// `role === "owner" ? \`owner:${sessionId}\` : approvedRemoteSessions.get(sessionId)`
/// — the map is empty in a fresh process, so non-owner always resolves
/// undefined.
pub fn desktop_update_authority_for_session(
    session_id: String,
    role: String,
) -> Result<serde_json::Value, String> {
    if role == "owner" {
        Ok(Value::String(format!("owner:{session_id}")))
    } else {
        Ok(json!({ "$undefined": true }))
    }
}
