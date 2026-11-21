//! The Rust port of `apps/web/src/threadDetailCatchupPolicy.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined") == Some(&Value::Bool(true)))
}

/// JS `obj[key]` on objects; undefined elsewhere (member reads throw only on null/undefined).
fn member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    })
}

/// `===` strict equality: same type and value; `NaN === NaN` is false, `-0 === 0` is true.
fn strict_eq(a: &Value, b: &Value) -> bool {
    // `$num` markers are numbers too; NaN compares false here, matching JS.
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| strict_eq(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|o| strict_eq(v, o)))
        }
        _ => is_undefined(a) && is_undefined(b),
    }
}

/// Port of `isThreadDetailVerifiedInSync` (`apps/web/src/threadDetailCatchupPolicy.ts:36`).
pub fn is_thread_detail_verified_in_sync(evidence: serde_json::Value) -> Result<bool, String> {
    let empty = member(&evidence, "emptyReplayAtEventSerial")?;
    let applied = member(&evidence, "appliedEventSerial")?;
    Ok(!empty.is_null() && strict_eq(&empty, &applied))
}

/// Port of `isLiveThreadStreamDelivering` (`apps/web/src/threadDetailCatchupPolicy.ts:56`).
pub fn is_live_thread_stream_delivering(
    last_live_event_at: Option<f64>,
    now_ms: f64,
    window_ms: f64,
) -> Result<bool, String> {
    // `lastLiveEventAt !== null`: an undefined/missing value still makes the
    // subtraction NaN, which fails the `<` comparison the same way.
    Ok(match last_live_event_at {
        Some(at) => now_ms - at < window_ms,
        None => false,
    })
}
