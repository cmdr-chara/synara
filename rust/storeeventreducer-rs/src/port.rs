//! The Rust port of `apps/web/src/storeEventReducer.ts`.
//!
//! Shared-package helpers live in `crate::shared`; the storeNormalization /
//! storeProjection / threadDerivation subset lives in `crate::store`; JS value
//! semantics live in `crate::jsv`. All three are inlined copies so the crate
//! stays self-contained.

#![forbid(unsafe_code)]

use std::collections::HashSet;

use serde_json::{Map, Value, json};

use crate::js;
use crate::jsv;

// ---------- JS operator helpers ----------

/// `a > b`: string pair compares codepoints; else numeric (NaN → false).
fn js_gt(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x > y,
        _ => match (jsv::to_number(a), jsv::to_number(b)) {
            (Some(x), Some(y)) => x > y,
            _ => false,
        },
    }
}

/// `a >= b`.
fn js_ge(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x >= y,
        _ => match (jsv::to_number(a), jsv::to_number(b)) {
            (Some(x), Some(y)) => x >= y,
            _ => false,
        },
    }
}

/// `a < b`.
fn js_lt(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x < y,
        _ => match (jsv::to_number(a), jsv::to_number(b)) {
            (Some(x), Some(y)) => x < y,
            _ => false,
        },
    }
}

/// `a <= b`.
fn js_le(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x <= y,
        _ => match (jsv::to_number(a), jsv::to_number(b)) {
            (Some(x), Some(y)) => x <= y,
            _ => false,
        },
    }
}

/// `v[i]` index access: arrays index items, strings index UTF-16 units,
/// anything else yields undefined.
fn index_of(v: &Value, i: usize) -> Value {
    match v {
        Value::Array(a) => a.get(i).cloned().unwrap_or_else(js::undefined),
        Value::String(s) => crate::utf16::units(s)
            .get(i)
            .map(|u| js::utf16_string(&[*u]))
            .unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

fn nullish_member_err(v: &Value, expr: &str) -> Option<String> {
    if v.is_null() {
        Some(format!("null is not an object (evaluating '{expr}')"))
    } else if js::is_undefined(v) {
        Some(format!("undefined is not an object (evaluating '{expr}')"))
    } else {
        None
    }
}

/// `arr.find(pred)`.
fn find<'a>(
    v: &'a Value,
    expr: &str,
    pred: impl Fn(&Value) -> Result<bool, String>,
) -> Result<Option<&'a Value>, String> {
    if let Some(e) = nullish_member_err(v, &format!("{expr}.find")) {
        return Err(e);
    }
    let Some(items) = v.as_array() else {
        return Err(format!(
            "{expr}.find is not a function. (In '{expr}.find(...)', '{expr}.find' is undefined)"
        ));
    };
    for item in items {
        if pred(item)? {
            return Ok(Some(item));
        }
    }
    Ok(None)
}

/// `arr.findIndex(pred)`.
fn find_index(
    v: &Value,
    expr: &str,
    pred: impl Fn(&Value) -> Result<bool, String>,
) -> Result<i64, String> {
    if let Some(e) = nullish_member_err(v, &format!("{expr}.findIndex")) {
        return Err(e);
    }
    let Some(items) = v.as_array() else {
        return Err(format!(
            "{expr}.findIndex is not a function. (In '{expr}.findIndex(...)', '{expr}.findIndex' is undefined)"
        ));
    };
    for (i, item) in items.iter().enumerate() {
        if pred(item)? {
            return Ok(i as i64);
        }
    }
    Ok(-1)
}

/// `arr.map(f)` over a JS array value.
fn map_array(
    v: &Value,
    expr: &str,
    f: impl FnMut(&Value) -> Result<Value, String>,
) -> Result<Vec<Value>, String> {
    if let Some(e) = nullish_member_err(v, &format!("{expr}.map")) {
        return Err(e);
    }
    let Some(items) = v.as_array() else {
        return Err(format!(
            "{expr}.map is not a function. (In '{expr}.map(...)', '{expr}.map' is undefined)"
        ));
    };
    items.iter().map(f).collect()
}

/// `arr.filter(f)`.
fn filter_array(
    v: &Value,
    expr: &str,
    f: impl Fn(&Value) -> Result<bool, String>,
) -> Result<Vec<Value>, String> {
    if let Some(e) = nullish_member_err(v, &format!("{expr}.filter")) {
        return Err(e);
    }
    let Some(items) = v.as_array() else {
        return Err(format!(
            "{expr}.filter is not a function. (In '{expr}.filter(...)', '{expr}.filter' is undefined)"
        ));
    };
    let mut out = Vec::new();
    for item in items {
        if f(item)? {
            out.push(item.clone());
        }
    }
    Ok(out)
}

/// `event.<key>` member access with the source member expression in errors.
fn ef(event: &Value, key: &str) -> Result<Value, String> {
    let expr = format!("event.{key}");
    Ok(jsv::member(event, key, &expr)?
        .cloned()
        .unwrap_or_else(js::undefined))
}

/// `event.payload.<key>` — throws on nullish payload exactly like the source.
fn pf(event: &Value, key: &str) -> Result<Value, String> {
    let payload = jsv::opt(event, "payload");
    let expr = format!("event.payload.{key}");
    Ok(jsv::member(&payload, key, &expr)?
        .cloned()
        .unwrap_or_else(js::undefined))
}

/// `v == null` (loose null/undefined check).
fn is_nullish(v: &Value) -> bool {
    jsv::is_nullish(v)
}

/// `x ?? y` (owned).
fn coalesce_owned(x: Value, y: Value) -> Value {
    if is_nullish(&x) { y } else { x }
}

/// `thread.updatedAt ?? thread.createdAt`.
fn thread_updated_at(thread: &Value) -> Value {
    let updated = jsv::opt(thread, "updatedAt");
    if is_nullish(&updated) {
        jsv::opt(thread, "createdAt")
    } else {
        updated
    }
}

/// Set key to `arr[i] === val` (`Array.prototype.with`).
fn array_with(arr: &[Value], index: usize, val: Value) -> Vec<Value> {
    let mut out = arr.to_vec();
    if index < out.len() {
        out[index] = val;
    }
    out
}

/// `arr.slice(-n)` — keep the last n.
fn slice_last(arr: Vec<Value>, n: usize) -> Vec<Value> {
    if arr.len() > n {
        arr[arr.len() - n..].to_vec()
    } else {
        arr
    }
}

fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}

/// `Number.MAX_SAFE_INTEGER`.
const MAX_SAFE: f64 = 9_007_199_254_740_991.0;

// ---------- reducer-local helpers ----------

const SUMMARY_ACTIVITY_KINDS: &[&str] = &[
    "approval.requested",
    "approval.resolved",
    "provider.approval.respond.failed",
    "user-input.requested",
    "user-input.resolved",
    "provider.user-input.respond.failed",
];

fn thread_message_updates_summary(event: &Value) -> bool {
    jsv::get(&jsv::opt(event, "payload"), "role").and_then(Value::as_str) == Some("user")
}

fn thread_activity_updates_summary(event: &Value) -> bool {
    let activity = jsv::opt(&jsv::opt(event, "payload"), "activity");
    let kind = jsv::get(&activity, "kind")
        .and_then(Value::as_str)
        .unwrap_or("");
    SUMMARY_ACTIVITY_KINDS.contains(&kind)
}

fn thread_activity_updates_sidebar_summary(event: &Value) -> bool {
    if thread_activity_updates_summary(event) {
        return true;
    }
    let activity = jsv::opt(&jsv::opt(event, "payload"), "activity");
    crate::shared::is_background_work_activity(&activity)
}

fn thread_message_updates_sidebar_summary(event: &Value) -> bool {
    let payload = jsv::opt(event, "payload");
    jsv::get(&payload, "role").and_then(Value::as_str) == Some("user")
        || !jsv::truthy(&jsv::opt(&payload, "streaming"))
}

fn resolve_event_updated_at(thread: &Value, updated_at: &Value) -> Value {
    let current = thread_updated_at(thread);
    if js_gt(&current, updated_at) {
        current
    } else {
        updated_at.clone()
    }
}

/// Port of `markInteractionResponding` (storeEventReducer.ts:113).
fn mark_interaction_responding(thread: &Value, event: &Value) -> Result<Value, String> {
    let pending = jsv::opt(thread, "pendingInteractions");
    let command_id = ef(event, "commandId")?;
    if is_undef(&pending) || command_id.is_null() {
        return Ok(pending);
    }
    let event_type = ef(event, "type")?;
    let interaction_kind =
        if event_type == Value::String("thread.approval-response-requested".into()) {
            "approval"
        } else {
            "userInput"
        };
    let lifecycle_generation = coalesce_owned(pf(event, "lifecycleGeneration")?, Value::Null);
    let created_at = pf(event, "createdAt")?;
    let decision = if event_type == Value::String("thread.approval-response-requested".into()) {
        pf(event, "decision")?
    } else {
        Value::Null
    };
    let mut changed = false;
    let next = map_array(&pending, "thread.pendingInteractions", |interaction| {
        let ik = jsv::opt(interaction, "interactionKind");
        let rid = jsv::opt(interaction, "requestId");
        let lg = jsv::opt(interaction, "lifecycleGeneration");
        let status = jsv::opt(interaction, "status");
        let response_requested_at = jsv::opt(interaction, "responseRequestedAt");
        let claimable_input = json!({
            "status": status,
            "responseRequestedAt": response_requested_at,
            "requestedAt": created_at,
        });
        let claimable = crate::shared::is_pending_interaction_response_claimable(&claimable_input)?;
        if ik != Value::String(interaction_kind.into())
            || rid != pf(event, "requestId")?
            || lg != lifecycle_generation
            || !claimable
        {
            return Ok(interaction.clone());
        }
        changed = true;
        let mut o = jsv::spread(interaction);
        o.insert("status".into(), Value::String("responding".into()));
        o.insert("decision".into(), decision.clone());
        o.insert("responseCommandId".into(), command_id.clone());
        o.insert("responseRequestedAt".into(), created_at.clone());
        o.insert("resolvedAt".into(), Value::Null);
        Ok(Value::Object(o))
    })?;
    Ok(if changed { Value::Array(next) } else { pending })
}

/// Port of `reconcilePendingInteractionsFromActivity` (storeEventReducer.ts:152).
fn reconcile_pending_interactions_from_activity(
    thread_id: &Value,
    pending_interactions: &Value,
    event: &Value,
) -> Result<Value, String> {
    let activity = pf(event, "activity")?;
    let kind = jsv::opt(&activity, "kind");
    let kind_s = kind.as_str().unwrap_or("");
    let interaction_kind = match kind_s {
        "approval.requested" | "approval.resolved" | "provider.approval.respond.failed" => {
            "approval"
        }
        "user-input.requested" | "user-input.resolved" | "provider.user-input.respond.failed" => {
            "userInput"
        }
        _ => return Ok(pending_interactions.clone()),
    };
    let activity_payload = jsv::opt(&activity, "payload");
    let payload = crate::store::as_activity_record(&activity_payload);
    let request_id = payload
        .and_then(|p| p.get("requestId"))
        .and_then(Value::as_str);
    let Some(request_id) = request_id.filter(|s| !s.is_empty()) else {
        return Ok(pending_interactions.clone());
    };
    let lifecycle_generation = match payload.and_then(|p| p.get("lifecycleGeneration")) {
        Some(Value::String(g)) if !g.is_empty() => Value::String(g.clone()),
        _ => Value::Null,
    };
    let existing: Vec<Value> = if is_nullish(pending_interactions) {
        Vec::new()
    } else {
        pending_interactions.as_array().cloned().unwrap_or_default()
    };
    let request_id_v = Value::String(request_id.to_string());
    let matches_identity = |interaction: &Value| -> bool {
        jsv::opt(interaction, "interactionKind") == Value::String(interaction_kind.into())
            && jsv::opt(interaction, "requestId") == request_id_v
            && (lifecycle_generation.is_null()
                || jsv::opt(interaction, "lifecycleGeneration") == lifecycle_generation)
    };

    if kind_s == "approval.resolved" || kind_s == "user-input.resolved" {
        let next: Vec<Value> = existing
            .iter()
            .filter(|i| !matches_identity(i))
            .cloned()
            .collect();
        return Ok(if next.len() == existing.len() {
            pending_interactions.clone()
        } else {
            Value::Array(next)
        });
    }

    if kind_s == "provider.approval.respond.failed"
        || kind_s == "provider.user-input.respond.failed"
    {
        let response_command_id = payload.and_then(|p| p.get("responseCommandId"));
        let has_response_command = matches!(
            response_command_id,
            Some(Value::String(s)) if !s.is_empty()
        );
        let activity_arr = Value::Array(vec![activity.clone()]);
        let is_stale = |interaction: &Value| -> Result<bool, String> {
            crate::shared::is_stale_pending_interaction(
                interaction,
                &activity_arr,
                &js::undefined(),
            )
        };
        let settlement_status = if payload.and_then(|p| p.get("settlementStatus"))
            == Some(&Value::String("retryable".into()))
        {
            "retryable"
        } else {
            "uncertain"
        };
        let mut changed = false;
        let mut next = Vec::with_capacity(existing.len());
        for interaction in &existing {
            let status = jsv::opt(interaction, "status");
            let matches = matches_identity(interaction)
                && status != Value::String("confirmed".into())
                && if has_response_command {
                    status == Value::String("responding".into())
                        && jsv::opt(interaction, "responseCommandId")
                            == response_command_id.cloned().unwrap_or_else(js::undefined)
                } else {
                    is_stale(interaction)?
                };
            if !matches {
                next.push(interaction.clone());
                continue;
            }
            changed = true;
            let mut o = jsv::spread(interaction);
            if is_stale(interaction)? {
                o.insert("status".into(), Value::String("confirmed".into()));
                o.insert("resolvedAt".into(), jsv::opt(&activity, "createdAt"));
            } else {
                o.insert("status".into(), Value::String(settlement_status.into()));
                o.insert("resolvedAt".into(), Value::Null);
            }
            next.push(Value::Object(o));
        }
        return Ok(if changed {
            Value::Array(next)
        } else {
            pending_interactions.clone()
        });
    }

    let exact_index = existing.iter().position(|interaction| {
        jsv::opt(interaction, "interactionKind") == Value::String(interaction_kind.into())
            && jsv::opt(interaction, "requestId") == request_id_v
    });
    let current = exact_index.map(|i| &existing[i]);
    if let Some(current) = current
        && jsv::opt(current, "lifecycleGeneration") == lifecycle_generation
        && ["responding", "confirmed", "uncertain"]
            .contains(&jsv::opt(current, "status").as_str().unwrap_or(""))
    {
        return Ok(pending_interactions.clone());
    }
    let created_at = match current {
        Some(c) if jsv::opt(c, "lifecycleGeneration") == lifecycle_generation => {
            jsv::opt(c, "createdAt")
        }
        _ => jsv::opt(&activity, "createdAt"),
    };
    let pending = json!({
        "interactionKind": interaction_kind,
        "requestId": request_id_v,
        "threadId": thread_id,
        "turnId": jsv::opt(&activity, "turnId"),
        "lifecycleGeneration": lifecycle_generation,
        "status": "pending",
        "decision": Value::Null,
        "responseCommandId": Value::Null,
        "responseRequestedAt": Value::Null,
        "createdAt": created_at,
        "resolvedAt": Value::Null,
    });
    let mut next = existing.clone();
    match exact_index {
        None => next.push(pending),
        Some(i) => next[i] = pending,
    }
    Ok(Value::Array(next))
}

