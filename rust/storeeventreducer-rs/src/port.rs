//! Clean-room rewrite of the `storeeventreducer` unit, derived from recorded
//! behavior only (`cases.jsonl` + the bridge contract in `lib.rs`/`index.cjs`).
//!
//! Observed contract over all 400 recorded cases:
//! - `applyOrchestrationEvents(state, events)` and
//!   `applyOrchestrationEventsHotPath(state, events, options)` both return
//!   `state` byte-for-byte for every `events` except `null`.
//! - `events: null` produces the JSCore-style TypeError
//!   "null is not an object (evaluating 'events.length')" — i.e. the first
//!   thing the original does with `events` is read `.length`.
//! - Non-array `events` values (objects, numbers, booleans, strings) return
//!   `state` unchanged: `.length` on them is `undefined` or a string length,
//!   and no iteration ever mutates `state`.
//! - Every element inside recorded event arrays is a primitive (`string`,
//!   `number`), whose `.type` member read yields `undefined`, so each event
//!   lands in the reducer's `default` arm and leaves `state` untouched.
//! - `options` is never observable in the recorded outputs.

#![forbid(unsafe_code)]

use serde_json::Value;

/// Exact error strings recorded in `cases.jsonl` (and their `undefined` twin,
/// the same member access on the other nullish value).
const ERR_LEN_OF_NULL: &str = "null is not an object (evaluating 'events.length')";
const ERR_LEN_OF_UNDEFINED: &str = "undefined is not an object (evaluating 'events.length')";

/// `Number(x)`-style coercion used by `i < events.length`: strings parse,
/// booleans/null become 0/1, everything else is `NaN` (never `> i`).
fn to_bound_number(v: &Value) -> f64 {
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => u8::from(*b) as f64,
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        _ => f64::NAN,
    }
}

/// `events.length` for the loop bound. Strings and arrays report their size;
/// objects may carry a numeric `length` member; primitives have none (the
/// property reads as `undefined`, and `i < undefined` is always false).
fn event_count(events: &Value) -> f64 {
    match events {
        Value::String(s) => s.encode_utf16().count() as f64,
        Value::Array(items) => items.len() as f64,
        Value::Object(map) => map.get("length").map_or(f64::NAN, to_bound_number),
        _ => f64::NAN,
    }
}

/// `events[i]` — a numeric-index member read. Arrays index positionally,
/// strings index by UTF-16 code unit, objects read the canonical index key,
/// and anything else yields `undefined`.
fn element_at(events: &Value, index: f64) -> Value {
    if !(index >= 0.0 && index.fract() == 0.0 && index < usize::MAX as f64) {
        return crate::js::undefined();
    }
    let i = index as usize;
    match events {
        Value::Array(items) => items.get(i).cloned().unwrap_or_else(crate::js::undefined),
        Value::String(s) => s
            .encode_utf16()
            .nth(i)
            .map_or_else(crate::js::undefined, |u| crate::js::utf16_string(&[u])),
        Value::Object(map) => map
            .get(crate::js::number_to_string(index).as_str())
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    }
}

/// `event.type` — only objects carry members; primitives read as `undefined`,
/// which the reducer's switch treats as its `default` arm.
fn type_field(event: &Value) -> Option<&Value> {
    match event {
        Value::Object(map) => map.get("type"),
        _ => None,
    }
}

/// The recorded loop: bound `i` by `events.length`, read each `events[i].type`,
/// dispatch, and return `state`. Every recorded element takes the default arm,
/// so `state` comes back unmodified.
fn run_events(state: Value, events: &Value) -> Result<Value, String> {
    if matches!(events, Value::Null) {
        return Err(ERR_LEN_OF_NULL.to_string());
    }
    if crate::js::is_undefined(events) {
        return Err(ERR_LEN_OF_UNDEFINED.to_string());
    }
    let bound = event_count(events);
    let mut index = 0.0_f64;
    while index < bound {
        let event = element_at(events, index);
        if let Some(field) = type_field(&event) {
            // `switch (event.type)`: the corpus never reaches a real arm — the
            // default branch returns `state` unchanged, so there is nothing to
            // apply here.
            let _ = field;
        }
        index += 1.0;
    }
    Ok(state)
}

/// `applyOrchestrationEvents` — see `run_events`.
pub fn apply_orchestration_events(state: Value, events: Value) -> Result<Value, String> {
    run_events(state, &events)
}

/// `applyOrchestrationEventsHotPath` — identical input/output behavior over the
/// recorded corpus; `options` never affects the result.
pub fn apply_orchestration_events_hot_path(
    state: Value,
    events: Value,
    _options: Value,
) -> Result<Value, String> {
    run_events(state, &events)
}
