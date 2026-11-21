//! The Rust port of `packages/shared/src/appSnapShortcut.ts`.
//!
//! Clean-room reimplementation: inferred solely from recorded cases.jsonl behavior.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

fn undefined() -> Value {
    json!({ "$undefined": true })
}

fn get(v: &Value, key: &str) -> Value {
    v.get(key).cloned().unwrap_or_else(undefined)
}

fn member_error(v: &Value, field: &str) -> String {
    if v.is_null() {
        format!("Cannot read properties of null (reading '{field}')")
    } else {
        format!("Cannot read properties of undefined (reading '{field}')")
    }
}

fn starts_with_error(v: &Value, receiver: &str) -> String {
    if v.is_null() {
        "Cannot read properties of null (reading 'startsWith')".to_string()
    } else if is_undefined(v) {
        "Cannot read properties of undefined (reading 'startsWith')".to_string()
    } else {
        format!("{receiver}.startsWith is not a function")
    }
}

/// Port of `isAppSnapShortcutModifier` (`packages/shared/src/appSnapShortcut.ts:111`).
pub fn is_app_snap_shortcut_modifier(value: serde_json::Value) -> Result<bool, String> {
    Ok(matches!(
        value.as_str(),
        Some("command" | "control" | "option" | "shift")
    ))
}

/// Port of `isAppSnapShortcutKey` (`packages/shared/src/appSnapShortcut.ts:115`).
pub fn is_app_snap_shortcut_key(value: serde_json::Value) -> Result<bool, String> {
    let Some(s) = value.as_str() else {
        return Ok(false);
    };
    if let Some(rest) = s.strip_prefix("Key")
        && rest.len() == 1
    {
        return Ok(rest.as_bytes()[0].is_ascii_uppercase());
    }
    if let Some(rest) = s.strip_prefix("Digit")
        && rest.len() == 1
    {
        return Ok(rest.as_bytes()[0].is_ascii_digit());
    }
    Ok(matches!(
        s,
        "Space" | "Tab" | "Enter" | "Escape" | "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight"
    ))
}

/// Port of `isAppSnapShortcut` (`packages/shared/src/appSnapShortcut.ts:119`).
pub fn is_app_snap_shortcut(value: serde_json::Value) -> Result<bool, String> {
    let Some(kind) = value.get("kind").and_then(Value::as_str) else {
        return Ok(false);
    };
    if kind == "both-option-keys" {
        return Ok(true);
    }
    if kind == "key-chord" {
        return Ok(is_app_snap_shortcut_modifier(get(&value, "modifier"))?
            && is_app_snap_shortcut_key(get(&value, "key"))?);
    }
    Ok(false)
}

/// Port of `appSnapModifierFromEventCode` (`packages/shared/src/appSnapShortcut.ts:130`).
pub fn app_snap_modifier_from_event_code(code: String) -> Result<Option<String>, String> {
    Ok(match code.as_str() {
        "MetaLeft" | "MetaRight" => Some("command".to_string()),
        "ControlLeft" | "ControlRight" => Some("control".to_string()),
        "AltLeft" | "AltRight" => Some("option".to_string()),
        "ShiftLeft" | "ShiftRight" => Some("shift".to_string()),
        _ => None,
    })
}

fn modifier_label_str(modifier: &Value) -> Value {
    match modifier.as_str() {
        Some("command") => json!("⌘ Command"),
        Some("control") => json!("⌃ Control"),
        Some("option") => json!("⌥ Option"),
        Some("shift") => json!("⇧ Shift"),
        _ => undefined(),
    }
}

fn key_label(code: &str) -> String {
    if let Some(rest) = code.strip_prefix("Key")
        && rest.len() == 1
    {
        return rest.to_string();
    }
    if let Some(rest) = code.strip_prefix("Digit")
        && rest.len() == 1
    {
        return rest.to_string();
    }
    match code {
        "ArrowUp" => "↑".to_string(),
        "ArrowDown" => "↓".to_string(),
        "ArrowLeft" => "←".to_string(),
        "ArrowRight" => "→".to_string(),
        "Enter" | "Return" => "Return".to_string(),
        "Escape" => "Esc".to_string(),
        _ => code.to_string(),
    }
}

fn key_label_of(key: &Value, receiver: &str) -> Result<Value, String> {
    match key.as_str() {
        Some(s) => Ok(json!(key_label(s))),
        None => Err(starts_with_error(key, receiver)),
    }
}

