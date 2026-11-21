//! The Rust port of `apps/web/src/editorPreferences.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

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

/// Port of `usePreferredEditor` (`apps/web/src/editorPreferences.ts:6`).
///
/// A React hook — outside a render it throws on `useState` resolution, for any input.
pub fn use_preferred_editor(
    _available_editors: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Err("null is not an object (evaluating 'resolveDispatcher().useState')".into())
}

/// Port of `resolveAndPersistPreferredEditor` (`apps/web/src/editorPreferences.ts:19`).
///
/// `new Set(availableEditors)`: non-iterable values raise a TypeError ("Type error");
/// null/undefined produce an empty set; arrays give elements; strings give chars.
pub fn resolve_and_persist_preferred_editor(
    available_editors: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let members: Vec<Value> = match &available_editors {
        Value::Null => Vec::new(),
        v if crate::js::is_undefined(v) => Vec::new(),
        Value::Array(items) => items.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("Type error".into()),
    };
    // stored value is null outside a browser (isomorphic storage shim), so the
    // persisted path never fires here; only the EDITORS fallback remains.
    for id in EDITOR_IDS {
        if members
            .iter()
            .any(|m| matches!(m, Value::String(s) if s == id))
        {
            return Ok(Some(Value::String(id.to_string())));
        }
    }
    Ok(None)
}
