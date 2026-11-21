//! Shared JS-semantics helpers for the port: member access, equality, truthiness,
//! numeric/string coercion and object-spread utilities. Errors match the
//! JavaScriptCore strings the corpus recorded ("X is not an object
//! (evaluating 'expr')", "X is not a function. (In 'call', 'X' is undefined)").

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

pub(crate) fn is_nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

pub(crate) fn truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// `v.key` where `v` is already known to be a plain object.
pub(crate) fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

/// `v.key` coalesced to `undefined` when `v` is a non-nullish non-object or the
/// key is missing.
pub(crate) fn opt(v: &Value, key: &str) -> Value {
    get(v, key).cloned().unwrap_or_else(crate::js::undefined)
}

/// `v.key` member access: throws on null/undefined receivers, returns `undefined`
/// for a missing property on anything else (primitives box to objects in JS).
pub(crate) fn member<'a>(v: &'a Value, key: &str, expr: &str) -> Result<Option<&'a Value>, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{expr}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!("undefined is not an object (evaluating '{expr}')"));
    }
    Ok(v.as_object().and_then(|o| o.get(key)))
}

/// `a === b` (strict): undefined === undefined; NaN never equals.
pub(crate) fn strict_eq(a: &Value, b: &Value) -> bool {
    if crate::js::is_undefined(a) || crate::js::is_undefined(b) {
        return crate::js::is_undefined(a) && crate::js::is_undefined(b);
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().unwrap_or(f64::NAN) == y.as_f64().unwrap_or(f64::NAN)
        }
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

/// `+v`/`Number(v)` coercion.
pub(crate) fn to_number(v: &Value) -> Option<f64> {
    if crate::js::is_undefined(v) {
        return None;
    }
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Bool(b) => Some(*b as i64 as f64),
        Value::Null => Some(0.0),
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                Some(0.0)
            } else {
                t.parse::<f64>().ok()
            }
        }
        _ => None,
    }
}

pub(crate) fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if is_nullish(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if o.contains_key("$undefined") {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// `String.prototype.localeCompare`: codepoint order is exact for the ASCII
/// timestamp/id strings this code compares.
pub(crate) fn locale_compare(a: &str, b: &str) -> i64 {
    a.cmp(b) as i64
}

/// `Math.max` with JS NaN semantics (Rust `f64::max` drops NaN).
pub(crate) fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `{...v}` object spread: only own enumerable string-keyed properties.
pub(crate) fn spread(v: &Value) -> Map<String, Value> {
    match v {
        Value::Object(o) => o.clone(),
        _ => Map::new(),
    }
}

/// Build `{key: value, ...}` skipping keys whose value is `undefined` — matches
/// object literals that conditionally include fields via `...(x !== undefined
/// ? {k: x} : {})`.
pub(crate) fn set_if_defined(o: &mut Map<String, Value>, key: &str, v: &Value) {
    if !crate::js::is_undefined(v) {
        o.insert(key.to_string(), v.clone());
    }
}