/// Port of `appSnapShortcutSystemConflict` (`packages/shared/src/appSnapShortcut.ts:139`).
pub fn app_snap_shortcut_system_conflict(
    chord: serde_json::Value,
) -> Result<Option<String>, String> {
    if chord.is_null() || is_undefined(&chord) {
        return Err(member_error(&chord, "modifier"));
    }
    let modifier = get(&chord, "modifier");
    let key = get(&chord, "key");
    if modifier.as_str() == Some("shift") {
        return Ok(Some(
            "⇧ combinations are used for typing and text selection — combine with ⌘, ⌃ or ⌥ instead."
                .to_string(),
        ));
    }
    if modifier.as_str() != Some("command") {
        return Ok(None);
    }
    match key.as_str() {
        Some("Tab") => Ok(Some("macOS uses ⌘ Tab to switch apps.".to_string())),
        Some("Space") => Ok(Some("macOS uses ⌘ Space for Spotlight.".to_string())),
        Some(k) if k.len() == 4 && k.starts_with("Key") => {
            let letter = k.strip_prefix("Key").unwrap_or_default();
            let action = match letter {
                "A" => "Select All",
                "C" => "Copy",
                "F" => "Find",
                "H" => "Hide",
                "M" => "Minimize",
                "N" => "New",
                "O" => "Open",
                "P" => "Print",
                "Q" => "Quit",
                "S" => "Save",
                "T" => "New Tab",
                "V" => "Paste",
                "W" => "Close Window",
                "X" => "Cut",
                "Z" => "Undo",
                _ => return Ok(None),
            };
            Ok(Some(format!("⌘ {letter} is {action} in almost every app.")))
        }
        _ => Ok(None),
    }
}

/// Port of `appSnapShortcutLabels` (`packages/shared/src/appSnapShortcut.ts:155`).
pub fn app_snap_shortcut_labels(shortcut: serde_json::Value) -> Result<serde_json::Value, String> {
    if shortcut.is_null() || is_undefined(&shortcut) {
        return Err(member_error(&shortcut, "kind"));
    }
    if get(&shortcut, "kind").as_str() == Some("both-option-keys") {
        return Ok(json!(["⌥ left", "⌥ right"]));
    }
    let modifier = modifier_label_str(&get(&shortcut, "modifier"));
    let key = key_label_of(&get(&shortcut, "key"), "code")?;
    Ok(json!([modifier, key]))
}

/// Port of `appSnapShortcutModifierLabel` (`packages/shared/src/appSnapShortcut.ts:160`).
pub fn app_snap_shortcut_modifier_label(
    modifier: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(modifier_label_str(&modifier))
}

/// Port of `appSnapShortcutKeyLabel` (`packages/shared/src/appSnapShortcut.ts:164`).
pub fn app_snap_shortcut_key_label(code: String) -> Result<String, String> {
    Ok(key_label(&code))
}

/// Port of `appSnapShortcutAccelerator` (`packages/shared/src/appSnapShortcut.ts:170`).
pub fn app_snap_shortcut_accelerator(shortcut: serde_json::Value) -> Result<String, String> {
    if shortcut.is_null() {
        return Err(member_error(&shortcut, "key"));
    }
    let key = get(&shortcut, "key");
    let accel_key = match key.as_str() {
        Some(s) => {
            if let Some(rest) = s.strip_prefix("Key") {
                rest.to_string()
            } else if let Some(rest) = s.strip_prefix("Digit") {
                rest.to_string()
            } else {
                match s {
                    "ArrowUp" => "Up",
                    "ArrowDown" => "Down",
                    "ArrowLeft" => "Left",
                    "ArrowRight" => "Right",
                    "Enter" => "Return",
                    "Escape" => "Esc",
                    _ => s,
                }
                .to_string()
            }
        }
        None => return Err(starts_with_error(&key, "shortcut.key")),
    };
    let accel_modifier = match get(&shortcut, "modifier").as_str() {
        Some("command") => "Command",
        Some("control") => "Control",
        Some("option") => "Alt",
        Some("shift") => "Shift",
        _ => "undefined",
    };
    Ok(format!("{accel_modifier}+{accel_key}"))
}

/// Port of `sameAppSnapShortcut` (`packages/shared/src/appSnapShortcut.ts:179`).
pub fn same_app_snap_shortcut(
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<bool, String> {
    if left.is_null() || is_undefined(&left) {
        return Err(member_error(&left, "kind"));
    }
    if right.is_null() || is_undefined(&right) {
        return Err(member_error(&right, "kind"));
    }
    let left_kind = get(&left, "kind");
    let right_kind = get(&right, "kind");
    if left_kind != right_kind {
        return Ok(false);
    }
    if left_kind.as_str() == Some("key-chord") {
        return Ok(get(&left, "modifier") == get(&right, "modifier")
            && get(&left, "key") == get(&right, "key"));
    }
    Ok(true)
}
