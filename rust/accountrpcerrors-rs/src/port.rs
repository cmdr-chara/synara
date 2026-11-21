//! The Rust port of `apps/server/src/accountRpcErrors.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `unwrapTryPromiseError` (`apps/server/src/accountRpcErrors.ts:16`).
pub fn unwrap_try_promise_error(cause: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(cause)
}
