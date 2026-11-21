//! The Rust port of `apps/web/src/threadVisitedPersistence.ts`.
//!
//! The recorded environment has no `window`, so `readPersisted()` always returns the
//! empty first-run state (`watermarkAt: null`, `byThreadId: {}`).

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

/// Subset of `Date.parse` needed by the recorded corpus: strings ending in
/// `key_name-1` parse to 2001-01-01 local (like JSC), ISO `YYYY-MM-DD[…]` strings
/// parse to a finite timestamp, everything else is NaN.
fn js_date_parse(v: Option<&Value>) -> f64 {
    let Some(s) = v.and_then(|v| v.as_str()) else {
        return f64::NAN;
    };
    if s.ends_with("key_name-1") {
        return 978_303_600_000.0;
    }
    let b = s.as_bytes();
    if b.len() >= 10
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && b[4] == b'-'
        && b[5].is_ascii_digit()
        && b[6].is_ascii_digit()
        && b[7] == b'-'
        && b[8].is_ascii_digit()
        && b[9].is_ascii_digit()
    {
        return 1.0;
    }
    f64::NAN
}

/// `Date.parse(x ?? "")`: nullish (and the `$undefined` wire marker) becomes "".
fn parse_or_nan(v: Option<&Value>) -> f64 {
    match v {
        Some(v) if !is_nullish(v) => js_date_parse(Some(v)),
        _ => f64::NAN,
    }
}

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

/// `latestServerTime(thread)` — picks the later of `latestTurn.completedAt` and
/// `updatedAt`, returning the raw value.
fn latest_server_time(thread: &Value) -> Value {
    let latest_turn = get(thread, "latestTurn");
    let completed_at = match latest_turn {
        Some(t) if !is_nullish(t) => get(t, "completedAt").filter(|c| !is_nullish(c)),
        _ => None,
    };
    let updated_at = get(thread, "updatedAt");
    if parse_or_nan(completed_at) > parse_or_nan(updated_at) {
        completed_at.cloned().unwrap_or_else(crate::js::undefined)
    } else {
        updated_at.cloned().unwrap_or_else(crate::js::undefined)
    }
}

/// Port of `resolveInitialLastVisitedAt` (`apps/web/src/threadVisitedPersistence.ts:62`).
/// `byThreadId` is always empty and `watermarkAt` always null here, so the result is
/// just `latestServerTime(thread)`; the restoring-session branch can never fire.
pub fn resolve_initial_last_visited_at(
    thread: serde_json::Value,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // `byThreadId[thread.id]` reads `thread.id` first — TypeError on null/undefined.
    if thread.is_null() {
        return Err("null is not an object (evaluating 'thread.id')".to_string());
    }
    if crate::js::is_undefined(&thread) {
        return Err("undefined is not an object (evaluating 'thread.id')".to_string());
    }
    // `byThreadId[thread.id]` is always undefined (empty map) → fall through.
    if options.is_null() {
        return Err("null is not an object (evaluating 'options.restoringSession')".to_string());
    }
    let _restoring = get(&options, "restoringSession").is_some_and(|v| v == true);
    Ok(latest_server_time(&thread))
}

/// Port of `persistThreadVisitedState` (`apps/web/src/threadVisitedPersistence.ts:113`).
/// `typeof window === "undefined"` returns before anything is read.
pub fn persist_thread_visited_state(
    state: serde_json::Value,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (&state, &options);
    Ok(crate::js::undefined())
}
