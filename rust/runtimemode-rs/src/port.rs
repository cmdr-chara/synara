//! The Rust port of `packages/shared/src/runtimeMode.ts` (clean-room port).

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn js_str(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        return "undefined".into();
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_str).collect::<Vec<_>>().join(","),
        _ => "[object Object]".into(),
    }
}

fn supports(provider: &Value) -> bool {
    matches!(provider.as_str(), Some("codex") | Some("claudeAgent"))
}

pub fn provider_supports_auto_runtime_mode(provider: Value) -> Result<bool, String> {
    Ok(supports(&provider))
}

pub fn unsupported_auto_runtime_mode_message(provider: Value) -> Result<String, String> {
    Ok(format!(
        "Provider \"{}\" does not support Auto runtime mode. Auto is available only for Codex and Claude Code.",
        js_str(&provider)
    ))
}

pub fn auto_runtime_mode_selection_issue(input: Value) -> Result<Option<String>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'runtimeMode')".into());
    }
    if crate::js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'runtimeMode')".into());
    }
    if input.get("runtimeMode").and_then(Value::as_str) != Some("auto") {
        return Ok(None);
    }
    let ms = input.get("modelSelection");
    match ms {
        None => return Err("Cannot read properties of undefined (reading 'provider')".into()),
        Some(v) if crate::js::is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'provider')".into());
        }
        Some(v) if v.is_null() => {
            return Err("Cannot read properties of null (reading 'provider')".into());
        }
        _ => {}
    }
    let Some(ms) = ms else { return Ok(None) };
    let provider = ms.get("provider").cloned().unwrap_or(Value::Null);
    if !supports(&provider) {
        return unsupported_auto_runtime_mode_message(provider).map(Some);
    }
    if provider.as_str() == Some("claudeAgent")
        && ms.get("supportsAutoMode") == Some(&Value::Bool(false))
    {
        let model = ms.get("model").map_or("undefined".to_string(), js_str);
        return Ok(Some(format!(
            "Claude model \"{model}\" does not support Auto mode."
        )));
    }
    Ok(None)
}

pub fn runtime_mode_escalates_privilege(
    caller_runtime_mode: Value,
    target_runtime_mode: Value,
) -> Result<bool, String> {
    fn rank(v: &Value) -> i32 {
        match v.as_str() {
            Some("approval-required") => 0,
            Some("auto") => 1,
            Some("full-access") => 2,
            _ => -1,
        }
    }
    let (a, b) = (rank(&caller_runtime_mode), rank(&target_runtime_mode));
    Ok(a >= 0 && b > a)
}

pub fn normalize_runtime_mode_for_provider(
    runtime_mode: Value,
    provider: Value,
) -> Result<Value, String> {
    if runtime_mode.as_str() == Some("auto") && !supports(&provider) {
        return Ok(json!("approval-required"));
    }
    Ok(runtime_mode)
}
