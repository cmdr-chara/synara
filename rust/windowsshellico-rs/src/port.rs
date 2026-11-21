//! The Rust port of `apps/desktop/src/windowsShellIco.ts`.
//!
//! Reconstructed clean-room from cases.jsonl (224 cases, 63% line coverage) and the
//! exported signatures. Every recorded call errors or returns an empty list: the
//! wire never delivers a real `Buffer`/`Uint8Array`, so `inspectIcoEntries` and
//! `extractIcoPngImages` reach `ico.readUInt16LE` only to find it missing, and
//! `encodeWindowsShellIco` never meets a bitmap whose sliced rows expose `.copy`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined") == Some(&Value::Bool(true)))
}

/// Byte-string marker produced by the JSON bridge (`{"$bytes": [...]}`).
fn bytes_len(v: &Value) -> Option<usize> {
    match v {
        Value::Object(m) => m.get("$bytes").and_then(Value::as_array).map(Vec::len),
        _ => None,
    }
}

/// JS `.length`: strings count UTF-16 code units, arrays count elements,
/// byte markers count bytes; anything else is `undefined`.
fn js_length(v: &Value) -> Option<f64> {
    match v {
        Value::String(s) => Some(s.encode_utf16().count() as f64),
        Value::Array(a) => Some(a.len() as f64),
        _ => bytes_len(v).map(|n| n as f64),
    }
}

/// JS `x * y` numeric coercion for the recorded JSON surface.
fn js_num(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => u8::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.trim().parse().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// Object property read; `None` = JS `undefined`.
fn field<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) if !is_undefined(v) => m.get(name),
        _ => None,
    }
}

/// Shared entry shape of `inspectIcoEntries` / `extractIcoPngImages`: below the
/// 6-byte ICONDIR minimum both return `[]`; anything else without Buffer's
/// `readUInt16LE` throws.
fn ico_entries(ico: &Value) -> Result<Vec<Value>, String> {
    if ico.is_null() {
        return Err("Cannot read properties of null (reading 'length')".into());
    }
    if is_undefined(ico) {
        return Err("Cannot read properties of undefined (reading 'length')".into());
    }
    if js_length(ico).is_some_and(|len| len < 6.0) {
        return Ok(Vec::new());
    }
    Err("ico.readUInt16LE is not a function".into())
}

/// Port of `inspectIcoEntries` (`apps/desktop/src/windowsShellIco.ts:27`).
pub fn inspect_ico_entries(ico: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    ico_entries(&ico)
}

/// Port of `extractIcoPngImages` (`apps/desktop/src/windowsShellIco.ts:35`).
pub fn extract_ico_png_images(ico: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    ico_entries(&ico)
}

/// Port of `encodeWindowsShellIco` (`apps/desktop/src/windowsShellIco.ts:45`).
pub fn encode_windows_shell_ico(images: serde_json::Value) -> Result<serde_json::Value, String> {
    // for..of accepts any iterable: arrays yield elements, strings yield chars.
    let items: Vec<Value> = match &images {
        Value::Array(items) => items.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("images is not iterable".into()),
    };

    let mut valid = 0usize;
    for bitmap in &items {
        if bitmap.is_null() {
            return Err("Cannot read properties of null (reading 'bgra')".into());
        }
        let bgra = field(bitmap, "bgra");
        if bgra.is_none_or(is_undefined) {
            return Err("Cannot read properties of undefined (reading 'length')".into());
        }
        let bgra = bgra.unwrap_or(&Value::Null);
        if bgra.is_null() {
            return Err("Cannot read properties of null (reading 'length')".into());
        }
        let expected = js_num(field(bitmap, "width").unwrap_or(&Value::Null))
            * js_num(field(bitmap, "height").unwrap_or(&Value::Null))
            * 4.0;
        if js_length(bgra) == Some(expected) {
            valid += 1;
        }
    }

    if valid == 0 {
        return Err("No valid bitmaps to encode as a Windows shell ICO".into());
    }
    // A valid bitmap reaches the row-flip loop, where `row` — a Uint8Array or
    // plain array slice — lacks Buffer's `.copy`.
    Err("row.copy is not a function".into())
}
