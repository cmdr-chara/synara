//! Live browser-frame serving for `/api/browser/frame`: the native shell's
//! browser panel polls this endpoint and paints the bytes. Frames come from
//! the same CDP session the agent's browser tools use, keyed by thread id —
//! no separate browser instance, so the panel always shows what the agent
//! sees. When no Chrome is available or no tab exists for the thread, the
//! endpoint answers 204 so the panel can show its empty state instead.

use axum::{
    extract::RawQuery,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use base64::Engine;
use serde_json::Value;

pub async fn browser_frame(RawQuery(query): RawQuery) -> Response {
    // Thread ids are URL-safe, so plain query splitting needs no decoding.
    let thread = query
        .as_deref()
        .and_then(|q| {
            q.split('&').find_map(|pair| {
                pair.strip_prefix("thread_id=")
                    .or_else(|| pair.strip_prefix("thread="))
            })
        })
        .unwrap_or_default()
        .to_string();
    if !crate::browser_cdp::cdp_available() {
        return StatusCode::NO_CONTENT.into_response();
    }
    let result = crate::browser_cdp::call_cdp_tool(
        "browser_screenshot",
        &Value::Object(serde_json::Map::new()),
        if thread.is_empty() { None } else { Some(thread.as_str()) },
    )
    .await;
    let value = match result {
        Ok(v) => v,
        Err(_) => return StatusCode::NO_CONTENT.into_response(),
    };
    let b64 = value
        .get("image")
        .and_then(|img| img.get("data"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if b64.is_empty() {
        return StatusCode::NO_CONTENT.into_response();
    }
    let bytes = match base64::engine::general_purpose::STANDARD.decode(b64) {
        Ok(b) => b,
        Err(_) => return StatusCode::NO_CONTENT.into_response(),
    };
    (
        [(header::CONTENT_TYPE, "image/png"), (header::CACHE_CONTROL, "no-store")],
        bytes,
    )
        .into_response()
}