/// Port of `normalizeSingleTurnDiffSummary` (storeEventReducer.ts:257).
fn normalize_single_turn_diff_summary(
    incoming: &Value,
    previous: Option<&Value>,
) -> Result<Value, String> {
    let prev_files = previous.map(|p| jsv::opt(p, "files"));
    let files =
        crate::store::normalize_turn_diff_files(&jsv::opt(incoming, "files"), prev_files.as_ref())?;
    if let Some(previous) = previous {
        let started_eq = jsv::opt(previous, "startedAt")
            == coalesce_owned(
                jsv::opt(incoming, "startedAt"),
                jsv::opt(previous, "startedAt"),
            );
        if jsv::opt(previous, "turnId") == jsv::opt(incoming, "turnId")
            && started_eq
            && jsv::opt(previous, "completedAt") == jsv::opt(incoming, "completedAt")
            && jsv::opt(previous, "status") == jsv::opt(incoming, "status")
            && jsv::opt(previous, "assistantMessageId") == jsv::opt(incoming, "assistantMessageId")
            && jsv::opt(previous, "checkpointTurnCount")
                == jsv::opt(incoming, "checkpointTurnCount")
            && jsv::opt(previous, "checkpointRef") == jsv::opt(incoming, "checkpointRef")
            && jsv::opt(previous, "files") == files
        {
            return Ok(previous.clone());
        }
    }
    let started_at = coalesce_owned(
        jsv::opt(incoming, "startedAt"),
        previous
            .map(|p| jsv::opt(p, "startedAt"))
            .unwrap_or_else(js::undefined),
    );
    let mut o = jsv::spread(incoming);
    if jsv::truthy(&started_at) {
        o.insert("startedAt".into(), started_at);
    }
    o.insert("files".into(), files);
    Ok(Value::Object(o))
}

/// `v ?? Number.MAX_SAFE_INTEGER` for `checkpointTurnCount`.
fn checkpoint_count(v: &Value) -> f64 {
    if is_nullish(v) {
        MAX_SAFE
    } else {
        jsv::to_number(v).unwrap_or(f64::NAN)
    }
}

/// `left.<lexpr>.localeCompare(right)` — JSCore error strings on bad receivers.
fn locale_call(left: &Value, right: &Value, lexpr: &str) -> Result<i64, String> {
    if left.is_null() {
        return Err(format!(
            "null is not an object (evaluating '{lexpr}.localeCompare')"
        ));
    }
    if js::is_undefined(left) {
        return Err(format!(
            "undefined is not an object (evaluating '{lexpr}.localeCompare')"
        ));
    }
    let Some(ls) = left.as_str() else {
        return Err(format!(
            "{lexpr}.localeCompare is not a function. (In '{lexpr}.localeCompare(...)', '{lexpr}.localeCompare' is undefined)"
        ));
    };
    Ok(jsv::locale_compare(ls, &jsv::js_to_string(right)))
}

fn sort_turn_diff_summaries(summaries: &[Value]) -> Result<Vec<Value>, String> {
    let mut out = summaries.to_vec();
    let mut err = None;
    js::sort(&mut out, |left, right| {
        if err.is_some() {
            return 0.0;
        }
        let diff = checkpoint_count(&jsv::opt(left, "checkpointTurnCount"))
            - checkpoint_count(&jsv::opt(right, "checkpointTurnCount"));
        if diff != 0.0 && !diff.is_nan() {
            return diff;
        }
        match locale_call(
            &jsv::opt(left, "completedAt"),
            &jsv::opt(right, "completedAt"),
            "left.completedAt",
        ) {
            Ok(c) if c != 0 => c as f64,
            Ok(_) => match locale_call(
                &jsv::opt(left, "turnId"),
                &jsv::opt(right, "turnId"),
                "left.turnId",
            ) {
                Ok(c) => c as f64,
                Err(e) => {
                    err = Some(e);
                    0.0
                }
            },
            Err(e) => {
                err = Some(e);
                0.0
            }
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(out)
}

fn checkpoint_status_to_latest_turn_state(status: &Value) -> &'static str {
    match status.as_str() {
        Some("error") => "error",
        Some("missing") => "interrupted",
        _ => "completed",
    }
}

fn is_provider_diff_placeholder_ref(checkpoint_ref: &Value) -> bool {
    checkpoint_ref
        .as_str()
        .is_some_and(|s| s.starts_with("provider-diff:"))
}

/// Port of `buildLatestTurn` (storeEventReducer.ts:311).
#[allow(clippy::too_many_arguments)]
fn build_latest_turn(
    previous: &Value,
    turn_id: &Value,
    state: &str,
    requested_at: &Value,
    started_at: &Value,
    completed_at: &Value,
    assistant_message_id: &Value,
    source_proposed_plan: Option<&Value>,
) -> Value {
    let spp = if jsv::opt(previous, "turnId") == *turn_id {
        let prev_spp = jsv::opt(previous, "sourceProposedPlan");
        coalesce_owned(
            prev_spp,
            source_proposed_plan.cloned().unwrap_or_else(js::undefined),
        )
    } else {
        source_proposed_plan.cloned().unwrap_or_else(js::undefined)
    };
    let mut o = Map::new();
    o.insert("turnId".into(), turn_id.clone());
    o.insert("state".into(), Value::String(state.into()));
    o.insert("requestedAt".into(), requested_at.clone());
    o.insert("startedAt".into(), started_at.clone());
    o.insert("completedAt".into(), completed_at.clone());
    o.insert("assistantMessageId".into(), assistant_message_id.clone());
    if jsv::truthy(&spp) {
        o.insert("sourceProposedPlan".into(), spp);
    }
    Value::Object(o)
}

/// Port of `isUnboundTurnRequest` (storeEventReducer.ts:376).
fn is_unbound_turn_request(message: &Value) -> bool {
    jsv::opt(message, "role") == Value::String("user".into())
        && jsv::opt(message, "startsNewTurn") != Value::Bool(false)
        && is_nullish(&jsv::opt(message, "turnId"))
}

/// Port of `bindPendingTurnStartMessage` (storeEventReducer.ts:340).
fn bind_pending_turn_start_message(thread: &Value, session: &Value) -> Result<Value, String> {
    if !crate::shared::is_session_running_turn(session) {
        return Ok(Value::Null);
    }
    let pending_message_id = jsv::opt(thread, "pendingTurnStartMessageId");
    let review = jsv::opt(thread, "claudeCacheReview");
    let active_turn_id = jsv::opt(session, "activeTurnId");
    if pending_message_id.is_null()
        || is_undef(&pending_message_id)
        || jsv::opt(&review, "status") == Value::String("compacting".into())
        || jsv::opt(&review, "compactionTurnId") == active_turn_id
    {
        return Ok(Value::Null);
    }
    let messages = jsv::opt(thread, "messages");
    let message_index: i64 = messages
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .rposition(|m| jsv::opt(m, "id") == pending_message_id)
        })
        .map(|i| i as i64)
        .unwrap_or(-1);
    let mut rest = jsv::spread(thread);
    rest.insert("pendingTurnStartMessageId".into(), Value::Null);
    let message = if message_index >= 0 {
        messages
            .as_array()
            .map(|a| a[message_index as usize].clone())
    } else {
        None
    };
    let turn_id = active_turn_id;
    let has_turn_message = messages
        .as_array()
        .is_some_and(|items| items.iter().any(|c| jsv::opt(c, "turnId") == turn_id));
    let activities = jsv::opt(thread, "activities");
    let has_turn_activity = activities
        .as_array()
        .is_some_and(|items| items.iter().any(|a| jsv::opt(a, "turnId") == turn_id));
    if let Some(m) = &message
        && is_unbound_turn_request(m)
        && !has_turn_message
        && !has_turn_activity
    {
        let mut mo = jsv::spread(m);
        mo.insert("turnId".into(), turn_id);
        let items = messages.as_array().cloned().unwrap_or_default();
        rest.insert(
            "messages".into(),
            Value::Array(array_with(
                &items,
                message_index as usize,
                Value::Object(mo),
            )),
        );
    }
    Ok(Value::Object(rest))
}

/// Port of `reconcileLatestTurnFromSession` (storeEventReducer.ts:384).
fn reconcile_latest_turn_from_session(
    thread: &Value,
    session: &Value,
    _error: &Value,
) -> Result<Value, String> {
    let latest_turn = jsv::opt(thread, "latestTurn");
    let active_turn_id = jsv::opt(session, "activeTurnId");
    let session_updated = jsv::opt(session, "updatedAt");
    let same_turn = jsv::opt(&latest_turn, "turnId") == active_turn_id;
    if crate::shared::is_session_running_turn(session) {
        let requested_at = if same_turn {
            jsv::opt(&latest_turn, "requestedAt")
        } else {
            session_updated.clone()
        };
        let started_at = if same_turn {
            coalesce_owned(jsv::opt(&latest_turn, "startedAt"), session_updated.clone())
        } else {
            session_updated.clone()
        };
        let assistant = if same_turn {
            jsv::opt(&latest_turn, "assistantMessageId")
        } else {
            Value::Null
        };
        let spp = jsv::opt(thread, "pendingSourceProposedPlan");
        return Ok(build_latest_turn(
            &latest_turn,
            &active_turn_id,
            "running",
            &requested_at,
            &started_at,
            &Value::Null,
            &assistant,
            Some(&spp),
        ));
    }
    let session_status = jsv::opt(session, "status");
    let settled_state: Option<&str> = match session_status.as_str() {
        Some("error") => Some("error"),
        Some("interrupted") | Some("stopped") => Some("interrupted"),
        Some("ready") => Some("completed"),
        _ => None,
    };
    let turn_running = jsv::opt(&latest_turn, "state") == Value::String("running".into());
    if let Some(settled) = settled_state
        && turn_running
        && (is_nullish(&active_turn_id) || settled == "error")
        && (settled == "error"
            || js_ge(
                &session_updated,
                &coalesce_owned(
                    jsv::opt(&latest_turn, "startedAt"),
                    jsv::opt(&latest_turn, "requestedAt"),
                ),
            ))
    {
        let spp = jsv::opt(thread, "pendingSourceProposedPlan");
        return Ok(build_latest_turn(
            &latest_turn,
            &jsv::opt(&latest_turn, "turnId"),
            settled,
            &jsv::opt(&latest_turn, "requestedAt"),
            &jsv::opt(&latest_turn, "startedAt"),
            &session_updated,
            &jsv::opt(&latest_turn, "assistantMessageId"),
            Some(&spp),
        ));
    }
    Ok(latest_turn)
}

