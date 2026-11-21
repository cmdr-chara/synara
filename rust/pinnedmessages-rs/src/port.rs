//! The Rust port of `packages/shared/src/pinnedMessages.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn undef() -> Value {
    json!({ "$undefined": true })
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn prop(v: &Value, key: &str) -> Value {
    v.get(key).cloned().unwrap_or_else(undef)
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ if crate::js::is_undefined(a) || crate::js::is_undefined(b) => {
            crate::js::is_undefined(a) && crate::js::is_undefined(b)
        }
        (Value::Number(_), _) | (_, Value::Number(_)) => {
            match (crate::js::as_number(a), crate::js::as_number(b)) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            }
        }
        _ => false,
    }
}

fn js_spread(v: &Value) -> Map<String, Value> {
    match v {
        Value::Object(o) => o.clone(),
        Value::String(s) => s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect(),
        _ => Map::new(),
    }
}

fn pins_map(
    pins: &Value,
    f: impl Fn(&Value) -> Result<Value, String>,
) -> Result<Vec<Value>, String> {
    match pins {
        Value::Array(items) => items.iter().map(f).collect(),
        _ if is_nullish(pins) => Ok(vec![]),
        _ => Err("existingPins.map is not a function".to_string()),
    }
}

/// Port of `isMessagePinned` (`packages/shared/src/pinnedMessages.ts:17`).
pub fn is_message_pinned(
    pins: serde_json::Value,
    message_id: serde_json::Value,
) -> Result<bool, String> {
    match &pins {
        Value::Array(items) => Ok(items
            .iter()
            .any(|p| js_strict_eq(&prop(p, "messageId"), &message_id))),
        _ if is_nullish(&pins) => Ok(false),
        _ => Err("pins?.some is not a function".to_string()),
    }
}

/// Port of `addPinnedMessage` (`packages/shared/src/pinnedMessages.ts:24`).
pub fn add_pinned_message(
    pins: serde_json::Value,
    pin: serde_json::Value,
) -> Result<Vec<Value>, String> {
    let message_id = if pin.is_null() {
        return Err("Cannot read properties of null (reading 'messageId')".to_string());
    } else if crate::js::is_undefined(&pin) {
        return Err("Cannot read properties of undefined (reading 'messageId')".to_string());
    } else {
        prop(&pin, "messageId")
    };
    let existing = match &pins {
        Value::Array(items) => items.clone(),
        _ if is_nullish(&pins) => vec![],
        _ => return Err("pins?.some is not a function".to_string()),
    };
    if existing
        .iter()
        .any(|p| js_strict_eq(&prop(p, "messageId"), &message_id))
    {
        return Ok(existing);
    }
    let mut out = existing;
    out.push(pin);
    Ok(out)
}

/// Port of `removePinnedMessage` (`packages/shared/src/pinnedMessages.ts:34`).
pub fn remove_pinned_message(
    pins: serde_json::Value,
    message_id: serde_json::Value,
) -> Result<Vec<Value>, String> {
    match &pins {
        Value::Array(items) => Ok(items
            .iter()
            .filter(|p| !js_strict_eq(&prop(p, "messageId"), &message_id))
            .cloned()
            .collect()),
        _ if is_nullish(&pins) => Ok(vec![]),
        _ => Err("existingPins.filter is not a function".to_string()),
    }
}

/// Port of `setPinnedMessageDone` (`packages/shared/src/pinnedMessages.ts:43`).
pub fn set_pinned_message_done(
    pins: serde_json::Value,
    message_id: serde_json::Value,
    done: bool,
) -> Result<Vec<Value>, String> {
    pins_map(&pins, |p| {
        if js_strict_eq(&prop(p, "messageId"), &message_id) {
            let mut o = js_spread(p);
            o.insert("done".to_string(), Value::Bool(done));
            Ok(Value::Object(o))
        } else {
            Ok(p.clone())
        }
    })
}

/// Port of `togglePinnedMessageDone` (`packages/shared/src/pinnedMessages.ts:60`).
pub fn toggle_pinned_message_done(
    pins: serde_json::Value,
    message_id: serde_json::Value,
) -> Result<Vec<Value>, String> {
    pins_map(&pins, |p| {
        if js_strict_eq(&prop(p, "messageId"), &message_id) {
            let mut o = js_spread(p);
            o.insert(
                "done".to_string(),
                Value::Bool(!truthy(p.get("done").unwrap_or(&Value::Null))),
            );
            Ok(Value::Object(o))
        } else {
            Ok(p.clone())
        }
    })
}

fn js_trim(s: &str) -> &str {
    const JS_WS: &[char] = &[
        '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}',
        '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}',
        '\u{2009}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}',
        '\u{feff}',
    ];
    s.trim_matches(|c| JS_WS.contains(&c))
}

fn utf16_truncate(s: &str, max_units: usize) -> String {
    let units = crate::utf16::units(s);
    if units.len() <= max_units {
        return s.to_string();
    }
    crate::utf16::slice(&units, 0, max_units)
}

/// Port of `normalizePinLabel` (`packages/shared/src/pinnedMessages.ts:76`).
pub fn normalize_pin_label(label: serde_json::Value) -> Result<Option<String>, String> {
    let Some(s) = label.as_str() else {
        return Ok(None);
    };
    let trimmed = js_trim(s);
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(utf16_truncate(trimmed, 60)))
}

/// Port of `setPinnedMessageLabel` (`packages/shared/src/pinnedMessages.ts:86`).
pub fn set_pinned_message_label(
    pins: serde_json::Value,
    message_id: serde_json::Value,
    label: serde_json::Value,
) -> Result<Vec<Value>, String> {
    let normalized = normalize_pin_label(label)?;
    let label_value = match normalized {
        Some(s) => Value::String(s),
        None => Value::Null,
    };
    pins_map(&pins, |p| {
        if js_strict_eq(&prop(p, "messageId"), &message_id) {
            let mut o = js_spread(p);
            o.insert("label".to_string(), label_value.clone());
            Ok(Value::Object(o))
        } else {
            Ok(p.clone())
        }
    })
}

/// Port of `clampThreadNotes` (`packages/shared/src/pinnedMessages.ts:104`).
pub fn clamp_thread_notes(notes: String) -> Result<String, String> {
    Ok(utf16_truncate(&notes, 16_384))
}
