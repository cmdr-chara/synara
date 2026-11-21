//! The Rust port of `packages/shared/src/uiTreeTargeting.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js::{as_number, is_undefined, number, number_to_string};

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
        Value::String(s) => js_string_to_number(s),
        Value::Number(_) | Value::Object(_) => as_number(v).unwrap_or(f64::NAN),
        Value::Array(_) => f64::NAN,
    }
}

fn js_string_to_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}' || c == '\u{00a0}');
    if t.is_empty() {
        return 0.0;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16)
            .map(|n| n as f64)
            .unwrap_or(f64::NAN);
    }
    if t.eq_ignore_ascii_case("infinity") || t == "+Infinity" || t == "-Infinity" {
        return if t.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn js_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) | Value::Object(_) => as_number(v)
            .map(number_to_string)
            .unwrap_or_else(|| "[object Object]".to_string()),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if x.is_null() || is_undefined(x) {
                    String::new()
                } else {
                    js_str(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
    }
}

enum Sum {
    Num(f64),
    Str(String),
}

fn js_add(a: &Value, b: f64) -> Sum {
    if let Value::String(_) = a {
        return Sum::Str(js_str(a) + &number_to_string(b));
    }
    Sum::Num(to_number(a) + b)
}

/// Port of `uiTreeActivationPoint` (`packages/shared/src/uiTreeTargeting.ts:62`).
pub fn ui_tree_activation_point(node: serde_json::Value) -> Result<serde_json::Value, String> {
    if node.is_null() {
        return Err("Cannot read properties of null (reading 'activationPoint')".to_string());
    }
    let get = |key: &str| node.get(key).cloned().unwrap_or_else(crate::js::undefined);
    let ap = get("activationPoint");
    if !ap.is_null() && !is_undefined(&ap) {
        return Ok(ap);
    }
    let frame = get("frame");
    if frame.is_null() || is_undefined(&frame) || !frame.is_object() {
        return Err("Cannot read properties of undefined (reading 'x')".to_string());
    }
    let field = |key: &str| frame.get(key).cloned().unwrap_or_else(crate::js::undefined);
    let w = to_number(&field("width")) / 2.0;
    let h = to_number(&field("height")) / 2.0;
    let x = match js_add(&field("x"), w) {
        Sum::Str(s) => json!(s),
        Sum::Num(n) => number(n),
    };
    let y = match js_add(&field("y"), h) {
        Sum::Str(s) => json!(s),
        Sum::Num(n) => number(n),
    };
    Ok(json!({ "x": x, "y": y }))
}
