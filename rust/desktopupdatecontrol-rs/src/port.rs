//! The Rust port of `apps/desktop/src/desktopUpdateControl.ts`.

#![forbid(unsafe_code)]

/// Port of `supportsDesktopUpdateControl` (`apps/desktop/src/desktopUpdateControl.ts:15`).
pub fn supports_desktop_update_control(
    is_packaged: bool,
    platform: serde_json::Value,
) -> Result<bool, String> {
    Ok(is_packaged && platform == "darwin")
}
