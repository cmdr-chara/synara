//! The Rust port of `apps/server/src/codexServiceTier.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// Port of `resolveCodexServiceTier` (`apps/server/src/codexServiceTier.ts:3`).
pub fn resolve_codex_service_tier(
    model_selection: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let provider = model_selection
        .as_ref()
        .and_then(|v| v.get("provider"))
        .and_then(Value::as_str);
    if provider != Some("codex") {
        return Ok(js::undefined());
    }
    let options = model_selection
        .as_ref()
        .and_then(|v| v.get("options"))
        .filter(|o| !o.is_null());
    let service_tier = options
        .and_then(|o| o.get("serviceTier"))
        .and_then(Value::as_str);
    if service_tier == Some("ultrafast") {
        return Ok(Value::String("ultrafast".to_string()));
    }
    let fast_mode = options.and_then(|o| o.get("fastMode"));
    match fast_mode {
        None => Ok(js::undefined()),
        Some(v) if js::is_undefined(v) => Ok(js::undefined()),
        Some(v) => Ok(Value::String(
            if truthy(v) { "fast" } else { "default" }.to_string(),
        )),
    }
}
