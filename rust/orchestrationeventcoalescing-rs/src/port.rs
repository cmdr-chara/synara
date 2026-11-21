//! The Rust port of `apps/web/src/orchestrationEventCoalescing.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use crate::js;

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !js::is_undefined(v),
        _ => !js::as_number(v).is_some_and(|n| n == 0.0 || n.is_nan()),
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| strict_eq(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| strict_eq(v, w)))
        }
        _ => match (js::as_number(a), js::as_number(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
    }
}

fn js_repr(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|x| match x {
                Value::Null => String::new(),
                Value::Object(o) if js::is_undefined(&Value::Object(o.clone())) => String::new(),
                _ => js_repr(x),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::Object(o) => {
            if js::is_undefined(v) {
                "undefined".to_string()
            } else if let Some(m) = o.get("$num").and_then(|x| x.as_str()) {
                m.to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn is_nullish(v: Option<&Value>) -> bool {
    v.is_none_or(|x| x.is_null() || js::is_undefined(x))
}

fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Null => Value::Null,
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

/// JS `+`: string concatenation when either operand stringifies, else numeric add.
fn js_add(a: &Value, b: &Value) -> Value {
    let a_str = matches!(a, Value::String(_));
    let b_str = matches!(b, Value::String(_));
    if a_str || b_str {
        return Value::String(format!("{}{}", js_repr(a), js_repr(b)));
    }
    let an = js::as_number(a).unwrap_or(f64::NAN);
    let bn = js::as_number(b).unwrap_or(f64::NAN);
    js::number(an + bn)
}

fn merge_message_sent(previous: &Value, event: &Value) -> Value {
    let prev_p = member(previous, "payload");
    let ev_p = member(event, "payload");
    let mut payload = match &ev_p {
        Value::Object(o) => o.clone(),
        _ => Map::new(),
    };
    for key in ["attachments", "skills", "mentions"] {
        if is_nullish(payload.get(key)) {
            let prev_v = match &prev_p {
                Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
                _ => js::undefined(),
            };
            if js::is_undefined(&prev_v) {
                payload.remove(key);
            } else {
                payload.insert(key.to_string(), prev_v);
            }
        }
    }
    if let Value::Object(o) = &prev_p {
        for key in ["segmentStartedAt", "segmentSequence"] {
            if let Some(v) = o.get(key).filter(|v| !js::is_undefined(v)) {
                payload.insert(key.to_string(), v.clone());
            }
        }
        if let Some(v) = o.get("createdAt") {
            payload.insert("createdAt".to_string(), v.clone());
        }
    }
    let text = js_add(&member(&prev_p, "text"), &member(&ev_p, "text"));
    payload.insert("text".to_string(), text);
    let mut out = match event {
        Value::Object(o) => o.clone(),
        _ => Map::new(),
    };
    out.insert("payload".to_string(), Value::Object(payload));
    Value::Object(out)
}

/// Port of `coalesceOrchestrationUiEvents` (`apps/web/src/orchestrationEventCoalescing.ts:45`).
pub fn coalesce_orchestration_ui_events(
    events: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let Value::Array(list) = &events else {
        return Err("events is not iterable".to_string());
    };
    let mut coalesced: Vec<Value> = Vec::new();
    let mut last_slot: Vec<(Value, usize)> = Vec::new();
    for event in list {
        match event {
            Value::Null => {
                return Err("Cannot read properties of null (reading 'payload')".to_string());
            }
            Value::Object(_) if js::is_undefined(event) => {
                return Err("Cannot read properties of undefined (reading 'payload')".to_string());
            }
            _ => {}
        }
        let payload = member(event, "payload");
        if !matches!(&payload, Value::Object(_) | Value::Array(_)) || js::is_undefined(&payload) {
            let repr = js_repr(&payload);
            return Err(format!(
                "Cannot use 'in' operator to search for 'threadId' in {repr}"
            ));
        }
        // `"threadId" in payload`: own key on objects; never true for arrays.
        let has_thread = matches!(&payload, Value::Object(o) if o.contains_key("threadId"));
        if !has_thread {
            last_slot.clear();
        } else {
            let thread_id = member(&payload, "threadId");
            let slot = last_slot
                .iter()
                .find(|(k, _)| {
                    strict_eq(k, &thread_id) || {
                        js::as_number(k).is_some_and(|n| n.is_nan())
                            && js::as_number(&thread_id).is_some_and(|n| n.is_nan())
                    }
                })
                .map(|(_, i)| *i);
            let previous = slot.map(|i| coalesced[i].clone());
            let prev_payload = previous.as_ref().map(|p| member(p, "payload"));
            let merge = member(event, "type").as_str() == Some("thread.message-sent")
                && previous
                    .as_ref()
                    .is_some_and(|p| member(p, "type").as_str() == Some("thread.message-sent"))
                && prev_payload.as_ref().is_some_and(|p| {
                    strict_eq(&member(p, "messageId"), &member(&payload, "messageId"))
                        && strict_eq(&member(p, "turnId"), &member(&payload, "turnId"))
                        && strict_eq(&member(p, "role"), &member(&payload, "role"))
                })
                && prev_payload
                    .as_ref()
                    .is_some_and(|p| truthy(&member(p, "streaming")))
                && truthy(&member(&payload, "streaming"))
                && js::is_undefined(&member(&payload, "segmentStartedAt"));
            if merge && let Some(i) = slot {
                coalesced[i] = merge_message_sent(previous.as_ref().unwrap_or(&Value::Null), event);
                continue;
            }
            last_slot.retain(|(k, _)| !strict_eq(k, &thread_id));
            last_slot.push((thread_id, coalesced.len()));
        }
        coalesced.push(event.clone());
    }
    Ok(coalesced)
}
