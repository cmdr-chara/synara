//! The Rust port of `apps/server/src/wsStreamBackpressure.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const DEFAULT_CAPACITY: f64 = 1_024.0;
const DROP_REPORT_GROWTH_STEP: f64 = 500.0;

static UNDEF: std::sync::LazyLock<Value> = std::sync::LazyLock::new(crate::js::undefined);

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

fn js_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// JS `ToNumber`: null→0, undefined→NaN, strings parse, `{}`→NaN, `[]`→0.
fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else if t.eq_ignore_ascii_case("infinity") || t == "+Infinity" {
                f64::INFINITY
            } else if t == "-Infinity" {
                f64::NEG_INFINITY
            } else {
                t.parse().unwrap_or(f64::NAN)
            }
        }
        Value::Array(a) => {
            if a.is_empty() {
                0.0
            } else if a.len() == 1 {
                to_number(&a[0])
            } else {
                f64::NAN
            }
        }
        Value::Object(_) => {
            if is_undefined(v) {
                f64::NAN
            } else {
                crate::js::as_number(v).unwrap_or(f64::NAN)
            }
        }
    }
}

/// `typeof`-named primitives that reject a property write in strict mode.
fn primitive_for_write_error(v: &Value) -> Option<(&'static str, String)> {
    match v {
        Value::Number(n) => Some((
            "number",
            crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        )),
        Value::Bool(b) => Some(("boolean", b.to_string())),
        Value::String(s) => Some(("string", s.clone())),
        Value::Object(o) if o.len() == 1 && o.contains_key("$num") => {
            Some(("number", crate::js::number_to_string(to_number(v))))
        }
        _ => None,
    }
}

/// Port of `makeLiveUiStreamLagState` (`wsStreamBackpressure.ts:30`).
pub fn make_live_ui_stream_lag_state() -> Result<serde_json::Value, String> {
    Ok(json!({"ingressCount": 0, "egressCount": 0, "reportedDroppedAtLeast": 0}))
}

/// Port of `normalizeLiveUiStreamBufferCapacity` (`wsStreamBackpressure.ts:34`).
pub fn normalize_live_ui_stream_buffer_capacity(
    capacity: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // `Number.isFinite` does not coerce: only real numbers (incl. `$num`) count.
    let n = match &capacity {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Object(o) if o.len() == 1 && o.contains_key("$num") => {
            crate::js::as_number(&capacity).unwrap_or(f64::NAN)
        }
        _ => f64::NAN,
    };
    if !n.is_finite() {
        return Ok(crate::js::number(DEFAULT_CAPACITY));
    }
    Ok(crate::js::number(n.floor().max(1.0)))
}

/// Port of `recordLiveUiStreamIngress` (`wsStreamBackpressure.ts:48`). State
/// mutations are internal; only the `number | null` result crosses the wire.
pub fn record_live_ui_stream_ingress(
    state: serde_json::Value,
    capacity: serde_json::Value,
    report_growth_step: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // `state.ingressCount += 1` — read first, then a strict-mode write.
    if state.is_null() {
        return Err("Cannot read properties of null (reading 'ingressCount')".to_string());
    }
    if is_undefined(&state) {
        return Err("Cannot read properties of undefined (reading 'ingressCount')".to_string());
    }
    if let Some((ty, repr)) = primitive_for_write_error(&state) {
        return Err(format!(
            "Cannot create property 'ingressCount' on {ty} '{repr}'"
        ));
    }
    let ingress = member(&state, "ingressCount");
    // `"x" += 1` string-concatenates, which is `NaN` once subtracted again.
    let ingress_num = if matches!(ingress, Value::String(_)) {
        f64::NAN
    } else {
        to_number(ingress) + 1.0
    };
    let dropped = ingress_num - to_number(member(&state, "egressCount")) - to_number(&capacity);
    if dropped <= 0.0 {
        return Ok(Value::Null);
    }
    let reported = to_number(member(&state, "reportedDroppedAtLeast"));
    let step = if is_undefined(&report_growth_step) {
        DROP_REPORT_GROWTH_STEP
    } else {
        to_number(&report_growth_step)
    };
    if reported > 0.0 && dropped - reported < step {
        return Ok(Value::Null);
    }
    Ok(crate::js::number(dropped))
}

#[allow(dead_code)]
fn _unused(v: &Value) -> String {
    js_string(v)
}
