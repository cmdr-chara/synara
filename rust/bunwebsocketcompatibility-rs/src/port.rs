//! The Rust port of `apps/server/src/bunWebSocketCompatibility.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `patchBunWebSocketCloseEventCompatibility` (`apps/server/src/bunWebSocketCompatibility.ts:38`).
pub fn patch_bun_web_socket_close_event_compatibility(
    _env: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(crate::js::undefined())
}
