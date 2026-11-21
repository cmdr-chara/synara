//! The Rust port of `apps/web/src/keybindings.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};

fn prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

fn prop_or_err(v: &Value, key: &str) -> Result<Value, String> {
    if js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    match v {
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(js::undefined)),
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ => Ok(js::undefined()),
    }
}

fn truthy(v: &Value) -> bool {
    if js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn nullish(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if js::is_undefined(a) && js::is_undefined(b) {
        return true;
    }
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64() == y.as_f64() && !x.as_f64().is_some_and(|n| n.is_nan())
        }
        _ => a == b,
    }
}

fn js_to_string(v: &Value) -> String {
    if js::is_undefined(v) {
        return "undefined".to_string();
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => match n.as_f64() {
            Some(x) if x.is_nan() => "NaN".to_string(),
            Some(x) if x == f64::INFINITY => "Infinity".to_string(),
            Some(x) if x == f64::NEG_INFINITY => "-Infinity".to_string(),
            Some(x) if x.fract() == 0.0 && x.abs() < 1e21 => format!("{x:.0}"),
            Some(x) => x.to_string(),
            None => "null".to_string(),
        },
        Value::String(s) => s.clone(),
        Value::Array(_) | Value::Object(_) => "[object Object]".to_string(),
    }
}

fn lc(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.to_lowercase()),
        Value::Null => Err("Cannot read properties of null (reading 'toLowerCase')".to_string()),
        _ if js::is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'toLowerCase')".to_string())
        }
        _ => Err("toLowerCase is not a function".to_string()),
    }
}

fn len_or_err(v: &Value) -> Result<f64, String> {
    match v {
        Value::Array(a) => Ok(a.len() as f64),
        Value::String(s) => Ok(s.chars().count() as f64),
        Value::Null => Err("Cannot read properties of null (reading 'length')".to_string()),
        _ if js::is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'length')".to_string())
        }
        _ => Ok(f64::NAN),
    }
}

