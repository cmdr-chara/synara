//! The Rust port of `apps/marketing/src/app/llms-full.txt/`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `GET` (`apps/marketing/src/app/llms-full.txt/route.ts:9`).
pub async fn get() -> Result<serde_json::Value, String> {
    Err("getText('processed') requires `includeProcessedMarkdown` to be enabled in your collection config.".to_string())
}
