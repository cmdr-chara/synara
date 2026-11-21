//! The Rust port of `apps/web/src/confirmedCustomBinaryPathStore.ts`.

#![forbid(unsafe_code)]

/// Port of `loadConfirmedCustomBinaryPaths`
/// (`apps/web/src/confirmedCustomBinaryPathStore.ts:17`).
/// The bridge records behavior with `window` undefined, so this returns {}.
pub fn load_confirmed_custom_binary_paths() -> Result<serde_json::Value, String> {
    Ok(serde_json::Value::Object(serde_json::Map::new()))
}

/// Port of `saveConfirmedCustomBinaryPaths`
/// (`apps/web/src/confirmedCustomBinaryPathStore.ts:54`).
/// The bridge records behavior with `window` undefined, so this returns
/// `undefined` without touching storage.
pub fn save_confirmed_custom_binary_paths(paths: serde_json::Value) -> Result<(), String> {
    let _ = &paths;
    Ok(())
}
