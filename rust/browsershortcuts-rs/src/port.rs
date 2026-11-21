//! The Rust port of `packages/shared/src/browserShortcuts.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

/// JS truthiness.
fn truthy(v: &Value) -> bool {
    match crate::js::as_number(v) {
        Some(n) => n != 0.0 && !n.is_nan(),
        None => {
            !crate::js::is_undefined(v)
                && !matches!(v, Value::Bool(false) | Value::Null)
                && !matches!(v, Value::String(s) if s.is_empty())
        }
    }
}

fn prop<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}

/// Port of `isBrowserCopyLinkChord` (`packages/shared/src/browserShortcuts.ts:27`).
pub fn is_browser_copy_link_chord(chord: serde_json::Value, is_mac: bool) -> Result<bool, String> {
    // The original calls `chord.key.toLowerCase()` — throws on non-objects and missing keys.
    let key = chord
        .get("key")
        .and_then(|k| k.as_str())
        .ok_or("TypeError: chord.key.toLowerCase is not a function")?;
    let (primary, other) = if is_mac {
        ("meta", "ctrl")
    } else {
        ("ctrl", "meta")
    };
    Ok(key.to_lowercase() == "c"
        && truthy(prop(&chord, "shift"))
        && !truthy(prop(&chord, "alt"))
        && truthy(prop(&chord, primary))
        && !truthy(prop(&chord, other)))
}

/// Port of `isKeyboardShortcutsHelpChord` (`packages/shared/src/browserShortcuts.ts:37`).
pub fn is_keyboard_shortcuts_help_chord(
    chord: serde_json::Value,
    platform: serde_json::Value,
) -> Result<bool, String> {
    // `null`/`undefined` throws reading `.key`; other non-objects read `undefined` -> false.
    if chord.is_null() || crate::js::is_undefined(&chord) {
        return Err("TypeError: cannot read properties of null".into());
    }
    // `type` is checked via `.toLowerCase()` when present — non-strings throw.
    match prop(&chord, "type") {
        Value::Null => {}
        v if crate::js::is_undefined(v) => {}
        Value::String(s) if s.to_lowercase() == "keydown" => {}
        Value::String(_) => return Ok(false),
        _ => return Err("TypeError: chord.type.toLowerCase is not a function".into()),
    }
    if truthy(prop(&chord, "shift"))
        || truthy(prop(&chord, "alt"))
        || truthy(prop(&chord, "repeat"))
    {
        return Ok(false);
    }
    // `platform` is read before the key/code comparison — `null` throws.
    if platform.is_null() || crate::js::is_undefined(&platform) {
        return Err("TypeError: platform.isWindows is not readable".into());
    }
    let key_match =
        prop(&chord, "key") == &Value::from("/") || prop(&chord, "code") == &Value::from("Slash");
    let (primary, other) = if truthy(prop(&platform, "isMac")) {
        ("meta", "ctrl")
    } else {
        ("ctrl", "meta")
    };
    Ok(key_match && truthy(prop(&chord, primary)) && !truthy(prop(&chord, other)))
}
