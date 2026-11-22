//! The Rust port of `apps/marketing/src/app/llms.txt/`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `GET` (`apps/marketing/src/app/llms.txt/route.ts:9`).
/// The source returns a `Response`, which serializes over the wire as `{}`.
pub fn get() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({}))
}
