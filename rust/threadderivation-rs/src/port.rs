//! The Rust port of `apps/web/src/threadDerivation.ts` —
//! `selectThreadActivities`, `getThreadFromState`, `getThreadsFromState`.
//! The WeakMap caches are identity caches only; on the wire each call is fresh.

#![forbid(unsafe_code)]

use crate::js::{idx, is_undefined, js_length, prop, spread_value, to_key, truthy};
use serde_json::Value;

/// `collectByIds(ids, byId, emptyValue)` — falsy ids/empty ids/falsy byId give
/// the empty array; a truthy non-array `ids` throws on `.forEach`; each entry
/// is `byId[ToPropertyKey(id)]` and only truthy values are kept.
fn collect_by_ids(ids: Option<Value>, by_id: Option<Value>) -> Result<Vec<Value>, String> {
    let Some(ids) = ids else {
        return Ok(Vec::new());
    };
    if !truthy(Some(&ids)) || js_length(&ids) == Some(0.0) {
        return Ok(Vec::new());
    }
    let Some(by_id) = by_id else {
        return Ok(Vec::new());
    };
    if !truthy(Some(&by_id)) {
        return Ok(Vec::new());
    }
    let Value::Array(list) = ids else {
        return Err("ids.forEach is not a function".into());
    };
    let mut out = Vec::new();
    for id in &list {
        if let Some(v) = idx(Some(&by_id), &to_key(id))
            && truthy(Some(&v))
        {
            out.push(v);
        }
    }
    Ok(out)
}

/// Shared derivation body of `getThreadFromState` given the already-coerced key.
fn derive(state: &Value, key: &str) -> Result<Option<Value>, String> {
    let shell = idx(prop(state, "threadShellById")?, key);
    let Some(shell) = shell.filter(|s| truthy(Some(s))) else {
        return Ok(None);
    };
    let session = idx(prop(state, "threadSessionById")?, key)
        .filter(|v| !is_undefined(v))
        .unwrap_or(Value::Null);
    let turn_state = idx(prop(state, "threadTurnStateById")?, key).filter(|v| !is_undefined(v));
    let messages = collect_by_ids(
        idx(prop(state, "messageIdsByThreadId")?, key),
        idx(prop(state, "messageByThreadId")?, key),
    )?;
    let activities = collect_by_ids(
        idx(prop(state, "activityIdsByThreadId")?, key),
        idx(prop(state, "activityByThreadId")?, key),
    )?;
    let proposed_plans = collect_by_ids(
        idx(prop(state, "proposedPlanIdsByThreadId")?, key),
        idx(prop(state, "proposedPlanByThreadId")?, key),
    )?;
    let turn_diff_summaries = collect_by_ids(
        idx(prop(state, "turnDiffIdsByThreadId")?, key),
        idx(prop(state, "turnDiffSummaryByThreadId")?, key),
    )?;
    let mut thread = spread_value(&shell);
    thread.insert("session".into(), session);
    thread.insert(
        "latestTurn".into(),
        idx(turn_state.as_ref(), "latestTurn")
            .filter(|v| !is_undefined(v))
            .unwrap_or(Value::Null),
    );
    thread.insert(
        "pendingSourceProposedPlan".into(),
        idx(turn_state.as_ref(), "pendingSourceProposedPlan").unwrap_or_else(crate::js::undefined),
    );
    if let Some(v) = idx(turn_state.as_ref(), "pendingTurnStartMessageId")
        && !is_undefined(&v)
    {
        thread.insert("pendingTurnStartMessageId".into(), v);
    }
    thread.insert("messages".into(), Value::Array(messages));
    thread.insert("activities".into(), Value::Array(activities));
    thread.insert("proposedPlans".into(), Value::Array(proposed_plans));
    thread.insert(
        "turnDiffSummaries".into(),
        Value::Array(turn_diff_summaries),
    );
    Ok(Some(Value::Object(thread)))
}

/// Port of `selectThreadActivities` (`apps/web/src/threadDerivation.ts:82`).
pub fn select_thread_activities(
    state: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let key = to_key(&thread_id);
    let activities = collect_by_ids(
        idx(prop(&state, "activityIdsByThreadId")?, &key),
        idx(prop(&state, "activityByThreadId")?, &key),
    )?;
    Ok(Value::Array(activities))
}

/// Port of `getThreadFromState` (`apps/web/src/threadDerivation.ts:109`).
pub fn get_thread_from_state(
    state: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    derive(&state, &to_key(&thread_id))
}

/// Port of `getThreadsFromState` (`apps/web/src/threadDerivation.ts:176`).
pub fn get_threads_from_state(state: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    let ids = prop(&state, "threadIds")?
        .filter(|v| !is_undefined(v))
        .cloned();
    let list = match ids {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(a)) => a,
        _ => return Err("state.threadIds.flatMap is not a function".into()),
    };
    let mut out = Vec::new();
    for id in list {
        if let Some(thread) = derive(&state, &to_key(&id))? {
            out.push(thread);
        }
    }
    Ok(out)
}
