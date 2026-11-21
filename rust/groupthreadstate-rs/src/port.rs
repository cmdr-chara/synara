//! The Rust port of `packages/shared/src/groupThreadState.ts`.

#![forbid(unsafe_code)]

use crate::js::{is_undefined, undefined};
use serde_json::{Value, json};

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undefined(v) {
                return false;
            }
            !matches!(
                v.get("$num").and_then(|m| m.as_str()),
                Some("NaN") | Some("-0") | Some("0")
            )
        }
    }
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn field<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&Value::Null)
}

/// `session.orchestrationStatus ?? session.status`.
fn resolved_session_status(session: &Value) -> Value {
    let orch = field(session, "orchestrationStatus");
    if !is_nullish(orch) {
        return orch.clone();
    }
    session.get("status").cloned().unwrap_or(Value::Null)
}

fn is_dead_status(v: &Value) -> bool {
    matches!(v, Value::String(s) if s == "closed" || s == "error" || s == "stopped")
}

fn can_answer(session: &Value) -> bool {
    if is_nullish(session) {
        return true;
    }
    !is_dead_status(field(session, "status"))
        && !is_dead_status(field(session, "orchestrationStatus"))
}

fn turn_settled(latest_turn: &Value, session: &Value) -> bool {
    if is_nullish(latest_turn) || !truthy(field(latest_turn, "startedAt")) {
        return false;
    }
    if !truthy(field(latest_turn, "completedAt")) {
        return false;
    }
    if matches!(field(latest_turn, "state"), Value::String(s) if s == "interrupted" || s == "error")
    {
        return true;
    }
    if is_nullish(session) {
        return true;
    }
    if resolved_session_status(session) == json!("running") {
        return false;
    }
    true
}

fn pending_or_error(v: &Value, key: &str) -> bool {
    v.get(key) == Some(&Value::Bool(true))
}

fn archived(v: &Value) -> bool {
    !is_nullish(field(v, "archivedAt"))
}

fn actively_working(thread: &Value) -> bool {
    if pending_or_error(thread, "hasLiveTailWork") {
        return true;
    }
    let session = field(thread, "session");
    !is_nullish(session)
        && resolved_session_status(session) == json!("running")
        && !turn_settled(field(thread, "latestTurn"), session)
}

fn connecting_or_starting(thread: &Value) -> bool {
    let session = field(thread, "session");
    if is_nullish(session) {
        return false;
    }
    matches!(
        resolved_session_status(session),
        Value::String(s) if s == "connecting" || s == "starting"
    )
}

fn thread_errored(thread: &Value) -> bool {
    let session = field(thread, "session");
    if !is_nullish(session) && resolved_session_status(session) == json!("error") {
        return true;
    }
    let turn = field(thread, "latestTurn");
    !is_nullish(turn)
        && field(turn, "state") == &json!("error")
        && truthy(field(turn, "startedAt"))
        && truthy(field(turn, "completedAt"))
}

fn pending_request(thread: &Value) -> bool {
    can_answer(field(thread, "session"))
        && (pending_or_error(thread, "hasPendingApprovals")
            || pending_or_error(thread, "hasPendingUserInput"))
}

/// Port of `groupThreadStateLabel` (`packages/shared/src/groupThreadState.ts:40`).
pub fn group_thread_state_label(state: Value) -> Result<Value, String> {
    let label = match &state {
        Value::String(s) => match s.as_str() {
            "waiting" => "Waiting on you",
            "working" => "Working",
            "review" => "Ready for review",
            "landing" => "Landing",
            "idle" => "Idle",
            "resolved" => "Resolved",
            _ => return Ok(undefined()),
        },
        _ => return Ok(undefined()),
    };
    Ok(json!(label))
}

/// Port of `canSessionAnswerPendingRequests` (`packages/shared/src/groupThreadState.ts:88`).
pub fn can_session_answer_pending_requests(
    session: Option<serde_json::Value>,
) -> Result<bool, String> {
    Ok(can_answer(&session.unwrap_or(Value::Null)))
}

/// Port of `isLatestTurnSettled` (`packages/shared/src/groupThreadState.ts:104`).
pub fn is_latest_turn_settled(
    latest_turn: Option<serde_json::Value>,
    session: Option<serde_json::Value>,
) -> Result<bool, String> {
    Ok(turn_settled(
        &latest_turn.unwrap_or(Value::Null),
        &session.unwrap_or(Value::Null),
    ))
}

