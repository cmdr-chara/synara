//! The Rust port of `apps/desktop/src/menuShortcuts.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};

fn rf<'a>(v: &'a Value, k: &str) -> Result<&'a Value, String> {
    if js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        ));
    }
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{k}')"));
    }
    Ok(v.as_object().and_then(|o| o.get(k)).unwrap_or(&Value::Null))
}

fn rfv(v: &Value, k: &str) -> Result<Value, String> {
    rf(v, k)?;
    match v.as_object().and_then(|o| o.get(k)) {
        Some(m) => Ok(m.clone()),
        None => Ok(js::undefined()),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !js::is_undefined(v),
    }
}

fn s_eq(v: &Value, s: &str) -> bool {
    v.as_str() == Some(s)
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

const COMMANDS: [&str; 3] = ["terminal.new", "sidebar.toggle", "browser.toggle"];

/// Port of `resolveDesktopPhysicalZoomAction` (`apps/desktop/src/menuShortcuts.ts:34`).
pub fn resolve_desktop_physical_zoom_action(
    platform: Value,
    input: Value,
) -> Result<Value, String> {
    if !s_eq(&platform, "win32")
        || !s_eq(rf(&input, "type")?, "keyDown")
        || !truthy(rf(&input, "control")?)
        || truthy(rf(&input, "meta")?)
        || truthy(rf(&input, "shift")?)
        || truthy(rf(&input, "alt")?)
    {
        return Ok(Value::Null);
    }
    let key = rf(&input, "key")?;
    let code = rf(&input, "code")?;
    if s_eq(key, "-") || s_eq(code, "Minus") || s_eq(code, "NumpadSubtract") {
        return Ok(json!("zoomOut"));
    }
    Ok(Value::Null)
}

/// Port of `resolveDesktopZoomShortcutAction` (`apps/desktop/src/menuShortcuts.ts:64`).
pub fn resolve_desktop_zoom_shortcut_action(
    platform: Value,
    input: Value,
) -> Result<Value, String> {
    if !s_eq(&platform, "linux")
        || !s_eq(rf(&input, "type")?, "keyDown")
        || !truthy(rf(&input, "control")?)
        || truthy(rf(&input, "meta")?)
        || truthy(rf(&input, "alt")?)
    {
        return Ok(Value::Null);
    }
    let key = rf(&input, "key")?.clone();
    let code = rf(&input, "code")?.clone();
    if s_eq(&code, "NumpadAdd") && s_eq(&key, "Add") {
        return Ok(json!("zoomIn"));
    }
    if s_eq(&code, "NumpadSubtract") && s_eq(&key, "Subtract") {
        return Ok(if truthy(rf(&input, "shift")?) {
            Value::Null
        } else {
            json!("zoomOut")
        });
    }
    if s_eq(&key, "+") || s_eq(&key, "=") {
        return Ok(json!("zoomIn"));
    }
    if (s_eq(&key, "-") || s_eq(&key, "_")) && !truthy(rf(&input, "shift")?) {
        return Ok(json!("zoomOut"));
    }
    if s_eq(&key, "0") && !truthy(rf(&input, "shift")?) {
        return Ok(json!("resetZoom"));
    }
    Ok(Value::Null)
}

/// Port of `resolveDesktopMenuAccelerator` (`apps/desktop/src/menuShortcuts.ts:97`).
pub fn resolve_desktop_menu_accelerator(
    platform: Value,
    accelerator: Value,
) -> Result<Option<Value>, String> {
    if s_eq(&platform, "linux") {
        return Ok(None);
    }
    Ok(Some(accelerator))
}

fn named_accelerator_key(key: &str) -> Option<&'static str> {
    Some(match key {
        " " => "Space",
        "+" => "Plus",
        "escape" => "Escape",
        "enter" => "Enter",
        "tab" => "Tab",
        "backspace" => "Backspace",
        "delete" => "Delete",
        "insert" => "Insert",
        "home" => "Home",
        "end" => "End",
        "pageup" => "PageUp",
        "pagedown" => "PageDown",
        "arrowup" => "Up",
        "arrowdown" => "Down",
        "arrowleft" => "Left",
        "arrowright" => "Right",
        _ => return None,
    })
}

fn is_function_key(key: &str) -> bool {
    let Some(digits) = key.strip_prefix('f') else {
        return false;
    };
    digits
        .parse::<u32>()
        .is_ok_and(|n| (1..=24).contains(&n) && digits.len() <= 2)
}

fn is_literal_accelerator_char(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(c) = chars.next() else {
        return false;
    };
    chars.next().is_none()
        && (c.is_ascii_digit() || "`~!@#$%^&*()-_=[]{}\\|;:'\",.<>/?".contains(c))
}

fn accelerator_key(key: &Value) -> Option<Value> {
    if let Some(s) = key.as_str() {
        if let Some(named) = named_accelerator_key(s) {
            return Some(json!(named));
        }
        if (s.len() == 1
            && s.is_ascii()
            && s.chars().next().is_some_and(|c| c.is_ascii_lowercase()))
            || is_function_key(s)
        {
            return Some(json!(s.to_uppercase()));
        }
        if is_literal_accelerator_char(s) {
            return Some(json!(s));
        }
        return None;
    }
    // JS regex tests coerce the key to a string.
    let text = js_repr(key);
    if is_literal_accelerator_char(&text) || is_function_key(&text) {
        return Some(key.clone());
    }
    None
}

