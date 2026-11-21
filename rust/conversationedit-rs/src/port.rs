//! The Rust port of `packages/shared/src/conversationEdit.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn truthy(v: &Value) -> bool {
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

fn get_messages<'a>(input: &'a Value, reader: &str) -> Result<&'a Vec<Value>, String> {
    if input.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'messages')".into());
    }
    match input.get("messages") {
        None => Err(format!(
            "TypeError: Cannot read properties of undefined (reading '{reader}')"
        )),
        Some(Value::Null) => Err(format!(
            "TypeError: Cannot read properties of null (reading '{reader}')"
        )),
        Some(Value::Array(m)) => Ok(m),
        Some(_) => Err("TypeError: input.messages.findIndex is not a function".into()),
    }
}

/// JS `===` for our purposes: scalars compare by value (NaN never equal),
/// arrays/objects only by identity, which two decoded values never share.
fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Array(_), _) | (Value::Object(_), _) => false,
        (_, Value::Array(_)) | (_, Value::Object(_)) => false,
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().zip(y.as_f64()).is_some_and(|(x, y)| x == y)
        }
        _ => a == b,
    }
}

/// `messages.findIndex(m => m.id === want)` — stops at first match; a null
/// element throws only if reached before the match.
fn find_id(messages: &[Value], want: Option<&Value>) -> Result<Option<usize>, String> {
    for (i, m) in messages.iter().enumerate() {
        if m.is_null() {
            return Err("TypeError: Cannot read properties of null (reading 'id')".into());
        }
        if m.get("id")
            .is_some_and(|id| want.is_some_and(|w| strict_eq(id, w)))
        {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

/// Unique truthy `turnId`s from the message at `start` through the tail.
fn tail_turn_ids(messages: &[Value], start: usize) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for m in &messages[start..] {
        if let Some(t) = m.get("turnId")
            && truthy(t)
            && !out.contains(t)
        {
            out.push(t.clone());
        }
    }
    out
}

/// Port of `collectTailTurnIds` (`packages/shared/src/conversationEdit.ts:11`).
pub fn collect_tail_turn_ids(input: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    let messages = get_messages(&input, "findIndex")?;
    Ok(find_id(messages, input.get("messageId"))?
        .map(|i| tail_turn_ids(messages, i))
        .unwrap_or_default())
}

fn result(editable: bool, rest: Value) -> Value {
    let mut o = json!({ "editable": editable });
    if let Value::Object(extra) = rest {
        o.as_object_mut().unwrap().extend(extra);
    }
    o
}

fn resolve(
    input: &Value,
    messages: &[Value],
    target_index: Option<usize>,
) -> Result<Value, String> {
    let index = match target_index {
        Some(i) => Some(i),
        None => match input.get("messageId") {
            Some(id) if !crate::js::is_undefined(id) => find_id(messages, Some(id))?,
            _ => None,
        },
    };
    let index = index.unwrap_or(usize::MAX);
    let Some(message) = messages.get(index) else {
        return Ok(result(false, json!({ "reason": "missing-message" })));
    };
    if target_index.is_none() && message.get("role").and_then(|v| v.as_str()) != Some("user") {
        return Ok(result(false, json!({ "reason": "not-user-message" })));
    }
    if message.get("source").and_then(|v| v.as_str()) != Some("native") {
        return Ok(result(false, json!({ "reason": "non-native-message" })));
    }
    let linked = messages.iter().any(|m| {
        m.get("asyncUserInput")
            .and_then(|a| a.get("response"))
            .and_then(|r| r.get("messageId"))
            .is_some_and(|id| message.get("id").is_some_and(|mid| strict_eq(id, mid)))
    });
    if linked {
        return Ok(result(false, json!({ "reason": "structured-answer" })));
    }
    if target_index.is_none() {
        let last_user = messages
            .iter()
            .rposition(|m| m.get("role").and_then(|v| v.as_str()) == Some("user"))
            .unwrap_or(usize::MAX);
        if last_user != index {
            return Ok(result(
                false,
                json!({ "reason": "not-latest-native-user-message" }),
            ));
        }
    }
    let removed = tail_turn_ids(messages, index);
    if removed.len() > 1 {
        return Ok(result(false, json!({ "reason": "spans-multiple-turns" })));
    }
    if removed.is_empty() {
        let active = input.get("activeTurnId");
        if !active.is_some_and(truthy) {
            return Ok(result(false, json!({ "reason": "missing-turn-metadata" })));
        }
        return Ok(result(
            true,
            json!({
                "messageId": message.get("id"),
                "messageIndex": index,
                "mode": "active-tail",
                "rollbackTurnCount": 0,
                "removedTurnIds": [],
            }),
        ));
    }
    Ok(result(
        true,
        json!({
            "messageId": message.get("id"),
            "messageIndex": index,
            "mode": "rollback",
            "rollbackTurnCount": removed.len(),
            "removedTurnIds": removed,
        }),
    ))
}

/// Port of `resolveTailUserMessageEditTarget` (`packages/shared/src/conversationEdit.ts:46`).
pub fn resolve_tail_user_message_edit_target(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let messages = get_messages(&input, "findIndex")?;
    resolve(&input, messages, None)
}

/// Port of `resolveLatestTailUserMessageEditTarget` (`packages/shared/src/conversationEdit.ts:84`).
pub fn resolve_latest_tail_user_message_edit_target(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'messages')".into());
    }
    let messages: Vec<Value> = match input.get("messages") {
        None => {
            return Err("TypeError: Cannot read properties of undefined (reading 'length')".into());
        }
        Some(Value::Null) => {
            return Err("TypeError: Cannot read properties of null (reading 'length')".into());
        }
        Some(Value::Array(m)) => m.clone(),
        Some(Value::String(s)) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        Some(_) => Vec::new(),
    };
    let Some(last_user) = messages
        .iter()
        .rposition(|m| m.get("role").and_then(|v| v.as_str()) == Some("user"))
    else {
        return Ok(result(false, json!({ "reason": "missing-message" })));
    };
    resolve(&input, &messages, Some(last_user))
}