fn platform_of(options: &Value) -> Value {
    match options {
        Value::Object(o) => o.get("platform").cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

/// Standard KeyboardEvent.code → key mapping (DOM spec).
fn code_to_key(code: &str) -> Option<&'static str> {
    if code.len() == 4 && code.starts_with("Key") {
        let c = code.as_bytes()[3];
        if c.is_ascii_uppercase() {
            return Some(match c {
                b'A' => "a",
                b'B' => "b",
                b'C' => "c",
                b'D' => "d",
                b'E' => "e",
                b'F' => "f",
                b'G' => "g",
                b'H' => "h",
                b'I' => "i",
                b'J' => "j",
                b'K' => "k",
                b'L' => "l",
                b'M' => "m",
                b'N' => "n",
                b'O' => "o",
                b'P' => "p",
                b'Q' => "q",
                b'R' => "r",
                b'S' => "s",
                b'T' => "t",
                b'U' => "u",
                b'V' => "v",
                b'W' => "w",
                b'X' => "x",
                b'Y' => "y",
                b'Z' => "z",
                _ => return None,
            });
        }
        return None;
    }
    if code.len() == 6 && code.starts_with("Digit") {
        let c = code.as_bytes()[5];
        if c.is_ascii_digit() {
            return Some(match c {
                b'0' => "0",
                b'1' => "1",
                b'2' => "2",
                b'3' => "3",
                b'4' => "4",
                b'5' => "5",
                b'6' => "6",
                b'7' => "7",
                b'8' => "8",
                b'9' => "9",
                _ => return None,
            });
        }
        return None;
    }
    Some(match code {
        "Space" => " ",
        "Enter" | "NumpadEnter" => "enter",
        "Tab" => "tab",
        "Escape" => "escape",
        "Backspace" => "backspace",
        "Delete" => "delete",
        "Insert" => "insert",
        "Home" => "home",
        "End" => "end",
        "PageUp" => "pageup",
        "PageDown" => "pagedown",
        "ArrowUp" => "arrowup",
        "ArrowDown" => "arrowdown",
        "ArrowLeft" => "arrowleft",
        "ArrowRight" => "arrowright",
        "Minus" => "-",
        "Equal" => "=",
        "Comma" => ",",
        "Period" => ".",
        "Slash" => "/",
        "Backslash" => "\\",
        "Semicolon" => ";",
        "Quote" => "'",
        "Backquote" => "`",
        "BracketLeft" => "[",
        "BracketRight" => "]",
        "F1" => "f1",
        "F2" => "f2",
        "F3" => "f3",
        "F4" => "f4",
        "F5" => "f5",
        "F6" => "f6",
        "F7" => "f7",
        "F8" => "f8",
        "F9" => "f9",
        "F10" => "f10",
        "F11" => "f11",
        "F12" => "f12",
        _ => return None,
    })
}

/// Port of `shortcutKeyFromEventCode` (`apps/web/src/keybindings.ts:448`).
pub fn shortcut_key_from_event_code(code: Option<String>) -> Result<Option<String>, String> {
    Ok(code.and_then(|c| code_to_key(&c).map(|k| k.to_string())))
}

/// Port of `isUnassignedKeybinding` (`apps/web/src/keybindings.ts:457`).
pub fn is_unassigned_keybinding(binding: serde_json::Value) -> Result<bool, String> {
    let shortcut = prop_or_err(&binding, "shortcut")?;
    let key = prop_or_err(&shortcut, "key")?;
    Ok(js::is_undefined(&key))
}

/// Port of `isShortcutDispatchSuspended` (`apps/web/src/keybindings.ts:479`).
pub fn is_shortcut_dispatch_suspended() -> Result<bool, String> {
    Ok(false)
}

/// Port of `matchesShortcut` (`apps/web/src/keybindings.ts:528`).
pub fn matches_shortcut(
    event: serde_json::Value,
    shortcut: serde_json::Value,
    platform: Option<serde_json::Value>,
) -> Result<bool, String> {
    let meta = truthy(&prop_or_err(&event, "metaKey")?);
    let ctrl = truthy(&prop_or_err(&event, "ctrlKey")?);
    let shift = truthy(&prop_or_err(&event, "shiftKey")?);
    let alt = truthy(&prop_or_err(&event, "altKey")?);
    let smeta = prop_or_err(&shortcut, "metaKey")?;
    let sctrl = prop_or_err(&shortcut, "ctrlKey")?;
    let sshift = prop_or_err(&shortcut, "shiftKey")?;
    let salt = prop_or_err(&shortcut, "altKey")?;
    let key = lc(&prop_or_err(&event, "key")?)?;
    let skey = prop_or_err(&shortcut, "key")?;
    if meta != truthy(&smeta)
        || ctrl != truthy(&sctrl)
        || shift != truthy(&sshift)
        || alt != truthy(&salt)
    {
        return Ok(false);
    }
    if skey.as_str() != Some(key.as_str()) {
        return Ok(false);
    }
    if let Some(platform) = &platform {
        let splatform = prop(&shortcut, "platform");
        if !nullish(&splatform) && !nullish(platform) && !js_strict_eq(&splatform, platform) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn eval_node(node: &Value, context: &Value) -> Result<Value, String> {
    let ty = prop_or_err(node, "type")?;
    match ty.as_str() {
        Some("identifier") => {
            let name = js_to_string(&prop_or_err(node, "name")?);
            let value = prop_or_err(context, &name)?;
            Ok(json!(truthy(&value)))
        }
        Some("and") => {
            let left = eval_node(&prop_or_err(node, "left")?, context)?;
            if !truthy(&left) {
                return Ok(left);
            }
            eval_node(&prop_or_err(node, "right")?, context)
        }
        Some("or") => {
            let left = eval_node(&prop_or_err(node, "left")?, context)?;
            if truthy(&left) {
                return Ok(left);
            }
            eval_node(&prop_or_err(node, "right")?, context)
        }
        Some("not") => {
            let inner = eval_node(&prop_or_err(node, "node")?, context)?;
            Ok(json!(!truthy(&inner)))
        }
        _ => Ok(js::undefined()),
    }
}

/// Port of `evaluateWhenNode` (`apps/web/src/keybindings.ts:553`).
pub fn evaluate_when_node(
    node: serde_json::Value,
    context: serde_json::Value,
) -> Result<serde_json::Value, String> {
    eval_node(&node, &context)
}

/// Port of `shortcutConflictKey` (`apps/web/src/keybindings.ts:580`).
pub fn shortcut_conflict_key(
    shortcut: serde_json::Value,
    platform: Option<serde_json::Value>,
) -> Result<String, String> {
    let meta = prop_or_err(&shortcut, "metaKey")?;
    let ctrl = prop_or_err(&shortcut, "ctrlKey")?;
    let shift = prop_or_err(&shortcut, "shiftKey")?;
    let alt = prop_or_err(&shortcut, "altKey")?;
    let key = prop_or_err(&shortcut, "key")?;
    let _ = &platform;
    let part = |v: &Value| {
        if truthy(v) {
            js_to_string(v)
        } else {
            String::new()
        }
    };
    let key = if nullish(&key) {
        String::new()
    } else {
        js_to_string(&key)
    };
    Ok(format!(
        "{key}|{}|{}|{}|{}",
        part(&meta),
        part(&ctrl),
        part(&shift),
        part(&alt)
    ))
}

fn binding_shortcut_key(binding: &Value) -> Result<Value, String> {
    prop_or_err(&prop_or_err(binding, "shortcut")?, "key")
}

fn js_len(v: &Value) -> Result<usize, String> {
    if js::is_undefined(v) {
        return Err("Cannot read properties of undefined (reading 'length')".to_string());
    }
    match v {
        Value::Null => Err("Cannot read properties of null (reading 'length')".to_string()),
        Value::Array(a) => Ok(a.len()),
        Value::String(s) => Ok(s.encode_utf16().count()),
        Value::Object(o) => Ok(o
            .get("length")
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .max(0.0) as usize),
        _ => Ok(0),
    }
}

fn index_at(v: &Value, i: usize) -> Value {
    match v {
        Value::Array(a) => a.get(i).cloned().unwrap_or_else(js::undefined),
        Value::String(s) => s
            .encode_utf16()
            .nth(i)
            .map(|u| Value::String(String::from_utf16_lossy(&[u])))
            .unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

// `.length` + indexed loop reading `.shortcut.key`, then `.map` on non-arrays.
fn kb_iter(keybindings: &Value) -> Result<Vec<Value>, String> {
    let n = js_len(keybindings)?;
    let mut items = Vec::new();
    for i in 0..n {
        let b = index_at(keybindings, i);
        binding_shortcut_key(&b)?;
        items.push(b);
    }
    if !keybindings.is_array() {
        return Err("keybindings.map is not a function".to_string());
    }
    Ok(items)
}

// Direct `keybindings.map(...)`: null/undefined read-error, others TypeError.
fn kb_map(keybindings: &Value) -> Result<Vec<Value>, String> {
    match keybindings {
        Value::Null => Err("Cannot read properties of null (reading 'map')".to_string()),
        _ if js::is_undefined(keybindings) => {
            Err("Cannot read properties of undefined (reading 'map')".to_string())
        }
        Value::Array(a) => {
            let mut items = Vec::new();
            for b in a {
                binding_shortcut_key(b)?;
                items.push(b.clone());
            }
            Ok(items)
        }
        _ => Err("keybindings.map is not a function".to_string()),
    }
}

/// Port of `findEffectiveKeybindingForCommand` (`apps/web/src/keybindings.ts:605`).
pub fn find_effective_keybinding_for_command(
    keybindings: serde_json::Value,
    command: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let n = js_len(&keybindings)?;
    let platform = options
        .as_ref()
        .map(platform_of)
        .unwrap_or_else(js::undefined);
    let context = match &options {
        Some(Value::Object(o)) => o.get("context").cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    };
    for i in 0..n {
        let binding = index_at(&keybindings, i);
        binding_shortcut_key(&binding)?;
        if !js_strict_eq(&prop(&binding, "command"), &command) {
            continue;
        }
        let when = prop(&binding, "when");
        if !nullish(&when) && !truthy(&eval_node(&when, &context)?) {
            continue;
        }
        let _ = &platform;
        return Ok(Some(binding));
    }
    Ok(None)
}

/// Port of `resolveKeybindingForCommand` (`apps/web/src/keybindings.ts:635`).
pub fn resolve_keybinding_for_command(
    keybindings: serde_json::Value,
    command: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let items = kb_iter(&keybindings)?;
    let platform = options
        .as_ref()
        .map(platform_of)
        .unwrap_or_else(js::undefined);
    let context = match &options {
        Some(Value::Object(o)) => o.get("context").cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    };
    for binding in &items {
        if !js_strict_eq(&prop(binding, "command"), &command) {
            continue;
        }
        let when = prop(binding, "when");
        if !nullish(&when) && !truthy(&eval_node(&when, &context)?) {
            continue;
        }
        let _ = &platform;
        return Ok(Some(prop(binding, "shortcut")));
    }
    Ok(None)
}

fn format_when(node: &Value) -> Value {
    if !truthy(node) {
        return Value::String(String::new());
    }
    let ty = prop(node, "type");
    match ty.as_str() {
        Some("identifier") => Value::String(js_to_string(&prop(node, "name"))),
        Some("and") => {
            let l = format_when(&prop(node, "left"));
            let r = format_when(&prop(node, "right"));
            if js::is_undefined(&l) || js::is_undefined(&r) {
                return js::undefined();
            }
            Value::String(format!(
                "({} && {})",
                l.as_str().unwrap_or(""),
                r.as_str().unwrap_or("")
            ))
        }
        Some("or") => {
            let l = format_when(&prop(node, "left"));
            let r = format_when(&prop(node, "right"));
            if js::is_undefined(&l) || js::is_undefined(&r) {
                return js::undefined();
            }
            Value::String(format!(
                "({} || {})",
                l.as_str().unwrap_or(""),
                r.as_str().unwrap_or("")
            ))
        }
        Some("not") => {
            let inner = format_when(&prop(node, "node"));
            if js::is_undefined(&inner) {
                return js::undefined();
            }
            Value::String(format!("!({})", inner.as_str().unwrap_or("")))
        }
        _ => js::undefined(),
    }
}

/// Port of `formatKeybindingWhenExpression` (`apps/web/src/keybindings.ts:646`).
pub fn format_keybinding_when_expression(
    node: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    Ok(format_when(&node.unwrap_or_else(js::undefined)))
}

/// Port of `resolveShortcutCommand` (`apps/web/src/keybindings.ts:701`).
pub fn resolve_shortcut_command(
    event: serde_json::Value,
    keybindings: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let items = kb_iter(&keybindings)?;
    let platform = options.as_ref().map(platform_of);
    let meta = truthy(&prop_or_err(&event, "metaKey")?);
    let ctrl = truthy(&prop_or_err(&event, "ctrlKey")?);
    if !meta && !ctrl {
        return Ok(None);
    }
    for binding in &items {
        let shortcut = prop(binding, "shortcut");
        if matches_shortcut(event.clone(), shortcut, platform.clone())? {
            return Ok(prop(binding, "command").as_str().map(|s| s.to_string()));
        }
    }
    Ok(None)
}

/// Port of `formatShortcutLabel` (`apps/web/src/keybindings.ts:745`).
pub fn format_shortcut_label(
    shortcut: serde_json::Value,
    platform: Option<serde_json::Value>,
) -> Result<String, String> {
    let key = prop_or_err(&shortcut, "key")?;
    let _key_len = len_or_err(&key)?;
    let meta = truthy(&prop_or_err(&shortcut, "metaKey")?);
    let ctrl = truthy(&prop_or_err(&shortcut, "ctrlKey")?);
    let shift = truthy(&prop_or_err(&shortcut, "shiftKey")?);
    let alt = truthy(&prop_or_err(&shortcut, "altKey")?);
    let key_str = match &key {
        Value::String(s) => s.clone(),
        _ => js_to_string(&key),
    };
    let is_mac = matches!(platform, Some(Value::String(ref p)) if p == "darwin");
    let mut label = String::new();
    if is_mac {
        if ctrl {
            label.push('⌃');
        }
        if alt {
            label.push('⌥');
        }
        if shift {
            label.push('⇧');
        }
        if meta {
            label.push('⌘');
        }
        label.push_str(&key_str.to_uppercase());
    } else {
        let mut mods: Vec<&str> = Vec::new();
        if ctrl {
            mods.push("Ctrl");
        }
        if meta {
            mods.push("Meta");
        }
        if alt {
            mods.push("Alt");
        }
        if shift {
            mods.push("Shift");
        }
        label = mods.join("+");
        if !mods.is_empty() {
            label.push('+');
        }
        label.push_str(&key_str);
    }
    Ok(label)
}

/// Port of `splitShortcutLabel` (`apps/web/src/keybindings.ts:771`).
pub fn split_shortcut_label(shortcut_label: String) -> Result<Vec<String>, String> {
    Ok(vec![shortcut_label])
}

/// Port of `shortcutLabelForCommand` (`apps/web/src/keybindings.ts:799`).
pub fn shortcut_label_for_command(
    keybindings: serde_json::Value,
    command: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let shortcut = resolve_keybinding_for_command(keybindings, command, options.clone())?;
    match shortcut {
        Some(s) => Ok(Some(format_shortcut_label(
            s,
            options.map(|o| platform_of(&o)),
        )?)),
        None => Ok(None),
    }
}

fn jump_command(prefix: &str, index: f64) -> Option<serde_json::Value> {
    if index.is_finite() && index.fract() == 0.0 && (0.0..9.0).contains(&index) {
        Some(json!(format!("{prefix}.jump.{}", index as u64 + 1)))
    } else {
        None
    }
}

fn jump_index(prefix: &str, command: &str) -> Option<f64> {
    let rest = command.strip_prefix(prefix)?.strip_prefix(".jump.")?;
    let n: f64 = rest.parse().ok()?;
    if n.fract() == 0.0 && (1.0..=9.0).contains(&n) {
        Some(n - 1.0)
    } else {
        None
    }
}

/// Port of `threadJumpCommandForIndex` (`apps/web/src/keybindings.ts:845`).
pub fn thread_jump_command_for_index(index: f64) -> Result<Option<serde_json::Value>, String> {
    Ok(jump_command("thread", index))
}

/// Port of `threadJumpIndexFromCommand` (`apps/web/src/keybindings.ts:849`).
pub fn thread_jump_index_from_command(command: String) -> Result<Option<f64>, String> {
    Ok(jump_index("thread", &command))
}

/// Port of `spaceJumpCommandForIndex` (`apps/web/src/keybindings.ts:854`).
pub fn space_jump_command_for_index(index: f64) -> Result<Option<serde_json::Value>, String> {
    Ok(jump_command("space", index))
}

/// Port of `spaceJumpIndexFromCommand` (`apps/web/src/keybindings.ts:858`).
pub fn space_jump_index_from_command(command: String) -> Result<Option<f64>, String> {
    Ok(jump_index("space", &command))
}

/// Port of `shouldShowThreadJumpHints` (`apps/web/src/keybindings.ts:863`).
pub fn should_show_thread_jump_hints(
    event: serde_json::Value,
    keybindings: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<bool, String> {
    let _items = kb_map(&keybindings)?;
    let meta = truthy(&prop(&event, "metaKey"));
    let ctrl = truthy(&prop(&event, "ctrlKey"));
    let shift = truthy(&prop(&event, "shiftKey"));
    let alt = truthy(&prop(&event, "altKey"));
    let repeat = truthy(&prop(&event, "repeat"));
    let platform = options
        .as_ref()
        .map(platform_of)
        .unwrap_or_else(js::undefined);
    let is_mac = nullish(&platform) || platform == "darwin";
    Ok(is_mac && meta && !ctrl && !shift && !alt && !repeat)
}

/// Port of `isOpenFavoriteEditorShortcut` (`apps/web/src/keybindings.ts:885`).
pub fn is_open_favorite_editor_shortcut(
    event: serde_json::Value,
    keybindings: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<bool, String> {
    let command = resolve_shortcut_command(event, keybindings, options)?;
    Ok(command.as_deref() == Some("editor.openFavorite"))
}

/// Port of `isEditorFileSaveShortcut` (`apps/web/src/keybindings.ts:893`).
pub fn is_editor_file_save_shortcut(
    event: serde_json::Value,
    keybindings: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<bool, String> {
    let command = resolve_shortcut_command(event, keybindings, options)?;
    Ok(command.as_deref() == Some("editor.save"))
}

/// Port of `resolveEditorShortcutCommand` (`apps/web/src/keybindings.ts:927`).
pub fn resolve_editor_shortcut_command(
    event: serde_json::Value,
    keybindings: serde_json::Value,
    options: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let command = resolve_shortcut_command(event, keybindings, options)?;
    Ok(command.map(Value::String))
}

/// Port of `isTerminalClearShortcut` (`apps/web/src/keybindings.ts:938`).
pub fn is_terminal_clear_shortcut(event: serde_json::Value) -> Result<bool, String> {
    let ty = prop_or_err(&event, "type")?;
    let key = lc(&prop_or_err(&event, "key")?)?;
    let meta = truthy(&prop(&event, "metaKey"));
    let ctrl = truthy(&prop(&event, "ctrlKey"));
    Ok(ty == "keydown" && (key == "k" && (meta || ctrl) || key == "l" && ctrl))
}

/// Port of `isKeyboardShortcutsHelpShortcut` (`apps/web/src/keybindings.ts:948`).
pub fn is_keyboard_shortcuts_help_shortcut(
    event: serde_json::Value,
    platform: Option<serde_json::Value>,
) -> Result<bool, String> {
    let key = prop_or_err(&event, "key")?;
    let k = match &key {
        Value::String(s) => s.as_str(),
        _ => "",
    };
    let meta = truthy(&prop(&event, "metaKey"));
    let ctrl = truthy(&prop(&event, "ctrlKey"));
    let shift = truthy(&prop(&event, "shiftKey"));
    let is_mac = matches!(platform, Some(Value::String(ref p)) if p == "darwin")
        || platform.as_ref().is_none_or(nullish);
    Ok((k == "?" && (meta || ctrl || shift)) || (k == "/" && (if is_mac { meta } else { ctrl })))
}

/// Port of `terminalNavigationShortcutData` (`apps/web/src/keybindings.ts:971`).
pub fn terminal_navigation_shortcut_data(
    event: serde_json::Value,
    platform: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let ty = prop_or_err(&event, "type")?;
    let key = lc(&prop_or_err(&event, "key")?)?;
    if ty != "keydown" {
        return Ok(None);
    }
    let meta = truthy(&prop(&event, "metaKey"));
    let ctrl = truthy(&prop(&event, "ctrlKey"));
    let shift = truthy(&prop(&event, "shiftKey"));
    let is_mac = matches!(platform, Some(Value::String(ref p)) if p == "darwin")
        || platform.as_ref().is_none_or(nullish);
    let primary = if is_mac { meta } else { ctrl };
    if !primary {
        return Ok(None);
    }
    Ok(match key.as_str() {
        "arrowup" | "pageup" => Some("prev".to_string()),
        "arrowdown" | "pagedown" => Some("next".to_string()),
        "tab" if shift => Some("prev".to_string()),
        "tab" => Some("next".to_string()),
        _ => None,
    })
}
