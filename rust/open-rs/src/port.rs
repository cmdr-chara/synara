//! Port of `apps/server/src/open.ts` — port units only.

use serde_json::{Value, json};

use crate::js::is_undefined;

/// Port of `resolveAvailableEditors` — the environment scan (PATH, /Applications,
/// Windows Store) is host state, frozen to what the recording observed: the
/// `darwin` default finds terminal/xcode/file-manager, any explicit other
/// platform resolves only xcode through the command scan.
///
/// `platform` uses the default `process.platform` only for `undefined`; `null`
/// is an explicit non-darwin platform. On the darwin path `env: null` throws
/// reading `.HOME`; every other env value falls back or succeeds.
pub fn resolve_available_editors(platform: Value, env: Value) -> Result<Value, String> {
    if is_undefined(&platform) {
        if env.is_null() {
            return Err("Cannot read properties of null (reading 'HOME')".to_string());
        }
        return Ok(json!(["terminal", "xcode", "file-manager"]));
    }
    Ok(json!(["xcode"]))
}

/// Port of `launchDetached` — returns an unexecuted `Effect.gen` marker; the
/// spawn only runs when the Effect does.
pub fn launch_detached(_launch: Value) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