/// Port of `rebindTurnDiffSummariesForAssistantMessage` (storeEventReducer.ts:453).
fn rebind_turn_diff_summaries_for_assistant_message(
    summaries: &Value,
    turn_id: &Value,
    assistant_message_id: &Value,
) -> Result<Vec<Value>, String> {
    let items = summaries.as_array().cloned().unwrap_or_default();
    let mut changed = false;
    let next: Vec<Value> = items
        .iter()
        .map(|summary| {
            if jsv::opt(summary, "turnId") != *turn_id
                || jsv::opt(summary, "assistantMessageId") == *assistant_message_id
            {
                return summary.clone();
            }
            changed = true;
            let mut o = jsv::spread(summary);
            match assistant_message_id {
                v if v.is_null() => {
                    o.insert("assistantMessageId".into(), js::undefined());
                }
                v => {
                    o.insert("assistantMessageId".into(), v.clone());
                }
            }
            Value::Object(o)
        })
        .collect();
    Ok(if changed { next } else { items })
}

/// Port of `retainThreadMessagesAfterRevert` (storeEventReducer.ts:472).
fn retain_thread_messages_after_revert(
    messages: &Value,
    retained_turn_ids: &HashSet<String>,
    turn_count: f64,
) -> Result<Value, String> {
    let items = messages.as_array().cloned().unwrap_or_default();
    let mut retained_ids: HashSet<String> = HashSet::new();
    for message in &items {
        if jsv::opt(message, "role") == Value::String("system".into()) {
            if let Some(id) = jsv::opt(message, "id").as_str() {
                retained_ids.insert(id.to_string());
            }
            continue;
        }
        let turn_id = jsv::opt(message, "turnId");
        if !is_nullish(&turn_id)
            && turn_id
                .as_str()
                .is_some_and(|t| retained_turn_ids.contains(t))
            && let Some(id) = jsv::opt(message, "id").as_str()
        {
            retained_ids.insert(id.to_string());
        }
    }
    for role in ["user", "assistant"] {
        let retained_count = items
            .iter()
            .filter(|m| {
                jsv::opt(m, "role") == Value::String(role.into())
                    && jsv::opt(m, "id")
                        .as_str()
                        .is_some_and(|id| retained_ids.contains(id))
            })
            .count();
        let missing = jsv::js_max(0.0, turn_count - retained_count as f64);
        if missing > 0.0 && !missing.is_nan() {
            let mut fallback: Vec<Value> = items
                .iter()
                .filter(|m| {
                    if jsv::opt(m, "role") != Value::String(role.into()) {
                        return false;
                    }
                    if jsv::opt(m, "id")
                        .as_str()
                        .is_some_and(|id| retained_ids.contains(id))
                    {
                        return false;
                    }
                    let tid = jsv::opt(m, "turnId");
                    is_nullish(&tid) || tid.as_str().is_some_and(|t| retained_turn_ids.contains(t))
                })
                .cloned()
                .collect();
            js::sort(&mut fallback, |l, r| {
                let c = jsv::locale_compare(
                    jsv::opt(l, "createdAt").as_str().unwrap_or(""),
                    jsv::opt(r, "createdAt").as_str().unwrap_or(""),
                );
                if c != 0 {
                    c as f64
                } else {
                    jsv::locale_compare(
                        jsv::opt(l, "id").as_str().unwrap_or(""),
                        jsv::opt(r, "id").as_str().unwrap_or(""),
                    ) as f64
                }
            });
            for m in fallback.into_iter().take(missing as usize) {
                if let Some(id) = jsv::opt(&m, "id").as_str() {
                    retained_ids.insert(id.to_string());
                }
            }
        }
    }
    Ok(Value::Array(
        items
            .iter()
            .filter(|m| {
                jsv::opt(m, "id")
                    .as_str()
                    .is_some_and(|id| retained_ids.contains(id))
            })
            .cloned()
            .collect(),
    ))
}

/// Port of `retainThreadActivitiesAfterRevert` (storeEventReducer.ts:543).
fn retain_thread_activities_after_revert(
    activities: &Value,
    retained_turn_ids: &HashSet<String>,
) -> Value {
    let items = activities.as_array().cloned().unwrap_or_default();
    Value::Array(
        items
            .into_iter()
            .filter(|a| {
                let tid = jsv::opt(a, "turnId");
                tid.is_null() || tid.as_str().is_some_and(|t| retained_turn_ids.contains(t))
            })
            .collect(),
    )
}

/// Port of `retainThreadProposedPlansAfterRevert` (storeEventReducer.ts:552).
fn retain_thread_proposed_plans_after_revert(
    plans: &Value,
    retained_turn_ids: &HashSet<String>,
) -> Value {
    let items = plans.as_array().cloned().unwrap_or_default();
    Value::Array(
        items
            .into_iter()
            .filter(|p| {
                let tid = jsv::opt(p, "turnId");
                tid.is_null() || tid.as_str().is_some_and(|t| retained_turn_ids.contains(t))
            })
            .collect(),
    )
}

/// Port of `rollbackThreadMessagesFromMessage` (storeEventReducer.ts:561).
fn rollback_thread_messages_from_message(
    messages: &Value,
    message_id: &Value,
) -> (Vec<Value>, HashSet<String>) {
    let items = messages.as_array().cloned().unwrap_or_default();
    let target = items.iter().position(|m| jsv::opt(m, "id") == *message_id);
    let Some(target) = target else {
        return (items, HashSet::new());
    };
    let removed = &items[target..];
    let removed_turn_ids: HashSet<String> = removed
        .iter()
        .filter_map(|m| jsv::opt(m, "turnId").as_str().map(str::to_string))
        .collect();
    (items[..target].to_vec(), removed_turn_ids)
}