fn js_repr(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || js::is_undefined(i) {
                    String::new()
                } else {
                    js_repr(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn shortcut_to_accelerator(shortcut: &Value, platform: &Value) -> Result<Value, String> {
    let key = accelerator_key(rf(shortcut, "key")?);
    let Some(key) = key else {
        return Ok(Value::Null);
    };
    let has_command_modifier = truthy(rf(shortcut, "modKey")?)
        || truthy(rf(shortcut, "metaKey")?)
        || truthy(rf(shortcut, "ctrlKey")?)
        || truthy(rf(shortcut, "altKey")?);
    let key_text = rf(shortcut, "key")?;
    let key_is_function = key_text
        .as_str()
        .map(is_function_key)
        .unwrap_or_else(|| is_function_key(&js_repr(key_text)));
    if !has_command_modifier && !key_is_function {
        return Ok(Value::Null);
    }

    let is_mac = s_eq(platform, "darwin");
    let mut parts: Vec<String> = Vec::new();
    if truthy(rf(shortcut, "modKey")?) {
        parts.push("CmdOrCtrl".to_string());
    }
    if truthy(rf(shortcut, "ctrlKey")?) && (is_mac || !truthy(rf(shortcut, "modKey")?)) {
        parts.push("Ctrl".to_string());
    }
    if truthy(rf(shortcut, "altKey")?) {
        parts.push("Alt".to_string());
    }
    if truthy(rf(shortcut, "shiftKey")?) {
        parts.push("Shift".to_string());
    }
    if truthy(rf(shortcut, "metaKey")?) && !(is_mac && truthy(rf(shortcut, "modKey")?)) {
        parts.push(if is_mac { "Cmd" } else { "Super" }.to_string());
    }
    parts.push(js_repr(&key));
    Ok(json!(parts.join("+")))
}

/// Port of `keybindingShortcutToAccelerator` (`apps/desktop/src/menuShortcuts.ts:150`).
pub fn keybinding_shortcut_to_accelerator(
    shortcut: Value,
    platform: Value,
) -> Result<Value, String> {
    shortcut_to_accelerator(&shortcut, &platform)
}

fn valid_menu_shortcut(v: &Value) -> bool {
    if v.is_null() {
        return true;
    }
    let Some(o) = v.as_object() else {
        return false;
    };
    let key_ok = o
        .get("key")
        .and_then(|k| k.as_str())
        .is_some_and(|k| (1..=64).contains(&utf16_len(k)));
    key_ok
        && ["metaKey", "ctrlKey", "shiftKey", "altKey", "modKey"]
            .iter()
            .all(|f| o.get(*f).is_some_and(|b| b.is_boolean()))
}

/// Port of `resolveReportedMenuAccelerators` (`apps/desktop/src/menuShortcuts.ts:175`).
pub fn resolve_reported_menu_accelerators(
    payload: Value,
    platform: Value,
) -> Result<Value, String> {
    let Some(o) = payload.as_object() else {
        return Ok(Value::Null);
    };
    if !COMMANDS
        .iter()
        .all(|c| o.get(*c).is_some_and(valid_menu_shortcut))
    {
        return Ok(Value::Null);
    }
    let mut out = Map::new();
    for command in COMMANDS {
        let shortcut = o.get(command).cloned().unwrap_or(Value::Null);
        let accelerator = if shortcut.is_null() {
            Value::Null
        } else {
            shortcut_to_accelerator(&shortcut, &platform)?
        };
        out.insert(command.to_string(), accelerator);
    }
    Ok(Value::Object(out))
}

/// Port of `sameDesktopMenuAccelerators` (`apps/desktop/src/menuShortcuts.ts:190`).
pub fn same_desktop_menu_accelerators(left: Value, right: Value) -> Result<bool, String> {
    for command in COMMANDS {
        let l = rfv(&left, command)?;
        let r = rfv(&right, command)?;
        if !strict_eq(&l, &r) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    if js::is_undefined(a) || js::is_undefined(b) {
        return js::is_undefined(a) && js::is_undefined(b);
    }
    match (a, b) {
        (Value::Number(_), Value::Number(_)) => js::as_number(a) == js::as_number(b),
        (Value::Array(_), _) | (_, Value::Array(_)) => false,
        (Value::Object(_), _) | (_, Value::Object(_)) => false,
        _ => a == b,
    }
}

/// Port of `shouldUseNativeZoomMenuRoles` (`apps/desktop/src/menuShortcuts.ts:197`).
pub fn should_use_native_zoom_menu_roles(platform: Value) -> Result<bool, String> {
    Ok(!s_eq(&platform, "linux"))
}

/// Port of `resolveKeyboardShortcutsMenuAccelerator` (`apps/desktop/src/menuShortcuts.ts:205`).
pub fn resolve_keyboard_shortcuts_menu_accelerator(platform: Value) -> Result<Value, String> {
    Ok(if s_eq(&platform, "darwin") {
        json!("Cmd+/")
    } else {
        js::undefined()
    })
}
