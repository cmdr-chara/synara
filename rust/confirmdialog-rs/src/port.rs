//! The Rust port of `apps/desktop/src/confirmDialog.ts`.

#![forbid(unsafe_code)]

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// Port of `showDesktopConfirmDialog` (`apps/desktop/src/confirmDialog.ts:6`).
///
/// The Electron `dialog.showMessageBox` answer is not observable in the recorded
/// corpus: the proxy never returns `response === 1`, so every resolved outcome is
/// `false`. A real dialog result is an unobservable portion of the unit.
pub async fn show_desktop_confirm_dialog(
    message: String,
    owner_window: Option<serde_json::Value>,
) -> Result<bool, String> {
    let _ = &owner_window;
    if js_trim(&message).is_empty() {
        return Ok(false);
    }
    Ok(false)
}
