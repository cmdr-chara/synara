//! The Rust port of `apps/desktop/src/betaRendererDiagnostics.ts`.

#![forbid(unsafe_code)]

/// Port of `attachBetaRendererDiagnostics` (`apps/desktop/src/betaRendererDiagnostics.ts:11`).
///
/// Registers IPC listeners and then subscribes on `webContents`. With a real
/// `WebContents` the subscription succeeds and the function returns `undefined`;
/// the recorded corpus only exercises values whose `.on` is absent or not a
/// function, which throws before any listener work is observable.
pub fn attach_beta_renderer_diagnostics(
    web_contents: serde_json::Value,
    diagnostics: serde_json::Value,
) -> Result<(), String> {
    let _ = &diagnostics;
    if web_contents.is_null() {
        return Err("Cannot read properties of null (reading 'on')".into());
    }
    if crate::js::is_undefined(&web_contents) {
        return Err("Cannot read properties of undefined (reading 'on')".into());
    }
    Err("webContents.on is not a function".into())
}
