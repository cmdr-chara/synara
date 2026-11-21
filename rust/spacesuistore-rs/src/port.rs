//! The Rust port of `apps/web/src/spacesUiStore.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `readActiveSpaceId` (`apps/web/src/spacesUiStore.ts:172`).
pub fn read_active_space_id() -> Result<Option<serde_json::Value>, String> {
    Ok(None)
}
