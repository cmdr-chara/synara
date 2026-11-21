//! The Rust port of `packages/shared/src/automationCompletionPolicy.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn is_undefined_value(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// Port of `completionPolicyFromStopWhen` (`packages/shared/src/automationCompletionPolicy.ts`).
pub fn completion_policy_from_stop_when(stop_when: String) -> Result<serde_json::Value, String> {
    let trimmed = js_trim(&stop_when);
    if trimmed.is_empty() {
        return Ok(json!({ "type": "none" }));
    }
    Ok(json!({
        "type": "ai-evaluated",
        "stopWhen": trimmed,
        "confidenceThreshold": 0.8,
    }))
}

/// Port of `stopWhenFromCompletionPolicy` (`packages/shared/src/automationCompletionPolicy.ts`).
pub fn stop_when_from_completion_policy(
    policy: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if policy.is_null() {
        return Err("Cannot read properties of null (reading 'type')".to_string());
    }
    if is_undefined_value(&policy) {
        return Err("Cannot read properties of undefined (reading 'type')".to_string());
    }
    if policy.get("type") != Some(&Value::String("ai-evaluated".to_string())) {
        return Ok(Some(Value::String(String::new())));
    }
    Ok(match policy.get("stopWhen") {
        None => None,
        Some(v) => Some(v.clone()),
    })
}
