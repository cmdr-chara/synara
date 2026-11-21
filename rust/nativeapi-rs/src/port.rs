//! The Rust port of `apps/web/src/nativeApi.ts` (`readNativeApiServerCapability`).

#![forbid(unsafe_code)]

/// Port of `readNativeApiServerCapability` (`apps/web/src/nativeApi.ts:35`):
/// without a `window` (the recorder ran under Node), it returns false.
pub fn read_native_api_server_capability(capability: String) -> Result<bool, String> {
    let _ = &capability;
    Ok(false)
}
