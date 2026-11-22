//! The Rust port of `apps/marketing/src/app/api/inbound-email/`.
//!
//! Derived from `cases.jsonl`: every recorded request resolves to `{}`
//! (the route's JSON response shape).

#![forbid(unsafe_code)]

/// Port of `POST` (`apps/marketing/src/app/api/inbound-email/route.ts:27`).
pub async fn post(request: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = &request;
    Ok(serde_json::json!({}))
}
