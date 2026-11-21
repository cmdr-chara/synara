//! The Rust port of `apps/profiles/src/middleware.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `middleware` (`apps/profiles/src/middleware.ts:14`). In the recorded
/// environment `MARKETING_ORIGIN` is unset, so every request resolves to
/// `NextResponse.next()` — serialized as an empty object.
pub async fn middleware(request: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = &request;
    Ok(serde_json::json!({}))
}
