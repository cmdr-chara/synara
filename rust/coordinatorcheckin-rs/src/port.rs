//! The Rust port of `packages/shared/src/coordinatorCheckin.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn js_ws(c: char) -> bool {
    (c.is_whitespace() || c == '\u{feff}') && c != '\u{85}'
}

fn nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

/// Port of `isSilentCoordinatorCheckinReply` (`packages/shared/src/coordinatorCheckin.ts:9`).
pub fn is_silent_coordinator_checkin_reply(text: String) -> Result<bool, String> {
    let t = text.trim_matches(js_ws);
    if t.is_empty() {
        return Ok(true);
    }
    let Some(rest) = t.get(6..) else {
        return Ok(false);
    };
    if !t[..6].eq_ignore_ascii_case("silent") {
        return Ok(false);
    }
    Ok(rest.chars().all(|c| !c.is_alphanumeric()))
}

/// Port of `isCoordinatorCheckinUserMessage` (`packages/shared/src/coordinatorCheckin.ts:23`).
pub fn is_coordinator_checkin_user_message(message: serde_json::Value) -> Result<bool, String> {
    if message.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'role')".into());
    }
    if crate::js::is_undefined(&message) {
        return Err("TypeError: Cannot read properties of undefined (reading 'role')".into());
    }
    Ok(message.get("role").and_then(|v| v.as_str()) == Some("user")
        && message.get("dispatchOrigin").and_then(|v| v.as_str()) == Some("automation"))
}

fn messages_of<'a>(input: &'a Value, reader: &str) -> Result<&'a Vec<Value>, String> {
    if input.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'messages')".into());
    }
    match input.get("messages") {
        Some(Value::Array(m)) => Ok(m),
        None => Err(format!(
            "TypeError: Cannot read properties of undefined (reading '{reader}')"
        )),
        Some(Value::Null) => Err(format!(
            "TypeError: Cannot read properties of null (reading '{reader}')"
        )),
        Some(_) => Err(format!(
            "TypeError: Cannot read properties of undefined (reading '{reader}')"
        )),
    }
}

/// `a === b` where nullish values are all equal to each other (null/undefined).
fn turn_eq(a: &Value, b: &Value) -> bool {
    (nullish(a) && nullish(b)) || a == b
}

/// Port of `coordinatorCheckinTurnReport` (`packages/shared/src/coordinatorCheckin.ts:43`).
pub fn coordinator_checkin_turn_report(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let messages = messages_of(&input, "findIndex")?;
    let turn_id = input
        .get("turnId")
        .cloned()
        .unwrap_or_else(|| json!({ "$undefined": true }));
    let checkin = messages.iter().position(|m| {
        m.get("role").and_then(|v| v.as_str()) == Some("user")
            && m.get("dispatchOrigin").and_then(|v| v.as_str()) == Some("automation")
            && turn_eq(m.get("turnId").unwrap_or(&Value::Null), &turn_id)
    });
    let Some(ci) = checkin else { return Ok(None) };
    let reply = messages[ci + 1..].iter().find(|m| {
        m.get("role").and_then(|v| v.as_str()) == Some("assistant")
            && turn_eq(m.get("turnId").unwrap_or(&Value::Null), &turn_id)
    });
    let Some(reply) = reply else {
        return Ok(Some(json!({ "silent": true, "replyText": null })));
    };
    let text = reply.get("text").cloned().unwrap_or(Value::Null);
    let silent = match &text {
        Value::String(s) => is_silent_coordinator_checkin_reply(s.clone())?,
        Value::Null => {
            return Err("TypeError: Cannot read properties of null (reading 'trim')".into());
        }
        _ => return Err("TypeError: text.trim is not a function".into()),
    };
    Ok(Some(json!({ "silent": silent, "replyText": text })))
}

/// Port of `suppressCoordinatorCheckinMessages` (`packages/shared/src/coordinatorCheckin.ts:73`).
pub fn suppress_coordinator_checkin_messages(
    messages: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let messages = match messages {
        Value::Array(m) => m,
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("TypeError: messages is not iterable".into()),
    };
    let mut kept: Vec<Value> = Vec::new();
    let mut turn_ids: Vec<Value> = Vec::new();
    let mut open: Option<Value> = None;
    for m in &messages {
        let role = m.get("role").and_then(|v| v.as_str());
        if role == Some("user")
            && m.get("dispatchOrigin").and_then(|v| v.as_str()) == Some("automation")
        {
            let tid = m.get("turnId").cloned().unwrap_or(Value::Null);
            if !nullish(&tid) && !turn_ids.contains(&tid) {
                turn_ids.push(tid.clone());
            }
            open = Some(tid);
            continue;
        }
        if let Some(tid) = &open
            && role == Some("assistant")
            && turn_eq(m.get("turnId").unwrap_or(&Value::Null), tid)
        {
            let silent = match m.get("text") {
                Some(Value::String(s)) => is_silent_coordinator_checkin_reply(s.clone())?,
                _ => false,
            };
            if !silent {
                kept.push(m.clone());
            }
            open = None;
            continue;
        }
        kept.push(m.clone());
    }
    Ok(json!({ "messages": kept, "checkinTurnIds": { "$set": turn_ids } }))
}
