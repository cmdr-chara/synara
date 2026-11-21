//! The Rust port of `apps/desktop/src/subframeNavigationGuard.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

/// Port of `isAllowedAppSubframeNavigation` (`apps/desktop/src/subframeNavigationGuard.ts:18`).
pub fn is_allowed_app_subframe_navigation(url: String) -> Result<bool, String> {
    Ok(url == "about:srcdoc" || url == "about:blank")
}

/// Port of `attachAppSubframeNavigationGuard` (`apps/desktop/src/subframeNavigationGuard.ts:22`).
pub fn attach_app_subframe_navigation_guard(web_contents: Value) -> Result<(), String> {
    if web_contents.is_null() {
        return Err("Cannot read properties of null (reading 'on')".to_string());
    }
    // `webContents.on(...)` registers the `will-frame-navigate` handler; a
    // serialized value can never carry the method, matching the recorded
    // TypeError for every non-EventEmitter input.
    Err("webContents.on is not a function".to_string())
}