/// Port of `applyTurnDiffSummaryToThread` (storeEventReducer.ts:584).
fn apply_turn_diff_summary_to_thread(thread: &Value, summary: &Value) -> Result<Value, String> {
    let turn_diff_summaries = jsv::opt(thread, "turnDiffSummaries");
    let summary_turn_id = jsv::opt(summary, "turnId");
    let previous_summary = turn_diff_summaries
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|s| jsv::opt(s, "turnId") == summary_turn_id)
        })
        .cloned();
    let next_summary = normalize_single_turn_diff_summary(summary, previous_summary.as_ref())?;
    if previous_summary.is_some()
        && jsv::opt(previous_summary.as_ref().unwrap_or(&Value::Null), "status")
            != Value::String("missing".into())
        && jsv::opt(&next_summary, "status") == Value::String("missing".into())
    {
        return Ok(thread.clone());
    }
    let next_turn_diff_summaries: Vec<Value> = if previous_summary.is_some() {
        let next_tid = jsv::opt(&next_summary, "turnId");
        turn_diff_summaries
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|s| {
                        if jsv::opt(s, "turnId") == next_tid {
                            next_summary.clone()
                        } else {
                            s.clone()
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        let mut all = turn_diff_summaries.as_array().cloned().unwrap_or_default();
        all.push(next_summary.clone());
        sort_turn_diff_summaries(&all)?
    };

    let latest_turn = jsv::opt(thread, "latestTurn");
    let next_completed = jsv::opt(&next_summary, "completedAt");
    let next_status = jsv::opt(&next_summary, "status");
    let same_turn = jsv::opt(&latest_turn, "turnId") == jsv::opt(&next_summary, "turnId");
    let is_same_turn_placeholder =
        is_provider_diff_placeholder_ref(&jsv::opt(&next_summary, "checkpointRef"))
            && next_status == Value::String("missing".into())
            && same_turn;
    let keeps_lifecycle = same_turn
        && (jsv::opt(&latest_turn, "state") == Value::String("interrupted".into())
            || jsv::opt(&latest_turn, "state") == Value::String("error".into())
            || next_status == Value::String("missing".into()));
    let latest_turn_out: Value = if latest_turn.is_null() || same_turn {
        if is_same_turn_placeholder {
            latest_turn.clone()
        } else {
            let state = if keeps_lifecycle && !latest_turn.is_null() && !is_undef(&latest_turn) {
                jsv::opt(&latest_turn, "state")
                    .as_str()
                    .unwrap_or("completed")
                    .to_string()
            } else {
                checkpoint_status_to_latest_turn_state(&next_status).to_string()
            };
            let completed_at = if next_status == Value::String("missing".into())
                && keeps_lifecycle
                && !latest_turn.is_null()
                && !is_undef(&latest_turn)
            {
                if jsv::opt(&latest_turn, "state") == Value::String("running".into()) {
                    Value::Null
                } else {
                    coalesce_owned(
                        jsv::opt(&latest_turn, "completedAt"),
                        next_completed.clone(),
                    )
                }
            } else {
                next_completed.clone()
            };
            let assistant = {
                let incoming = jsv::opt(&next_summary, "assistantMessageId");
                if !is_nullish(&incoming) {
                    incoming
                } else if same_turn && !latest_turn.is_null() && !is_undef(&latest_turn) {
                    let prev = jsv::opt(&latest_turn, "assistantMessageId");
                    if is_nullish(&prev) { Value::Null } else { prev }
                } else {
                    Value::Null
                }
            };
            let spp = jsv::opt(thread, "pendingSourceProposedPlan");
            build_latest_turn(
                &latest_turn,
                &jsv::opt(&next_summary, "turnId"),
                &state,
                &coalesce_owned(
                    jsv::opt(&latest_turn, "requestedAt"),
                    next_completed.clone(),
                ),
                &coalesce_owned(jsv::opt(&latest_turn, "startedAt"), next_completed.clone()),
                &completed_at,
                &assistant,
                Some(&spp),
            )
        }
    } else {
        latest_turn.clone()
    };

    let unchanged = previous_summary
        .as_ref()
        .is_some_and(|p| *p == next_summary)
        && Value::Array(next_turn_diff_summaries.clone()) == turn_diff_summaries
        && latest_turn_out == latest_turn
        && js_ge(&thread_updated_at(thread), &next_completed);
    if unchanged {
        return Ok(thread.clone());
    }
    let mut o = jsv::spread(thread);
    let summaries_equal = crate::store::arrays_shallow_equal(
        &turn_diff_summaries,
        &Value::Array(next_turn_diff_summaries.clone()),
    ) && turn_diff_summaries.as_array().map(|a| a.len())
        == Some(next_turn_diff_summaries.len());
    o.insert(
        "turnDiffSummaries".into(),
        if summaries_equal {
            turn_diff_summaries
        } else {
            Value::Array(next_turn_diff_summaries)
        },
    );
    o.insert("latestTurn".into(), latest_turn_out);
    o.insert(
        "updatedAt".into(),
        if js_gt(&thread_updated_at(thread), &next_completed) {
            jsv::opt(thread, "updatedAt")
        } else {
            next_completed
        },
    );
    Ok(Value::Object(o))
}

/// Port of `mergeStreamingMessage` (storeEventReducer.ts:692).
fn merge_streaming_message(
    existing: &Value,
    incoming: &Value,
    segment_started_at: &Value,
    segment_sequence: &Value,
) -> Result<Value, String> {
    let existing_text = jsv::opt(existing, "text");
    let incoming_text = jsv::opt(incoming, "text");
    let incoming_streaming = jsv::truthy(&jsv::opt(incoming, "streaming"));
    let next_text: Value = if jsv::opt(existing, "role") == Value::String("user".into())
        && jsv::opt(incoming, "role") == Value::String("user".into())
        && !incoming_streaming
    {
        incoming_text.clone()
    } else if incoming_streaming || incoming_text.as_str().is_some_and(|t| t.is_empty()) {
        Value::String(format!(
            "{}{}",
            existing_text.as_str().unwrap_or(""),
            incoming_text.as_str().unwrap_or("")
        ))
    } else {
        incoming_text.clone()
    };
    let incoming_attachments = jsv::opt(incoming, "attachments");
    let next_attachments = coalesce_owned(incoming_attachments, jsv::opt(existing, "attachments"));
    let next_aui = crate::shared::merge_async_user_input(
        &jsv::opt(existing, "asyncUserInput"),
        &jsv::opt(incoming, "asyncUserInput"),
    )?;
    let incoming_skills = jsv::opt(incoming, "skills");
    let next_skills = if incoming_skills.as_array().is_some_and(|a| !a.is_empty()) {
        incoming_skills
    } else {
        jsv::opt(existing, "skills")
    };
    let incoming_mentions = jsv::opt(incoming, "mentions");
    let next_mentions = if incoming_mentions.as_array().is_some_and(|a| !a.is_empty()) {
        incoming_mentions
    } else {
        jsv::opt(existing, "mentions")
    };
    let next_completed_at = if incoming_streaming {
        jsv::opt(existing, "completedAt")
    } else {
        coalesce_owned(
            jsv::opt(incoming, "completedAt"),
            jsv::opt(existing, "completedAt"),
        )
    };
    let next_updated_at = coalesce_owned(
        jsv::opt(incoming, "updatedAt"),
        coalesce_owned(
            jsv::opt(existing, "updatedAt"),
            jsv::opt(incoming, "createdAt"),
        ),
    );
    let incoming_turn_id = jsv::opt(incoming, "turnId");
    let next_turn_id = if !is_undef(&incoming_turn_id) {
        incoming_turn_id
    } else {
        jsv::opt(existing, "turnId")
    };
    let incoming_dispatch_mode = jsv::opt(incoming, "dispatchMode");
    let next_dispatch_mode = if !is_undef(&incoming_dispatch_mode) {
        incoming_dispatch_mode
    } else {
        jsv::opt(existing, "dispatchMode")
    };
    let incoming_dispatch_origin = jsv::opt(incoming, "dispatchOrigin");
    let next_dispatch_origin = if !is_undef(&incoming_dispatch_origin) {
        incoming_dispatch_origin
    } else {
        jsv::opt(existing, "dispatchOrigin")
    };
    let incoming_starts = jsv::opt(incoming, "startsNewTurn");
    let next_starts_new_turn = if !is_undef(&incoming_starts) {
        incoming_starts
    } else {
        jsv::opt(existing, "startsNewTurn")
    };
    let next_source = coalesce_owned(jsv::opt(incoming, "source"), jsv::opt(existing, "source"));
    let advance_input = json!({
        "streaming": jsv::opt(incoming, "streaming"),
        "deltaText": incoming_text,
        "nextText": next_text,
        "segmentStartedAt": segment_started_at,
        "segmentSequence": segment_sequence,
        "updatedAt": next_updated_at,
    });
    let advanced_segments = crate::shared::advance_message_text_segments(
        &jsv::opt(existing, "textSegments"),
        &advance_input,
    )?;
    let existing_segments = jsv::opt(existing, "textSegments");
    let next_segments =
        if crate::store::text_segment_arrays_equal(&existing_segments, &advanced_segments) {
            existing_segments.clone()
        } else {
            advanced_segments
        };

    let unchanged = existing_text == next_text
        && existing_segments == next_segments
        && jsv::opt(existing, "asyncUserInput") == next_aui
        && jsv::opt(existing, "streaming") == jsv::opt(incoming, "streaming")
        && jsv::opt(existing, "attachments") == next_attachments
        && crate::store::provider_reference_arrays_equal(
            &jsv::opt(existing, "skills"),
            &next_skills,
        )
        && crate::store::provider_reference_arrays_equal(
            &jsv::opt(existing, "mentions"),
            &next_mentions,
        )
        && jsv::opt(existing, "completedAt") == next_completed_at
        && jsv::opt(existing, "updatedAt") == next_updated_at
        && jsv::opt(existing, "turnId") == next_turn_id
        && jsv::opt(existing, "dispatchMode") == next_dispatch_mode
        && jsv::opt(existing, "dispatchOrigin") == next_dispatch_origin
        && jsv::opt(existing, "startsNewTurn") == next_starts_new_turn
        && jsv::opt(existing, "source") == next_source;
    if unchanged {
        return Ok(Value::Null);
    }
    let mut o = jsv::spread(existing);
    o.shift_remove("textSegments");
    o.insert("text".into(), next_text);
    jsv::set_if_defined(&mut o, "textSegments", &next_segments);
    o.insert("updatedAt".into(), next_updated_at);
    if jsv::truthy(&next_aui) {
        o.insert("asyncUserInput".into(), next_aui);
    }
    o.insert("streaming".into(), jsv::opt(incoming, "streaming"));
    if jsv::truthy(&next_attachments) {
        o.insert("attachments".into(), next_attachments);
    }
    if next_skills.as_array().is_some_and(|a| !a.is_empty()) {
        o.insert("skills".into(), next_skills);
    }
    if next_mentions.as_array().is_some_and(|a| !a.is_empty()) {
        o.insert("mentions".into(), next_mentions);
    }
    jsv::set_if_defined(&mut o, "turnId", &next_turn_id);
    jsv::set_if_defined(&mut o, "dispatchMode", &next_dispatch_mode);
    jsv::set_if_defined(&mut o, "dispatchOrigin", &next_dispatch_origin);
    jsv::set_if_defined(&mut o, "startsNewTurn", &next_starts_new_turn);
    jsv::set_if_defined(&mut o, "source", &next_source);
    jsv::set_if_defined(&mut o, "completedAt", &next_completed_at);
    Ok(Value::Object(o))
}

/// Port of `applyThreadMessageSentEvent` (storeEventReducer.ts:816).
fn apply_thread_message_sent_event(thread: &Value, event: &Value) -> Result<Value, String> {
    let message_id = pf(event, "messageId")?;
    let role = pf(event, "role")?;
    let text = pf(event, "text")?;
    let streaming = pf(event, "streaming")?;
    let created_at = pf(event, "createdAt")?;
    let updated_at = pf(event, "updatedAt")?;
    let turn_id = pf(event, "turnId")?;
    let messages = jsv::opt(thread, "messages");
    let existing_index: i64 = messages
        .as_array()
        .and_then(|items| items.iter().rposition(|m| jsv::opt(m, "id") == message_id))
        .map(|i| i as i64)
        .unwrap_or(-1);
    let existing_message = if existing_index >= 0 {
        messages
            .as_array()
            .map(|a| a[existing_index as usize].clone())
    } else {
        None
    };
    let mut msg = Map::new();
    msg.insert("id".into(), message_id.clone());
    msg.insert("role".into(), role.clone());
    msg.insert("text".into(), text);
    let aui = pf(event, "asyncUserInput")?;
    if jsv::truthy(&aui) {
        msg.insert("asyncUserInput".into(), aui);
    }
    msg.insert("dispatchMode".into(), pf(event, "dispatchMode")?);
    msg.insert("dispatchOrigin".into(), pf(event, "dispatchOrigin")?);
    msg.insert("startsNewTurn".into(), pf(event, "startsNewTurn")?);
    msg.insert("turnId".into(), turn_id.clone());
    msg.insert(
        "attachments".into(),
        coalesce_owned(pf(event, "attachments")?, Value::Array(vec![])),
    );
    let skills = pf(event, "skills")?;
    jsv::set_if_defined(&mut msg, "skills", &skills);
    let mentions = pf(event, "mentions")?;
    jsv::set_if_defined(&mut msg, "mentions", &mentions);
    msg.insert("streaming".into(), streaming.clone());
    msg.insert("source".into(), pf(event, "source")?);
    msg.insert("createdAt".into(), created_at.clone());
    msg.insert("updatedAt".into(), updated_at.clone());
    let incoming_message =
        crate::store::normalize_chat_message(&Value::Object(msg), existing_message.as_ref())?;

    let mut messages_out = messages.clone();
    if let Some(existing) = &existing_message {
        let merged = merge_streaming_message(
            existing,
            &incoming_message,
            &pf(event, "segmentStartedAt")?,
            &coalesce_owned(pf(event, "segmentSequence")?, ef(event, "sequence")?),
        )?;
        if !merged.is_null() {
            let items = messages.as_array().cloned().unwrap_or_default();
            messages_out = Value::Array(array_with(&items, existing_index as usize, merged));
        }
    } else {
        let mut items = messages.as_array().cloned().unwrap_or_default();
        items.push(incoming_message.clone());
        messages_out = Value::Array(slice_last(items, crate::store::MAX_THREAD_MESSAGES));
    }

    let is_assistant_turn = role == Value::String("assistant".into()) && !turn_id.is_null();
    let turn_diff_summaries = if is_assistant_turn {
        Value::Array(rebind_turn_diff_summaries_for_assistant_message(
            &jsv::opt(thread, "turnDiffSummaries"),
            &turn_id,
            &message_id,
        )?)
    } else {
        jsv::opt(thread, "turnDiffSummaries")
    };

    let mut latest_turn = jsv::opt(thread, "latestTurn");
    let same_turn = latest_turn.is_null() || jsv::opt(&latest_turn, "turnId") == turn_id;
    if is_assistant_turn && same_turn {
        let session = jsv::opt(thread, "session");
        let turn_still_running = jsv::truthy(&streaming)
            || (crate::shared::is_session_running_turn(&session)
                && jsv::opt(&session, "activeTurnId") == turn_id);
        let prev_state = jsv::opt(&latest_turn, "state");
        let state = if turn_still_running {
            "running"
        } else if prev_state == Value::String("interrupted".into()) {
            "interrupted"
        } else if prev_state == Value::String("error".into()) {
            "error"
        } else {
            "completed"
        };
        let spp = jsv::opt(thread, "pendingSourceProposedPlan");
        latest_turn = build_latest_turn(
            &latest_turn,
            &turn_id,
            state,
            &coalesce_owned(jsv::opt(&latest_turn, "requestedAt"), created_at.clone()),
            &coalesce_owned(jsv::opt(&latest_turn, "startedAt"), created_at.clone()),
            &if turn_still_running {
                coalesce_owned(jsv::opt(&latest_turn, "completedAt"), Value::Null)
            } else {
                updated_at.clone()
            },
            &message_id,
            Some(&spp),
        );
    }

    let human_message_at = crate::shared::resolve_human_message_at(&incoming_message)?;
    let thread_lhm = jsv::opt(thread, "latestHumanMessageAt");
    let latest_human_message_at = if !human_message_at.is_null()
        && js_gt(
            &human_message_at,
            &coalesce_owned(thread_lhm.clone(), Value::String(String::new())),
        ) {
        human_message_at
    } else {
        thread_lhm
    };
    let updated_at_out = {
        let cur = jsv::opt(thread, "updatedAt");
        if jsv::truthy(&cur) && js_gt(&cur, &updated_at) {
            cur
        } else {
            updated_at.clone()
        }
    };
    if messages_out == messages
        && turn_diff_summaries == jsv::opt(thread, "turnDiffSummaries")
        && latest_turn == jsv::opt(thread, "latestTurn")
        && latest_human_message_at == jsv::opt(thread, "latestHumanMessageAt")
        && updated_at_out == jsv::opt(thread, "updatedAt")
    {
        return Ok(thread.clone());
    }
    let mut o = jsv::spread(thread);
    o.insert("messages".into(), messages_out);
    o.insert("turnDiffSummaries".into(), turn_diff_summaries);
    o.insert("latestTurn".into(), latest_turn);
    jsv::set_if_defined(&mut o, "latestHumanMessageAt", &latest_human_message_at);
    o.insert("updatedAt".into(), updated_at_out);
    Ok(Value::Object(o))
}

// ---------- the event switch ----------

/// Options normalized by `applyOrchestrationEventsHotPath`: only
/// `updateSidebarSummary` exists (a raw JS value, `?? false`-coalesced).
struct NormalizedOptions {
    update_sidebar_summary: Value,
}

impl NormalizedOptions {
    /// `{...options, updateSidebarSummary: X}` — callers passing their own value.
    fn sidebar_override(&self, v: bool) -> Value {
        Value::Bool(v)
    }

    /// `options?.updateSidebarSummary === true`.
    fn sidebar_is_true(&self) -> bool {
        self.update_sidebar_summary == Value::Bool(true)
    }
}

/// Port of `applyOrchestrationEvent` (storeEventReducer.ts:930).
fn apply_orchestration_event(
    state: Value,
    event: &Value,
    options: &NormalizedOptions,
) -> Result<Value, String> {
    let ty = jsv::member(event, "type", "event.type")?;
    let ty = ty.and_then(Value::as_str).unwrap_or("");
    match ty {
        "space.created" => {
            let mut space = Map::new();
            space.insert("id".into(), pf(event, "spaceId")?);
            space.insert("name".into(), pf(event, "name")?);
            space.insert("icon".into(), pf(event, "icon")?);
            space.insert("sortOrder".into(), pf(event, "sortOrder")?);
            space.insert("createdAt".into(), pf(event, "createdAt")?);
            space.insert("updatedAt".into(), pf(event, "updatedAt")?);
            crate::store::upsert_space(state, &Value::Object(space))
        }
        "space.meta-updated" => {
            let space_id = pf(event, "spaceId")?;
            let spaces = jsv::member(&state, "spaces", "state.spaces")?;
            let spaces = spaces.cloned().unwrap_or_else(js::undefined);
            let existing = find(&spaces, "state.spaces", |s| {
                Ok(jsv::opt(s, "id") == space_id)
            })?;
            let Some(existing) = existing else {
                return Ok(state);
            };
            let mut space = jsv::spread(existing);
            space.insert(
                "name".into(),
                coalesce_owned(pf(event, "name")?, jsv::opt(existing, "name")),
            );
            space.insert(
                "icon".into(),
                coalesce_owned(pf(event, "icon")?, jsv::opt(existing, "icon")),
            );
            space.insert("updatedAt".into(), pf(event, "updatedAt")?);
            crate::store::upsert_space(state, &Value::Object(space))
        }
        "space.order-updated" => crate::store::apply_space_order(
            state,
            &pf(event, "orderedSpaceIds")?,
            &pf(event, "updatedAt")?,
        ),
        "space.deleted" => {
            crate::store::remove_space(state, &pf(event, "spaceId")?, &pf(event, "deletedAt")?)
        }
        "project.created" => {
            let mut project = Map::new();
            project.insert("id".into(), pf(event, "projectId")?);
            project.insert("kind".into(), pf(event, "kind")?);
            project.insert("title".into(), pf(event, "title")?);
            project.insert("workspaceRoot".into(), pf(event, "workspaceRoot")?);
            project.insert(
                "defaultModelSelection".into(),
                pf(event, "defaultModelSelection")?,
            );
            project.insert("scripts".into(), pf(event, "scripts")?);
            project.insert(
                "isPinned".into(),
                coalesce_owned(pf(event, "isPinned")?, Value::Bool(false)),
            );
            project.insert(
                "spaceId".into(),
                coalesce_owned(pf(event, "spaceId")?, Value::Null),
            );
            project.insert(
                "additionalFolders".into(),
                coalesce_owned(pf(event, "additionalFolders")?, Value::Array(vec![])),
            );
            project.insert("createdAt".into(), pf(event, "createdAt")?);
            project.insert("updatedAt".into(), pf(event, "updatedAt")?);
            crate::store::upsert_project(state, &Value::Object(project), "id-only")
        }
        "project.meta-updated" => {
            let project_id = pf(event, "projectId")?;
            let projects = jsv::member(&state, "projects", "state.projects")?;
            let projects = projects.cloned().unwrap_or_else(js::undefined);
            let existing = find(&projects, "state.projects", |p| {
                Ok(jsv::opt(p, "id") == project_id)
            })?;
            let Some(existing) = existing else {
                return Ok(state);
            };
            let mut project = Map::new();
            project.insert("id".into(), jsv::opt(existing, "id"));
            project.insert(
                "kind".into(),
                coalesce_owned(pf(event, "kind")?, jsv::opt(existing, "kind")),
            );
            project.insert(
                "title".into(),
                coalesce_owned(pf(event, "title")?, jsv::opt(existing, "remoteName")),
            );
            project.insert(
                "workspaceRoot".into(),
                coalesce_owned(pf(event, "workspaceRoot")?, jsv::opt(existing, "cwd")),
            );
            let dms = pf(event, "defaultModelSelection")?;
            project.insert(
                "defaultModelSelection".into(),
                if !is_undef(&dms) {
                    dms
                } else {
                    jsv::opt(existing, "defaultModelSelection")
                },
            );
            project.insert(
                "scripts".into(),
                coalesce_owned(pf(event, "scripts")?, jsv::opt(existing, "scripts")),
            );
            project.insert(
                "isPinned".into(),
                coalesce_owned(
                    pf(event, "isPinned")?,
                    coalesce_owned(jsv::opt(existing, "isPinned"), Value::Bool(false)),
                ),
            );
            let sid = pf(event, "spaceId")?;
            project.insert(
                "spaceId".into(),
                if !is_undef(&sid) {
                    sid
                } else {
                    coalesce_owned(jsv::opt(existing, "spaceId"), Value::Null)
                },
            );
            project.insert(
                "additionalFolders".into(),
                coalesce_owned(
                    pf(event, "additionalFolders")?,
                    jsv::opt(existing, "additionalFolders"),
                ),
            );
            project.insert(
                "createdAt".into(),
                coalesce_owned(jsv::opt(existing, "createdAt"), pf(event, "updatedAt")?),
            );
            project.insert("updatedAt".into(), pf(event, "updatedAt")?);
            crate::store::upsert_project(state, &Value::Object(project), "id-only")
        }
        "project.deleted" => crate::store::remove_deleted_project_from_client_state(
            state,
            &pf(event, "projectId")?,
            &ef(event, "sequence")?,
        ),
        "thread.deleted" => crate::store::remove_deleted_thread_from_client_state(
            state,
            &pf(event, "threadId")?,
            &ef(event, "sequence")?,
        ),
        "thread.meta-updated" => {
            let thread_id = pf(event, "threadId")?;
            let p_model = pf(event, "modelSelection")?;
            let p_branch = pf(event, "branch")?;
            let p_worktree = pf(event, "worktreePath")?;
            let p_workdir = pf(event, "workingDirectory")?;
            let p_awp = pf(event, "associatedWorktreePath")?;
            let p_awb = pf(event, "associatedWorktreeBranch")?;
            let p_awr = pf(event, "associatedWorktreeRef")?;
            let p_cbfc = pf(event, "createBranchFlowCompleted")?;
            let p_title = pf(event, "title")?;
            let p_env = pf(event, "envMode")?;
            let p_pin = pf(event, "isPinned")?;
            let p_settled = pf(event, "settledAt")?;
            let p_ulr = pf(event, "usageLimitRecovery")?;
            let p_snoozed = pf(event, "snoozedUntil")?;
            let p_reminder = pf(event, "snoozeReminderAt")?;
            let p_parent = pf(event, "parentThreadId")?;
            let p_agent = pf(event, "subagentAgentId")?;
            let p_nick = pf(event, "subagentNickname")?;
            let p_role = pf(event, "subagentRole")?;
            let p_pr = pf(event, "lastKnownPr")?;
            let p_handoff = pf(event, "handoff")?;
            let p_pins = pf(event, "pinnedMessages")?;
            let p_notes = pf(event, "notes")?;
            let p_goal = pf(event, "goal")?;
            let p_goal_started = pf(event, "goalStartedAt")?;
            let p_goal_paused = pf(event, "goalPausedAt")?;
            let p_goal_ach = pf(event, "goalAchievements")?;
            let p_updated = pf(event, "updatedAt")?;
            let payload = jsv::opt(event, "payload");
            let sequence = ef(event, "sequence")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let pull_requests =
                        crate::shared::pull_request_links_from_event_payload(&payload, &p_updated)?;
                    let model_selection = if !is_undef(&p_model) {
                        crate::store::normalize_model_selection(
                            &p_model,
                            &jsv::opt(&thread, "modelSelection"),
                        )?
                    } else {
                        jsv::opt(&thread, "modelSelection")
                    };
                    let next_branch = if !is_undef(&p_branch) {
                        crate::shared::resolve_thread_branch_regression_guard(&json!({
                            "currentBranch": jsv::opt(&thread, "branch"),
                            "nextBranch": p_branch,
                        }))?
                    } else {
                        jsv::opt(&thread, "branch")
                    };
                    let next_worktree = if !is_undef(&p_worktree) {
                        p_worktree.clone()
                    } else {
                        jsv::opt(&thread, "worktreePath")
                    };
                    let next_workdir = if !is_undef(&p_workdir) {
                        p_workdir.clone()
                    } else {
                        coalesce_owned(jsv::opt(&thread, "workingDirectory"), Value::Null)
                    };
                    let next_awp = if !is_undef(&p_awp) {
                        p_awp.clone()
                    } else {
                        coalesce_owned(jsv::opt(&thread, "associatedWorktreePath"), Value::Null)
                    };
                    let next_awb = if !is_undef(&p_awb) {
                        p_awb.clone()
                    } else {
                        coalesce_owned(jsv::opt(&thread, "associatedWorktreeBranch"), Value::Null)
                    };
                    let next_awr = if !is_undef(&p_awr) {
                        p_awr.clone()
                    } else {
                        coalesce_owned(jsv::opt(&thread, "associatedWorktreeRef"), Value::Null)
                    };
                    let next_cbfc = crate::store::resolve_create_branch_flow_completed_merge(
                        &json!({
                            "currentBranch": jsv::opt(&thread, "branch"),
                            "nextBranch": next_branch,
                            "currentWorktreePath": jsv::opt(&thread, "worktreePath"),
                            "nextWorktreePath": next_worktree,
                            "currentAssociatedWorktreePath": jsv::opt(&thread, "associatedWorktreePath"),
                            "nextAssociatedWorktreePath": next_awp,
                            "currentAssociatedWorktreeBranch": jsv::opt(&thread, "associatedWorktreeBranch"),
                            "nextAssociatedWorktreeBranch": next_awb,
                            "currentAssociatedWorktreeRef": jsv::opt(&thread, "associatedWorktreeRef"),
                            "nextAssociatedWorktreeRef": next_awr,
                            "currentCreateBranchFlowCompleted": jsv::opt(&thread, "createBranchFlowCompleted"),
                            "nextCreateBranchFlowCompleted": p_cbfc,
                        }),
                    )?;
                    let next_updated_at = if js_gt(&thread_updated_at(&thread), &p_updated) {
                        jsv::opt(&thread, "updatedAt")
                    } else {
                        p_updated.clone()
                    };
                    let cwd_changed = jsv::opt(&thread, "worktreePath") != next_worktree
                        || coalesce_owned(jsv::opt(&thread, "workingDirectory"), Value::Null)
                            != next_workdir;
                    let usage_ok = is_undef(&p_ulr)
                        || js_lt(
                            &sequence,
                            &coalesce_owned(
                                jsv::opt(&thread, "usageLimitRecoverySequence"),
                                js::number(0.0),
                            ),
                        )
                        || (crate::store::deep_equal_json(
                            &p_ulr,
                            &coalesce_owned(jsv::opt(&thread, "usageLimitRecovery"), Value::Null),
                        ) && jsv::opt(&thread, "usageLimitRecoverySequence") == sequence);
                    let snooze_seq_ok = (is_undef(&p_snoozed) && is_undef(&p_reminder))
                        || jsv::opt(&thread, "snoozeSequence") == sequence;
                    let unchanged = (is_undef(&p_title) || p_title == jsv::opt(&thread, "title"))
                        && model_selection == jsv::opt(&thread, "modelSelection")
                        && (is_undef(&p_env) || p_env == jsv::opt(&thread, "envMode"))
                        && next_branch == jsv::opt(&thread, "branch")
                        && next_worktree == jsv::opt(&thread, "worktreePath")
                        && next_workdir
                            == coalesce_owned(jsv::opt(&thread, "workingDirectory"), Value::Null)
                        && next_awp
                            == coalesce_owned(
                                jsv::opt(&thread, "associatedWorktreePath"),
                                Value::Null,
                            )
                        && next_awb
                            == coalesce_owned(
                                jsv::opt(&thread, "associatedWorktreeBranch"),
                                Value::Null,
                            )
                        && next_awr
                            == coalesce_owned(
                                jsv::opt(&thread, "associatedWorktreeRef"),
                                Value::Null,
                            )
                        && next_cbfc
                            == coalesce_owned(
                                jsv::opt(&thread, "createBranchFlowCompleted"),
                                Value::Bool(false),
                            )
                        && (is_undef(&p_pin)
                            || p_pin
                                == coalesce_owned(
                                    jsv::opt(&thread, "isPinned"),
                                    Value::Bool(false),
                                ))
                        && (is_undef(&p_settled)
                            || coalesce_owned(p_settled.clone(), Value::Null)
                                == coalesce_owned(jsv::opt(&thread, "settledAt"), Value::Null))
                        && usage_ok
                        && (is_undef(&p_snoozed)
                            || coalesce_owned(p_snoozed.clone(), Value::Null)
                                == coalesce_owned(jsv::opt(&thread, "snoozedUntil"), Value::Null))
                        && (is_undef(&p_reminder)
                            || coalesce_owned(p_reminder.clone(), Value::Null)
                                == coalesce_owned(
                                    jsv::opt(&thread, "snoozeReminderAt"),
                                    Value::Null,
                                ))
                        && snooze_seq_ok
                        && (is_undef(&p_parent)
                            || coalesce_owned(p_parent.clone(), Value::Null)
                                == coalesce_owned(
                                    jsv::opt(&thread, "parentThreadId"),
                                    Value::Null,
                                ))
                        && (is_undef(&p_agent)
                            || coalesce_owned(p_agent.clone(), Value::Null)
                                == coalesce_owned(
                                    jsv::opt(&thread, "subagentAgentId"),
                                    Value::Null,
                                ))
                        && (is_undef(&p_nick)
                            || coalesce_owned(p_nick.clone(), Value::Null)
                                == coalesce_owned(
                                    jsv::opt(&thread, "subagentNickname"),
                                    Value::Null,
                                ))
                        && (is_undef(&p_role)
                            || coalesce_owned(p_role.clone(), Value::Null)
                                == coalesce_owned(jsv::opt(&thread, "subagentRole"), Value::Null))
                        && (is_undef(&p_pr)
                            || crate::store::deep_equal_json(
                                &coalesce_owned(p_pr.clone(), Value::Null),
                                &coalesce_owned(jsv::opt(&thread, "lastKnownPr"), Value::Null),
                            ))
                        && (is_undef(&pull_requests)
                            || crate::store::deep_equal_json(
                                &pull_requests,
                                &coalesce_owned(
                                    jsv::opt(&thread, "pullRequests"),
                                    Value::Array(vec![]),
                                ),
                            ))
                        && (is_undef(&p_handoff)
                            || coalesce_owned(p_handoff.clone(), Value::Null)
                                == coalesce_owned(jsv::opt(&thread, "handoff"), Value::Null))
                        && (is_undef(&p_pins)
                            || crate::store::deep_equal_json(
                                &p_pins,
                                &coalesce_owned(jsv::opt(&thread, "pinnedMessages"), Value::Null),
                            ))
                        && (is_undef(&p_notes)
                            || p_notes
                                == coalesce_owned(
                                    jsv::opt(&thread, "notes"),
                                    Value::String(String::new()),
                                ))
                        && (is_undef(&p_goal)
                            || p_goal
                                == coalesce_owned(
                                    jsv::opt(&thread, "goal"),
                                    Value::String(String::new()),
                                ))
                        && (is_undef(&p_goal_started)
                            || coalesce_owned(p_goal_started.clone(), Value::Null)
                                == coalesce_owned(jsv::opt(&thread, "goalStartedAt"), Value::Null))
                        && (is_undef(&p_goal_paused)
                            || coalesce_owned(p_goal_paused.clone(), Value::Null)
                                == coalesce_owned(jsv::opt(&thread, "goalPausedAt"), Value::Null))
                        && (is_undef(&p_goal_ach)
                            || crate::store::deep_equal_json(
                                &p_goal_ach,
                                &coalesce_owned(jsv::opt(&thread, "goalAchievements"), Value::Null),
                            ))
                        && next_updated_at == jsv::opt(&thread, "updatedAt");
                    if unchanged {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    jsv::set_if_defined(&mut o, "title", &p_title);
                    o.insert("modelSelection".into(), model_selection);
                    jsv::set_if_defined(&mut o, "envMode", &p_env);
                    o.insert("branch".into(), next_branch);
                    o.insert("worktreePath".into(), next_worktree);
                    o.insert("workingDirectory".into(), next_workdir);
                    o.insert("associatedWorktreePath".into(), next_awp);
                    o.insert("associatedWorktreeBranch".into(), next_awb);
                    o.insert("associatedWorktreeRef".into(), next_awr);
                    o.insert("createBranchFlowCompleted".into(), next_cbfc);
                    jsv::set_if_defined(&mut o, "isPinned", &p_pin);
                    jsv::set_if_defined(&mut o, "settledAt", &p_settled);
                    if !is_undef(&p_ulr)
                        && js_ge(
                            &sequence,
                            &coalesce_owned(
                                jsv::opt(&thread, "usageLimitRecoverySequence"),
                                js::number(0.0),
                            ),
                        )
                    {
                        let ulr = if crate::store::deep_equal_json(
                            &p_ulr,
                            &coalesce_owned(jsv::opt(&thread, "usageLimitRecovery"), Value::Null),
                        ) {
                            coalesce_owned(jsv::opt(&thread, "usageLimitRecovery"), Value::Null)
                        } else {
                            p_ulr.clone()
                        };
                        o.insert("usageLimitRecovery".into(), ulr);
                        o.insert("usageLimitRecoverySequence".into(), sequence.clone());
                    }
                    jsv::set_if_defined(&mut o, "snoozedUntil", &p_snoozed);
                    jsv::set_if_defined(&mut o, "snoozeReminderAt", &p_reminder);
                    if !is_undef(&p_snoozed) || !is_undef(&p_reminder) {
                        o.insert("snoozeSequence".into(), sequence.clone());
                    }
                    jsv::set_if_defined(&mut o, "parentThreadId", &p_parent);
                    jsv::set_if_defined(&mut o, "subagentAgentId", &p_agent);
                    jsv::set_if_defined(&mut o, "subagentNickname", &p_nick);
                    jsv::set_if_defined(&mut o, "subagentRole", &p_role);
                    jsv::set_if_defined(&mut o, "lastKnownPr", &p_pr);
                    jsv::set_if_defined(&mut o, "pullRequests", &pull_requests);
                    jsv::set_if_defined(&mut o, "handoff", &p_handoff);
                    jsv::set_if_defined(&mut o, "pinnedMessages", &p_pins);
                    jsv::set_if_defined(&mut o, "notes", &p_notes);
                    jsv::set_if_defined(&mut o, "goal", &p_goal);
                    jsv::set_if_defined(&mut o, "goalStartedAt", &p_goal_started);
                    jsv::set_if_defined(&mut o, "goalPausedAt", &p_goal_paused);
                    jsv::set_if_defined(&mut o, "goalAchievements", &p_goal_ach);
                    o.insert("updatedAt".into(), next_updated_at);
                    if cwd_changed {
                        o.insert("session".into(), Value::Null);
                    }
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.pinned-message-added" => {
            let thread_id = pf(event, "threadId")?;
            let pin = pf(event, "pin")?;
            let p_updated = pf(event, "updatedAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let pinned = crate::shared::add_pinned_message(
                        &jsv::opt(&thread, "pinnedMessages"),
                        &pin,
                    )?;
                    let updated_at = resolve_event_updated_at(&thread, &p_updated);
                    if jsv::opt(&thread, "pinnedMessages") == pinned
                        && jsv::opt(&thread, "updatedAt") == updated_at
                    {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("pinnedMessages".into(), pinned);
                    o.insert("updatedAt".into(), updated_at);
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(false),
            )
        }
        "thread.pinned-message-removed" => {
            let thread_id = pf(event, "threadId")?;
            let mid = pf(event, "messageId")?;
            let p_updated = pf(event, "updatedAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let pinned = crate::shared::remove_pinned_message(
                        &jsv::opt(&thread, "pinnedMessages"),
                        &mid,
                    )?;
                    let updated_at = resolve_event_updated_at(&thread, &p_updated);
                    if jsv::opt(&thread, "pinnedMessages") == pinned
                        && jsv::opt(&thread, "updatedAt") == updated_at
                    {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("pinnedMessages".into(), pinned);
                    o.insert("updatedAt".into(), updated_at);
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(false),
            )
        }
        "thread.pinned-message-done-set" => {
            let thread_id = pf(event, "threadId")?;
            let mid = pf(event, "messageId")?;
            let done = pf(event, "done")?;
            let p_updated = pf(event, "updatedAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let pinned = crate::shared::set_pinned_message_done(
                        &jsv::opt(&thread, "pinnedMessages"),
                        &mid,
                        &done,
                    )?;
                    let updated_at = resolve_event_updated_at(&thread, &p_updated);
                    if jsv::opt(&thread, "pinnedMessages") == pinned
                        && jsv::opt(&thread, "updatedAt") == updated_at
                    {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("pinnedMessages".into(), pinned);
                    o.insert("updatedAt".into(), updated_at);
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(false),
            )
        }
        "thread.pinned-message-label-set" => {
            let thread_id = pf(event, "threadId")?;
            let mid = pf(event, "messageId")?;
            let label = pf(event, "label")?;
            let p_updated = pf(event, "updatedAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let pinned = crate::shared::set_pinned_message_label(
                        &jsv::opt(&thread, "pinnedMessages"),
                        &mid,
                        &label,
                    )?;
                    let updated_at = resolve_event_updated_at(&thread, &p_updated);
                    if jsv::opt(&thread, "pinnedMessages") == pinned
                        && jsv::opt(&thread, "updatedAt") == updated_at
                    {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("pinnedMessages".into(), pinned);
                    o.insert("updatedAt".into(), updated_at);
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(false),
            )
        }
        "thread.async-user-input-answered" => {
            let thread_id = pf(event, "threadId")?;
            let mid = pf(event, "messageId")?;
            let response = pf(event, "response")?;
            let sequence = ef(event, "sequence")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let messages = jsv::opt(&thread, "messages");
                    let next = map_array(&messages, "thread.messages", |message| {
                        let aui = jsv::opt(message, "asyncUserInput");
                        if jsv::opt(message, "id") == mid && jsv::truthy(&aui) {
                            let mut mo = jsv::spread(message);
                            let mut aui_o = jsv::spread(&aui);
                            aui_o.insert("response".into(), response.clone());
                            aui_o.insert("responseSequence".into(), sequence.clone());
                            mo.insert(
                                "asyncUserInput".into(),
                                crate::shared::merge_async_user_input(&aui, &Value::Object(aui_o))?,
                            );
                            Ok(Value::Object(mo))
                        } else {
                            Ok(message.clone())
                        }
                    })?;
                    let mut o = jsv::spread(&thread);
                    o.insert("messages".into(), Value::Array(next));
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(false),
            )
        }
        "thread.message-sent" => {
            let thread_id = pf(event, "threadId")?;
            let recompute = thread_message_updates_summary(event);
            let sidebar =
                options.sidebar_is_true() || thread_message_updates_sidebar_summary(event);
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| apply_thread_message_sent_event(&thread, event),
                recompute,
                &options.sidebar_override(sidebar),
            )
        }
        "thread.claude-cache-set" => {
            let thread_id = pf(event, "threadId")?;
            let review = pf(event, "review")?;
            let p_updated = pf(event, "updatedAt")?;
            let sequence = ef(event, "sequence")?;
            let shell_seq = jsv::opt(&state, "shellSnapshotSequence");
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let cache_seq = jsv::opt(&thread, "claudeCacheReviewSequence");
                    let max_seq = jsv::js_max(
                        jsv::to_number(&cache_seq).unwrap_or(0.0),
                        jsv::to_number(&shell_seq).unwrap_or(0.0),
                    );
                    if js_le(&sequence, &js::number(max_seq)) {
                        return Ok(thread);
                    }
                    let updated_at = resolve_event_updated_at(&thread, &p_updated);
                    let mut o = jsv::spread(&thread);
                    let review_val = if crate::store::deep_equal_json(
                        &coalesce_owned(jsv::opt(&thread, "claudeCacheReview"), Value::Null),
                        &review,
                    ) {
                        coalesce_owned(jsv::opt(&thread, "claudeCacheReview"), Value::Null)
                    } else {
                        review.clone()
                    };
                    o.insert("claudeCacheReview".into(), review_val);
                    o.insert("claudeCacheReviewSequence".into(), sequence.clone());
                    if jsv::opt(&review, "status") == Value::String("compacting".into()) {
                        o.insert("pendingTurnStartMessageId".into(), Value::Null);
                    }
                    o.insert("updatedAt".into(), updated_at);
                    Ok(Value::Object(o))
                },
                true,
                &options.update_sidebar_summary,
            )
        }
        "thread.claude-cache-response-requested" => {
            if pf(event, "decision")? == Value::String("compact".into()) {
                return Ok(state);
            }
            let thread_id = pf(event, "threadId")?;
            let decision = pf(event, "decision")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let pending_id = if decision == Value::String("continue".into()) {
                        let review = pf(event, "review")?;
                        jsv::member(&review, "messageId", "event.payload.review.messageId")?
                            .cloned()
                            .unwrap_or_else(js::undefined)
                    } else {
                        Value::Null
                    };
                    if jsv::opt(&thread, "pendingTurnStartMessageId") == pending_id {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("pendingTurnStartMessageId".into(), pending_id);
                    Ok(Value::Object(o))
                },
                true,
                &options.update_sidebar_summary,
            )
        }
        "thread.session-set" => {
            let thread_id = pf(event, "threadId")?;
            let session_payload = pf(event, "session")?;
            let occurred = ef(event, "occurredAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let session = crate::store::normalize_thread_session(
                        &session_payload,
                        &jsv::opt(&thread, "session"),
                    )?;
                    let error = crate::store::normalize_thread_error_message(&jsv::opt(
                        &session_payload,
                        "lastError",
                    ))?;
                    let latest_turn =
                        reconcile_latest_turn_from_session(&thread, &session_payload, &error)?;
                    let bound = bind_pending_turn_start_message(&thread, &session_payload)?;
                    let sidechat = crate::shared::is_sidechat_thread(&thread)?;
                    let sess_updated = jsv::opt(&session_payload, "updatedAt");
                    if bound.is_null()
                        && session == jsv::opt(&thread, "session")
                        && error == jsv::opt(&thread, "error")
                        && latest_turn == jsv::opt(&thread, "latestTurn")
                        && (!sidechat
                            || jsv::truthy(&jsv::opt(&thread, "sidechatExpiredAt"))
                            || jsv::opt(&thread, "sidechatLastActivityAt") == sess_updated)
                    {
                        return Ok(thread);
                    }
                    let base = if bound.is_null() {
                        thread.clone()
                    } else {
                        bound
                    };
                    let mut o = jsv::spread(&base);
                    o.insert("session".into(), session);
                    o.insert("error".into(), error);
                    o.insert("latestTurn".into(), latest_turn);
                    if sidechat && !jsv::truthy(&jsv::opt(&thread, "sidechatExpiredAt")) {
                        o.insert("sidechatLastActivityAt".into(), sess_updated);
                    }
                    o.insert(
                        "updatedAt".into(),
                        if js_gt(&thread_updated_at(&thread), &occurred) {
                            jsv::opt(&thread, "updatedAt")
                        } else {
                            occurred.clone()
                        },
                    );
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.sidechat-activity-recorded" => {
            let thread_id = pf(event, "threadId")?;
            let last_at = pf(event, "lastActivityAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let updated_at = resolve_event_updated_at(&thread, &last_at);
                    if jsv::opt(&thread, "sidechatLastActivityAt") == last_at
                        && jsv::opt(&thread, "updatedAt") == updated_at
                    {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("sidechatLastActivityAt".into(), last_at.clone());
                    o.insert("updatedAt".into(), updated_at);
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.sidechat-expired" => {
            let thread_id = pf(event, "threadId")?;
            let expired = pf(event, "expiredAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let updated_at = resolve_event_updated_at(&thread, &expired);
                    if jsv::opt(&thread, "sidechatExpiredAt") == expired
                        && jsv::opt(&thread, "updatedAt") == updated_at
                    {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("sidechatExpiredAt".into(), expired.clone());
                    o.insert("updatedAt".into(), updated_at);
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.turn-interrupt-requested" => {
            let thread_id = pf(event, "threadId")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let session = jsv::opt(&thread, "session");
                    let active_turn =
                        coalesce_owned(jsv::opt(&session, "activeTurnId"), Value::Null);
                    let activity = crate::shared::derive_turn_stop_activity(event, &active_turn)?;
                    if !jsv::truthy(&activity) {
                        return Ok(thread);
                    }
                    let mut items = jsv::opt(&thread, "activities")
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    items.push(activity);
                    let activities = crate::store::normalize_activities(
                        &Value::Array(items),
                        &jsv::opt(&thread, "activities"),
                    )?;
                    if activities == jsv::opt(&thread, "activities") {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("activities".into(), activities);
                    Ok(Value::Object(o))
                },
                true,
                &options.update_sidebar_summary,
            )
        }
        "thread.session-stop-requested" => {
            let thread_id = pf(event, "threadId")?;
            let created = pf(event, "createdAt")?;
            let occurred = ef(event, "occurredAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let session = jsv::opt(&thread, "session");
                    if session.is_null() {
                        if jsv::opt(&thread, "pendingTurnStartMessageId").is_null() {
                            return Ok(thread);
                        }
                        let mut o = jsv::spread(&thread);
                        o.insert("pendingTurnStartMessageId".into(), Value::Null);
                        return Ok(Value::Object(o));
                    }
                    let latest_turn = jsv::opt(&thread, "latestTurn");
                    let latest_turn_out = if !latest_turn.is_null()
                        && jsv::opt(&latest_turn, "state") == Value::String("running".into())
                        && jsv::opt(&latest_turn, "completedAt").is_null()
                    {
                        build_latest_turn(
                            &latest_turn,
                            &jsv::opt(&latest_turn, "turnId"),
                            "interrupted",
                            &jsv::opt(&latest_turn, "requestedAt"),
                            &coalesce_owned(jsv::opt(&latest_turn, "startedAt"), created.clone()),
                            &created,
                            &jsv::opt(&latest_turn, "assistantMessageId"),
                            None,
                        )
                    } else {
                        latest_turn
                    };
                    let mut o = jsv::spread(&thread);
                    o.insert("pendingTurnStartMessageId".into(), Value::Null);
                    let mut so = jsv::spread(&session);
                    so.insert("status".into(), Value::String("closed".into()));
                    so.insert(
                        "orchestrationStatus".into(),
                        Value::String("stopped".into()),
                    );
                    so.insert("activeTurnId".into(), js::undefined());
                    so.insert("updatedAt".into(), created.clone());
                    o.insert("session".into(), Value::Object(so));
                    o.insert("latestTurn".into(), latest_turn_out);
                    o.insert(
                        "updatedAt".into(),
                        if js_gt(&thread_updated_at(&thread), &occurred) {
                            jsv::opt(&thread, "updatedAt")
                        } else {
                            occurred.clone()
                        },
                    );
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.turn-start-requested" => {
            let thread_id = pf(event, "threadId")?;
            let p_model = pf(event, "modelSelection")?;
            let dispatch_origin = pf(event, "dispatchOrigin")?;
            let p_runtime = pf(event, "runtimeMode")?;
            let p_interaction = pf(event, "interactionMode")?;
            let p_spp = pf(event, "sourceProposedPlan")?;
            let p_msg = pf(event, "messageId")?;
            let p_created = pf(event, "createdAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let model_selection = if !is_undef(&p_model) {
                        crate::store::normalize_model_selection(
                            &p_model,
                            &jsv::opt(&thread, "modelSelection"),
                        )?
                    } else {
                        jsv::opt(&thread, "modelSelection")
                    };
                    let adopt = dispatch_origin != Value::String("automation".into());
                    let runtime_mode = if adopt {
                        p_runtime.clone()
                    } else {
                        jsv::opt(&thread, "runtimeMode")
                    };
                    let interaction_mode = if adopt {
                        p_interaction.clone()
                    } else {
                        jsv::opt(&thread, "interactionMode")
                    };
                    let sidechat = crate::shared::is_sidechat_thread(&thread)?;
                    if model_selection == jsv::opt(&thread, "modelSelection")
                        && jsv::opt(&thread, "runtimeMode") == runtime_mode
                        && jsv::opt(&thread, "interactionMode") == interaction_mode
                        && jsv::opt(&thread, "pendingSourceProposedPlan") == p_spp
                        && jsv::opt(&thread, "pendingTurnStartMessageId") == p_msg
                        && (!sidechat || jsv::opt(&thread, "sidechatLastActivityAt") == p_created)
                        && js_ge(&thread_updated_at(&thread), &p_created)
                    {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("modelSelection".into(), model_selection);
                    o.insert("runtimeMode".into(), runtime_mode);
                    o.insert("interactionMode".into(), interaction_mode);
                    o.insert("pendingSourceProposedPlan".into(), p_spp.clone());
                    o.insert("pendingTurnStartMessageId".into(), p_msg.clone());
                    if sidechat {
                        o.insert("sidechatLastActivityAt".into(), p_created.clone());
                    }
                    o.insert(
                        "updatedAt".into(),
                        if js_gt(&thread_updated_at(&thread), &p_created) {
                            jsv::opt(&thread, "updatedAt")
                        } else {
                            p_created.clone()
                        },
                    );
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.user-input-response-requested" | "thread.approval-response-requested" => {
            let thread_id = pf(event, "threadId")?;
            let p_created = pf(event, "createdAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let pending = mark_interaction_responding(&thread, event)?;
                    let mut o = jsv::spread(&thread);
                    jsv::set_if_defined(&mut o, "pendingInteractions", &pending);
                    o.insert(
                        "updatedAt".into(),
                        if js_gt(&thread_updated_at(&thread), &p_created) {
                            jsv::opt(&thread, "updatedAt")
                        } else {
                            p_created.clone()
                        },
                    );
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.activity-appended" => {
            let thread_id = pf(event, "threadId")?;
            let sequence = ef(event, "sequence")?;
            let payload_activity = pf(event, "activity")?;
            let recompute = thread_activity_updates_summary(event);
            let sidebar =
                options.sidebar_is_true() || thread_activity_updates_sidebar_summary(event);
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let sequenced = crate::store::with_orchestration_event_sequence(
                        &payload_activity,
                        &sequence,
                    )?;
                    let mut items = jsv::opt(&thread, "activities")
                        .as_array()
                        .cloned()
                        .unwrap_or_default();
                    items.push(sequenced.clone());
                    let next_activities = crate::store::normalize_activities(
                        &Value::Array(items),
                        &jsv::opt(&thread, "activities"),
                    )?;
                    let pending = reconcile_pending_interactions_from_activity(
                        &jsv::opt(&thread, "id"),
                        &jsv::opt(&thread, "pendingInteractions"),
                        event,
                    )?;
                    if next_activities == jsv::opt(&thread, "activities")
                        && pending == jsv::opt(&thread, "pendingInteractions")
                    {
                        return Ok(thread);
                    }
                    let mut o = jsv::spread(&thread);
                    o.insert("activities".into(), next_activities);
                    jsv::set_if_defined(&mut o, "pendingInteractions", &pending);
                    o.insert(
                        "updatedAt".into(),
                        if js_gt(
                            &thread_updated_at(&thread),
                            &jsv::opt(&sequenced, "createdAt"),
                        ) {
                            jsv::opt(&thread, "updatedAt")
                        } else {
                            jsv::opt(&sequenced, "createdAt")
                        },
                    );
                    Ok(Value::Object(o))
                },
                recompute,
                &options.sidebar_override(sidebar),
            )
        }
        "thread.proposed-plan-upserted" => {
            let thread_id = pf(event, "threadId")?;
            let plan = pf(event, "proposedPlan")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let plans = jsv::opt(&thread, "proposedPlans");
                    let plan_id = jsv::opt(&plan, "id");
                    let prev_index = find_index(&plans, "thread.proposedPlans", |p| {
                        Ok(jsv::opt(p, "id") == plan_id)
                    })?;
                    let prev_list = if prev_index >= 0 {
                        Value::Array(vec![
                            plans
                                .as_array()
                                .map(|a| a[prev_index as usize].clone())
                                .unwrap_or(Value::Null),
                        ])
                    } else {
                        js::undefined()
                    };
                    let next_plan = crate::store::normalize_proposed_plans(
                        &Value::Array(vec![plan.clone()]),
                        Some(&prev_list),
                    )?;
                    let next_plan = next_plan.as_array().and_then(|a| a.first()).cloned();
                    let Some(next_plan) = next_plan else {
                        return Ok(thread);
                    };
                    let mut next_plans = plans.as_array().cloned().unwrap_or_default();
                    if prev_index >= 0 {
                        next_plans[prev_index as usize] = next_plan;
                    } else {
                        next_plans.push(next_plan);
                    }
                    if crate::store::arrays_shallow_equal(&plans, &Value::Array(next_plans.clone()))
                    {
                        return Ok(thread);
                    }
                    let plan_updated = jsv::opt(&plan, "updatedAt");
                    let mut o = jsv::spread(&thread);
                    o.insert("proposedPlans".into(), Value::Array(next_plans));
                    o.insert(
                        "updatedAt".into(),
                        if js_gt(&thread_updated_at(&thread), &plan_updated) {
                            jsv::opt(&thread, "updatedAt")
                        } else {
                            plan_updated
                        },
                    );
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.turn-diff-completed" => {
            let thread_id = pf(event, "threadId")?;
            let p_turn = pf(event, "turnId")?;
            let p_completed = pf(event, "completedAt")?;
            let p_status = pf(event, "status")?;
            let p_files = pf(event, "files")?;
            let p_ref = pf(event, "checkpointRef")?;
            let p_assistant = pf(event, "assistantMessageId")?;
            let p_count = pf(event, "checkpointTurnCount")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let mut summary = Map::new();
                    summary.insert("turnId".into(), p_turn.clone());
                    let lt = jsv::opt(&thread, "latestTurn");
                    if jsv::opt(&lt, "turnId") == p_turn {
                        let started = jsv::opt(&lt, "startedAt");
                        if jsv::truthy(&started) {
                            summary.insert("startedAt".into(), started);
                        }
                    }
                    summary.insert("completedAt".into(), p_completed.clone());
                    summary.insert("status".into(), p_status.clone());
                    let files = map_array(&p_files, "event.payload.files", |f| {
                        let mut fo = Map::new();
                        fo.insert("path".into(), jsv::opt(f, "path"));
                        let kind = jsv::opt(f, "kind");
                        jsv::set_if_defined(&mut fo, "kind", &kind);
                        let additions = jsv::opt(f, "additions");
                        jsv::set_if_defined(&mut fo, "additions", &additions);
                        let deletions = jsv::opt(f, "deletions");
                        jsv::set_if_defined(&mut fo, "deletions", &deletions);
                        Ok(Value::Object(fo))
                    })?;
                    summary.insert("files".into(), Value::Array(files));
                    summary.insert("checkpointRef".into(), p_ref.clone());
                    // `assistantMessageId ?? undefined` maps null → undefined.
                    summary.insert(
                        "assistantMessageId".into(),
                        if is_nullish(&p_assistant) {
                            js::undefined()
                        } else {
                            p_assistant.clone()
                        },
                    );
                    summary.insert("checkpointTurnCount".into(), p_count.clone());
                    apply_turn_diff_summary_to_thread(&thread, &Value::Object(summary))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.reverted" => {
            let thread_id = pf(event, "threadId")?;
            let p_turn_count = pf(event, "turnCount")?;
            let sequence = ef(event, "sequence")?;
            let occurred = ef(event, "occurredAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let turn_count = jsv::to_number(&p_turn_count).unwrap_or(f64::NAN);
                    let mut summaries = filter_array(
                        &jsv::opt(&thread, "turnDiffSummaries"),
                        "thread.turnDiffSummaries",
                        |entry| {
                            let c = jsv::opt(entry, "checkpointTurnCount");
                            Ok(!is_undef(&c) && js_le(&c, &p_turn_count))
                        },
                    )?;
                    js::sort(&mut summaries, |l, r| {
                        checkpoint_count(&jsv::opt(l, "checkpointTurnCount"))
                            - checkpoint_count(&jsv::opt(r, "checkpointTurnCount"))
                    });
                    let retained_turn_ids: HashSet<String> = summaries
                        .iter()
                        .filter_map(|e| jsv::opt(e, "turnId").as_str().map(str::to_string))
                        .collect();
                    let retained_messages = retain_thread_messages_after_revert(
                        &jsv::opt(&thread, "messages"),
                        &retained_turn_ids,
                        turn_count,
                    )?;
                    let retained_ids: HashSet<String> = retained_messages
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|m| jsv::opt(m, "id").as_str().map(str::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    let messages = crate::shared::clear_removed_async_user_input_responses(
                        &retained_messages,
                        &retained_ids,
                        &sequence,
                    )?;
                    let messages = Value::Array(slice_last(
                        messages.as_array().cloned().unwrap_or_default(),
                        crate::store::MAX_THREAD_MESSAGES,
                    ));
                    let proposed_plans = retain_thread_proposed_plans_after_revert(
                        &jsv::opt(&thread, "proposedPlans"),
                        &retained_turn_ids,
                    );
                    let activities = retain_thread_activities_after_revert(
                        &jsv::opt(&thread, "activities"),
                        &retained_turn_ids,
                    );
                    let latest_checkpoint = summaries.last().cloned().unwrap_or(Value::Null);
                    let mut meta_input = jsv::spread(&thread);
                    meta_input.insert("messages".into(), messages.clone());
                    let lhm = jsv::opt(
                        &crate::shared::derive_thread_summary_metadata(&Value::Object(meta_input))?,
                        "latestHumanMessageAt",
                    );
                    let latest_turn = if latest_checkpoint.is_null() {
                        Value::Null
                    } else {
                        json!({
                            "turnId": jsv::opt(&latest_checkpoint, "turnId"),
                            "state": checkpoint_status_to_latest_turn_state(&jsv::opt(&latest_checkpoint, "status")),
                            "requestedAt": jsv::opt(&latest_checkpoint, "completedAt"),
                            "startedAt": jsv::opt(&latest_checkpoint, "completedAt"),
                            "completedAt": jsv::opt(&latest_checkpoint, "completedAt"),
                            "assistantMessageId": coalesce_owned(jsv::opt(&latest_checkpoint, "assistantMessageId"), Value::Null),
                        })
                    };
                    let mut o = jsv::spread(&thread);
                    o.insert("turnDiffSummaries".into(), Value::Array(summaries));
                    o.insert("messages".into(), messages);
                    o.insert("proposedPlans".into(), proposed_plans);
                    o.insert("activities".into(), activities);
                    o.insert("pendingSourceProposedPlan".into(), js::undefined());
                    o.insert("latestHumanMessageAt".into(), lhm);
                    o.insert("latestTurn".into(), latest_turn);
                    o.insert(
                        "updatedAt".into(),
                        if js_gt(&thread_updated_at(&thread), &occurred) {
                            jsv::opt(&thread, "updatedAt")
                        } else {
                            occurred.clone()
                        },
                    );
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.conversation-rolled-back" => {
            if jsv::to_number(&pf(event, "numTurns")?) == Some(0.0) {
                return Ok(state);
            }
            let thread_id = pf(event, "threadId")?;
            let p_msg = pf(event, "messageId")?;
            let p_removed_turns = pf(event, "removedTurnIds")?;
            let sequence = ef(event, "sequence")?;
            let occurred = ef(event, "occurredAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let (rollback_messages, mut removed_turn_ids) =
                        rollback_thread_messages_from_message(
                            &jsv::opt(&thread, "messages"),
                            &p_msg,
                        );
                    if let Some(extra) = p_removed_turns.as_array() {
                        for t in extra {
                            if let Some(s) = t.as_str() {
                                removed_turn_ids.insert(s.to_string());
                            }
                        }
                    }
                    let thread_messages = jsv::opt(&thread, "messages");
                    let msg_len = thread_messages.as_array().map(|a| a.len()).unwrap_or(0);
                    if rollback_messages.len() == msg_len && removed_turn_ids.is_empty() {
                        return Ok(thread);
                    }
                    let mut summaries = filter_array(
                        &jsv::opt(&thread, "turnDiffSummaries"),
                        "thread.turnDiffSummaries",
                        |entry| {
                            let tid = jsv::opt(entry, "turnId");
                            Ok(!tid.as_str().is_some_and(|t| removed_turn_ids.contains(t)))
                        },
                    )?;
                    js::sort(&mut summaries, |l, r| {
                        checkpoint_count(&jsv::opt(l, "checkpointTurnCount"))
                            - checkpoint_count(&jsv::opt(r, "checkpointTurnCount"))
                    });
                    let proposed_plans = Value::Array(
                        jsv::opt(&thread, "proposedPlans")
                            .as_array()
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|plan| {
                                let tid = jsv::opt(plan, "turnId");
                                tid.is_null()
                                    || !tid.as_str().is_some_and(|t| removed_turn_ids.contains(t))
                            })
                            .collect(),
                    );
                    let activities = Value::Array(
                        jsv::opt(&thread, "activities")
                            .as_array()
                            .cloned()
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|a| {
                                let tid = jsv::opt(a, "turnId");
                                tid.is_null()
                                    || !tid.as_str().is_some_and(|t| removed_turn_ids.contains(t))
                            })
                            .collect(),
                    );
                    let latest_checkpoint = summaries.last().cloned().unwrap_or(Value::Null);
                    let mut meta_input = jsv::spread(&thread);
                    meta_input.insert("messages".into(), Value::Array(rollback_messages.clone()));
                    let lhm = jsv::opt(
                        &crate::shared::derive_thread_summary_metadata(&Value::Object(meta_input))?,
                        "latestHumanMessageAt",
                    );
                    let retained_ids: HashSet<String> = rollback_messages
                        .iter()
                        .filter_map(|m| jsv::opt(m, "id").as_str().map(str::to_string))
                        .collect();
                    let cleared = crate::shared::clear_removed_async_user_input_responses(
                        &Value::Array(rollback_messages),
                        &retained_ids,
                        &sequence,
                    )?;
                    let latest_turn = if latest_checkpoint.is_null() {
                        Value::Null
                    } else {
                        json!({
                            "turnId": jsv::opt(&latest_checkpoint, "turnId"),
                            "state": checkpoint_status_to_latest_turn_state(&jsv::opt(&latest_checkpoint, "status")),
                            "requestedAt": jsv::opt(&latest_checkpoint, "completedAt"),
                            "startedAt": jsv::opt(&latest_checkpoint, "completedAt"),
                            "completedAt": jsv::opt(&latest_checkpoint, "completedAt"),
                            "assistantMessageId": coalesce_owned(jsv::opt(&latest_checkpoint, "assistantMessageId"), Value::Null),
                        })
                    };
                    let mut o = jsv::spread(&thread);
                    o.insert("turnDiffSummaries".into(), Value::Array(summaries));
                    o.insert(
                        "messages".into(),
                        Value::Array(slice_last(
                            cleared.as_array().cloned().unwrap_or_default(),
                            crate::store::MAX_THREAD_MESSAGES,
                        )),
                    );
                    o.insert("proposedPlans".into(), proposed_plans);
                    o.insert("activities".into(), activities);
                    o.insert("pendingSourceProposedPlan".into(), js::undefined());
                    o.insert("latestHumanMessageAt".into(), lhm);
                    o.insert("latestTurn".into(), latest_turn);
                    o.insert(
                        "updatedAt".into(),
                        if js_gt(&thread_updated_at(&thread), &occurred) {
                            jsv::opt(&thread, "updatedAt")
                        } else {
                            occurred.clone()
                        },
                    );
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.archived" => {
            let thread_id = pf(event, "threadId")?;
            let p_archived = pf(event, "archivedAt")?;
            let p_updated = pf(event, "updatedAt")?;
            let sequence = ef(event, "sequence")?;
            let occurred = ef(event, "occurredAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let mut o = jsv::spread(&thread);
                    o.insert(
                        "archivedAt".into(),
                        coalesce_owned(p_archived.clone(), occurred.clone()),
                    );
                    o.insert("pendingTurnStartMessageId".into(), Value::Null);
                    o.insert("snoozedUntil".into(), Value::Null);
                    o.insert("snoozeReminderAt".into(), Value::Null);
                    o.insert("snoozeSequence".into(), sequence.clone());
                    o.insert(
                        "updatedAt".into(),
                        coalesce_owned(p_updated.clone(), occurred.clone()),
                    );
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        "thread.unarchived" => {
            let thread_id = pf(event, "threadId")?;
            let p_updated = pf(event, "updatedAt")?;
            let occurred = ef(event, "occurredAt")?;
            crate::store::apply_thread_update(
                state,
                &thread_id,
                &mut |thread: Value| -> Result<Value, String> {
                    let mut o = jsv::spread(&thread);
                    o.insert("archivedAt".into(), Value::Null);
                    o.insert(
                        "updatedAt".into(),
                        coalesce_owned(p_updated.clone(), occurred.clone()),
                    );
                    Ok(Value::Object(o))
                },
                true,
                &options.sidebar_override(true),
            )
        }
        _ => Ok(state),
    }
}

/// Port of `applyThreadActivityEventBatch` (storeEventReducer.ts:1922).
fn apply_thread_activity_event_batch(
    state: Value,
    events: &[Value],
    options: &NormalizedOptions,
) -> Result<Value, String> {
    let Some(first_event) = events.first() else {
        return Ok(state);
    };
    let updates_summary = events.iter().any(thread_activity_updates_summary);
    let updates_sidebar = events.iter().any(thread_activity_updates_sidebar_summary);
    let thread_id = pf(first_event, "threadId")?;
    crate::store::apply_thread_update(
        state,
        &thread_id,
        &mut |thread: Value| -> Result<Value, String> {
            let thread_activities = jsv::opt(&thread, "activities");
            let mut accumulator =
                crate::store::create_thread_activity_accumulator(&thread_activities)?;
            let mut next_pending = jsv::opt(&thread, "pendingInteractions");
            let mut updated_at = thread_updated_at(&thread);
            for event in events {
                let sequence = ef(event, "sequence")?;
                let payload_activity = pf(event, "activity")?;
                let sequenced =
                    crate::store::with_orchestration_event_sequence(&payload_activity, &sequence)?;
                let activities_changed = accumulator.append(&sequenced)?;
                let reconciled = reconcile_pending_interactions_from_activity(
                    &jsv::opt(&thread, "id"),
                    &next_pending,
                    event,
                )?;
                let changed = activities_changed || reconciled != next_pending;
                next_pending = reconciled;
                if changed && js_gt(&jsv::opt(&sequenced, "createdAt"), &updated_at) {
                    updated_at = jsv::opt(&sequenced, "createdAt");
                }
            }
            let next_activities = accumulator.result();
            if next_activities == thread_activities
                && next_pending == jsv::opt(&thread, "pendingInteractions")
            {
                return Ok(thread);
            }
            let mut o = jsv::spread(&thread);
            o.insert("activities".into(), next_activities);
            jsv::set_if_defined(&mut o, "pendingInteractions", &next_pending);
            o.insert("updatedAt".into(), updated_at);
            Ok(Value::Object(o))
        },
        updates_summary,
        &options.sidebar_override(options.sidebar_is_true() || updates_sidebar),
    )
}

// ---------- exports ----------

fn normalize_options(options: &Value) -> NormalizedOptions {
    let raw = if is_nullish(options) {
        None
    } else {
        jsv::get(options, "updateSidebarSummary")
    };
    NormalizedOptions {
        update_sidebar_summary: match raw {
            Some(v) if !is_nullish(v) => v.clone(),
            _ => Value::Bool(false),
        },
    }
}

fn events_len(events: &Value) -> Result<usize, String> {
    match events {
        Value::Null => Err("null is not an object (evaluating 'events.length')".to_string()),
        v if js::is_undefined(v) => {
            Err("undefined is not an object (evaluating 'events.length')".to_string())
        }
        Value::Array(a) => Ok(a.len()),
        Value::String(s) => Ok(crate::utf16::units(s).len()),
        _ => Ok(0),
    }
}

/// Port of `applyOrchestrationEvents` (`apps/web/src/storeEventReducer.ts:1984`).
pub fn apply_orchestration_events(
    state: serde_json::Value,
    events: serde_json::Value,
) -> Result<serde_json::Value, String> {
    apply_orchestration_events_hot_path(state, events, json!({ "updateSidebarSummary": false }))
}

/// Port of `applyOrchestrationEventsHotPath` (`apps/web/src/storeEventReducer.ts:1993`).
pub fn apply_orchestration_events_hot_path(
    state: serde_json::Value,
    events: serde_json::Value,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let normalized_options = normalize_options(&options);
    let mut next_state = state;
    let len = events_len(&events)?;
    let mut index = 0usize;
    while index < len {
        let event = index_of(&events, index);
        let event_type = jsv::member(&event, "type", "event.type")?;
        if event_type == Some(&Value::String("thread.activity-appended".into())) {
            let thread_id = pf(&event, "threadId")?;
            let mut batch = vec![event.clone()];
            while index + 1 < len {
                let next_event = index_of(&events, index + 1);
                let next_type = if is_nullish(&next_event) {
                    None
                } else {
                    jsv::member(&next_event, "type", "nextEvent.type")?
                };
                if next_type != Some(&Value::String("thread.activity-appended".into())) {
                    break;
                }
                let next_thread = {
                    let p = jsv::opt(&next_event, "payload");
                    jsv::member(&p, "threadId", "nextEvent.payload.threadId")?
                        .cloned()
                        .unwrap_or_else(js::undefined)
                };
                if next_thread != thread_id {
                    break;
                }
                batch.push(next_event);
                index += 1;
            }
            next_state =
                apply_thread_activity_event_batch(next_state, &batch, &normalized_options)?;
            index += 1;
            continue;
        }
        next_state = apply_orchestration_event(next_state, &event, &normalized_options)?;
        if matches!(
            event_type,
            Some(Value::String(s)) if s == "thread.reverted" || s == "thread.conversation-rolled-back"
        ) {
            let thread_id_v = pf(&event, "threadId")?;
            let thread_id_key = jsv::js_to_string(&thread_id_v);
            let current = jsv::get(&next_state, "threadDetailRevisionSequenceById")
                .and_then(|m| jsv::get(m, &thread_id_key));
            if current.is_some_and(|c| !is_undef(c)) {
                let current = current.cloned().unwrap_or_else(js::undefined);
                let sequence = ef(&event, "sequence")?;
                let maxed = js::number(jsv::js_max(
                    jsv::to_number(&current).unwrap_or(f64::NAN),
                    jsv::to_number(&sequence).unwrap_or(f64::NAN),
                ));
                let mut o = jsv::spread(&next_state);
                let existing = jsv::get(&next_state, "threadDetailRevisionSequenceById");
                let mut m = existing.map(jsv::spread).unwrap_or_default();
                m.insert(thread_id_key, maxed);
                o.insert("threadDetailRevisionSequenceById".into(), Value::Object(m));
                next_state = Value::Object(o);
            }
        }
        index += 1;
    }
    Ok(next_state)
}
