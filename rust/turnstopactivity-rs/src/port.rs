//! The Rust port of `packages/shared/src/turnStopActivity.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `deriveTurnStopActivity` (`packages/shared/src/turnStopActivity.ts:5`).
pub fn derive_turn_stop_activity(
    event: serde_json::Value,
    active_turn_id: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let _ = (&event, &active_turn_id);
    unimplemented!("port `deriveTurnStopActivity` from packages/shared/src/turnStopActivity.ts")
}
