//! The Rust port of `packages/shared/src/wsRequestClass.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

/// Port of `classifyWsRequest` (`packages/shared/src/wsRequestClass.ts:84`).
pub fn classify_ws_request(method: Value) -> Result<String, String> {
    let Some(m) = method.as_str() else {
        return Ok("standard".to_string());
    };
    Ok(match m {
        "orchestration.dispatchCommand"
        | "orchestration.listProviderDeliveryBlockers"
        | "orchestration.reconcileProviderDelivery" => "control",
        "orchestration.getSnapshot" | "orchestration.replayEvents" | "stats.getUsageAnalytics" => {
            "expensive-read"
        }
        _ => "standard",
    }
    .to_string())
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || crate::js::is_undefined(i) {
                    String::new()
                } else {
                    js_to_string(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => {
            if crate::js::is_undefined(v) {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// Port of `makeWsRequestCapacityError` (`packages/shared/src/wsRequestClass.ts:98`).
pub fn make_ws_request_capacity_error(request_class: Value) -> Result<Value, String> {
    Ok(json!({
        "$error": format!("WebSocket {} request capacity exceeded.", js_to_string(&request_class))
    }))
}
