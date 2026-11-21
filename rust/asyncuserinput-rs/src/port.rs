//! The Rust port of `packages/shared/src/asyncUserInput.ts`.
//!
//! Clean-room reimplementation: inferred solely from recorded cases.jsonl behavior
//! and the exported contract types.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

fn undefined() -> Value {
    json!({ "$undefined": true })
}

fn get(v: &Value, key: &str) -> Value {
    v.get(key).cloned().unwrap_or_else(undefined)
}

fn member_error(v: &Value, field: &str) -> String {
    if v.is_null() {
        format!("Cannot read properties of null (reading '{field}')")
    } else {
        format!("Cannot read properties of undefined (reading '{field}')")
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !is_undefined(v),
    }
}

fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(js_str).collect::<Vec<_>>().join(","),
        Value::Object(_) => {
            if is_undefined(v) {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn num_to_json(n: f64) -> Value {
    if !n.is_finite() {
        Value::Null
    } else if n.fract() == 0.0 && n.abs() <= 9.007_199_254_740_992e15 {
        json!(n as i64)
    } else {
        json!(n)
    }
}

/// Port of `mergeAsyncUserInput` (`packages/shared/src/asyncUserInput.ts:14`).
pub fn merge_async_user_input(
    previous: serde_json::Value,
    incoming: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if truthy(&previous) && !truthy(&incoming) {
        Ok(previous)
    } else {
        Ok(incoming)
    }
}

/// Port of `clearRemovedAsyncUserInputResponses` (`packages/shared/src/asyncUserInput.ts:29`).
pub fn clear_removed_async_user_input_responses(
    messages: serde_json::Value,
    retained_message_ids: serde_json::Value,
    sequence: f64,
) -> Result<serde_json::Value, String> {
    if messages.is_null() {
        return Err("Cannot read properties of null (reading 'map')".to_string());
    }
    let Some(list) = messages.as_array() else {
        return Err("messages.map is not a function".to_string());
    };
    let mut out = Vec::with_capacity(list.len());
    for message in list {
        if message.is_null() {
            return Err(member_error(message, "asyncUserInput"));
        }
        let aui = get(message, "asyncUserInput");
        let response = get(&aui, "response");
        if !truthy(&response) {
            out.push(message.clone());
            continue;
        }
        let retained = match retained_message_ids.get("$set").and_then(Value::as_array) {
            Some(ids) => ids,
            None => return Err("retainedMessageIds.has is not a function".to_string()),
        };
        let message_id = get(&response, "messageId");
        if retained.contains(&message_id) {
            out.push(message.clone());
            continue;
        }
        let mut next = match message.as_object() {
            Some(obj) => obj.clone(),
            None => Map::new(),
        };
        let mut aui_out = Map::new();
        aui_out.insert("questions".to_string(), get(&aui, "questions"));
        aui_out.insert("responseSequence".to_string(), num_to_json(sequence));
        next.insert("asyncUserInput".to_string(), Value::Object(aui_out));
        out.push(Value::Object(next));
    }
    Ok(Value::Array(out))
}

/// Port of `formatAsyncUserInputResponse` (`packages/shared/src/asyncUserInput.ts:42`).
pub fn format_async_user_input_response(
    questions: serde_json::Value,
    answers: serde_json::Value,
) -> Result<String, String> {
    if questions.is_null() {
        return Err("Cannot read properties of null (reading 'map')".to_string());
    }
    let Some(list) = questions.as_array() else {
        return Err("questions.map is not a function".to_string());
    };
    let mut parts = Vec::with_capacity(list.len());
    for (i, question) in list.iter().enumerate() {
        if question.is_null() || is_undefined(question) {
            return Err(member_error(question, "title"));
        }
        let title = match get(question, "title") {
            Value::String(s) => s,
            other => js_str(&other),
        };
        let answer = match &answers {
            Value::Array(items) => match items.get(i) {
                Some(v) if !v.is_null() && !is_undefined(v) => js_str(v),
                _ => String::new(),
            },
            Value::String(s) => s
                .encode_utf16()
                .nth(i)
                .and_then(|u| char::from_u32(u as u32))
                .map(String::from)
                .unwrap_or_default(),
            Value::Null => return Err("Cannot read properties of null (reading '0')".to_string()),
            v if is_undefined(v) => {
                return Err("Cannot read properties of undefined (reading '0')".to_string());
            }
            _ => String::new(),
        };
        parts.push(format!("{title}\n{answer}"));
    }
    Ok(parts.join("\n\n"))
}