/// Port of `hasLiveLatestTurn` (`packages/shared/src/groupThreadState.ts:118`).
pub fn has_live_latest_turn(
    latest_turn: Option<serde_json::Value>,
    session: Option<serde_json::Value>,
) -> Result<bool, String> {
    let turn = latest_turn.unwrap_or(Value::Null);
    let session = session.unwrap_or(Value::Null);
    if is_nullish(&turn) || !truthy(field(&turn, "startedAt")) {
        return Ok(false);
    }
    Ok(!turn_settled(&turn, &session))
}

/// Port of `isThreadActivelyWorking` (`packages/shared/src/groupThreadState.ts:133`).
pub fn is_thread_actively_working(thread: serde_json::Value) -> Result<bool, String> {
    if thread.is_null() {
        return Err("Cannot read properties of null (reading 'hasLiveTailWork')".into());
    }
    if is_undefined(&thread) {
        return Err("Cannot read properties of undefined (reading 'hasLiveTailWork')".into());
    }
    Ok(actively_working(&thread))
}

/// Port of `groupThreadNeedsAttention` (`packages/shared/src/groupThreadState.ts:163`).
pub fn group_thread_needs_attention(thread: serde_json::Value) -> Result<bool, String> {
    if thread.is_null() {
        return Err("Cannot read properties of null (reading 'archivedAt')".into());
    }
    if is_undefined(&thread) {
        return Err("Cannot read properties of undefined (reading 'archivedAt')".into());
    }
    if archived(&thread) {
        return Ok(false);
    }
    Ok(
        pending_or_error(&thread, "needsYou")
            || pending_request(&thread)
            || thread_errored(&thread),
    )
}

/// Port of `resolveGroupCoordinatorStatus` (`packages/shared/src/groupThreadState.ts:197`).
pub fn resolve_group_coordinator_status(input: serde_json::Value) -> Result<String, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'configured')".into());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'configured')".into());
    }
    if !truthy(field(&input, "configured")) {
        return Ok("unconfigured".into());
    }
    let goal = field(&input, "goalStatus");
    if let Value::String(g) = goal {
        match g.as_str() {
            "paused" => return Ok("paused".into()),
            "stopped" | "cancelled" => return Ok("stopped".into()),
            "active" => return Ok("running".into()),
            _ => {}
        }
    }
    let thread = field(&input, "thread");
    if is_nullish(thread) || archived(thread) {
        return Ok("idle".into());
    }
    if pending_request(thread) || actively_working(thread) || connecting_or_starting(thread) {
        return Ok("running".into());
    }
    Ok("idle".into())
}

/// Port of `resolveGroupThreadState` (`packages/shared/src/groupThreadState.ts:242`).
pub fn resolve_group_thread_state(input: serde_json::Value) -> Result<String, String> {
    if input.is_null() {
        return Err("Cannot destructure property 'thread' of 'input' as it is null.".into());
    }
    if is_undefined(&input) {
        return Err("Cannot destructure property 'thread' of 'input' as it is undefined.".into());
    }
    let thread = field(&input, "thread");
    if thread.is_null() {
        return Err("Cannot read properties of null (reading 'archivedAt')".into());
    }
    if is_undefined(thread) {
        return Err("Cannot read properties of undefined (reading 'archivedAt')".into());
    }
    let task = field(&input, "task");
    if archived(thread)
        || truthy(field(&input, "indexArchived"))
        || archived(task)
        || matches!(field(task, "status"), Value::String(s) if s == "done" || s == "cancelled")
    {
        return Ok("resolved".into());
    }
    if pending_request(thread) || pending_or_error(thread, "needsYou") || thread_errored(thread) {
        return Ok("waiting".into());
    }
    if actively_working(thread) || connecting_or_starting(thread) {
        return Ok("working".into());
    }
    let pr = field(&input, "pullRequest");
    if !is_nullish(pr) {
        if field(pr, "state") == &json!("open") {
            if !truthy(field(pr, "isDraft")) {
                return Ok("review".into());
            }
        } else {
            return Ok("resolved".into());
        }
    }
    Ok("idle".into())
}
