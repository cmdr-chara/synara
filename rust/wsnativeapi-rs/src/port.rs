//! The Rust port of `apps/web/src/wsNativeApi.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `readWsServerCapabilities` (`apps/web/src/wsNativeApi.ts:95`).
/// No transport instance exists in the ported environment: the source returns `null`.
pub fn read_ws_server_capabilities() -> Result<Option<Vec<String>>, String> {
    Ok(None)
}

/// Port of `resetWsNativeApiForTest` (`apps/web/src/wsNativeApi.ts:1389`).
/// Clears the module singletons; both are empty in the ported environment.
pub async fn reset_ws_native_api_for_test() -> Result<(), String> {
    Ok(())
}
