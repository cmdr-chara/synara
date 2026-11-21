//! The Rust port of `packages/shared/src/desktopTitleBar.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn supports(platform: &str) -> bool {
    platform == "win32" || platform == "linux"
}

/// Port of `supportsCustomTitleBar` (`packages/shared/src/desktopTitleBar.ts:10`).
pub fn supports_custom_title_bar(platform: String) -> Result<bool, String> {
    Ok(supports(&platform))
}

/// Port of `defaultCustomTitleBarPreference` (`packages/shared/src/desktopTitleBar.ts:19`).
pub fn default_custom_title_bar_preference(platform: String) -> Result<bool, String> {
    Ok(supports(&platform))
}

/// Port of `resolveCustomTitleBarActive` (`packages/shared/src/desktopTitleBar.ts:27`).
pub fn resolve_custom_title_bar_active(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() || crate::js::is_undefined(&input) {
        return Err(format!(
            "Cannot read properties of {} (reading 'platform')",
            if crate::js::is_undefined(&input) {
                "undefined"
            } else {
                "null"
            }
        ));
    }
    let supported = input
        .get("platform")
        .and_then(Value::as_str)
        .is_some_and(supports);
    if !supported {
        return Ok(Value::Bool(false));
    }
    match input.get("preference") {
        Some(Value::Null) => Ok(Value::Bool(true)),
        Some(v) if !crate::js::is_undefined(v) => Ok(v.clone()),
        _ => Ok(crate::js::undefined()),
    }
}
