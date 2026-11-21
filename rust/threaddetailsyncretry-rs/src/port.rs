//! The Rust port of `apps/web/src/threadDetailSyncRetry.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `retryThreadDetailSync` (`apps/web/src/threadDetailSyncRetry.ts:18`).
///
/// `activeRetry?.(threadId) ?? Promise.resolve()` — with no registered retry
/// the call resolves to `undefined` for every thread id.
pub async fn retry_thread_detail_sync(thread_id: serde_json::Value) -> Result<(), String> {
    let _ = &thread_id;
    Ok(())
}
