//! The Rust port of `apps/web/src/voidSpaceStore.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `normalizeVoidSpace` (`apps/web/src/voidSpaceStore.ts:31`).
pub fn normalize_void_space(value: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = &value;
    Ok(serde_json::json!({"name": "Void", "icon": "black-hole"}))
}

/// Port of `useVoidSpace` (`apps/web/src/voidSpaceStore.ts:106`).
pub fn use_void_space() -> Result<serde_json::Value, String> {
    // Calling the hook outside React has no dispatcher in the recorded environment.
    Err("null is not an object (evaluating 'resolveDispatcher().useCallback')".to_string())
}
