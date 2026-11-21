//! The Rust port of `packages/shared/src/deviceHelperCache.ts`.

#![forbid(unsafe_code)]

use std::cmp::Ordering;

use serde_json::Value;

use crate::js;

fn is_js_space(c: char) -> bool {
    matches!(c, '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}'
        | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

fn utf16_decode(v: &Value) -> String {
    let units: Vec<u16> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_u64().map(|u| u as u16))
                .collect()
        })
        .unwrap_or_default();
    String::from_utf16_lossy(&units)
}

/// JS `ToString` for the values that reach these two functions.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|e| match e {
                Value::Null => String::new(),
                _ if js::is_undefined(e) => String::new(),
                _ => js_to_string(e),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) if o.len() == 1 => match o.keys().next().map(String::as_str) {
            Some("$undefined") => "undefined".into(),
            Some("$utf16") => utf16_decode(&o["$utf16"]),
            Some("$num") => js::as_number(v)
                .map(js::number_to_string)
                .unwrap_or_default(),
            _ => "[object Object]".into(),
        },
        _ => "[object Object]".into(),
    }
}

/// JS truthiness.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ if js::is_undefined(v) => false,
        _ => true,
    }
}

/// `obj.key`: missing property reads `undefined`, a nullish receiver throws.
fn member(v: &Value, k: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{k}')")),
        _ if js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        )),
        Value::Object(o) => Ok(o.get(k).cloned().unwrap_or_else(js::undefined)),
        _ => Ok(js::undefined()),
    }
}

/// `/Xcode\s+([\d.]+)/u.exec` — leftmost match.
fn xcode_version(s: &str) -> Option<String> {
    s.match_indices("Xcode").find_map(|(i, _)| {
        let after = &s[i + 5..];
        let rest = after.trim_start_matches(is_js_space);
        if rest.len() == after.len() {
            return None; // `\s+` needs at least one space
        }
        let d: String = rest
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        (!d.is_empty()).then_some(d)
    })
}

/// `/Build version\s+(\S+)/u.exec` — leftmost match.
fn build_version(s: &str) -> Option<String> {
    s.match_indices("Build version").find_map(|(i, _)| {
        let after = &s[i + 13..];
        let rest = after.trim_start_matches(is_js_space);
        if rest.len() == after.len() {
            return None;
        }
        let w: String = rest.chars().take_while(|c| !is_js_space(*c)).collect();
        (!w.is_empty()).then_some(w)
    })
}

/// Port of `deviceHelperCacheKey` (`packages/shared/src/deviceHelperCache.ts:44`).
pub fn device_helper_cache_key(
    xcodebuild_version_output: serde_json::Value,
    source_revision: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let s = js_to_string(&xcodebuild_version_output);
    let (version, build) = (xcode_version(&s), build_version(&s));
    if version.is_none() && build.is_none() {
        return Ok(None);
    }
    let toolchain = format!(
        "{}-{}",
        version.as_deref().unwrap_or("unknown"),
        build.as_deref().unwrap_or("unknown")
    );
    Ok(Some(Value::String(match source_revision.filter(truthy) {
        Some(r) => format!("{toolchain}-{}", js_to_string(&r)),
        None => toolchain,
    })))
}

fn to_num(s: &str) -> f64 {
    let t = s.trim_matches(is_js_space);
    if t.is_empty() {
        0.0
    } else {
        t.parse().unwrap_or(f64::NAN)
    }
}

/// `a < b` then `a > b` the way the JS comparator applies them.
fn js_cmp(a: &Value, b: &Value) -> Ordering {
    if js::is_undefined(a) || js::is_undefined(b) {
        return Ordering::Equal; // undefined never compares less or greater
    }
    // `ToPrimitive`: numbers/bools/null go numeric, strings stay strings.
    let num = |v: &Value| match v {
        Value::Number(n) => n.as_f64(),
        Value::Null => Some(0.0),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    };
    match (num(a), num(b)) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        (None, None) => js_to_string(a)
            .encode_utf16()
            .cmp(js_to_string(b).encode_utf16()),
        (x, y) => {
            let x = x.unwrap_or_else(|| to_num(&js_to_string(a)));
            let y = y.unwrap_or_else(|| to_num(&js_to_string(b)));
            x.partial_cmp(&y).unwrap_or(Ordering::Equal)
        }
    }
}

/// Port of `deviceHelperSourceRevision` (`packages/shared/src/deviceHelperCache.ts:69`).
pub fn device_helper_source_revision(files: serde_json::Value) -> Result<String, String> {
    // `[...files]`: arrays spread as-is, strings spread into chars, the rest throws.
    let items: Vec<Value> = match &files {
        Value::Array(a) => a.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => utf16_decode(&o["$utf16"])
            .chars()
            .map(|c| Value::String(c.to_string()))
            .collect(),
        _ => return Err("files is not iterable".to_string()),
    };
    let mut keyed: Vec<(Value, Value)> = Vec::with_capacity(items.len());
    for f in &items {
        keyed.push((member(f, "name")?, member(f, "contents")?));
    }
    keyed.sort_by(|a, b| js_cmp(&a.0, &b.0));
    let canonical = keyed
        .iter()
        .map(|(n, c)| format!("{}\0{}", js_to_string(n), js_to_string(c)))
        .collect::<Vec<_>>()
        .join("\0");
    let mut hash: u32 = 0x811c9dc5;
    for unit in canonical.encode_utf16() {
        hash ^= unit as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    Ok(format!("{hash:08x}"))
}
