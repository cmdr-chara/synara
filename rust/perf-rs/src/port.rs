//! The Rust port of `apps/api/src/perf.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `isPerfEnabled` (`apps/api/src/perf.ts:18`).
pub fn is_perf_enabled(env: serde_json::Value) -> Result<bool, String> {
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'SYNARA_API_PERF')".to_string());
    }
    Ok(matches!(env.get("SYNARA_API_PERF"), Some(serde_json::Value::String(s)) if s == "1"))
}
