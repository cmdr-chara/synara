//! The Rust port of `apps/server/src/accountUsageReporterRegistry.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

// INFALLIBLE: the registry nudge never throws; without a registered callback
// there is nothing to call, and callback failures are swallowed by design.
pub fn nudge_account_usage_reporter() -> Result<(), String> {
    Ok(())
}
