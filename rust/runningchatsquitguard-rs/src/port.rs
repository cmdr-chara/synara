//! The Rust port of `apps/desktop/src/runningChatsQuitGuard.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.get("$undefined").and_then(Value::as_bool) == Some(true))
}

/// `value == null` (null or undefined).
fn nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// `typeof v === "object" && v != null` (JS arrays count as objects).
fn is_object_like(v: &Value) -> bool {
    !nullish(v) && matches!(v, Value::Object(_) | Value::Array(_))
}

/// Port of `shouldPromptForRunningChatsBeforeQuit` (`apps/desktop/src/runningChatsQuitGuard.ts:15`).
pub fn should_prompt_for_running_chats_before_quit(reason: String) -> Result<bool, String> {
    Ok(reason == "window-close" || reason == "before-quit")
}

/// Port of `quitConfirmationPresentationForPlatform` (`apps/desktop/src/runningChatsQuitGuard.ts:19`).
pub fn quit_confirmation_presentation_for_platform() -> Result<String, String> {
    Ok("in-app".into())
}

/// Port of `parseQuitConfirmationRequest` (`apps/desktop/src/runningChatsQuitGuard.ts:23`).
pub fn parse_quit_confirmation_request(
    payload: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if !is_object_like(&payload) {
        return Ok(None);
    }
    let request_id = payload
        .get("requestId")
        .cloned()
        .unwrap_or_else(|| json!(null));
    let request_id = match &request_id {
        Value::String(s) if !js_trim(s).is_empty() => s.clone(),
        _ => return Ok(None),
    };
    let presentation = match payload.get("presentation") {
        Some(Value::String(s)) if s == "native" => "native",
        _ => "in-app",
    };
    Ok(Some(json!({
        "requestId": request_id,
        "presentation": presentation,
    })))
}

fn parse_quit_confirmation_chats(value: &Value) -> Vec<Value> {
    let Value::Array(items) = value else {
        return Vec::new();
    };
    let mut chats = Vec::new();
    for item in items {
        if !is_object_like(item) {
            continue;
        }
        let id = item.get("id").cloned().unwrap_or(Value::Null);
        let id = match &id {
            Value::String(s) if !js_trim(s).is_empty() => s.clone(),
            _ => continue,
        };
        let title = match item.get("title") {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        };
        let mut chat = Map::new();
        chat.insert("id".into(), Value::String(id));
        chat.insert("title".into(), Value::String(title));
        chats.push(Value::Object(chat));
    }
    chats
}

/// Port of `parseQuitConfirmationResponse` (`apps/desktop/src/runningChatsQuitGuard.ts:62`).
pub fn parse_quit_confirmation_response(
    payload: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if !is_object_like(&payload) {
        return Ok(None);
    }
    let request_id = match payload.get("requestId") {
        Some(Value::String(s)) if !js_trim(s).is_empty() => s.clone(),
        _ => return Ok(None),
    };
    match payload.get("phase") {
        Some(Value::String(p)) if p == "ready" => {
            let running_count = match payload.get("runningCount") {
                Some(Value::Number(n)) if n.as_f64().is_some_and(|f| f.is_finite()) => n.clone(),
                _ => return Ok(None),
            };
            let chats = parse_quit_confirmation_chats(payload.get("chats").unwrap_or(&Value::Null));
            Ok(Some(json!({
                "requestId": request_id,
                "phase": "ready",
                "runningCount": running_count,
                "chats": chats,
            })))
        }
        Some(Value::String(p)) if p == "decision" => match payload.get("allow") {
            Some(Value::Bool(b)) => Ok(Some(json!({
                "requestId": request_id,
                "phase": "decision",
                "allow": *b,
            }))),
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}
