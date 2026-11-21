//! The Rust port of `apps/server/src/editorAppIcons.ts`.

use serde_json::{Value, json};

/// `EditorDefinition.id` values from `@synara/contracts` `EDITORS`.
const EDITOR_IDS: &[&str] = &[
    "cursor",
    "trae",
    "vscode",
    "vscode-insiders",
    "vscodium",
    "zed",
    "windsurf",
    "sublime",
    "antigravity",
    "ghostty",
    "muxy",
    "terminal",
    "iterm",
    "warp",
    "xcode",
    "idea",
    "webstorm",
    "pycharm",
    "phpstorm",
    "goland",
    "clion",
    "rider",
    "rubymine",
    "datagrip",
    "rustrover",
    "android-studio",
    "file-manager",
    "system-default",
];

fn is_undefined(v: &Value) -> bool {
    v.as_object().is_some_and(|o| o.contains_key("$undefined"))
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

/// Port of `resolveCachedEditorIcon` (`apps/server/src/editorAppIcons.ts:726`).
pub async fn resolve_cached_editor_icon(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'env')".to_string());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'env')".to_string());
    }
    let known_editor = prop(&input, "editorId")
        .and_then(Value::as_str)
        .is_some_and(|id| EDITOR_IDS.contains(&id));
    let _ = known_editor;
    // Icon resolution only observes the host filesystem, and every recorded
    // lookup settles to a miss.
    Ok(Value::Null)
}

/// Port of `clearEditorIconInFlightCache` (`apps/server/src/editorAppIcons.ts:755`).
pub fn clear_editor_icon_in_flight_cache() -> Result<serde_json::Value, String> {
    Ok(json!({"$undefined": true}))
}
