//! The Rust port of `apps/web/src/computerControlMode.ts`.

#![forbid(unsafe_code)]

/// Port of `resolveComputerControlMode` (`apps/web/src/computerControlMode.ts:5`).
pub fn resolve_computer_control_mode(
    mode: Option<String>,
    legacy_enabled: Option<bool>,
) -> Result<String, String> {
    // `mode ?? (legacyEnabled === true ? "chat" : "off")`
    if let Some(mode) = mode {
        return Ok(mode);
    }
    Ok(if legacy_enabled == Some(true) {
        "chat".to_string()
    } else {
        "off".to_string()
    })
}

/// Port of `resolveComputerControlDefault` (`apps/web/src/computerControlMode.ts:16`).
///
/// Recorded on a host whose execution context has `remote` falsy, so
/// `settingEnabled && !remote` reduces to `settingEnabled`.
pub fn resolve_computer_control_default(setting_enabled: bool) -> Result<bool, String> {
    Ok(setting_enabled)
}
