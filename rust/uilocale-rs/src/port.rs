//! The Rust port of `packages/shared/src/uiLocale.ts`.
//!
//! Behavior inferred clean-room from recorded cases and black-box probes:
//! `ns:key` shape — ns matches `^[a-z][a-z0-9_-]*$`, key matches
//! `^[A-Za-z0-9_][A-Za-z0-9_.-]*$`, and `namespaces` must be a plain object
//! that has `ns` as an own key. `message` is coerced with `String()` first.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.get("$undefined") == Some(&Value::Bool(true)))
}

/// JS `String(value)` for the values the wire can carry.
fn js_str(value: &Value) -> Result<String, String> {
    match value {
        Value::Null => Ok("null".into()),
        Value::Bool(b) => Ok(b.to_string()),
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(0.0);
            if f.is_nan() {
                Ok("NaN".into())
            } else if f == f64::INFINITY {
                Ok("Infinity".into())
            } else if f == f64::NEG_INFINITY {
                Ok("-Infinity".into())
            } else if f.fract() == 0.0 && f.abs() < 1e21 {
                Ok(format!("{}", f as i64))
            } else {
                Ok(format!("{f}"))
            }
        }
        Value::String(s) => Ok(s.clone()),
        Value::Array(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                parts.push(js_str(item)?);
            }
            Ok(parts.join(","))
        }
        Value::Object(map) => {
            if is_undefined(value) {
                return Ok("undefined".into());
            }
            // ToPrimitive with hint "string": an own, non-callable `toString`
            // makes `valueOf` (inherited, returns the object) the last resort.
            if map.contains_key("toString") {
                return Err("Cannot convert object to primitive value".into());
            }
            Ok("[object Object]".into())
        }
    }
}

fn is_ns(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn is_key(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

/// Port of `isUiMessageKey` (`packages/shared/src/uiLocale.ts:7`).
pub fn is_ui_message_key(message: Value, namespaces: Value) -> Result<bool, String> {
    let message = js_str(&message)?;
    let Value::Object(map) = &namespaces else {
        return Ok(false);
    };
    let mut parts = message.split(':');
    let (Some(ns), Some(key), None) = (parts.next(), parts.next(), parts.next()) else {
        return Ok(false);
    };
    Ok(is_ns(ns) && is_key(key) && map.contains_key(ns))
}
