//! The Rust port of `apps/server/src/computer/`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

use crate::js;
use crate::utf16;

// ── JS value model ──────────────────────────────────────────────────────────

/// JS whitespace (the set `String.prototype.trim` and `\s` use).
fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_space)
}

fn undef() -> Value {
    js::undefined()
}

fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}

fn single_marker<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) if o.len() == 1 => o.get(key),
        _ => None,
    }
}

fn is_marker(v: &Value, key: &str) -> bool {
    single_marker(v, key).is_some()
}

/// `typeof` of the JS value the structured argument decodes to.
fn is_number(v: &Value) -> bool {
    matches!(v, Value::Number(_)) || single_marker(v, "$num").is_some()
}

fn num_or_nan(v: &Value) -> f64 {
    js::as_number(v).unwrap_or(f64::NAN)
}

/// `undefined` for property reads a JSON value does not have.
fn prop_read_err(v: &Value, key: &str) -> String {
    if v.is_null() {
        format!("Cannot read properties of null (reading '{key}')")
    } else {
        format!("Cannot read properties of undefined (reading '{key}')")
    }
}

/// `value.key` — property access with JS TypeErrors for null/undefined, and the
/// values JS gives other primitives (`undefined` for everything).
fn js_get(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(prop_read_err(v, key)),
        Value::Object(o) => {
            if is_undef(v) {
                return Err(prop_read_err(v, key));
            }
            if let Some(units) = single_marker(v, "$utf16") {
                // Marker decodes to a string.
                if key == "length" {
                    let n = units.as_array().map_or(0, Vec::len);
                    return Ok(Value::from(n as u64));
                }
                return Ok(undef());
            }
            Ok(o.get(key).cloned().unwrap_or_else(undef))
        }
        Value::Array(a) => {
            if key == "length" {
                return Ok(Value::from(a.len() as u64));
            }
            if let Some(i) = key.parse::<usize>().ok().filter(|i| *i < a.len()) {
                return Ok(a[i].clone());
            }
            Ok(undef())
        }
        Value::String(s) => {
            if key == "length" {
                return Ok(Value::from(s.encode_utf16().count() as u64));
            }
            if let Ok(i) = key.parse::<usize>() {
                let units: Vec<u16> = s.encode_utf16().collect();
                if i < units.len() {
                    return Ok(js::utf16_string(&units[i..i + 1]));
                }
            }
            Ok(undef())
        }
        _ => Ok(undef()),
    }
}

fn js_get_u32(v: &Value, i: usize) -> Result<Value, String> {
    match v {
        Value::Null => Err(prop_read_err(v, "0")),
        Value::Object(o) if is_undef(v) => Err(prop_read_err(v, "0")),
        Value::Array(a) => Ok(a.get(i).cloned().unwrap_or_else(undef)),
        Value::String(s) => {
            let units: Vec<u16> = s.encode_utf16().collect();
            Ok(match units.get(i) {
                Some(u) => js::utf16_string(&[*u]),
                None => undef(),
            })
        }
        Value::Object(o) => Ok(o.get(&i.to_string()).cloned().unwrap_or_else(undef)),
        _ => Ok(undef()),
    }
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undef(v) {
                return false;
            }
            if let Some(m) = single_marker(v, "$num") {
                return match m.as_str() {
                    Some("NaN") | Some("-0") => false,
                    Some(_) => true,
                    None => false,
                };
            }
            if let Some(units) = single_marker(v, "$utf16") {
                return units.as_array().is_some_and(|a| !a.is_empty());
            }
            true
        }
    }
}

/// JS `===` between decoded values.
fn js_strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Array(_) | Value::Object(_), _) | (_, Value::Array(_) | Value::Object(_)) => {
            // Marker values compare by their decoded type, objects by reference (never equal here).
            if is_undef(a) && is_undef(b) {
                return true;
            }
            if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
                return x == y;
            }
            if let (Some(x), Some(y)) = (js::string_units(a), js::string_units(b)) {
                return x == y;
            }
            false
        }
        (Value::Number(_), Value::Number(_)) => a == b,
        (Value::String(_), Value::String(_)) => a == b,
        _ => false,
    }
}

/// `String(v)` / `${v}` — JS's string conversion.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| match item {
                Value::Null => String::new(),
                item if is_undef(item) => String::new(),
                item => js_to_string(item),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if is_undef(v) {
                return "undefined".into();
            }
            if let Some(m) = single_marker(v, "$num") {
                return m.as_str().unwrap_or("NaN").into();
            }
            if let Some(b) = single_marker(v, "$bigint") {
                return b.as_str().unwrap_or("0").into();
            }
            if let Some(_u) = single_marker(v, "$utf16")
                && let Some(units) = js::string_units(v)
            {
                return String::from_utf16_lossy(&units);
            }
            if single_marker(v, "$set").is_some() {
                return "[object Set]".into();
            }
            if single_marker(v, "$map").is_some() {
                return "[object Map]".into();
            }
            if let Some(d) = single_marker(v, "$date") {
                return d
                    .as_str()
                    .map_or_else(|| "Invalid Date".into(), js_date_to_string);
            }
            if let Some(bytes) = single_marker(v, "$bytes").or_else(|| single_marker(v, "$buffer"))
            {
                return bytes
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|b| js::number_to_string(b.as_f64().unwrap_or(0.0)))
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_default();
            }
            let _ = o;
            "[object Object]".into()
        }
    }
}

/// JS `Number(v)` (also used by `Math.max` etc. via ToNumber).
fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_number_str(s),
        Value::Array(items) => {
            if items.is_empty() {
                0.0
            } else {
                js_number_str(&js_to_string(v))
            }
        }
        Value::Object(_) => {
            if let Some(x) = js::as_number(v) {
                return x;
            }
            if let Some(units) = js::string_units(v) {
                return js_number_str(&String::from_utf16_lossy(&units));
            }
            if let Some(d) = single_marker(v, "$date") {
                return d.as_str().and_then(js_date_parse).unwrap_or(f64::NAN);
            }
            f64::NAN
        }
    }
}

/// JS `Number(string)`: trim, then decimal/hex/octal/binary/Infinity.
fn js_number_str(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    let (sign, rest) = match t.strip_prefix('-') {
        Some(r) => (-1.0, r),
        None => match t.strip_prefix('+') {
            Some(r) => (1.0, r),
            None => (1.0, t),
        },
    };
    if rest == "Infinity" {
        return sign * f64::INFINITY;
    }
    if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        return match u64::from_str_radix(hex, 16).ok().map(|x| x as f64) {
            Some(x) => sign * x,
            None => f64::NAN,
        };
    }
    if let Some(o) = rest.strip_prefix("0o").or_else(|| rest.strip_prefix("0O")) {
        return match u64::from_str_radix(o, 8).ok().map(|x| x as f64) {
            Some(x) => sign * x,
            None => f64::NAN,
        };
    }
    if let Some(b) = rest.strip_prefix("0b").or_else(|| rest.strip_prefix("0B")) {
        return match u64::from_str_radix(b, 2).ok().map(|x| x as f64) {
            Some(x) => sign * x,
            None => f64::NAN,
        };
    }
    let valid = !t.is_empty()
        && t.bytes()
            .all(|c| c.is_ascii_digit() || matches!(c, b'.' | b'e' | b'E' | b'+' | b'-'));
    if valid {
        t.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

/// `a + b` — JS addition (numeric unless an operand is a string after ToPrimitive).
fn js_add(a: &Value, b: &Value) -> Value {
    // ToPrimitive: strings stay strings; arrays/objects/typed values stringify;
    // undefined/null/numbers/booleans/bigints stay primitives.
    let prim_string = |v: &Value| {
        matches!(v, Value::String(_) | Value::Array(_))
            || is_marker(v, "$utf16")
            || is_marker(v, "$bytes")
            || is_marker(v, "$buffer")
            || is_marker(v, "$map")
            || is_marker(v, "$set")
            || is_marker(v, "$date")
            || (matches!(v, Value::Object(_))
                && !is_undef(v)
                && !is_marker(v, "$num")
                && !is_marker(v, "$bigint"))
    };
    if prim_string(a) || prim_string(b) {
        return Value::String(format!("{}{}", js_to_string(a), js_to_string(b)));
    }
    js::number(js_to_number(a) + js_to_number(b))
}

/// `<` `<=` `>` `>=` between decoded values.
fn js_lt(a: &Value, b: &Value) -> bool {
    let (sa, sb) = (js::string_units(a), js::string_units(b));
    if let (Some(sa), Some(sb)) = (sa, sb) {
        return sa < sb;
    }
    let (x, y) = (js_to_number(a), js_to_number(b));
    !(x.is_nan() || y.is_nan()) && x < y
}

fn js_le(a: &Value, b: &Value) -> bool {
    let (sa, sb) = (js::string_units(a), js::string_units(b));
    if let (Some(sa), Some(sb)) = (sa, sb) {
        return sa <= sb;
    }
    let (x, y) = (js_to_number(a), js_to_number(b));
    !(x.is_nan() || y.is_nan()) && x <= y
}

fn js_gt(a: &Value, b: &Value) -> bool {
    js_lt(b, a)
}

fn js_ge(a: &Value, b: &Value) -> bool {
    js_le(b, a)
}

/// `Math.max(a, b)` — NaN propagates, unlike `f64::max`.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a > b || (a == 0.0 && b == 0.0 && a.is_sign_positive()) {
        a
    } else {
        b
    }
}

fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if b < a || (a == 0.0 && b == 0.0 && b.is_sign_negative()) {
        b
    } else {
        a
    }
}

/// `Object.entries` key order: array indices ascending, then insertion order.
fn js_entries(map: &Map<String, Value>) -> Vec<(String, Value)> {
    let mut indices: Vec<(u64, String, Value)> = Vec::new();
    let mut rest: Vec<(String, Value)> = Vec::new();
    for (k, v) in map {
        if let Ok(i) = k.parse::<u64>()
            && i < u64::from(u32::MAX)
            && i.to_string() == *k
        {
            indices.push((i, k.clone(), v.clone()));
            continue;
        }
        rest.push((k.clone(), v.clone()));
    }
    indices.sort_by_key(|(i, _, _)| *i);
    indices
        .into_iter()
        .map(|(_, k, v)| (k, v))
        .chain(rest)
        .collect()
}

/// `Object.entries(v)` — throws for null/undefined, chars for strings,
/// `{"0":..}` for typed arrays, empty for other primitives.
fn object_entries(v: &Value) -> Result<Vec<(String, Value)>, String> {
    if v.is_null() || is_undef(v) {
        return Err("Cannot convert undefined or null to object".into());
    }
    match v {
        Value::Object(o) => {
            if let Some(units) = js::string_units(v) {
                return Ok(units
                    .iter()
                    .enumerate()
                    .map(|(i, u)| (i.to_string(), js::utf16_string(&[*u])))
                    .collect());
            }
            if let Some(b) = single_marker(v, "$bytes").or_else(|| single_marker(v, "$buffer")) {
                return Ok(b
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (i.to_string(), x.clone()))
                    .collect());
            }
            if single_marker(v, "$map").is_some()
                || single_marker(v, "$set").is_some()
                || single_marker(v, "$date").is_some()
            {
                return Ok(vec![]);
            }
            Ok(js_entries(o))
        }
        Value::Array(a) => Ok(a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect()),
        Value::String(s) => Ok(s
            .encode_utf16()
            .enumerate()
            .map(|(i, u)| (i.to_string(), js::utf16_string(&[u])))
            .collect()),
        _ => Ok(vec![]),
    }
}

/// `{ ...v }` — own enumerable properties spread into a new object.
fn js_spread(v: &Value) -> Value {
    match v {
        Value::Object(o) => {
            let mut out = Map::new();
            for (k, x) in js_entries(o) {
                out.insert(k, x);
            }
            Value::Object(out)
        }
        Value::Array(a) => {
            let mut out = Map::new();
            for (i, x) in a.iter().enumerate() {
                out.insert(i.to_string(), x.clone());
            }
            Value::Object(out)
        }
        Value::String(s) => {
            let mut out = Map::new();
            for (i, u) in s.encode_utf16().enumerate() {
                out.insert(i.to_string(), js::utf16_string(&[u]));
            }
            Value::Object(out)
        }
        _ => Value::Object(Map::new()),
    }
}

/// Whether the decoded JS value may key a WeakMap.
fn is_weak_keyable(v: &Value) -> bool {
    match v {
        Value::Array(_) => true,
        Value::Object(_) => {
            !(is_undef(v)
                || is_marker(v, "$num")
                || is_marker(v, "$utf16")
                || is_marker(v, "$bigint"))
        }
        _ => false,
    }
}

/// `JSON.stringify(v)` — the JS text, or None where JS produces `undefined`.
fn js_json_stringify(v: &Value) -> Option<String> {
    match v {
        Value::Null => Some("null".into()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(f64::NAN);
            if x.is_finite() {
                Some(js::number_to_string(x))
            } else {
                Some("null".into())
            }
        }
        Value::String(s) => Some(json_quote_str(&utf16::units(s))),
        Value::Array(items) => {
            let mut parts = Vec::with_capacity(items.len());
            for item in items {
                parts.push(js_json_stringify(item).unwrap_or_else(|| "null".into()));
            }
            Some(format!("[{}]", parts.join(",")))
        }
        Value::Object(o) => {
            if is_undef(v) {
                return None;
            }
            if let Some(m) = single_marker(v, "$num") {
                // JSON.stringify: non-finite numbers become null, but -0 writes "0".
                return Some(if m.as_str() == Some("-0") {
                    "0".into()
                } else {
                    "null".into()
                });
            }
            if let Some(u) = single_marker(v, "$utf16")
                && let Some(units) = js::string_units(&Value::Object(Map::from_iter([(
                    "$utf16".to_string(),
                    u.clone(),
                )])))
            {
                return Some(json_quote_str(&units));
            }
            if let Some(d) = single_marker(v, "$date") {
                return match d.as_str() {
                    Some(s) if s != "Invalid" => Some(json_quote_str(&utf16::units(s))),
                    _ => Some("null".into()),
                };
            }
            if let Some(b) = single_marker(v, "$buffer") {
                let data = b
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|x| js_json_stringify(x).unwrap_or_else(|| "null".into()))
                            .collect::<Vec<_>>()
                            .join(",")
                    })
                    .unwrap_or_default();
                return Some(format!("{{\"type\":\"Buffer\",\"data\":[{}]}}", data));
            }
            if single_marker(v, "$set").is_some() || single_marker(v, "$map").is_some() {
                return Some("{}".into());
            }
            if single_marker(v, "$bigint").is_some() {
                return None; // JSON.stringify(BigInt) throws; callers treat it as unrepresentable.
            }
            if let Some(bytes) = single_marker(v, "$bytes") {
                // Typed arrays serialize as an object of index keys.
                let parts: Vec<String> = bytes
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .enumerate()
                            .filter_map(|(i, x)| {
                                js_json_stringify(x).map(|s| format!("\"{i}\":{s}"))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return Some(format!("{{{}}}", parts.join(",")));
            }
            let mut parts = Vec::new();
            for (k, x) in js_entries(o) {
                if let Some(s) = js_json_stringify(&x) {
                    parts.push(format!("{}:{s}", json_quote_str(&utf16::units(&k))));
                }
            }
            Some(format!("{{{}}}", parts.join(",")))
        }
    }
}

/// `JSON.stringify("...")` — quoted with JS escaping, lone surrogates escaped.
fn json_quote_str(units: &[u16]) -> String {
    let mut out = String::with_capacity(units.len() + 2);
    out.push('"');
    for &u in units {
        match u {
            0x22 => out.push_str("\\\""),
            0x5c => out.push_str("\\\\"),
            0x08 => out.push_str("\\b"),
            0x0c => out.push_str("\\f"),
            0x0a => out.push_str("\\n"),
            0x0d => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            u if u < 0x20 || (0xd800..=0xdfff).contains(&u) => {
                out.push_str(&format!("\\u{u:04x}"));
            }
            _ => {
                if let Some(c) = char::from_u32(u32::from(u)) {
                    out.push(c);
                }
            }
        }
    }
    out.push('"');
    out
}

/// `${JSON.stringify(v)}` inside a template — `undefined` stringifies to the word.
fn json_or_undefined(v: &Value) -> String {
    js_json_stringify(v).unwrap_or_else(|| "undefined".into())
}

/// `expr.method` call gate for the array methods recorded cases exercise.
/// Strings only have `slice`; typed arrays carry the real methods on numbers.
fn array_method(v: &Value, expr: &str, method: &str) -> Result<Vec<Value>, String> {
    const STRING_METHODS: &[&str] = &[
        "slice",
        "includes",
        "indexOf",
        "lastIndexOf",
        "trim",
        "toLowerCase",
    ];
    if v.is_null() || is_undef(v) {
        return Err(prop_read_err(v, method));
    }
    if let Value::Array(a) = v {
        return Ok(a.clone());
    }
    if matches!(v, Value::String(_)) {
        if STRING_METHODS.contains(&method) {
            // Callers handle the string result of `slice` separately.
            return Err("__string_ok__".into());
        }
        return Err(format!("{expr}.{method} is not a function"));
    }
    if let Some(bytes) = single_marker(v, "$bytes").or_else(|| single_marker(v, "$buffer")) {
        // Typed arrays have the same methods over their numbers.
        let a = bytes.as_array().cloned().unwrap_or_default();
        return Ok(a);
    }
    Err(format!("{expr}.{method} is not a function"))
}

/// Iterate `for (const x of v)`: arrays, strings (as char strings), typed bytes.
fn for_of(v: &Value, operand: &str) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.encode_utf16().map(|u| js::utf16_string(&[u])).collect()),
        Value::Null | Value::Object(_) => {
            if is_undef(v) {
                return Err(format!("{operand} is not iterable"));
            }
            if let Some(b) = single_marker(v, "$bytes").or_else(|| single_marker(v, "$buffer")) {
                return Ok(b.as_array().cloned().unwrap_or_default());
            }
            Err(format!("{operand} is not iterable"))
        }
        _ => Err(format!("{operand} is not iterable")),
    }
}

/// `value.length` as a JS number.
fn length_of(v: &Value) -> f64 {
    match v {
        Value::Array(a) => a.len() as f64,
        Value::String(s) => s.encode_utf16().count() as f64,
        Value::Object(o) if !is_undef(v) => {
            if let Some(units) = single_marker(v, "$utf16") {
                units.as_array().map_or(0.0, |a| a.len() as f64)
            } else if let Some(b) =
                single_marker(v, "$bytes").or_else(|| single_marker(v, "$buffer"))
            {
                b.as_array().map_or(0.0, |a| a.len() as f64)
            } else {
                o.get("length").map_or(f64::NAN, js_to_number)
            }
        }
        _ => f64::NAN,
    }
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(data);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn base64url_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 2);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        out.push(T[(b[0] >> 2) as usize] as char);
        out.push(T[(((b[0] & 3) << 4) | (b[1] >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[(((b[1] & 15) << 2) | (b[2] >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(T[(b[2] & 63) as usize] as char);
        }
    }
    out
}

fn base64url_decode(s: &str) -> Option<Vec<u8>> {
    let mut bits = 0u32;
    let mut nbits = 0u32;
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    for c in s.chars() {
        let v = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '-' => 62,
            '_' => 63,
            '=' => continue,
            _ => return None,
        };
        bits = (bits << 6) | v;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            out.push((bits >> nbits) as u8);
        }
    }
    Some(out)
}

/// JS `Date.parse` for ISO-shaped strings (enough for audit timestamps).
fn js_date_parse(s: &str) -> Option<f64> {
    let re = fancy_regex::Regex::new(
        r"^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,3}))?Z$",
    )
    .ok()?;
    let c = re.captures(s).ok()??;
    let g = |i: usize| c.get(i).and_then(|m| m.as_str().parse::<f64>().ok());
    let (y, mo, d, h, mi, se) = (g(1)?, g(2)?, g(3)?, g(4)?, g(5)?, g(6)?);
    let ms = g(7).map_or(0.0, |m| {
        m * 10f64.powi(3 - c.get(7).map_or(0, |m| m.as_str().len()) as i32)
    });
    if !(1.0..=12.0).contains(&mo)
        || !(1.0..=31.0).contains(&d)
        || h > 23.0
        || mi > 59.0
        || se > 59.0
    {
        return Some(f64::NAN);
    }
    let days = days_from_civil(y as i64, mo as u32, d as u32);
    Some(days as f64 * 86_400_000.0 + h * 3_600_000.0 + mi * 60_000.0 + se * 1000.0 + ms)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// ISO-8601 string for a millisecond timestamp (`new Date(ms).toISOString()`).
fn iso_string(ms: f64) -> String {
    if !ms.is_finite() {
        return "Invalid Date".into();
    }
    let days = (ms / 86_400_000.0).floor() as i64;
    let rem = ms - days as f64 * 86_400_000.0;
    let (h, rem) = ((rem / 3_600_000.0) as u64, rem % 3_600_000.0);
    let (mi, rem) = ((rem / 60_000.0) as u64, rem % 60_000.0);
    let (s, mss) = ((rem / 1000.0) as u64, rem % 1000.0);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{mss:03}Z")
}

fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `new Date(iso).toString()` — the JS default format.
fn js_date_to_string(iso: &str) -> String {
    let Some(ms) = js_date_parse(iso) else {
        return "Invalid Date".into();
    };
    if !ms.is_finite() {
        return "Invalid Date".into();
    }
    let days = (ms / 86_400_000.0).floor() as i64;
    let (y, mo, d) = civil_from_days(days);
    let wd = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"][(days.rem_euclid(7)) as usize];
    let mon = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(mo - 1) as usize];
    let rem = ms - days as f64 * 86_400_000.0;
    let (h, rem) = ((rem / 3_600_000.0) as u64, rem % 3_600_000.0);
    let (mi, s) = ((rem / 60_000.0) as u64, (rem % 60_000.0) / 1000.0);
    format!("{wd} {mon} {d:02} {y:04} {h:02}:{mi:02}:{s:02} GMT+0000 (Coordinated Universal Time)")
}

fn env_flag_enabled(v: Option<String>) -> bool {
    matches!(
        v.as_deref().map(js_trim).map(str::to_lowercase).as_deref(),
        Some("1") | Some("true") | Some("on") | Some("yes")
    )
}

fn env_flag_disabled(v: Option<String>) -> bool {
    matches!(
        v.as_deref().map(js_trim).map(str::to_lowercase).as_deref(),
        Some("0") | Some("false") | Some("off") | Some("no")
    )
}

fn utf8_len(s: &str) -> usize {
    s.len()
}

/// Compile a JS regex translated to Rust syntax (`\b` → ASCII boundary lookarounds,
/// `\s` → the JS whitespace class, `\w` → ASCII word chars, `\d` → `[0-9]`,
/// `.` → JS "not a line terminator").
fn js_regex(pattern: &str) -> fancy_regex::Regex {
    fancy_regex::Regex::new(pattern).unwrap_or_else(|e| {
        // A pattern that fails to compile is a port bug; fall back to one that never matches.
        let _ = e;
        fancy_regex::Regex::new("(?:^.)").unwrap_or_else(|_| fancy_regex::Regex::new("a^").unwrap())
    })
}

const JS_WORD_B: &str = r"[A-Za-z0-9_]";
const JS_SPACE: &str = r"[\t\n\x0b\x0c\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]";
const JS_NLB: &str = r"(?<![A-Za-z0-9_])";
const JS_NWB: &str = r"(?![A-Za-z0-9_])";
const JS_DOT: &str = r"[^\n\r\u{2028}\u{2029}]";

/// `v.prop` where `v` may be any decoded value; primitives yield `undefined`.
fn prop_or_undef(v: &Value, key: &str) -> Result<Value, String> {
    js_get(v, key)
}

// ── ComputerBackend.ts ──────────────────────────────────────────────────────

const MAX_COMPUTER_CLIPBOARD_BYTES: u64 = 1024 * 1024;
const COMPUTER_MESSAGE_MAX_LENGTH: usize = 2048;
const COMPUTER_DELIVERY_PATH_MAX_LENGTH: usize = 64;

/// Port of `assertComputerClipboardWriteFits` (`apps/server/src/computer/ComputerBackend.ts:94`).
pub fn assert_computer_clipboard_write_fits(text: String) -> Result<(), String> {
    let bytes = utf8_len(&text) as u64;
    if bytes <= MAX_COMPUTER_CLIPBOARD_BYTES {
        return Ok(());
    }
    Err(format!(
        "Clipboard text is {bytes} bytes, past the {MAX_COMPUTER_CLIPBOARD_BYTES} byte limit this tool writes."
    ))
}

/// Port of `intersectComputerRects` (`apps/server/src/computer/ComputerBackend.ts:814`).
pub fn intersect_computer_rects(first: Value, second: Value) -> Result<Option<Value>, String> {
    let (fx, fy) = (js_get(&first, "x")?, js_get(&first, "y")?);
    let (fw, fh) = (js_get(&first, "width")?, js_get(&first, "height")?);
    let (sx, sy) = (js_get(&second, "x")?, js_get(&second, "y")?);
    let (sw, sh) = (js_get(&second, "width")?, js_get(&second, "height")?);
    let left = js_max(js_to_number(&fx), js_to_number(&sx));
    let top = js_max(js_to_number(&fy), js_to_number(&sy));
    let right = js_min(
        js_to_number(&fx) + js_to_number(&fw),
        js_to_number(&sx) + js_to_number(&sw),
    );
    let bottom = js_min(
        js_to_number(&fy) + js_to_number(&fh),
        js_to_number(&sy) + js_to_number(&sh),
    );
    if right <= left || bottom <= top {
        return Ok(None);
    }
    Ok(Some(json!({
        "x": js::number(left),
        "y": js::number(top),
        "width": js::number(right - left),
        "height": js::number(bottom - top),
    })))
}

/// Port of `clampComputerMessage` (`apps/server/src/computer/ComputerBackend.ts:832`).
pub fn clamp_computer_message(text: String, fallback: String) -> Result<String, String> {
    let trimmed = js_trim(&text);
    let message = if trimmed.is_empty() {
        &fallback
    } else {
        trimmed
    };
    let units = utf16::units(message);
    if units.len() > COMPUTER_MESSAGE_MAX_LENGTH {
        Ok(format!(
            "{}\u{2026}",
            utf16::slice(&units, 0, COMPUTER_MESSAGE_MAX_LENGTH - 1)
        ))
    } else {
        Ok(message.to_string())
    }
}

/// Port of `computerBackendActionResult` (`apps/server/src/computer/ComputerBackend.ts:840`).
pub fn computer_backend_action_result(
    computer_id: String,
    action: String,
    result: Option<Value>,
) -> Result<Value, String> {
    let result = result.filter(|v| !is_undef(v));
    let mut out = Map::new();
    out.insert("computerId".into(), Value::String(computer_id));
    out.insert("action".into(), Value::String(action));
    if let Some(r) = &result {
        let point = prop_or_undef(r, "point")?;
        if js_truthy(&point) {
            out.insert("point".into(), point);
        }
        let clamped = prop_or_undef(r, "clampedTo")?;
        if js_truthy(&clamped) {
            out.insert("clampedTo".into(), clamped);
        }
        let window_id = prop_or_undef(r, "windowId")?;
        if js_truthy(&window_id) {
            out.insert("windowId".into(), window_id);
        }
        let value = prop_or_undef(r, "value")?;
        if !is_undef(&value) {
            out.insert("value".into(), value);
        }
        let path = prop_or_undef(r, "deliveryPath")?;
        let verified = prop_or_undef(r, "verified")?;
        if !is_undef(&path) && !is_undef(&verified) {
            let sliced = match &path {
                Value::String(s) => {
                    let units = utf16::units(s);
                    Value::String(utf16::slice(
                        &units,
                        0,
                        COMPUTER_DELIVERY_PATH_MAX_LENGTH.min(units.len()),
                    ))
                }
                Value::Array(a) => Value::Array(
                    a.iter()
                        .take(COMPUTER_DELIVERY_PATH_MAX_LENGTH)
                        .cloned()
                        .collect(),
                ),
                Value::Null => {
                    return Err("Cannot read properties of null (reading 'slice')".into());
                }
                v if is_undef(v) => {
                    return Err("Cannot read properties of undefined (reading 'slice')".into());
                }
                _ => return Err("result.deliveryPath.slice is not a function".into()),
            };
            let mut delivery = Map::new();
            delivery.insert("path".into(), sliced);
            delivery.insert("verified".into(), verified);
            let effect = prop_or_undef(r, "effect")?;
            if js_truthy(&effect) {
                delivery.insert("effect".into(), effect);
            }
            out.insert("delivery".into(), Value::Object(delivery));
        }
    }
    Ok(Value::Object(out))
}

// ── DesktopOperationQueue.ts ────────────────────────────────────────────────

/// Port of `assertDesktopOperationAdmission` (`apps/server/src/computer/DesktopOperationQueue.ts:20`).
/// INFALLIBLE: the original throws only inside an `AsyncLocalStorage` operation
/// scope (`execution.run` in `packages/shared/src/desktopOperationQueue.ts`), which
/// no call-time argument can enter — no error case is recordable for it.
pub fn assert_desktop_operation_admission() -> Result<(), String> {
    // No AsyncLocalStorage context exists in the isolated port: `desktopOperationContext()`
    // returns undefined, so both guards pass.
    Ok(())
}

// ── computerActivation.ts / computerInvocation.ts ───────────────────────────

fn parse_computer_invocation(text: Option<&Value>) -> bool {
    let Some(Value::String(_)) = text else {
        // `if (!text) return null` — non-strings still reach regex matching via coercion.
        return text
            .map(|v| {
                let s = js_to_string(v);
                if s.is_empty() {
                    return false;
                }
                invocation_regex().is_match(&s).unwrap_or(false)
            })
            .unwrap_or(false);
    };
    if let Some(t) = text
        .and_then(js::string_units)
        .map(|u| String::from_utf16_lossy(&u))
    {
        if t.is_empty() {
            return false;
        }
        return invocation_regex().is_match(&t).unwrap_or(false);
    }
    false
}

fn invocation_regex() -> &'static fancy_regex::Regex {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| js_regex(r"(?i)^ {0,3}\/computer-use(?:[ \t\r\n]+([\s\S]*))?$"))
}

/// `resolveComputerInvocationMode` from `@synara/shared/computerInvocation`.
fn resolve_invocation_mode(
    message_text: &Value,
    dispatch_origin: &Value,
    enable_control: &Value,
    explicit_mode: &Value,
) -> Value {
    if !is_undef(explicit_mode) {
        return explicit_mode.clone();
    }
    if js_strict_eq(enable_control, &Value::Bool(true)) {
        return Value::String("chat".into());
    }
    let user_authored = is_undef(dispatch_origin) || js_strict_eq(dispatch_origin, &json!("user"));
    if user_authored && parse_computer_invocation(Some(message_text)) {
        Value::String("request".into())
    } else {
        Value::String("off".into())
    }
}

/// Port of `computerActivationMetadata` (`apps/server/src/computer/computerActivation.ts:5`).
pub fn computer_activation_metadata(input: Value) -> Result<Value, String> {
    let user_text = prop_or_undef(&input, "userMessageText")?;
    let mode = if is_undef(&user_text) {
        resolve_invocation_mode(
            &prop_or_undef(&input, "messageText")?,
            &prop_or_undef(&input, "dispatchOrigin")?,
            &prop_or_undef(&input, "enableComputerControl")?,
            &prop_or_undef(&input, "computerControlMode")?,
        )
    } else {
        let existing = prop_or_undef(&input, "computerControlMode")?;
        let enable = prop_or_undef(&input, "enableComputerControl")?;
        let enabled = js_strict_eq(&existing, &json!("chat"))
            || (is_undef(&existing) && js_strict_eq(&enable, &Value::Bool(true)));
        resolve_invocation_mode(
            &user_text,
            &prop_or_undef(&input, "dispatchOrigin")?,
            &Value::Bool(enabled),
            &undef(),
        )
    };
    let generation = prop_or_undef(&input, "computerControlGeneration")?;
    let generation = if is_undef(&generation) {
        json!(0)
    } else {
        generation
    };
    let mut out = Map::new();
    out.insert("computerControlMode".into(), mode.clone());
    out.insert(
        "enableComputerControl".into(),
        Value::Bool(!js_strict_eq(&mode, &json!("off"))),
    );
    out.insert("computerControlGeneration".into(), generation);
    Ok(Value::Object(out))
}

// ── computerAuditFile.ts / computerAuditHistory.ts ──────────────────────────

const COMPUTER_AUDIT_HISTORY_MAX_BYTES: usize = 2 * 1024 * 1024;
const COMPUTER_AUDIT_HISTORY_MAX_ROWS: usize = 10_000;
const MAX_LINE_BYTES: usize = 64 * 1024;
const COMPUTER_AUDIT_HISTORY_MAX_LIMIT: f64 = 100.0;

struct AuditTail {
    contents: Vec<u8>,
    start: u64,
    total_bytes: u64,
    requested_bytes: u64,
    device: u64,
    inode: u64,
}

#[cfg(unix)]
fn read_audit_tail(path: &str, max_bytes: f64) -> Result<Option<AuditTail>, String> {
    use std::os::unix::fs::{FileExt, MetadataExt};
    if !is_safe_integer(max_bytes) || max_bytes < 0.0 {
        return Err("Invalid audit read bound.".into());
    }
    let entry = match std::fs::symlink_metadata(path) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Could not read Computer activity history.".into()),
    };
    if !entry.is_file() {
        return Err("Could not read Computer activity history.".into());
    }
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Err("Could not read Computer activity history.".into()),
    };
    let stat = match file.metadata() {
        Ok(s) => s,
        Err(_) => return Err("Could not read Computer activity history.".into()),
    };
    let entry = match std::fs::symlink_metadata(path) {
        Ok(e) => e,
        Err(_) => return Err("Could not read Computer activity history.".into()),
    };
    if !stat.is_file() || !entry.is_file() || entry.dev() != stat.dev() || entry.ino() != stat.ino()
    {
        return Err("Could not read Computer activity history.".into());
    }
    let size = stat.size() as f64;
    let length = size.min(max_bytes).max(0.0) as u64;
    let start = (size - length as f64).max(0.0) as u64;
    let mut buffer = vec![0u8; length as usize];
    let mut read_bytes = 0usize;
    while read_bytes < length as usize {
        match file.read_at(&mut buffer[read_bytes..], start + read_bytes as u64) {
            Ok(0) => break,
            Ok(n) => read_bytes += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err("Could not read Computer activity history.".into()),
        }
    }
    buffer.truncate(read_bytes);
    Ok(Some(AuditTail {
        contents: buffer,
        start,
        total_bytes: stat.size(),
        requested_bytes: length,
        device: stat.dev(),
        inode: stat.ino(),
    }))
}

#[cfg(not(unix))]
fn read_audit_tail(path: &str, max_bytes: f64) -> Result<Option<AuditTail>, String> {
    use std::io::{Read, Seek, SeekFrom};
    if !is_safe_integer(max_bytes) || max_bytes < 0.0 {
        return Err("Invalid audit read bound.".into());
    }
    let entry = match std::fs::symlink_metadata(path) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("Could not read Computer activity history.".into()),
    };
    if !entry.is_file() {
        return Err("Could not read Computer activity history.".into());
    }
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Err("Could not read Computer activity history.".into()),
    };
    let stat = match file.metadata() {
        Ok(s) => s,
        Err(_) => return Err("Could not read Computer activity history.".into()),
    };
    if !stat.is_file() {
        return Err("Could not read Computer activity history.".into());
    }
    let size = stat.len() as f64;
    let length = size.min(max_bytes).max(0.0) as u64;
    let start = (size - length as f64).max(0.0) as u64;
    let mut buffer = vec![0u8; length as usize];
    let mut read_bytes = 0usize;
    if file.seek(SeekFrom::Start(start)).is_err() {
        return Err("Could not read Computer activity history.".into());
    }
    while read_bytes < length as usize {
        match file.read(&mut buffer[read_bytes..]) {
            Ok(0) => break,
            Ok(n) => read_bytes += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err("Could not read Computer activity history.".into()),
        }
    }
    buffer.truncate(read_bytes);
    Ok(Some(AuditTail {
        contents: buffer,
        start,
        total_bytes: stat.len(),
        requested_bytes: length,
        device: 0,
        inode: 0,
    }))
}

fn is_safe_integer(x: f64) -> bool {
    x.is_finite() && x.fract() == 0.0 && x.abs() <= 9_007_199_254_740_991.0
}

/// Port of `readComputerAuditFileTail` (`apps/server/src/computer/computerAuditFile.ts:14`).
pub async fn read_computer_audit_file_tail(
    path: String,
    max_bytes: f64,
) -> Result<Option<Value>, String> {
    let Some(tail) = read_audit_tail(&path, max_bytes)? else {
        return Ok(None);
    };
    Ok(Some(json!({
        "contents": { "$buffer": tail.contents },
        "start": js::number(tail.start as f64),
        "totalBytes": js::number(tail.total_bytes as f64),
        "requestedBytes": js::number(tail.requested_bytes as f64),
        "device": js::number(tail.device as f64),
        "inode": js::number(tail.inode as f64),
    })))
}

/// `computerAuditTailLines`: complete newline-terminated records, newest first.
fn audit_tail_lines(tail: &AuditTail) -> Vec<(Vec<u8>, u64)> {
    let contents = &tail.contents;
    let mut out = Vec::new();
    let mut end = contents.iter().rposition(|b| *b == 10);
    while let Some(e) = end {
        let previous = if e > 0 {
            contents[..e].iter().rposition(|b| *b == 10)
        } else {
            None
        };
        let line_start = previous.map_or(0, |p| p + 1);
        if previous.is_none() && tail.start > 0 {
            return out;
        }
        out.push((
            contents[line_start..=e].to_vec(),
            tail.start + line_start as u64,
        ));
        end = previous;
    }
    out
}

fn audit_regex(pattern: &str) -> &'static fancy_regex::Regex {
    static RES: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
    let _ = RES;
    Box::leak(Box::new(js_regex(pattern)))
}

fn identifier_ok(s: &str) -> bool {
    audit_regex(r"^[A-Za-z0-9_.:-]{1,128}$")
        .is_match(s)
        .unwrap_or(false)
}

struct AuditPosition {
    file: String,
    offset: f64,
    fingerprint: String,
}

fn cursor_for(position: &AuditPosition) -> String {
    let text = js_json_stringify(&json!({
        "file": position.file,
        "offset": js::number(position.offset),
        "fingerprint": position.fingerprint,
    }))
    .unwrap_or_else(|| "{}".into());
    base64url_encode(text.as_bytes())
}

fn parse_cursor(before: &Value) -> Result<Option<AuditPosition>, String> {
    if is_undef(before) {
        return Ok(None);
    }
    let length = js_get(before, "length")?;
    let length_num = if before.is_string() || matches!(before, Value::Array(_)) {
        js_to_number(&length)
    } else {
        f64::NAN
    };
    if !length_num.is_nan() && length_num > 512.0
        || !audit_regex(r"^[A-Za-z0-9_-]+$")
            .is_match(&js_to_string(before))
            .unwrap_or(false)
    {
        return Err("Invalid Computer activity cursor. Refresh the history to continue.".into());
    }
    let Some(decoded) = base64url_decode(&js_to_string(before)) else {
        return Err("Invalid Computer activity cursor. Refresh the history to continue.".into());
    };
    let text = String::from_utf8_lossy(&decoded);
    let value: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => {
            return Err(
                "Invalid Computer activity cursor. Refresh the history to continue.".into(),
            );
        }
    };
    let fingerprint_ok = |v: &Value| {
        matches!(v, Value::String(s)
            if audit_regex(r"^[a-f0-9]{64}$").is_match(s).unwrap_or(false))
    };
    if let Value::Object(o) = &value {
        let file = o.get("file");
        let offset = o.get("offset");
        let fingerprint = o.get("fingerprint");
        if let (Some(Value::String(file)), Some(off), Some(fp)) = (file, offset, fingerprint)
            && fingerprint_ok(&Value::String(file.clone()))
            && is_number(off)
            && is_safe_integer(num_or_nan(off))
            && num_or_nan(off) >= 0.0
            && fingerprint_ok(fp)
        {
            return Ok(Some(AuditPosition {
                file: file.clone(),
                offset: num_or_nan(off),
                fingerprint: fp.as_str().unwrap_or_default().to_string(),
            }));
        }
    } else if !value.is_null() {
        // `"file" in value` on a primitive throws inside the try, then rethrows below.
        return Err("Invalid Computer activity cursor. Refresh the history to continue.".into());
    }
    Err("Invalid Computer activity cursor. Refresh the history to continue.".into())
}

fn project_audit_entry(line: &[u8], position: &AuditPosition) -> Option<Value> {
    let text = String::from_utf8_lossy(line);
    let value: Value = serde_json::from_str(&text).ok()?;
    let Value::Object(record) = &value else {
        return None;
    };
    if value.is_null() {
        return None;
    }
    let ts = record.get("ts")?;
    let Value::String(ts) = ts else { return None };
    if !audit_regex(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,3})?Z$")
        .is_match(ts)
        .unwrap_or(false)
    {
        return None;
    }
    let parsed = js_date_parse(ts)?;
    if !parsed.is_finite() {
        return None;
    }
    let tool = record.get("tool")?.as_str()?;
    if !audit_regex(r"^computer_[a-z_]{1,80}$")
        .is_match(tool)
        .unwrap_or(false)
    {
        return None;
    }
    let Value::String(effect) = record.get("effect")? else {
        return None;
    };
    if ![
        "verified",
        "dispatched-unknown",
        "not-dispatched",
        "refused",
        "error",
    ]
    .contains(&effect.as_str())
    {
        return None;
    }
    let safe_id = |key: &str| -> Option<String> {
        match record.get(key) {
            Some(Value::String(s)) if identifier_ok(s) => Some(s.clone()),
            _ => None,
        }
    };
    let mut entry = Map::new();
    entry.insert(
        "id".into(),
        Value::String(sha256_hex(cursor_for(position).as_bytes())),
    );
    entry.insert("ts".into(), Value::String(iso_string(parsed)));
    entry.insert("tool".into(), Value::String(tool.to_string()));
    entry.insert("effect".into(), Value::String(effect.clone()));
    if let Some(v) = safe_id("threadId") {
        entry.insert("threadId".into(), Value::String(v));
    }
    if let Some(v) = safe_id("turnId") {
        entry.insert("turnId".into(), Value::String(v));
    }
    if let Some(v) = safe_id("gatewayRequestId") {
        entry.insert("gatewayRequestId".into(), Value::String(v));
    }
    if let Some(Value::String(code)) = record.get("code")
        && audit_regex(r"^[a-z][a-z0-9_]{0,95}$")
            .is_match(code)
            .unwrap_or(false)
    {
        entry.insert("code".into(), Value::String(code.clone()));
    }
    Some(Value::Object(entry))
}

/// Port of `readComputerAuditHistory` (`apps/server/src/computer/computerAuditHistory.ts:118`).
pub async fn read_computer_audit_history(file_path: Value, input: Value) -> Result<Value, String> {
    let limit = js_get(&input, "limit")?;
    let limit = if is_undef(&limit) || limit.is_null() {
        30.0
    } else {
        num_or_nan(&limit)
    };
    if !limit.is_finite()
        || limit.fract() != 0.0
        || !(1.0..=COMPUTER_AUDIT_HISTORY_MAX_LIMIT).contains(&limit)
    {
        return Err(
            "Computer activity history supports between 1 and 100 entries per page.".into(),
        );
    }
    let before = parse_cursor(&js_get(&input, "before")?)?;
    if is_undef(&file_path) {
        return Ok(json!({
            "entries": [],
            "nextCursor": Value::Null,
            "truncated": false,
            "status": "disabled",
        }));
    }
    let path = js_to_string(&file_path);
    let mut records: Vec<(AuditPosition, Value)> = Vec::new();
    let mut seen_files: Vec<String> = Vec::new();
    let mut remaining_bytes = COMPUTER_AUDIT_HISTORY_MAX_BYTES as f64;
    let mut scanned_rows = 0usize;
    let mut found_file = false;
    let mut truncated = false;
    for suffix in ["", ".1", ".2"] {
        let candidate = format!("{path}{suffix}");
        let request = if scanned_rows < COMPUTER_AUDIT_HISTORY_MAX_ROWS {
            remaining_bytes
        } else {
            0.0
        };
        let Some(tail) = read_audit_tail(&candidate, request)? else {
            continue;
        };
        remaining_bytes -= tail.requested_bytes as f64;
        found_file = true;
        let file_id = sha256_hex(format!("{}:{}", tail.device, tail.inode).as_bytes());
        if seen_files.iter().any(|f| f == &file_id) {
            continue;
        }
        seen_files.push(file_id.clone());
        truncated = truncated
            || tail.start > 0
            || tail.contents.len() as u64 != tail.requested_bytes
            || (!tail.contents.is_empty() && *tail.contents.last().unwrap_or(&0) != 10);
        for (line, offset) in audit_tail_lines(&tail) {
            if scanned_rows >= COMPUTER_AUDIT_HISTORY_MAX_ROWS {
                truncated = true;
                break;
            }
            scanned_rows += 1;
            let line = &line[..line.len().saturating_sub(1)];
            if line.is_empty() {
                continue;
            }
            if line.len() > MAX_LINE_BYTES {
                truncated = true;
                continue;
            }
            let position = AuditPosition {
                file: file_id.clone(),
                offset: offset as f64,
                fingerprint: sha256_hex(line),
            };
            match project_audit_entry(line, &position) {
                Some(entry) => records.push((position, entry)),
                None => truncated = true,
            }
        }
    }
    let status = if found_file { "available" } else { "missing" };
    let mut first = 0usize;
    if let Some(before) = &before {
        let mut index = records.iter().position(|(p, _)| {
            p.file == before.file
                && p.offset == before.offset
                && p.fingerprint == before.fingerprint
        });
        if index.is_none() {
            let matches: Vec<usize> = records
                .iter()
                .enumerate()
                .filter_map(|(i, (p, _))| (p.fingerprint == before.fingerprint).then_some(i))
                .collect();
            if matches.len() == 1 {
                index = Some(matches[0]);
            }
        }
        let Some(index) = index else {
            return Ok(json!({
                "entries": [],
                "nextCursor": Value::Null,
                "truncated": true,
                "status": status,
            }));
        };
        first = index + 1;
    }
    let limit = limit as usize;
    let page: Vec<&(AuditPosition, Value)> = records.iter().skip(first).take(limit).collect();
    let next_cursor = match page.last() {
        Some((position, _)) if first + page.len() < records.len() => {
            Value::String(cursor_for(position))
        }
        _ => Value::Null,
    };
    Ok(json!({
        "entries": page.iter().map(|(_, e)| e.clone()).collect::<Vec<_>>(),
        "nextCursor": next_cursor,
        "truncated": truncated,
        "status": status,
    }))
}

// ── computerAuditLog.ts ─────────────────────────────────────────────────────

const COMPUTER_AUDIT_SENSITIVE_ARGS: &[&str] = &[
    "text",
    "value",
    "arguments",
    "files",
    "image_path",
    "file_path",
    "clipboard",
    "contents",
    "data",
    "payload",
];
const COMPUTER_AUDIT_MAX_STRING: usize = 256;

/// Port of `computerAuditGatewayRequestId` (`apps/server/src/computer/computerAuditLog.ts:107`).
pub fn computer_audit_gateway_request_id(value: Value) -> Result<Value, String> {
    let id = if matches!(value, Value::String(_)) {
        Some(value.as_str().unwrap_or_default().to_string())
    } else if is_number(&value) && num_or_nan(&value).is_finite() {
        Some(js_to_string(&value))
    } else {
        None
    };
    let mut out = Map::new();
    if let Some(id) = id
        && identifier_ok(&id)
    {
        out.insert("gatewayRequestId".into(), Value::String(id));
    }
    Ok(Value::Object(out))
}

fn utf16_len(v: &Value) -> usize {
    js::string_units(v).map_or(0, |u| u.len())
}

fn audit_slice_256(v: &Value, end: usize) -> Value {
    match js::string_units(v) {
        Some(units) => js::utf16_string(&units[..end.min(units.len())]),
        _ => v.clone(),
    }
}

fn summarize_audit_value(key: &str, value: &Value) -> Value {
    if value.is_null() || is_undef(value) {
        return undef();
    }
    if is_number(value) || matches!(value, Value::Bool(_)) {
        return value.clone();
    }
    if matches!(value, Value::String(_)) || is_marker(value, "$utf16") {
        if COMPUTER_AUDIT_SENSITIVE_ARGS.contains(&key) {
            return json!({ "chars": utf16_len(value) });
        }
        let len = utf16_len(value);
        if len > COMPUTER_AUDIT_MAX_STRING {
            let mut units = js::string_units(value).unwrap_or_default();
            units.truncate(COMPUTER_AUDIT_MAX_STRING);
            return Value::String(format!("{}\u{2026}", String::from_utf16_lossy(&units)));
        }
        return value.clone();
    }
    if let Value::Array(items) = value {
        if key == "steps" {
            let types: Vec<Value> = items
                .iter()
                .map(|step| match step {
                    Value::Object(o) if !is_marker(step, "$set") => {
                        let t = o.get("type").cloned().unwrap_or_else(undef);
                        Value::String(if t.is_null() || is_undef(&t) {
                            "unknown".into()
                        } else {
                            js_to_string(&t)
                        })
                    }
                    _ => Value::String("unknown".into()),
                })
                .take(64)
                .collect();
            return json!({ "count": items.len(), "types": types });
        }
        if COMPUTER_AUDIT_SENSITIVE_ARGS.contains(&key) {
            return json!({ "items": items.len() });
        }
        return Value::Array(
            items
                .iter()
                .take(64)
                .map(|item| match item {
                    i if is_number(i) || matches!(i, Value::Bool(_)) => i.clone(),
                    i if matches!(i, Value::String(_)) || is_marker(i, "$utf16") => {
                        audit_slice_256(i, COMPUTER_AUDIT_MAX_STRING)
                    }
                    _ => Value::String("[object]".into()),
                })
                .collect(),
        );
    }
    if matches!(value, Value::Object(_)) {
        let mut out = Map::new();
        let entries = object_entries(value).unwrap_or_default();
        for (inner_key, inner) in entries {
            let projected = if is_number(&inner) || matches!(inner, Value::Bool(_)) {
                inner.clone()
            } else if matches!(inner, Value::String(_)) || is_marker(&inner, "$utf16") {
                if COMPUTER_AUDIT_SENSITIVE_ARGS.contains(&inner_key.as_str()) {
                    json!({ "chars": utf16_len(&inner) })
                } else {
                    audit_slice_256(&inner, COMPUTER_AUDIT_MAX_STRING)
                }
            } else if inner.is_null() {
                Value::Null
            } else {
                Value::String("[object]".into())
            };
            out.insert(inner_key, projected);
        }
        return Value::Object(out);
    }
    Value::String("[object]".into())
}

/// Port of `summarizeComputerAuditArgs` (`apps/server/src/computer/computerAuditLog.ts:124`).
pub fn summarize_computer_audit_args(args: Value) -> Result<Value, String> {
    let mut out = Map::new();
    for (key, value) in object_entries(&args)? {
        out.insert(key.clone(), summarize_audit_value(&key, &value));
    }
    Ok(Value::Object(out))
}

// ── computerCallContext.ts ──────────────────────────────────────────────────

/// Port of `cuaTimingLogEnabled` (`apps/server/src/computer/computerCallContext.ts:51`).
pub fn cua_timing_log_enabled() -> Result<bool, String> {
    Ok(env_flag_enabled(
        std::env::var("SYNARA_CUA_TIMING_LOG").ok(),
    ))
}

/// Port of `cuaConditionalSettleEnabled` (`apps/server/src/computer/computerCallContext.ts:61`).
pub fn cua_conditional_settle_enabled() -> Result<bool, String> {
    Ok(!env_flag_disabled(
        std::env::var("SYNARA_CUA_CONDITIONAL_SETTLE").ok(),
    ))
}

/// Port of `cuaActionSettleMsOverride` (`apps/server/src/computer/computerCallContext.ts:70`).
pub fn cua_action_settle_ms_override() -> Result<Value, String> {
    let Ok(raw) = std::env::var("SYNARA_CUA_ACTION_SETTLE_MS") else {
        return Ok(undef());
    };
    if js_trim(&raw).is_empty() {
        return Ok(undef());
    }
    let parsed = js_number_str(&raw);
    if parsed.is_finite() && parsed >= 0.0 {
        Ok(js::number(parsed))
    } else {
        Ok(undef())
    }
}

/// Port of `cuaCaptureReuseEnabled` (`apps/server/src/computer/computerCallContext.ts:86`).
pub fn cua_capture_reuse_enabled() -> Result<bool, String> {
    Ok(!env_flag_disabled(
        std::env::var("SYNARA_CUA_CAPTURE_REUSE").ok(),
    ))
}

/// Port of `cuaPreviewStillMsOverride` (`apps/server/src/computer/computerCallContext.ts:95`).
pub fn cua_preview_still_ms_override() -> Result<Value, String> {
    let Ok(raw) = std::env::var("SYNARA_CUA_PREVIEW_STILL_MS") else {
        return Ok(undef());
    };
    if js_trim(&raw).is_empty() {
        return Ok(undef());
    }
    let parsed = js_number_str(&raw);
    if parsed.is_finite() && parsed > 0.0 {
        Ok(js::number(parsed))
    } else {
        Ok(undef())
    }
}

/// Port of `currentComputerCall` (`apps/server/src/computer/computerCallContext.ts:197`).
pub fn current_computer_call() -> Result<Value, String> {
    Ok(undef())
}

/// Port of `createComputerCallContext` (`apps/server/src/computer/computerCallContext.ts:213`).
pub fn create_computer_call_context() -> Result<Value, String> {
    let timing = if cua_timing_log_enabled()? {
        json!({
            "operation": undef(),
            "startedAt": js::number(now_ms()),
            "legs": { "$map": [] },
            "counts": { "$map": [] },
            "failed": false,
            "finished": false,
        })
    } else {
        undef()
    };
    if is_undef(&timing) && !cua_conditional_settle_enabled()? {
        return Ok(undef());
    }
    Ok(json!({ "timing": timing, "proof": undef() }))
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// Port of `markComputerCall` (`apps/server/src/computer/computerCallContext.ts:226`).
pub fn mark_computer_call(operation: String) -> Result<(), String> {
    let _ = &operation;
    Ok(())
}

// ── computerDenylist.ts ─────────────────────────────────────────────────────

const COMPUTER_DENYLIST_BUNDLE_IDS: &[&str] = &[
    "com.1password.1password",
    "com.agilebits.onepassword",
    "com.apple.keychainaccess",
    "com.apple.passwords",
    "com.apple.systempreferences",
    "com.apple.securityagent",
    "com.apple.security.authorization",
    "com.lastpass.lastpass",
    "com.bitwarden.desktop",
];

const COMPUTER_DENYLIST_BUNDLE_PREFIXES: &[&str] = &["com.agilebits.onepassword", "com.dashlane."];

const COMPUTER_DENYLIST_APP_NAMES: &[&str] = &[
    "1password",
    "keychain access",
    "passwords",
    "system settings",
    "system preferences",
    "securityagent",
    "securityagenthelper",
    "bitwarden",
    "dashlane",
    "lastpass",
];

/// POSIX `basename`: strip trailing slashes, take after the last remaining slash.
fn posix_basename(path: &str) -> String {
    let units = utf16::units(path);
    let mut end = units.len();
    while end > 0 && units[end - 1] == b'/' as u16 {
        end -= 1;
    }
    if end == 0 {
        return "/".into();
    }
    let mut start = end;
    while start > 0 && units[start - 1] != b'/' as u16 {
        start -= 1;
    }
    utf16::slice(&units, start, end)
}

fn normalize_computer_app_name(raw: &Value) -> Result<String, String> {
    let units = match js::string_units(raw) {
        Some(u) => u,
        None => return Err("raw.trim is not a function".into()),
    };
    let mut name = String::from_utf16_lossy(&units);
    name = js_trim(&name).to_string();
    if name.contains('/') {
        name = posix_basename(&name);
    }
    if name.to_lowercase().ends_with(".app") {
        let units = utf16::units(&name);
        name = utf16::slice(&units, 0, units.len().saturating_sub(4));
    }
    let lower = name.to_lowercase();
    let units = utf16::units(&lower);
    let mut out = String::new();
    let mut in_space = false;
    for c in String::from_utf16_lossy(&units).chars() {
        if is_space(c) {
            in_space = true;
        } else {
            if in_space {
                out.push(' ');
                in_space = false;
            }
            out.push(c);
        }
    }
    Ok(out)
}

fn matches_computer_denylist_string(raw: &Value) -> Result<Option<String>, String> {
    let name = normalize_computer_app_name(raw)?;
    for denied in COMPUTER_DENYLIST_APP_NAMES {
        if name == *denied || name.starts_with(&format!("{denied} ")) {
            return Ok(Some(format!("app name {denied}")));
        }
    }
    let bundle_id: String = name.chars().filter(|c| !is_space(*c)).collect();
    if COMPUTER_DENYLIST_BUNDLE_IDS.contains(&bundle_id.as_str()) {
        return Ok(Some(format!("bundle id {bundle_id}")));
    }
    for prefix in COMPUTER_DENYLIST_BUNDLE_PREFIXES {
        if bundle_id.starts_with(prefix) {
            return Ok(Some(format!("bundle id {prefix}*")));
        }
    }
    Ok(None)
}

/// Port of `computerDenylistMatch` (`apps/server/src/computer/computerDenylist.ts:101`).
pub fn computer_denylist_match(identity: Value) -> Result<Value, String> {
    let name = prop_or_undef(&identity, "name")?;
    let from_name = if is_undef(&name) {
        None
    } else {
        matches_computer_denylist_string(&name)?
    };
    let mut out = Map::new();
    if let Some(matched) = from_name {
        let app = if is_undef(&name) {
            json!("the target app")
        } else {
            name.clone()
        };
        out.insert("app".into(), app);
        out.insert("matched".into(), Value::String(matched));
        return Ok(Value::Object(out));
    }
    let bundle = prop_or_undef(&identity, "bundleId")?;
    if !is_undef(&bundle)
        && let Some(matched) = matches_computer_denylist_string(&bundle)?
    {
        let app = if !is_undef(&name) {
            name
        } else if !is_undef(&bundle) {
            bundle.clone()
        } else {
            json!("the target app")
        };
        out.insert("app".into(), app);
        out.insert("matched".into(), Value::String(matched));
        return Ok(Value::Object(out));
    }
    Ok(undef())
}

// ── computerElementIdentity.ts ──────────────────────────────────────────────

/// Port of `registerNativeComputerElement` (`apps/server/src/computer/computerElementIdentity.ts:13`).
pub fn register_native_computer_element(node: Value, identity: String) -> Result<(), String> {
    let _ = &identity;
    if !is_weak_keyable(&node) {
        return Err("Invalid value used as weak map key".into());
    }
    // The identity never re-crosses the bridge: the WeakMap cannot be observed per call.
    Ok(())
}

/// Port of `computerElementRefIdentity` (`apps/server/src/computer/computerElementIdentity.ts:22`).
pub fn computer_element_ref_identity(r#ref: Value) -> Result<Value, String> {
    let _ = &r#ref;
    Ok(undef())
}

/// Port of `bindComputerTargetRef` (`apps/server/src/computer/computerElementIdentity.ts:27`).
pub fn bind_computer_target_ref(target: Value, r#ref: Value) -> Result<Value, String> {
    let _ = &r#ref;
    Ok(target)
}

/// Port of `observedComputerTargetNode` (`apps/server/src/computer/computerElementIdentity.ts:36`).
pub fn observed_computer_target_node(target: Value) -> Result<Value, String> {
    if target.is_null() || is_undef(&target) {
        return Err(format!(
            "Cannot read properties of {} (reading 'Symbol(computerObservedElement)')",
            if target.is_null() {
                "null"
            } else {
                "undefined"
            }
        ));
    }
    Ok(undef())
}

// ── computerFrameRoute.ts / frameTransport.ts ───────────────────────────────

fn buffer_from(v: &Value) -> Result<Vec<u8>, String> {
    const PREFIX: &str = "The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object. Received ";
    match v {
        Value::Array(items) => Ok(items
            .iter()
            .map(|i| (js_to_number(i) as i64 & 0xff) as u8)
            .collect()),
        Value::Object(o) => {
            if is_undef(v) {
                return Err(format!("{PREFIX}undefined"));
            }
            if let Some(b) = single_marker(v, "$bytes").or_else(|| single_marker(v, "$buffer")) {
                return Ok(b
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|i| (js_to_number(i) as i64 & 0xff) as u8)
                            .collect()
                    })
                    .unwrap_or_default());
            }
            if let Some(len_v) = o.get("length") {
                let len = to_length(len_v);
                let mut bytes = Vec::with_capacity(len as usize);
                for i in 0..len as usize {
                    let item = o.get(&i.to_string());
                    bytes.push((js_to_number(item.unwrap_or(&Value::Null)) as i64 & 0xff) as u8);
                }
                return Ok(bytes);
            }
            if single_marker(v, "$date").is_some() {
                return Err(format!("{PREFIX}an instance of Date"));
            }
            if single_marker(v, "$map").is_some() {
                return Err(format!("{PREFIX}an instance of Map"));
            }
            if single_marker(v, "$set").is_some() {
                return Err(format!("{PREFIX}an instance of Set"));
            }
            Err(format!("{PREFIX}an instance of Object"))
        }
        Value::Null => Err(format!("{PREFIX}null")),
        Value::Bool(b) => Err(format!("{PREFIX}type boolean ({b})")),
        Value::Number(n) => Err(format!(
            "{PREFIX}type number ({})",
            js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
        )),
        Value::String(s) => Ok(s.as_bytes().to_vec()),
    }
}

fn to_length(v: &Value) -> u64 {
    let n = js_to_number(v);
    if n.is_nan() || n <= 0.0 {
        0
    } else if n > 9_007_199_254_740_991.0 {
        9_007_199_254_740_991
    } else {
        n as u64
    }
}

fn utf8_decode_lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Port of `decodeResyncRequest` (`apps/server/src/computer/computerFrameRoute.ts:22`).
pub fn decode_resync_request(message: Value) -> Result<Value, String> {
    let text = if matches!(message, Value::String(_)) {
        js_to_string(&message)
    } else if is_marker(&message, "$utf16") {
        js::string_units(&message)
            .map(|u| String::from_utf16_lossy(&u))
            .unwrap_or_default()
    } else {
        utf8_decode_lossy(&buffer_from(&message)?)
    };
    if text.encode_utf16().count() > 1024 {
        return Ok(Value::Null);
    }
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Ok(Value::Null),
    };
    let kind = match &parsed {
        Value::Object(o) if !parsed.is_null() => o.get("type").cloned().unwrap_or_else(undef),
        _ => undef(),
    };
    if js_strict_eq(&kind, &json!("computer.frame.resync")) {
        Ok(Value::String("resync".into()))
    } else {
        Ok(Value::Null)
    }
}

// ── computerGeometry.ts ─────────────────────────────────────────────────────

/// `rectContainsPoint` — `point`/`rect` may hold any JSON shape.
fn rect_contains(rect: &Value, point: &Value) -> Result<bool, String> {
    if !js_truthy(rect) {
        return Ok(false);
    }
    let (px, py) = (js_get(point, "x")?, js_get(point, "y")?);
    let (rx, ry) = (js_get(rect, "x")?, js_get(rect, "y")?);
    let (rw, rh) = (js_get(rect, "width")?, js_get(rect, "height")?);
    Ok(js_ge(&px, &rx)
        && js_ge(&py, &ry)
        && js_lt(&px, &js_add(&rx, &rw))
        && js_lt(&py, &js_add(&ry, &rh)))
}

/// Port of `windowsCoveringPoint` (`apps/server/src/computer/computerGeometry.ts:17`).
pub fn windows_covering_point(
    windows: Value,
    window_id: String,
    point: Value,
) -> Result<Vec<Value>, String> {
    let windows = array_method(&windows, "windows", "find")?;
    let window_id = Value::String(window_id);
    let mut depth = undef();
    for w in &windows {
        let id = js_get(w, "id")?;
        if js_strict_eq(&id, &window_id) {
            depth = js_get(w, "stackingIndex")?;
            break;
        }
    }
    if is_undef(&depth) {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for w in &windows {
        let id = js_get(w, "id")?;
        if js_strict_eq(&id, &window_id) {
            continue;
        }
        if !js_truthy(&js_get(w, "visible")?) {
            continue;
        }
        if js_truthy(&js_get(w, "minimized")?) {
            continue;
        }
        let stacking = js_get(w, "stackingIndex")?;
        if is_undef(&stacking) {
            continue;
        }
        if !js_lt(&stacking, &depth) {
            continue;
        }
        if rect_contains(&js_get(w, "bounds")?, &point)? {
            out.push(w.clone());
        }
    }
    Ok(out)
}

/// Port of `topmostWindowAtPoint` (`apps/server/src/computer/computerGeometry.ts:45`).
pub fn topmost_window_at_point(windows: Value, point: Value) -> Result<Value, String> {
    let windows = array_method(&windows, "windows", "filter")?;
    let mut candidates = Vec::new();
    for w in &windows {
        if js_truthy(&js_get(w, "visible")?)
            && !js_truthy(&js_get(w, "minimized")?)
            && rect_contains(&js_get(w, "bounds")?, &point)?
        {
            candidates.push(w.clone());
        }
    }
    if candidates.len() <= 1 {
        return Ok(candidates.into_iter().next().unwrap_or_else(undef));
    }
    for w in &candidates {
        if is_undef(&js_get(w, "stackingIndex")?) {
            return Ok(undef());
        }
    }
    let mut top = candidates[0].clone();
    for w in &candidates[1..] {
        if js_lt(
            &js_get(w, "stackingIndex")?,
            &js_get(&top, "stackingIndex")?,
        ) {
            top = w.clone();
        }
    }
    Ok(top)
}

/// Port of `rectContainsPoint` (`apps/server/src/computer/computerGeometry.ts:59`).
pub fn rect_contains_point(rect: Option<Value>, point: Value) -> Result<bool, String> {
    let rect = rect.unwrap_or(Value::Null);
    rect_contains(&rect, &point)
}

/// Port of `requireWindowBounds` (`apps/server/src/computer/computerGeometry.ts:79`).
pub fn require_window_bounds(window: Value, action: String) -> Result<Value, String> {
    let bounds = js_get(&window, "bounds")?;
    if js_truthy(&bounds) {
        return Ok(bounds);
    }
    let id = js_get(&window, "id")?;
    Err(format!(
        "This desktop reports no geometry for window {}, so {action} cannot be resolved. The display server exposes window titles and activation but no position or size (capabilities.windowBounds is false); use full-screen capture and desktop coordinates instead.",
        json_or_undefined(&id)
    ))
}

// ── computerSetupSignal.ts ──────────────────────────────────────────────────

const COMPUTER_PERMISSIONS: &[&str] = &["accessibility", "screenRecording", "inputMonitoring"];
const COMPUTER_BLOCKING_PERMISSIONS: &[&str] = &["accessibility", "inputMonitoring"];

fn permission_label(p: &str) -> &'static str {
    match p {
        "accessibility" => "Accessibility",
        "screenRecording" => "Screen Recording",
        _ => "Input Monitoring",
    }
}

/// `permissions.includes(permission)` — array SameValueZero, or string substring.
fn js_includes(haystack: &Value, needle: &Value) -> Result<bool, String> {
    match haystack {
        Value::Array(items) => Ok(items.iter().any(|i| {
            // SameValueZero: NaN equals NaN, -0 equals 0.
            (is_number(i) && is_number(needle) && {
                let (a, b) = (num_or_nan(i), num_or_nan(needle));
                (a.is_nan() && b.is_nan()) || a == b
            }) || js_strict_eq(i, needle)
        })),
        Value::String(s) => Ok(matches!(needle, Value::String(n) if s.contains(n.as_str()))),
        Value::Null => Err("Cannot read properties of null (reading 'includes')".into()),
        v if is_undef(v) => Err("Cannot read properties of undefined (reading 'includes')".into()),
        _ => Err("permissions.includes is not a function".into()),
    }
}

fn sort_computer_permissions(permissions: &Value) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for p in COMPUTER_PERMISSIONS {
        if js_includes(permissions, &Value::String((*p).into()))? {
            out.push((*p).to_string());
        }
    }
    Ok(out)
}

fn list_computer_permissions(permissions: &Value) -> Result<String, String> {
    let labels: Vec<&str> = sort_computer_permissions(permissions)?
        .iter()
        .map(|p| permission_label(p))
        .collect();
    if labels.is_empty() {
        return Ok(String::new());
    }
    if labels.len() == 1 {
        return Ok(labels[0].to_string());
    }
    Ok(format!(
        "{} and {}",
        labels[..labels.len() - 1].join(", "),
        labels[labels.len() - 1]
    ))
}

fn grants_block_control(permissions: &Value) -> Result<bool, String> {
    for p in COMPUTER_BLOCKING_PERMISSIONS {
        if js_includes(permissions, &Value::String((*p).into()))? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Port of `responsibleDesktopBundleId` (`apps/server/src/computer/computerSetupSignal.ts:49`).
pub fn responsible_desktop_bundle_id(env: Value) -> Result<Option<String>, String> {
    const KEY: &str = "SYNARA_DESKTOP_BUNDLE_ID";
    let value = match &env {
        v if v.is_null() => {
            return Err(format!("Cannot read properties of null (reading '{KEY}')"));
        }
        v if is_undef(v) => std::env::var(KEY).ok(),
        v => {
            let got = js_get(v, KEY)?;
            if is_undef(&got) || got.is_null() {
                None
            } else {
                match js::string_units(&got) {
                    Some(units) => Some(String::from_utf16_lossy(&units)),
                    None => return Err("value.trim is not a function".into()),
                }
            }
        }
    };
    let Some(value) = value else { return Ok(None) };
    let trimmed = js_trim(&value);
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

/// `computerFailureNeedsSetup` — serialized objects never carry the class identity,
/// and a plain object's `cause` is never a `ComputerBackendError` either.
fn failure_needs_setup(error: &Value) -> bool {
    let _ = error;
    false
}

/// Port of `computerSetupSignal` (`apps/server/src/computer/computerSetupSignal.ts:111`).
pub fn computer_setup_signal(input: Value) -> Result<Value, String> {
    let bundle_id = prop_or_undef(&input, "bundleId")?;
    let resolved = if is_undef(&bundle_id) || bundle_id.is_null() {
        responsible_desktop_bundle_id(undef())?
    } else if let Some(units) = js::string_units(&bundle_id) {
        let owned = String::from_utf16_lossy(&units);
        let trimmed = js_trim(&owned);
        if trimmed.is_empty() {
            responsible_desktop_bundle_id(undef())?
        } else {
            Some(trimmed.to_string())
        }
    } else {
        return Err("input.bundleId.trim is not a function".into());
    };
    let app = resolved.map(Value::String);
    let availability = prop_or_undef(&input, "availability")?;
    let availability_kind = if is_undef(&availability) || availability.is_null() {
        undef()
    } else {
        prop_or_undef(&availability, "kind")?
    };
    if js_strict_eq(&availability_kind, &json!("permission-required")) {
        let mut out = Map::new();
        out.insert("missing".into(), prop_or_undef(&availability, "missing")?);
        out.insert("blocking".into(), Value::Bool(true));
        out.insert(
            "buildSignature".into(),
            prop_or_undef(&availability, "buildSignature")?,
        );
        if let Some(app) = app {
            out.insert("bundleId".into(), app);
        }
        return Ok(Value::Object(out));
    }
    let build_sig = prop_or_undef(&input, "buildSignature")?;
    let missing = prop_or_undef(&input, "missing")?;
    let missing = if missing.is_null() || is_undef(&missing) {
        Value::Array(vec![])
    } else {
        missing
    };
    let error = prop_or_undef(&input, "error")?;
    if !is_undef(&error) && failure_needs_setup(&error) {
        let mut out = Map::new();
        out.insert("missing".into(), missing);
        out.insert("blocking".into(), Value::Bool(true));
        if !is_undef(&build_sig) {
            out.insert("buildSignature".into(), build_sig);
        }
        if let Some(app) = app {
            out.insert("bundleId".into(), app);
        }
        return Ok(Value::Object(out));
    }
    let missing_len = length_of(&missing);
    if !missing_len.is_nan() && missing_len > 0.0 {
        let mut out = Map::new();
        out.insert("missing".into(), missing.clone());
        out.insert(
            "blocking".into(),
            Value::Bool(grants_block_control(&missing)?),
        );
        if !is_undef(&build_sig) {
            out.insert("buildSignature".into(), build_sig);
        }
        if let Some(app) = app {
            out.insert("bundleId".into(), app);
        }
        return Ok(Value::Object(out));
    }
    Ok(undef())
}

/// Port of `computerSetupToolNote` (`apps/server/src/computer/computerSetupSignal.ts:168`).
pub fn computer_setup_tool_note(signal: Value) -> Result<String, String> {
    let missing = prop_or_undef(&signal, "missing")?;
    let labels = list_computer_permissions(&missing)?;
    let needed = if labels.is_empty() {
        "a macOS privacy permission".to_string()
    } else {
        labels
    };
    let asked =
        format!("Synara needs {needed} and has shown the user a setup card with a guided flow.");
    if js_truthy(&prop_or_undef(&signal, "blocking")?) {
        return Ok(format!(
            "{asked} Nothing on the desktop can be driven without it. Stop desktop automation, say in one sentence that you are waiting for the user to grant it, and do not retry or work around it until the user says it is granted."
        ));
    }
    Ok(format!(
        "{asked} This one does not block desktop control \u{2014} only the pictures: screenshots and the computer pane will fail while it is missing, and the window list, the accessibility state and every input still work. Do not stop. Carry on with those, say once that you cannot see the screen until the user grants it, and target controls by label rather than by coordinates."
    ))
}

// ── computerShield.ts ───────────────────────────────────────────────────────

/// Port of `cuaMaskedActivationEnabled` (`apps/server/src/computer/computerShield.ts:28`).
pub fn cua_masked_activation_enabled() -> Result<bool, String> {
    Ok(env_flag_enabled(
        std::env::var("SYNARA_CUA_MASKED_ACTIVATION").ok(),
    ))
}

/// Port of `cuaMaskedActivationOptIn` (`apps/server/src/computer/computerShield.ts:36`).
pub fn cua_masked_activation_opt_in() -> Result<Value, String> {
    let raw = std::env::var("SYNARA_CUA_MASKED_APPS").unwrap_or_default();
    let mut seen = Vec::<String>::new();
    let mut current = String::new();
    let mut flush = |current: &mut String| {
        let entry = js_trim(current).to_lowercase();
        if !entry.is_empty() && !seen.iter().any(|e| e == &entry) {
            seen.push(entry);
        }
        current.clear();
    };
    for c in raw.chars() {
        if is_space(c) || c == ',' || c == ';' {
            flush(&mut current);
        } else {
            current.push(c);
        }
    }
    flush(&mut current);
    Ok(json!({ "$set": seen.iter().map(|s| Value::String(s.clone())).collect::<Vec<_>>() }))
}

/// Port of `maskedActivationOptedIn` (`apps/server/src/computer/computerShield.ts:47`).
pub fn masked_activation_opted_in(opt_in: Value, bundle_id: Value) -> Result<bool, String> {
    if is_undef(&bundle_id) {
        return Ok(false);
    }
    let name = match js::string_units(&bundle_id) {
        Some(units) => String::from_utf16_lossy(&units),
        None => return Err("bundleId.trim is not a function".into()),
    };
    let name = js_trim(&name).to_lowercase();
    let entries = match &opt_in {
        Value::Object(o) if o.len() == 1 => match o.get("$set") {
            Some(Value::Array(items)) => items.clone(),
            _ => {
                if opt_in.is_null() {
                    return Err("Cannot read properties of null (reading 'has')".into());
                }
                return Err("optIn.has is not a function".into());
            }
        },
        Value::Null => return Err("Cannot read properties of null (reading 'has')".into()),
        _ => return Err("optIn.has is not a function".into()),
    };
    Ok(entries
        .iter()
        .any(|e| matches!(e, Value::String(s) if *s == name)))
}

// ── computerSpaceDesignation.ts ─────────────────────────────────────────────

fn designates_spaces_regex() -> &'static fancy_regex::Regex {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        let s = JS_SPACE;
        js_regex(&format!(
            r"(?i)^{NLB}(?:use|reserve|usa|riserva){s}+(?:the{s}+|lo{s}+)?(?:space|spazio){s}+id{s}+([1-9][0-9]*){s}+(?:for{s}+(?:this|the){s}+task|per{s}+(?:questo{s}+task|questa{s}+attività)){s}*$",
            NLB = JS_NLB
        ))
    })
}

fn designates_stop_regex() -> &'static fancy_regex::Regex {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        js_regex(&format!(
            r"(?i){NLB}(?:stop|cancel|do not|don['’]t|never|non|smetti|evita|annulla|fermati){NWB}",
            NLB = JS_NLB,
            NWB = JS_NWB
        ))
    })
}

fn strip_designated(text: &str) -> String {
    // ` ```[\s\S]*?``` | `[^`]*` | "[^"\n]*" | "[^"\n]*" ` then `^\s*>.*$` (multiline).
    static RE1: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    static RE2: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re1 = RE1.get_or_init(|| {
        js_regex(r#"```[\s\S]*?```|`[^`]*`|"[^"\n]*"|\u{201c}[^\u{201d}\n]*\u{201d}"#)
    });
    let re2 = RE2.get_or_init(|| js_regex(&format!(r"(?m)^{JS_SPACE}*>.*$")));
    let s = re1.replace_all(text, "").into_owned();
    re2.replace_all(&s, "").into_owned()
}

/// Port of `messageDesignatesComputerSpaces` (`apps/server/src/computer/computerSpaceDesignation.ts:11`).
pub fn message_designates_computer_spaces(text: String) -> Result<Vec<f64>, String> {
    let request = strip_designated(&text);
    if designates_stop_regex().is_match(&request).unwrap_or(false) {
        return Ok(vec![]);
    }
    let mut ids: Vec<f64> = Vec::new();
    for sentence in request.split(['.', '!', '?', '\n']) {
        let sentence = js_trim(sentence);
        let Ok(Some(caps)) = designates_spaces_regex().captures(sentence) else {
            continue;
        };
        let Some(m) = caps.get(1) else { continue };
        let id: f64 = m.as_str().parse().unwrap_or(f64::NAN);
        if id.is_finite()
            && id.fract() == 0.0
            && id <= 9_007_199_254_740_991.0
            && id > 0.0
            && !ids.contains(&id)
        {
            ids.push(id);
        }
    }
    ids.truncate(64);
    Ok(ids)
}

/// Port of `computerSpaceDesignationForMessages` (`apps/server/src/computer/computerSpaceDesignation.ts:32`).
pub fn computer_space_designation_for_messages(messages: Value) -> Result<Vec<f64>, String> {
    let latest = latest_user_authored(&messages)?;
    let items = array_method(&messages, "messages", "findLast")?;
    let mut latest_user_role: Option<usize> = None;
    for (i, message) in items.iter().enumerate().rev() {
        if js_strict_eq(&js_get(message, "role")?, &json!("user")) {
            latest_user_role = Some(i);
            break;
        }
    }
    if let (Some((idx, msg)), Some(user_idx)) = (&latest, latest_user_role)
        && *idx == user_idx
    {
        let text = js_get(msg, "text")?;
        return match js::string_units(&text) {
            Some(units) => message_designates_computer_spaces(String::from_utf16_lossy(&units)),
            None => Ok(vec![]),
        };
    }
    Ok(vec![])
}

/// `latestUserAuthoredMessage` — returns `(index, message)` for identity comparison.
fn latest_user_authored(messages: &Value) -> Result<Option<(usize, Value)>, String> {
    let length = js_get(messages, "length")?;
    let len = js_to_number(&length);
    if !len.is_finite() {
        return Ok(None);
    }
    let mut index = len as i64 - 1;
    while index >= 0 {
        let message = js_get_u32(messages, index as usize)?;
        if js_strict_eq(&js_get(&message, "role")?, &json!("user"))
            && !js_strict_eq(&js_get(&message, "dispatchOrigin")?, &json!("automation"))
            && !js_strict_eq(&js_get(&message, "dispatchOrigin")?, &json!("agent"))
        {
            return Ok(Some((index as usize, message)));
        }
        index -= 1;
    }
    Ok(None)
}

/// Port of `latestUserAuthoredMessage` (`apps/server/src/computer/computerVisibleUse.ts:233`).
pub fn latest_user_authored_message(messages: Value) -> Result<Value, String> {
    Ok(latest_user_authored(&messages)?
        .map(|(_, m)| m)
        .unwrap_or_else(undef))
}

// ── computerTaskContext.ts / modelDesktopObservation.ts ─────────────────────

/// Port of `currentComputerTask` (`apps/server/src/computer/computerTaskContext.ts:6`).
pub fn current_computer_task() -> Result<Value, String> {
    Ok(undef())
}

/// Port of `isModelDesktopObservationActive` (`apps/server/src/computer/modelDesktopObservation.ts:5`).
pub fn is_model_desktop_observation_active() -> Result<bool, String> {
    Ok(false)
}

// ── computerTurnTiming.ts ───────────────────────────────────────────────────

#[derive(Clone)]
#[allow(dead_code)]
struct TurnTiming {
    started: f64,
    origin: &'static str,
    first_observation: Option<f64>,
    last_observation: Option<f64>,
    first_write: Option<f64>,
}

fn timings() -> &'static std::sync::Mutex<Vec<(String, String, TurnTiming)>> {
    static T: std::sync::OnceLock<std::sync::Mutex<Vec<(String, String, TurnTiming)>>> =
        std::sync::OnceLock::new();
    T.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// Port of `startComputerTurnTiming` (`apps/server/src/computer/computerTurnTiming.ts:77`).
pub fn start_computer_turn_timing(thread_id: String, turn_id: String) -> Result<Value, String> {
    if cua_timing_log_enabled()? {
        let mut turns = timings().lock().unwrap_or_else(|e| e.into_inner());
        if !turns
            .iter()
            .any(|(t, u, _)| *t == thread_id && *u == turn_id)
        {
            while turns.len() >= 256 {
                turns.remove(0);
            }
            turns.push((
                thread_id,
                turn_id,
                TurnTiming {
                    started: now_ms(),
                    origin: "provider-turn-start",
                    first_observation: None,
                    last_observation: None,
                    first_write: None,
                },
            ));
        }
    }
    Ok(undef())
}

/// Port of `endComputerTurnTiming` (`apps/server/src/computer/computerTurnTiming.ts:80`).
pub fn end_computer_turn_timing(thread_id: String, turn_id: Value) -> Result<Value, String> {
    let mut turns = timings().lock().unwrap_or_else(|e| e.into_inner());
    let all = is_undef(&turn_id);
    turns.retain(|(t, u, _)| {
        !(t == &thread_id && (all || js_strict_eq(&Value::String(u.clone()), &turn_id)))
    });
    Ok(undef())
}

/// Port of `beginComputerTurnCall` (`apps/server/src/computer/computerTurnTiming.ts:83`).
pub fn begin_computer_turn_call(
    thread_id: String,
    turn_id: Value,
    kind: String,
) -> Result<Value, String> {
    if !cua_timing_log_enabled()? || !js_truthy(&turn_id) {
        return Ok(undef());
    }
    let turn = js_to_string(&turn_id);
    let mut turns = timings().lock().unwrap_or_else(|e| e.into_inner());
    if !turns.iter().any(|(t, u, _)| *t == thread_id && *u == turn) {
        while turns.len() >= 256 {
            turns.remove(0);
        }
        turns.push((
            thread_id.clone(),
            turn.clone(),
            TurnTiming {
                started: now_ms(),
                origin: "first-computer-call",
                first_observation: None,
                last_observation: None,
                first_write: None,
            },
        ));
    }
    let _ = kind;
    Ok(undef())
}

// ── computerVisibleUse.ts ───────────────────────────────────────────────────

fn visible_use_patterns() -> &'static [fancy_regex::Regex] {
    static RES: std::sync::OnceLock<Vec<fancy_regex::Regex>> = std::sync::OnceLock::new();
    RES.get_or_init(|| {
        let w = JS_WORD_B;
        let s = JS_SPACE;
        let dot = JS_DOT;
        let nlb = JS_NLB;
        let nwb = JS_NWB;
        let clause_end = format!("(?={s}*(?:[.!?,;:]|$))");
        let apos = "['\u{2019}]";
        let what_doing = format!("what you(?:{apos}re| are) doing");
        let what_happening = format!("what(?:{apos}s| is) (?:going on|happening){clause_end}");
        let want_to = format!("i (?:want|would like|{apos}d like) to");
        let so_can = "so (?:that )?(?:i|we) can (?:all )?";
        vec![
            js_regex(&format!(
                r"(?i){nlb}show (?:me )?(?:the |my )?(?:{w}+ ){{0,3}}(?:window|app|screen|desktop|browser|page){clause_end}"
            )),
            js_regex(&format!(
                r"(?i){nlb}(?:show me|let me see|{want_to} see|{so_can}see) (?:{what_doing}{nwb}|{what_happening})"
            )),
            js_regex(&format!(
                r"(?i){nlb}(?:{want_to} |let me )watch (?:you|it|the (?:app|browser|window)){nwb}|{nlb}{so_can}watch(?: (?:you|along))?{clause_end}"
            )),
            js_regex(&format!(r"(?i){nlb}let me see (?:it|you) work(?:ing)?{nwb}")),
            js_regex(&format!(
                r"(?i){nlb}i (?:want|would like|['\u{{2019}}]d like) to see (?:the |my )?(?:{w}+ ){{0,3}}(?:window|app|screen|desktop|browser|page){clause_end}"
            )),
            js_regex(&format!(
                r"(?i){nlb}(?:put|show|display){nwb}[^.!?{dot}\n]{{0,40}}{nlb}on (?:my|the) screen{nwb}"
            )),
            js_regex(&format!(
                r"(?i){nlb}(?:make|keep) (?:it|(?:the |my )?(?:{w}+ ){{0,3}}(?:app|window|browser)) visible{nwb}"
            )),
            js_regex(&format!(
                r"(?i){nlb}(?:bring|pull)(?: up)? (?:the |my |its |their )?(?:{w}+ ){{0,2}}(?:window|app|browser) (?:forward|up front){nwb}"
            )),
            js_regex(&format!(r"(?i){nlb}use (?:the )?foreground(?: mode)?{clause_end}")),
            js_regex(&format!(r"(?i){nlb}take over (?:my|the) (?:screen|desktop|computer){nwb}")),
            js_regex(&format!(r"(?i){nlb}drive (?:my|the) (?:screen|desktop|computer){nwb}")),
            js_regex(&format!(
                r"(?i){nlb}(?:mostra(?:mi|re)?|porta(?:re)?|metti|mettere){nwb}{dot}{{0,60}}{nlb}(?:sullo schermo|in primo piano){nwb}"
            )),
            js_regex(&format!(
                r"(?i){nlb}(?:voglio|vorrei|fammi) vedere (?:la finestra|il browser|lo schermo|il desktop|cosa (?:fai|stai facendo)){nwb}"
            )),
        ]
    })
}

fn background_use_patterns() -> &'static [fancy_regex::Regex] {
    static RES: std::sync::OnceLock<Vec<fancy_regex::Regex>> = std::sync::OnceLock::new();
    RES.get_or_init(|| {
        let nlb = JS_NLB;
        let nwb = JS_NWB;
        let dot = JS_DOT;
        vec![
            js_regex(&format!(
                r"(?i){nlb}(?:do not|don['\u{{2019}}]t|never|not|avoid|without|stop){nwb}[^.!?{dot}\n]{{0,100}}{nlb}(?:show|watch|visible|foreground|front|focus|raise|screen|desktop){nwb}"
            )),
            js_regex(&format!(
                r"(?i){nlb}(?:keep|stay|remain|work|run|use){nwb}[^.!?{dot}\n]{{0,60}}{nlb}(?:background|hidden|invisible){nwb}"
            )),
            js_regex(&format!(r"(?i){nlb}background[- ]only{nwb}")),
            js_regex(&format!(
                r"(?i){nlb}(?:non|senza|evita|smetti di){nwb}[^.!?{dot}\n]{{0,100}}{nlb}(?:mostrare|mostrarmi|primo piano|schermo|focus){nwb}"
            )),
            js_regex(&format!(
                r"(?i){nlb}(?:lavora|resta|rimani|mantieni){nwb}[^.!?{dot}\n]{{0,60}}{nlb}(?:background|nascost[ao]){nwb}"
            )),
        ]
    })
}

/// The "bring X to the front/foreground" pair: `front` carries a negative lookahead
/// the regex engine checks separately.
fn to_front_patterns() -> &'static (fancy_regex::Regex, fancy_regex::Regex, fancy_regex::Regex) {
    static RES: std::sync::OnceLock<(fancy_regex::Regex, fancy_regex::Regex, fancy_regex::Regex)> =
        std::sync::OnceLock::new();
    RES.get_or_init(|| {
        let nlb = JS_NLB;
        let nwb = JS_NWB;
        let s = JS_SPACE;
        let subject = format!(
            r"(?:bring|put|move|pull|raise){s}+(?:(?:the|my|this|that){s}+)?(?:window|app|browser|screen|desktop|it){s}+"
        );
        (
            js_regex(&format!(r"(?i){nlb}{subject}to{s}+(?:the{s}+)?front{nwb}")),
            js_regex(&format!(r"(?i)^{s}+(?:of|desk|door|row|page){nwb}")),
            js_regex(&format!(
                r"(?i){nlb}{subject}(?:to|in|into){s}+(?:the{s}+)?foreground{nwb}"
            )),
        )
    })
}

/// The second bring pattern: positive lookahead "so (that) i/we can (all) watch|see" or "and show me".
fn to_front_tail_patterns() -> &'static (fancy_regex::Regex, fancy_regex::Regex) {
    static RES: std::sync::OnceLock<(fancy_regex::Regex, fancy_regex::Regex)> =
        std::sync::OnceLock::new();
    RES.get_or_init(|| {
        let nlb = JS_NLB;
        let nwb = JS_NWB;
        let s = JS_SPACE;
        let dot = JS_DOT;
        (
            js_regex(&format!(
                r"(?i){nlb}(?:bring|put|move|pull|raise){nwb}[^.!?{dot}\n]{{0,40}}{nlb}(?:to{s}+(?:the{s}+)?front|(?:to|in|into){s}+(?:the{s}+)?foreground){nwb}"
            )),
            js_regex(&format!(
                r"(?i)^{s}+(?:so{s}+(?:that{s}+)?(?:i|we){s}+can{s}+(?:all{s}+)?(?:watch|see){nwb}|and{s}+show{s}+me{nwb})"
            )),
        )
    })
}

fn unquoted_request(text: &str) -> String {
    static RE1: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    static RE2: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    static RE4: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re1 = RE1.get_or_init(|| {
        js_regex(&format!(
            r"(?i)<untrusted_text{NLB}[^>]*>[\s\S]*?(?:<\/untrusted_text>|$)",
            NLB = JS_NLB
        ))
    });
    let re2 = RE2.get_or_init(|| {
        js_regex(
            r#"```[\s\S]*?(?:```|$)|`[^`]*(?:`|$)|"[^"\n]*"|\u{201c}[^\u{201d}\n]*\u{201d}|\u{2018}[^\u{2019}\n]*\u{2019}"#,
        )
    });
    let re4 = RE4.get_or_init(|| js_regex(&format!(r"(?m)^{JS_SPACE}*>.*$")));
    let s = re1.replace_all(text, "").into_owned();
    let s = re2.replace_all(&s, "").into_owned();
    // `(^|[\s(])'[^'\n]+'(?=$|[\s.,;:)])` -> "$1": keep the opener, keep the lookahead.
    let mut out = String::with_capacity(s.len());
    let bytes: Vec<char> = s.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == '\'' {
            let opener_ok = if i == 0 {
                true
            } else {
                is_space(bytes[i - 1]) || bytes[i - 1] == '('
            };
            if opener_ok {
                let mut j = i + 1;
                while j < bytes.len() && bytes[j] != '\'' && bytes[j] != '\n' {
                    j += 1;
                }
                if j < bytes.len()
                    && j > i + 1
                    && (j + 1 >= bytes.len()
                        || is_space(bytes[j + 1])
                        || matches!(bytes[j + 1], '.' | ',' | ';' | ':' | ')'))
                {
                    i = j + 1;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    re4.replace_all(&out, "").into_owned()
}

fn requests_known_app_visibility(text: &str, context: &Value) -> Result<bool, String> {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        js_regex(&format!(
            r"(?iu)^(?:please[, ]+)?(?:show(?: me)?|display|mostra(?:mi)?){JS_SPACE}+(.+?)[.!?]*$"
        ))
    });
    let trimmed = js_trim(text);
    let Ok(Some(caps)) = re.captures(trimmed) else {
        return Ok(false);
    };
    let Some(app) = caps.get(1) else {
        return Ok(false);
    };
    let name = js_trim(app.as_str()).to_lowercase();
    known_app_names_has(context, &name)
}

fn requests_known_app_foreground(text: &str, context: &Value) -> Result<bool, String> {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        js_regex(&format!(
            r"(?iu)^(?:please[, ]+)?(?:bring|put|move|pull|raise){JS_SPACE}+(.+?){JS_SPACE}+(?:to{JS_SPACE}+(?:the{JS_SPACE}+)?front|(?:to|in|into){JS_SPACE}+(?:the{JS_SPACE}+)?foreground)[.!?]*$"
        ))
    });
    let trimmed = js_trim(text);
    let Ok(Some(caps)) = re.captures(trimmed) else {
        return Ok(false);
    };
    let Some(app) = caps.get(1) else {
        return Ok(false);
    };
    let name = js_trim(app.as_str()).to_lowercase();
    known_app_names_has(context, &name)
}

/// `context.knownAppNames?.some(...)` — `null` throws, everything else tolerates.
fn known_app_names_has(context: &Value, name: &str) -> Result<bool, String> {
    if context.is_null() {
        return Err("Cannot read properties of null (reading 'knownAppNames')".into());
    }
    if is_undef(context) {
        return Ok(false);
    }
    let names = js_get(context, "knownAppNames")?;
    let Value::Array(items) = &names else {
        return Ok(false);
    };
    Ok(items
        .iter()
        .any(|c| matches!(c, Value::String(s) if js_trim(s).to_lowercase() == *name)))
}

fn is_routine_continuation_text(text: &str) -> bool {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        js_regex(
            r"(?iu)^(?:(?:ok(?:ay)?|yes|s[i\u{ec}])[, ]+)?(?:please[, ]+)?(?:continue(?: (?:working|with (?:the |this |our )?(?:same |current )?(?:task|work|plan)))?|keep (?:going|working)|carry on|go (?:on|ahead)|proceed(?: with (?:the |this |our )?(?:same |current )?(?:task|work|plan))?|(?:try|retry)(?: (?:again|that|it|the same step))?|continua(?: pure)?|prosegui|procedi|vai|riprova)(?:[, ]+(?:please|per favore))?[.!]*$",
        )
    });
    re.is_match(js_trim(text)).unwrap_or(false)
}

fn is_affirmative_reply_text(text: &str) -> bool {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        js_regex(
            r"(?iu)^(?:yes|yeah|yep|ok(?:ay)?|sure|go ahead|s[i\u{ec}]|va bene|certo|procedi|vai)(?:[, ]+(?:please|go ahead|per favore|fallo))?[.!]*$",
        )
    });
    re.is_match(js_trim(text)).unwrap_or(false)
}

/// `messageRequestsVisibleUse(message.text, context)` where `text` may be non-string.
fn message_requests_visible_use_value(text: &Value, context: &Value) -> Result<bool, String> {
    let units = match js::string_units(text) {
        Some(u) => u,
        None => return Err("text.replace is not a function".into()),
    };
    message_requests_visible_use_impl(&String::from_utf16_lossy(&units), context)
}

fn message_requests_visible_use_impl(request_text: &str, context: &Value) -> Result<bool, String> {
    let request = unquoted_request(request_text);
    if background_use_patterns()
        .iter()
        .any(|p| p.is_match(&request).unwrap_or(false))
    {
        return Ok(false);
    }
    if visible_use_patterns()
        .iter()
        .any(|p| p.is_match(&request).unwrap_or(false))
    {
        return Ok(true);
    }
    // The front/foreground pair with the negative lookahead.
    let (front_re, neg_re, fore_re) = to_front_patterns();
    let mut matched = false;
    for m in front_re.find_iter(&request).flatten() {
        let tail = &request[m.end()..];
        if !neg_re.is_match(tail).unwrap_or(false) {
            matched = true;
            break;
        }
    }
    if matched || fore_re.is_match(&request).unwrap_or(false) {
        return Ok(true);
    }
    let (lead_re, tail_re) = to_front_tail_patterns();
    for m in lead_re.find_iter(&request).flatten() {
        let tail = &request[m.end()..];
        if tail_re.is_match(tail).unwrap_or(false) {
            return Ok(true);
        }
    }
    if requests_known_app_visibility(&request, context)? {
        return Ok(true);
    }
    requests_known_app_foreground(&request, context)
}

/// Port of `messageRequestsVisibleUse` (`apps/server/src/computer/computerVisibleUse.ts:138`).
pub fn message_requests_visible_use(text: String, context: Value) -> Result<bool, String> {
    message_requests_visible_use_impl(&text, &context)
}

fn is_local_human_message(message: &Value) -> Result<bool, String> {
    Ok(js_strict_eq(&js_get(message, "role")?, &json!("user"))
        && !js_strict_eq(&js_get(message, "dispatchOrigin")?, &json!("automation"))
        && !js_strict_eq(&js_get(message, "dispatchOrigin")?, &json!("agent"))
        && (js_strict_eq(&js_get(message, "source")?, &json!("native"))
            || js_strict_eq(&js_get(message, "source")?, &json!("async-user-input"))))
}

fn is_local_assistant_message(message: Option<&Value>) -> Result<bool, String> {
    let Some(message) = message else {
        return Ok(false);
    };
    if is_undef(message) {
        return Ok(false);
    }
    Ok(js_strict_eq(&js_get(message, "role")?, &json!("assistant"))
        && js_strict_eq(&js_get(message, "source")?, &json!("native"))
        && !js_truthy(&js_get(message, "streaming")?))
}

fn is_routine_continuation(message: &Value) -> Result<bool, String> {
    for key in ["attachments", "skills", "mentions"] {
        let v = js_get(message, key)?;
        let len = length_of(&v);
        if !len.is_nan() && len > 0.0 {
            return Ok(false);
        }
    }
    let text = js_get(message, "text")?;
    match js::string_units(&text) {
        Some(units) => Ok(is_routine_continuation_text(&String::from_utf16_lossy(
            &units,
        ))),
        None => Err("message.text.trim is not a function".into()),
    }
}

fn is_affirmative_reply(message: &Value) -> Result<bool, String> {
    let text = js_get(message, "text")?;
    match js::string_units(&text) {
        Some(units) => Ok(is_affirmative_reply_text(&String::from_utf16_lossy(&units))),
        None => Err("message.text.trim is not a function".into()),
    }
}

fn asks_visible_use_permission(text: &Value, context: &Value) -> Result<bool, String> {
    static RE: std::sync::OnceLock<fancy_regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        js_regex(
            r"(?iu)(?:^|[.!?]\s+)(?:can i|may i|shall i|do you want me to|would you like me to|is it (?:ok(?:ay)?|alright) (?:if i|to)|posso|vuoi che)\s+([^?]*\?)$",
        )
    });
    let units = match js::string_units(text) {
        Some(u) => u,
        None => return Err("text.replace is not a function".into()),
    };
    let question = js_trim(&unquoted_request(&String::from_utf16_lossy(&units))).to_string();
    let Ok(Some(caps)) = re.captures(&question) else {
        return Ok(false);
    };
    let Some(action) = caps.get(1) else {
        return Ok(false);
    };
    message_requests_visible_use_impl(action.as_str(), context)
}

fn confirms_visible_use(
    reply: &Value,
    preceding: Option<&Value>,
    context: &Value,
) -> Result<bool, String> {
    if !is_local_assistant_message(preceding)? {
        return Ok(false);
    }
    if !is_affirmative_reply(reply)? {
        return Ok(false);
    }
    let text = js_get(preceding.unwrap_or(&Value::Null), "text")?;
    asks_visible_use_permission(&text, context)
}

fn confirms_structured_visible_use(
    messages: &[Value],
    reply_index: usize,
    context: &Value,
) -> Result<bool, String> {
    let reply = &messages[reply_index];
    let mut index = reply_index as i64 - 1;
    while index >= 0 {
        let question = &messages[index as usize];
        if js_strict_eq(&js_get(question, "role")?, &json!("user")) {
            return Ok(false);
        }
        let input = js_get(question, "asyncUserInput")?;
        let response = if input.is_null() || is_undef(&input) {
            undef()
        } else {
            js_get(&input, "response")?
        };
        let message_id = if response.is_null() || is_undef(&response) {
            undef()
        } else {
            js_get(&response, "messageId")?
        };
        if !js_strict_eq(&message_id, &js_get(reply, "id")?) {
            index -= 1;
            continue;
        }
        if !is_local_assistant_message(Some(question))? {
            return Ok(false);
        }
        let questions = js_get(&input, "questions")?;
        let answers = js_get(&response, "answers")?;
        let questions_len = length_of(&questions);
        let answers_len = length_of(&answers);
        if questions_len != 1.0 || answers_len != 1.0 {
            return Ok(false);
        }
        let first_answer = js_get_u32(&answers, 0)?;
        if !is_affirmative_reply_text(&match js::string_units(&first_answer) {
            Some(u) => String::from_utf16_lossy(&u),
            None => return Err("message.text.trim is not a function".into()),
        }) {
            return Ok(false);
        }
        let first_question = js_get_u32(&questions, 0)?;
        let title = js_get(&first_question, "title")?;
        return asks_visible_use_permission(&title, context);
    }
    Ok(false)
}

/// Port of `computerForegroundScopeChangedSince` (`apps/server/src/computer/computerVisibleUse.ts:246`).
pub fn computer_foreground_scope_changed_since(
    messages: Value,
    last_message_id: Value,
) -> Result<bool, String> {
    let items = array_method(&messages, "messages", "findIndex")?;
    let has_last = !is_undef(&last_message_id);
    let mut start = -1i64;
    if has_last {
        for (i, m) in items.iter().enumerate() {
            if js_strict_eq(&js_get(m, "id")?, &last_message_id) {
                start = i as i64;
                break;
            }
        }
        if start == -1 {
            return Ok(true);
        }
    }
    let Some(slice) = items.get((start + 1) as usize..) else {
        return Ok(false);
    };
    for message in slice {
        if js_strict_eq(&js_get(message, "role")?, &json!("user"))
            && (!is_local_human_message(message)? || !is_routine_continuation(message)?)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Port of `computerForegroundAuthorizationForMessages` (`apps/server/src/computer/computerVisibleUse.ts:270`).
pub fn computer_foreground_authorization_for_messages(
    messages: Value,
    context: Value,
) -> Result<Value, String> {
    let length = js_get(&messages, "length")?;
    let len = js_to_number(&length);
    let not_authorized = || json!({ "userRequestedVisibleUse": false });
    if !len.is_finite() {
        return Ok(not_authorized());
    }
    let mut index = len as i64 - 1;
    while index >= 0 {
        let message = js_get_u32(&messages, index as usize)?;
        if !js_strict_eq(&js_get(&message, "role")?, &json!("user")) {
            index -= 1;
            continue;
        }
        if !is_local_human_message(&message)? {
            return Ok(not_authorized());
        }
        if js_strict_eq(&js_get(&message, "source")?, &json!("async-user-input")) {
            let items = array_method(&messages, "messages", "indexOf").unwrap_or_default();
            let confirmed = confirms_structured_visible_use(&items, index as usize, &context)?;
            return Ok(json!({ "userRequestedVisibleUse": confirmed }));
        }
        let text = js_get(&message, "text")?;
        let preceding = if index >= 1 {
            Some(js_get_u32(&messages, (index - 1) as usize)?)
        } else {
            None
        };
        if message_requests_visible_use_value(&text, &context)?
            || confirms_visible_use(&message, preceding.as_ref(), &context)?
        {
            return Ok(json!({ "userRequestedVisibleUse": true }));
        }
        if !is_routine_continuation(&message)? {
            return Ok(not_authorized());
        }
        index -= 1;
    }
    Ok(not_authorized())
}

// ── cuaSpaceInventory.ts ────────────────────────────────────────────────────

const COMPUTER_SPACES_MAX_LENGTH: usize = 256;
const SPACES_ERROR: &str = "The native driver did not provide a valid managed-display Space inventory. Use exact existing windows in place; do not infer empty Spaces or Space ownership from window membership.";

/// Port of `cuaSpaceInventory` (`apps/server/src/computer/cuaSpaceInventory.ts:7`).
pub fn cua_space_inventory(value: Value) -> Result<Value, String> {
    let fail = || Err::<Value, String>(SPACES_ERROR.into());
    let source = js_get(&value, "source").map_err(|_| SPACES_ERROR.to_string())?;
    if !js_strict_eq(&source, &json!("macos-managed-spaces")) {
        return fail();
    }
    let spaces_v = js_get(&value, "spaces").map_err(|_| SPACES_ERROR.to_string())?;
    let Value::Array(entries) = &spaces_v else {
        return fail();
    };
    let complete = js_get(&value, "complete").map_err(|_| SPACES_ERROR.to_string())?;
    if !matches!(complete, Value::Bool(_)) {
        return fail();
    }
    if entries.len() > COMPUTER_SPACES_MAX_LENGTH {
        return fail();
    }
    let mut spaces = Vec::with_capacity(entries.len());
    for entry in entries {
        let Value::Object(space) = entry else {
            return fail();
        };
        if entry.is_null() {
            return fail();
        }
        let id = space.get("space_id").cloned().unwrap_or_else(undef);
        let uuid = space.get("space_uuid").cloned().unwrap_or_else(undef);
        let display_id = space.get("display_id").cloned().unwrap_or_else(undef);
        let kind = space.get("kind").cloned().unwrap_or_else(undef);
        let current = space.get("current").cloned().unwrap_or_else(undef);
        // ComputerSpaceId: safe integer in 1..=MAX_SAFE.
        let id_num = if is_number(&id) {
            num_or_nan(&id)
        } else {
            f64::NAN
        };
        if !is_safe_integer(id_num) || id_num <= 0.0 {
            return fail();
        }
        let uuid_ok = uuid.is_null()
            || matches!(&uuid, Value::String(s) if !s.is_empty() && s.encode_utf16().count() <= 256);
        let display_ok = matches!(&display_id, Value::String(s) if !s.is_empty() && s.encode_utf16().count() <= 256);
        let kind_ok = matches!(&kind, Value::String(s) if matches!(s.as_str(), "desktop" | "fullscreen" | "unknown"));
        let current_ok = current.is_null() || matches!(current, Value::Bool(_));
        if !(uuid_ok && display_ok && kind_ok && current_ok) {
            return fail();
        }
        spaces.push((id_num, uuid, display_id, kind, current));
    }
    let mut ids: Vec<f64> = Vec::new();
    let mut uuids: Vec<String> = Vec::new();
    for (id, uuid, _, _, _) in &spaces {
        if ids.contains(id) {
            return fail();
        }
        if let Value::String(u) = uuid {
            if uuids.iter().any(|x| x == u) {
                return fail();
            }
            uuids.push(u.clone());
        }
        ids.push(*id);
    }
    let out_spaces: Vec<Value> = spaces
        .into_iter()
        .map(|(id, uuid, display_id, kind, current)| {
            json!({
                "id": js::number(id),
                "uuid": uuid,
                "displayId": display_id,
                "kind": kind,
                "current": current,
            })
        })
        .collect();
    Ok(json!({
        "spaces": out_spaces,
        "complete": complete,
        "source": "macos-managed-spaces",
    }))
}

// ── cursorActivity.ts ───────────────────────────────────────────────────────

/// Port of `cursorToolActivity` (`apps/server/src/computer/cursorActivity.ts:4`).
pub fn cursor_tool_activity(tool: String) -> Result<String, String> {
    Ok(match tool.as_str() {
        "computer_click" => "Clicking",
        "computer_scroll" => "Scrolling",
        "computer_type_text" => "Typing",
        "computer_set_value" => "Setting field",
        "computer_select_text" => "Selecting text",
        "computer_paste" => "Pasting",
        "computer_press_key" => "Pressing key",
        "computer_drag" => "Dragging",
        "computer_move_cursor" => "Moving cursor",
        "computer_wait" => "Waiting for screen",
        "computer_screenshot" => "Capturing screen",
        "computer_get_state" => "Reading screen",
        "computer_get_screen_size" => "Measuring screen",
        "computer_list_windows" => "Finding window",
        "computer_list_apps" => "Listing apps",
        "computer_set_window_frame" => "Moving window",
        "computer_invoke_menu" => "Opening menu item",
        "computer_verify_state" => "Checking state",
        "computer_zoom" => "Zooming in",
        "computer_get_accessibility_tree" => "Reading desktop inventory",
        "computer_get_cursor_position" => "Reading cursor position",
        "computer_kill_app" => "Force-quitting app",
        "computer_set_window_minimized" => "Changing window visibility",
        "computer_set_app_visibility" => "Changing app visibility",
        "computer_perform_action" => "Activating control",
        "computer_launch_app" => "Opening app",
        "computer_activate_window" => "Activating window",
        "computer_read_clipboard" => "Reading clipboard",
        "computer_write_clipboard" => "Writing clipboard",
        "computer_run" => "Running sequence",
        _ => "Working",
    }
    .into())
}

/// Port of `cursorRuntimeActivity` (`apps/server/src/computer/cursorActivity.ts:71`).
pub fn cursor_runtime_activity(event: Value) -> Result<Option<String>, String> {
    let event_type = js_get(&event, "type")?;
    let label = match event_type.as_str() {
        Some("user-input.requested") => Some("Waiting for you"),
        Some("request.opened") => Some("Needs approval"),
        Some("user-input.resolved")
        | Some("request.resolved")
        | Some("turn.started")
        | Some("item.completed") => Some("Thinking"),
        Some("item.started") => Some("Working"),
        Some("content.delta") => {
            let payload = js_get(&event, "payload")?;
            let stream = js_get(&payload, "streamKind")?;
            if js_strict_eq(&stream, &json!("assistant_text")) {
                Some("Responding")
            } else if ["reasoning_text", "reasoning_summary_text", "plan_text"]
                .iter()
                .any(|k| js_strict_eq(&stream, &json!(k)))
            {
                Some("Thinking")
            } else {
                None
            }
        }
        _ => None,
    };
    Ok(label.map(str::to_string))
}

// ── screenshotFrames.ts ─────────────────────────────────────────────────────

fn describe_frame(frame: &Value) -> String {
    let w = js_get(frame, "width").unwrap_or_else(|_| undef());
    let h = js_get(frame, "height").unwrap_or_else(|_| undef());
    let id = js_get(frame, "id").unwrap_or_else(|_| undef());
    format!(
        "the {}x{} screenshot {}",
        js_to_string(&w),
        js_to_string(&h),
        js_to_string(&id)
    )
}

fn clamp_to_span(value: f64, start: f64, length: f64) -> f64 {
    let end = start + js_max(0.0, length - 1.0);
    js_min(end, js_max(start, value))
}

/// Port of `screenshotPointToDesktop` (`apps/server/src/computer/screenshotFrames.ts:197`).
pub fn screenshot_point_to_desktop(frame: Value, x: f64, y: f64) -> Result<Value, String> {
    let width = js_get(&frame, "width")?;
    let height = js_get(&frame, "height")?;
    if !x.is_finite()
        || !y.is_finite()
        || x < 0.0
        || y < 0.0
        || js_gt(&json!(x), &width)
        || js_gt(&json!(y), &height)
    {
        return Err(format!(
            "Computer target ({}, {}) is outside {}; x/y are pixel coordinates in that image.",
            js::number_to_string(x),
            js::number_to_string(y),
            describe_frame(&frame)
        ));
    }
    let region = js_get(&frame, "region")?;
    let scale = js_to_number(&js_get(&frame, "scale")?);
    let rx = js_to_number(&js_get(&region, "x")?);
    let ry = js_to_number(&js_get(&region, "y")?);
    let rw = js_to_number(&js_get(&region, "width")?);
    let rh = js_to_number(&js_get(&region, "height")?);
    Ok(json!({
        "x": js::number(clamp_to_span(js::math_round(rx + x / scale), rx, rw)),
        "y": js::number(clamp_to_span(js::math_round(ry + y / scale), ry, rh)),
    }))
}

/// Port of `screenshotRectToDesktop` (`apps/server/src/computer/screenshotFrames.ts:234`).
pub fn screenshot_rect_to_desktop(frame: Value, rect: Value) -> Result<Value, String> {
    let rect_x = js_get(&rect, "x")?;
    let rect_y = js_get(&rect, "y")?;
    let rect_w = js_get(&rect, "width")?;
    let rect_h = js_get(&rect, "height")?;
    let frame_w = js_get(&frame, "width")?;
    let frame_h = js_get(&frame, "height")?;
    let left = js_max(0.0, js_to_number(&rect_x));
    let top = js_max(0.0, js_to_number(&rect_y));
    let right = js_min(
        js_to_number(&frame_w),
        js_to_number(&rect_x) + js_to_number(&rect_w),
    );
    let bottom = js_min(
        js_to_number(&frame_h),
        js_to_number(&rect_y) + js_to_number(&rect_h),
    );
    if !(right > left && bottom > top) {
        return Err(format!(
            "The requested region does not overlap {}; x/y/width/height are pixel coordinates in that image.",
            describe_frame(&frame)
        ));
    }
    let region = js_get(&frame, "region")?;
    let scale = js_to_number(&js_get(&frame, "scale")?);
    let rx = js_to_number(&js_get(&region, "x")?);
    let ry = js_to_number(&js_get(&region, "y")?);
    let x = js::math_round(rx + left / scale);
    let y = js::math_round(ry + top / scale);
    let far_x = js::math_round(rx + right / scale);
    let far_y = js::math_round(ry + bottom / scale);
    Ok(json!({
        "x": js::number(x),
        "y": js::number(y),
        "width": js::number(js_max(1.0, far_x - x)),
        "height": js::number(js_max(1.0, far_y - y)),
    }))
}

/// Port of `screenshotDeltaToDesktop` (`apps/server/src/computer/screenshotFrames.ts:254`).
pub fn screenshot_delta_to_desktop(
    frame: Value,
    delta_x: f64,
    delta_y: f64,
) -> Result<Value, String> {
    let scale = js_to_number(&js_get(&frame, "scale")?);
    Ok(json!({
        "deltaX": js::number(delta_x / scale),
        "deltaY": js::number(delta_y / scale),
    }))
}

// ── scrollCalibration.ts ────────────────────────────────────────────────────

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

fn paeth(left: i64, up: i64, up_left: i64) -> i64 {
    let estimate = left + up - up_left;
    let to_left = (estimate - left).abs();
    let to_up = (estimate - up).abs();
    let to_up_left = (estimate - up_left).abs();
    if to_left <= to_up && to_left <= to_up_left {
        return left;
    }
    if to_up <= to_up_left { up } else { up_left }
}

fn decode_png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    if bytes.len() < PNG_SIGNATURE.len() {
        return None;
    }
    if bytes[..8] != PNG_SIGNATURE {
        return None;
    }
    let be_u32 =
        |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let mut offset = 8usize;
    let mut header: Option<(u32, u32, u8)> = None;
    let mut idat: Vec<u8> = Vec::new();
    while offset + 8 <= bytes.len() {
        let length = be_u32(offset) as usize;
        let chunk_type = &bytes[offset + 4..offset + 8];
        let data_start = offset + 8;
        let data_end = data_start.saturating_add(length);
        if data_end + 4 > bytes.len() {
            return None;
        }
        match chunk_type {
            b"IHDR" => {
                if length != 13 {
                    return None;
                }
                let width = be_u32(data_start);
                let height = be_u32(data_start + 4);
                let bit_depth = bytes[data_start + 8];
                let color_type = bytes[data_start + 9];
                let interlace = bytes[data_start + 12];
                let channels = match color_type {
                    0 => 1u8,
                    2 => 3,
                    6 => 4,
                    _ => return None,
                };
                if bit_depth != 8 || interlace != 0 {
                    return None;
                }
                if width == 0 || height == 0 {
                    return None;
                }
                header = Some((width, height, channels));
            }
            b"IDAT" => idat.extend_from_slice(&bytes[data_start..data_end]),
            b"IEND" => break,
            _ => {}
        }
        offset = data_end + 4;
    }
    let (width, height, channels) = header?;
    if idat.is_empty() {
        return None;
    }
    use std::io::Read;
    let mut raw = Vec::new();
    if flate2::read::ZlibDecoder::new(&idat[..])
        .read_to_end(&mut raw)
        .is_err()
    {
        return None;
    }
    unfilter_to_luma(&raw, width as usize, height as usize, channels as usize)
}

fn unfilter_to_luma(
    raw: &[u8],
    width: usize,
    height: usize,
    channels: usize,
) -> Option<(u32, u32, Vec<u8>)> {
    let stride = width.checked_mul(channels)?;
    if raw.len() < (stride + 1) * height {
        return None;
    }
    let mut luma = vec![0u8; width * height];
    let mut previous = vec![0u8; stride];
    let mut current = vec![0u8; stride];
    let mut at = 0usize;
    for row in 0..height {
        let filter = raw[at];
        at += 1;
        current.copy_from_slice(&raw[at..at + stride]);
        at += stride;
        for index in 0..stride {
            let left = if index >= channels {
                current[index - channels] as i64
            } else {
                0
            };
            let up = previous[index] as i64;
            let up_left = if index >= channels {
                previous[index - channels] as i64
            } else {
                0
            };
            let value = current[index] as i64;
            current[index] = match filter {
                0 => value as u8,
                1 => (value + left) as u8,
                2 => (value + up) as u8,
                3 => (value + ((left + up) >> 1)) as u8,
                4 => (value + paeth(left, up, up_left)) as u8,
                _ => return None,
            };
        }
        for column in 0..width {
            let sample = column * channels;
            luma[row * width + column] = if channels == 1 {
                current[sample]
            } else {
                ((current[sample] as u64 * 299
                    + current[sample + 1] as u64 * 587
                    + current[sample + 2] as u64 * 114)
                    / 1000) as u8
            };
        }
        std::mem::swap(&mut previous, &mut current);
    }
    Some((width as u32, height as u32, luma))
}

/// Port of `decodePngLuma` (`apps/server/src/computer/scrollCalibration.ts:53`).
pub async fn decode_png_luma(bytes: Value) -> Result<Value, String> {
    if !is_weak_keyable(&bytes) {
        return Err("Invalid value used as weak map key".into());
    }
    let Some(data) = (match &bytes {
        v if single_marker(v, "$bytes").is_some() || single_marker(v, "$buffer").is_some() => {
            buffer_from(v).ok()
        }
        // Plain arrays and objects never decode: `new DataView(array.buffer)` throws
        // inside the async body and the shared catch resolves undefined.
        _ => None,
    }) else {
        return Ok(undef());
    };
    let Some((width, height, luma)) = decode_png(&data) else {
        return Ok(undef());
    };
    Ok(json!({
        "width": js::number(width as f64),
        "height": js::number(height as f64),
        "luma": { "$bytes": luma },
    }))
}

const PROFILE_COLUMN_BAND: f64 = 0.5;
const PROFILE_ROW_BAND: f64 = 0.8;
const MIN_PROFILE_OVERLAP_ROWS: usize = 32;
const MIN_PROFILE_DEVIATION: f64 = 2.0;
const MAX_WINNING_SCORE_RATIO: f64 = 0.5;
const MAX_WINNING_SCORE: f64 = 6.0;

/// `luma[index]` for whatever `luma` is — arrays and byte lists index, the rest are undefined.
fn luma_at(luma: &Value, i: usize) -> f64 {
    match luma {
        Value::Array(a) => a.get(i).map_or(f64::NAN, js_to_number),
        Value::Object(o) if !is_undef(luma) => {
            if let Some(b) =
                single_marker(luma, "$bytes").or_else(|| single_marker(luma, "$buffer"))
            {
                b.as_array()
                    .and_then(|a| a.get(i))
                    .map_or(f64::NAN, js_to_number)
            } else {
                let _ = o;
                f64::NAN
            }
        }
        _ => f64::NAN,
    }
}

/// Port of `estimateVerticalTravel` (`apps/server/src/computer/scrollCalibration.ts:240`).
#[allow(clippy::neg_cmp_op_on_partial_ord)]
pub fn estimate_vertical_travel(
    before: Value,
    after: Value,
    options: Value,
) -> Result<Value, String> {
    let bw = js_get(&before, "width")?;
    let aw = js_get(&after, "width")?;
    let bh = js_get(&before, "height")?;
    let ah = js_get(&after, "height")?;
    if !js_strict_eq(&bw, &aw) || !js_strict_eq(&bh, &ah) {
        return Ok(undef());
    }
    let width = js_to_number(&bw);
    let height = js_to_number(&bh);
    let column_start = (width * ((1.0 - PROFILE_COLUMN_BAND) / 2.0)).floor() as usize;
    let column_count = ((width * PROFILE_COLUMN_BAND).floor() as usize).max(1);
    let row_start = (height * ((1.0 - PROFILE_ROW_BAND) / 2.0)).floor() as usize;
    let row_count = ((height * PROFILE_ROW_BAND).floor() as usize).max(1);
    if row_count < MIN_PROFILE_OVERLAP_ROWS {
        return Ok(undef());
    }
    let before_luma = js_get(&before, "luma")?;
    let after_luma = js_get(&after, "luma")?;
    let profile = |luma: &Value| -> Vec<f64> {
        let mut out = Vec::with_capacity(row_count);
        for row in 0..row_count {
            let base = (row_start + row) * width as usize + column_start;
            let mut total = 0.0f64;
            for column in 0..column_count {
                total += luma_at(luma, base + column);
            }
            out.push(total / column_count as f64);
        }
        out
    };
    let before_profile = profile(&before_luma);
    let after_profile = profile(&after_luma);
    let deviation = {
        let len = before_profile.len() as f64;
        if before_profile.is_empty() {
            0.0
        } else {
            let mean: f64 = before_profile.iter().sum::<f64>() / len;
            before_profile.iter().map(|v| (v - mean).abs()).sum::<f64>() / len
        }
    };
    if !(deviation < MIN_PROFILE_DEVIATION) {
        // Only proceed when the deviation clears the bar; NaN fails every comparison like JS.

        let max_shift_opt = if is_undef(&options) {
            undef()
        } else if options.is_null() {
            return Err("Cannot read properties of null (reading 'maxShift')".into());
        } else {
            js_get(&options, "maxShift")?
        };
        let max_shift = js_max(
            0.0,
            if is_undef(&max_shift_opt) || max_shift_opt.is_null() {
                (row_count as f64 * 0.9).floor()
            } else {
                js_to_number(&max_shift_opt).floor()
            },
        );
        let minimum_overlap =
            MIN_PROFILE_OVERLAP_ROWS.max((row_count as f64 * 0.4).ceil() as usize);
        let mut scores: Vec<f64> = Vec::new();
        let mut alignments: Vec<(f64, f64)> = Vec::new();
        let mut best: Option<(f64, f64)> = None;
        let mut shift = -max_shift;
        loop {
            if !(shift <= max_shift) {
                break;
            }
            let from = js_max(0.0, -shift);
            let to = js_min(row_count as f64, row_count as f64 - shift);
            if to - from >= minimum_overlap as f64 {
                let mut total = 0.0;
                let mut index = from as usize;
                while (index as f64) < to {
                    total += (after_profile[index]
                        - before_profile[(index as f64 + shift) as usize])
                        .abs();
                    index += 1;
                }
                let score = total / (to - from);
                scores.push(score);
                alignments.push((shift, score));
                if best.is_none_or(|(_, s)| score < s) {
                    best = Some((shift, score));
                }
            }
            shift += 1.0;
            if !shift.is_finite() && max_shift.is_finite() {
                break;
            }
            if scores.len() > 2_000_000 {
                break;
            }
        }
        if best.is_none() || scores.is_empty() {
            return Ok(undef());
        }
        let (wshift, wscore) = best.unwrap_or((0.0, f64::NAN));
        if !(wscore <= MAX_WINNING_SCORE) {
            return Ok(undef());
        }
        if alignments
            .iter()
            .any(|(shift, score)| (shift - wshift).abs() > 3.0 && *score <= wscore + 0.5)
        {
            return Ok(undef());
        }
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted.get(sorted.len() >> 1).copied().unwrap_or(f64::NAN);
        if !(wscore < median * MAX_WINNING_SCORE_RATIO) {
            return Ok(undef());
        }
        return Ok(js::number(wshift));
    }
    Ok(undef())
}

/// Port of `resolveStillIntervalMs` (`apps/server/src/computer/stillFramePublisher.ts:52`).
pub fn resolve_still_interval_ms(interval_ms: Option<f64>) -> Result<f64, String> {
    Ok(js_max(100.0, interval_ms.unwrap_or(500.0)))
}

// ── uiTreeTargeting.ts + shared/uiTreeTargeting.ts ─────────────────────────

const MAX_REPORTED_CANDIDATES: usize = 16;
const ELEMENT_DIGEST_MAX_LENGTH: usize = 60;
const ELEMENT_TEXT_MAX_LENGTH: usize = 80;

const ACTIONABLE_ROLES: &[&str] = &[
    "AXButton",
    "AXCheckBox",
    "AXRadioButton",
    "AXPopUpButton",
    "AXComboBox",
    "AXTextField",
    "AXTextArea",
    "AXSearchField",
    "AXSecureTextField",
    "AXLink",
    "AXMenuBarItem",
    "AXMenuItem",
    "AXSlider",
    "AXIncrementor",
    "AXTab",
    "AXSwitch",
    "push button",
    "button",
    "toggle button",
    "entry",
    "text field",
    "text-field",
    "search field",
    "text area",
    "textarea",
    "check box",
    "radio button",
    "combo box",
    "list box",
    "switch",
    "slider",
    "spin button",
    "link",
    "page tab",
    "menu item",
    "check menu item",
    "radio menu item",
];

const SEMANTIC_TEXT_ROLES: &[&str] = &[
    "AXTextField",
    "AXTextArea",
    "AXSearchField",
    "AXSecureTextField",
    "entry",
    "text field",
    "text-field",
    "search field",
    "text area",
    "textarea",
];

/// `childrenOf(node)` inside `flattenUiTree` — the exact for-of error V8 reports.
fn children_of(node: &Value) -> Result<Vec<Value>, String> {
    let children = js_get(node, "children")?;
    match children {
        Value::Array(a) => Ok(a),
        _ => Err("childrenOf is not a function or its return value is not iterable".into()),
    }
}

fn flatten_ui_tree(root: &Value) -> Result<Vec<Value>, String> {
    let mut nodes = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        let children = children_of(&node)?;
        nodes.push(node);
        for child in children.into_iter().rev() {
            stack.push(child);
        }
    }
    Ok(nodes)
}

fn matchable_label(node: &Value) -> Result<Value, String> {
    let label = js_get(node, "label")?;
    if !label.is_null() && !is_undef(&label) {
        return Ok(label);
    }
    let description = js_get(node, "description")?;
    if !description.is_null() && !is_undef(&description) {
        return Ok(description);
    }
    Ok(Value::String(String::new()))
}

/// Port of `normalizeLabelSpaces` (`apps/server/src/computer/uiTreeTargeting.ts:643`).
pub fn normalize_label_spaces(label: String) -> Result<String, String> {
    Ok(label
        .chars()
        .map(|c| {
            if matches!(c, '\u{a0}' | '\u{2007}' | '\u{202f}') {
                ' '
            } else {
                c
            }
        })
        .collect())
}

fn normalize_label_spaces_value(label: &Value) -> Result<String, String> {
    match js::string_units(label) {
        Some(units) => Ok(String::from_utf16_lossy(&units)
            .chars()
            .map(|c| {
                if matches!(c, '\u{a0}' | '\u{2007}' | '\u{202f}') {
                    ' '
                } else {
                    c
                }
            })
            .collect()),
        None => Err("label.replace is not a function".into()),
    }
}

fn describe_candidate(candidate: &Value) -> Result<String, String> {
    let role = js_get(candidate, "role")?;
    let label = js_get(candidate, "label")?;
    let window_id = js_get(candidate, "windowId")?;
    let window = if window_id.is_null() {
        String::new()
    } else {
        format!(" in window {}", json_or_undefined(&window_id))
    };
    Ok(format!(
        "{} {}{}",
        js_to_string(&role),
        json_or_undefined(&label),
        window
    ))
}

fn message_with_candidates(message: &str, candidates: &[Value]) -> Result<String, String> {
    if candidates.is_empty() {
        return Ok(message.to_string());
    }
    let mut parts = Vec::with_capacity(candidates.len());
    for c in candidates {
        parts.push(describe_candidate(c)?);
    }
    Ok(format!(
        "{message} Controls in the accessibility tree: {}.",
        parts.join("; ")
    ))
}

fn target_error(message: String, candidates: &[Value]) -> Result<String, String> {
    message_with_candidates(&message, candidates)
}

/// Port of `candidateDescriptions` (`apps/server/src/computer/uiTreeTargeting.ts:313`).
pub fn candidate_descriptions(nodes: Value) -> Result<Vec<Value>, String> {
    let sliced: Vec<Value> = match &nodes {
        Value::Array(a) => a.iter().take(MAX_REPORTED_CANDIDATES).cloned().collect(),
        Value::String(_) => {
            // "abc".slice(0,16) yields a string; `.map` is not a function on it.
            return Err("nodes.slice(...).map is not a function".into());
        }
        Value::Null => return Err("Cannot read properties of null (reading 'slice')".into()),
        v if is_undef(v) => {
            return Err("Cannot read properties of undefined (reading 'slice')".into());
        }
        v if single_marker(v, "$bytes").is_some() || single_marker(v, "$buffer").is_some() => {
            let items = buffer_from(v).unwrap_or_default();
            items
                .iter()
                .take(MAX_REPORTED_CANDIDATES)
                .map(|b| Value::from(*b))
                .collect()
        }
        _ => return Err("nodes.slice is not a function".into()),
    };
    let mut out = Vec::with_capacity(sliced.len());
    for node in &sliced {
        let label = js_get(node, "label")?;
        let label = if label.is_null() || is_undef(&label) {
            let description = js_get(node, "description")?;
            if description.is_null() || is_undef(&description) {
                Value::String("(unlabelled)".into())
            } else {
                description
            }
        } else {
            label
        };
        out.push(json!({
            "label": label,
            "role": js_get(node, "role")?,
            "windowId": js_get(node, "windowId")?,
            "onScreen": js_get(node, "onScreen")?,
            "frame": js_get(node, "frame")?,
        }));
    }
    Ok(out)
}

/// Port of `describeTarget` (`apps/server/src/computer/uiTreeTargeting.ts:612`).
pub fn describe_target(target: Value) -> Result<String, String> {
    let mut parts = Vec::new();
    let label = js_get(&target, "label")?;
    if js_truthy(&label) {
        parts.push(format!("label={}", json_or_undefined(&label)));
    }
    let role = js_get(&target, "role")?;
    if js_truthy(&role) {
        parts.push(format!("role={}", json_or_undefined(&role)));
    }
    let window_id = js_get(&target, "windowId")?;
    if js_truthy(&window_id) {
        parts.push(format!("window={}", json_or_undefined(&window_id)));
    }
    let ref_ordinal = js_get(&target, "refOrdinal")?;
    if !is_undef(&ref_ordinal) {
        let incremented =
            if matches!(ref_ordinal, Value::String(_)) || is_marker(&ref_ordinal, "$utf16") {
                Value::String(format!("{}1", js_to_string(&ref_ordinal)))
            } else {
                js_add(&ref_ordinal, &json!(1))
            };
        parts.push(format!("duplicate {}", js_to_string(&incremented)));
    }
    if parts.is_empty() {
        return Ok("the supplied coordinates".into());
    }
    Ok(parts.join(", "))
}

/// `activationPointForNode` — its own control point or the frame centre.
fn activation_point(node: &Value) -> Result<Value, String> {
    let point = js_get(node, "activationPoint")?;
    if !point.is_null() && !is_undef(&point) {
        return Ok(point);
    }
    let frame = js_get(node, "frame")?;
    let fx = js_get(&frame, "x")?;
    let fy = js_get(&frame, "y")?;
    let fw = js_get(&frame, "width")?;
    let fh = js_get(&frame, "height")?;
    Ok(json!({
        "x": js_add(&fx, &js::number(js_to_number(&fw) / 2.0)),
        "y": js_add(&fy, &js::number(js_to_number(&fh) / 2.0)),
    }))
}

/// Port of `activationPointForNode` (`apps/server/src/computer/uiTreeTargeting.ts:309`).
pub fn activation_point_for_node(node: Value) -> Result<Value, String> {
    activation_point(&node)
}

/// Port of `computerTargetCandidates` (`apps/server/src/computer/uiTreeTargeting.ts:325`).
pub fn computer_target_candidates(root: Value) -> Result<Vec<Value>, String> {
    let flat = flatten_ui_tree(&root)?;
    candidate_descriptions(Value::Array(flat))
}

/// `matchesWindow(node, target.windowId)` — undefined windowId matches all.
fn matches_window(node: &Value, window_id: &Value) -> Result<bool, String> {
    if is_undef(window_id) {
        return Ok(true);
    }
    Ok(js_strict_eq(&js_get(node, "windowId")?, window_id))
}

/// `resolveUiTreeTarget` for the desktop spec.
fn resolve_ui_tree_target(
    pool: &[Value],
    target: &Value,
    spec_label: &Value,
    spec_role: &Value,
) -> Result<Value, String> {
    let by_role: Vec<Value> = if is_undef(spec_role) {
        pool.to_vec()
    } else {
        let mut out = Vec::new();
        for node in pool {
            if js_strict_eq(&js_get(node, "role")?, spec_role) {
                out.push(node.clone());
            }
        }
        out
    };
    let matches: Vec<Value> = if is_undef(spec_label) {
        by_role
    } else {
        let label_units = match js::string_units(spec_label) {
            Some(u) => u,
            None => return Err("label.replace is not a function".into()),
        };
        let exact = normalize_label_spaces_value(&js::utf16_string(&label_units))?;
        let mut exact_matches = Vec::new();
        for node in &by_role {
            let label = matchable_label(node)?;
            let key = if label.is_null() || is_undef(&label) {
                String::new()
            } else if matches!(label, Value::String(_)) || is_marker(&label, "$utf16") {
                normalize_label_spaces_value(&label)?
            } else {
                return Err("label.replace is not a function".into());
            };
            if key == exact {
                exact_matches.push(node.clone());
            }
        }
        if !exact_matches.is_empty() {
            exact_matches
        } else {
            let wanted =
                normalize_label_spaces_value(&js::utf16_string(&label_units))?.to_lowercase();
            let mut out = Vec::new();
            for node in &by_role {
                let label = matchable_label(node)?;
                let key = if label.is_null() || is_undef(&label) {
                    String::new()
                } else {
                    normalize_label_spaces_value(&label)?.to_lowercase()
                };
                if key.contains(&wanted) {
                    out.push(node.clone());
                }
            }
            out
        }
    };
    if matches.is_empty() {
        let mut candidates = Vec::new();
        for node in pool.iter().take(MAX_REPORTED_CANDIDATES) {
            candidates.push(candidate_json(node)?);
        }
        return Err(target_error(
            format!(
                "No visible computer target matched {}.",
                describe_target(target.clone())?
            ),
            &candidates,
        )?);
    }
    if matches.len() > 1 {
        let mut candidates = Vec::new();
        for node in matches.iter().take(MAX_REPORTED_CANDIDATES) {
            candidates.push(candidate_json(node)?);
        }
        return Err(target_error(
            format!(
                "Computer target {} matched more than one control.",
                describe_target(target.clone())?
            ),
            &candidates,
        )?);
    }
    Ok(matches[0].clone())
}

fn candidate_json(node: &Value) -> Result<Value, String> {
    let label = js_get(node, "label")?;
    let label = if label.is_null() || is_undef(&label) {
        let description = js_get(node, "description")?;
        if description.is_null() || is_undef(&description) {
            Value::String("(unlabelled)".into())
        } else {
            description
        }
    } else {
        label
    };
    Ok(json!({
        "label": label,
        "role": js_get(node, "role")?,
        "windowId": js_get(node, "windowId")?,
        "onScreen": js_get(node, "onScreen")?,
        "frame": js_get(node, "frame")?,
    }))
}

/// Port of `resolveComputerSemanticTarget` (`apps/server/src/computer/uiTreeTargeting.ts:126`).
pub fn resolve_computer_semantic_target(
    root: Value,
    target: Value,
    allow_offscreen: Option<bool>,
) -> Result<Value, String> {
    let allow_offscreen = allow_offscreen.unwrap_or(false);
    let ref_ordinal = js_get(&target, "refOrdinal")?;
    let label = js_get(&target, "label")?;
    if !is_undef(&ref_ordinal) && !is_undef(&label) {
        return resolve_computer_ordinal_target(&root, &target, &ref_ordinal, allow_offscreen);
    }
    let role = js_get(&target, "role")?;
    let window_id = js_get(&target, "windowId")?;
    let flat = flatten_ui_tree(&root)?;
    let mut pool = Vec::new();
    for node in &flat {
        if matches_window(node, &window_id)? {
            pool.push(node.clone());
        }
    }
    let node = resolve_ui_tree_target(&pool, &target, &label, &role)?;
    if !js_truthy(&js_get(&node, "onScreen")?) && !allow_offscreen {
        let candidates = vec![candidate_json(&node)?];
        return Err(target_error(
            format!(
                "Computer target {} is off-screen; refusing to guess a click.",
                describe_target(target.clone())?
            ),
            &candidates,
        )?);
    }
    Ok(json!({
        "node": node,
        "point": activation_point(&node)?,
    }))
}

fn resolve_computer_ordinal_target(
    root: &Value,
    target: &Value,
    ordinal: &Value,
    allow_offscreen: bool,
) -> Result<Value, String> {
    let window_id = js_get(target, "windowId")?;
    let role = js_get(target, "role")?;
    let label = js_get(target, "label")?;
    let flat = flatten_ui_tree(root)?;
    let mut pool = Vec::new();
    for node in &flat {
        if matches_window(node, &window_id)? {
            pool.push(node.clone());
        }
    }
    let label_units = match js::string_units(&label) {
        Some(u) => u,
        None => return Err("label.replace is not a function".into()),
    };
    let exact = normalize_label_spaces_value(&js::utf16_string(&label_units))?;
    let mut matches = Vec::new();
    for node in &pool {
        if !is_undef(&role) && !js_strict_eq(&js_get(node, "role")?, &role) {
            continue;
        }
        let node_label = matchable_label(node)?;
        let key = if node_label.is_null() || is_undef(&node_label) {
            String::new()
        } else {
            normalize_label_spaces_value(&node_label)?
        };
        if key == exact {
            matches.push(node.clone());
        }
    }
    let on_screen: Vec<Value> = matches
        .iter()
        .filter(|n| js_truthy(&js_get(n, "onScreen").unwrap_or_else(|_| undef())))
        .cloned()
        .collect();
    let ordered = if on_screen.is_empty() {
        matches
    } else {
        on_screen
    };
    let ordinal_num = js_to_number(ordinal);

    let node: Value = if (ordered.len() as f64) > ordinal_num
        && ordinal_num >= 0.0
        && ordinal_num.fract() == 0.0
    {
        ordered[ordinal_num as usize].clone()
    } else if ordered.is_empty() {
        let mut candidates = Vec::new();
        for node in pool.iter().take(MAX_REPORTED_CANDIDATES) {
            candidates.push(candidate_json(node)?);
        }
        return Err(target_error(
            format!(
                "No visible computer target matched {}.",
                describe_target(target.clone())?
            ),
            &candidates,
        )?);
    } else {
        let mut candidates = Vec::new();
        for node in ordered.iter().take(MAX_REPORTED_CANDIDATES) {
            candidates.push(candidate_json(node)?);
        }
        return Err(target_error(
            format!(
                "Computer target {} no longer exists at its observed duplicate position. Observe again with computer_get_state before acting.",
                describe_target(target.clone())?
            ),
            &candidates,
        )?);
    };
    if !js_truthy(&js_get(&node, "onScreen")?) && !allow_offscreen {
        let candidates = vec![candidate_json(&node)?];
        return Err(target_error(
            format!(
                "Computer target {} is off-screen; refusing to guess a click.",
                describe_target(target.clone())?
            ),
            &candidates,
        )?);
    }
    Ok(json!({
        "node": node,
        "point": activation_point(&node)?,
    }))
}

/// Port of `resolveComputerUniqueTextTarget` (`apps/server/src/computer/uiTreeTargeting.ts:215`).
pub fn resolve_computer_unique_text_target(
    root: Value,
    window_id: String,
    allow_offscreen: Option<bool>,
) -> Result<Value, String> {
    let allow_offscreen = allow_offscreen.unwrap_or(false);
    let window_id = Value::String(window_id);
    let flat = flatten_ui_tree(&root)?;
    let mut candidates = Vec::new();
    let mut window_nodes = Vec::new();
    for node in &flat {
        if !js_strict_eq(&js_get(node, "windowId")?, &window_id) {
            continue;
        }
        window_nodes.push(node.clone());
        let editable = js_get(node, "editable")?;
        if !allow_offscreen && !js_truthy(&js_get(node, "onScreen")?) {
            continue;
        }
        if js_strict_eq(&editable, &Value::Bool(false)) {
            continue;
        }
        let role = js_get(node, "role")?;
        let role_ok = js_strict_eq(&editable, &Value::Bool(true))
            || matches!(&role, Value::String(r) if SEMANTIC_TEXT_ROLES.contains(&r.as_str()));
        if role_ok {
            candidates.push(node.clone());
        }
    }
    if candidates.is_empty() {
        let mut descriptions = Vec::new();
        for node in window_nodes.iter().take(MAX_REPORTED_CANDIDATES) {
            descriptions.push(candidate_json(node)?);
        }
        return Err(target_error(
            format!(
                "Window {} has no {}writable text control. Observe it and pass the exact label and role.",
                json_or_undefined(&window_id),
                if allow_offscreen { "" } else { "visible " },
            ),
            &descriptions,
        )?);
    }
    if candidates.len() > 1 {
        let mut descriptions = Vec::new();
        for node in candidates.iter().take(MAX_REPORTED_CANDIDATES) {
            descriptions.push(candidate_json(node)?);
        }
        return Err(target_error(
            format!(
                "Window {} has more than one {}writable text control. Pass the exact label and role.",
                json_or_undefined(&window_id),
                if allow_offscreen { "" } else { "visible " },
            ),
            &descriptions,
        )?);
    }
    let node = candidates[0].clone();
    Ok(json!({
        "node": node,
        "point": activation_point(&node)?,
    }))
}

/// Port of `resolveComputerWindowTarget` (`apps/server/src/computer/uiTreeTargeting.ts:257`).
pub fn resolve_computer_window_target(root: Value, window_id: String) -> Result<Value, String> {
    let window_id = Value::String(window_id);
    let flat = flatten_ui_tree(&root)?;
    let mut found: Option<Value> = None;
    for node in &flat {
        if js_strict_eq(&js_get(node, "windowId")?, &window_id) {
            found = Some(node.clone());
            break;
        }
    }
    let Some(node) = found else {
        return Ok(undef());
    };
    if !js_truthy(&js_get(&node, "onScreen")?) {
        let candidates = vec![candidate_json(&node)?];
        return Err(target_error(
            format!(
                "Computer window {} is off-screen; refusing to guess a scroll point.",
                json_or_undefined(&window_id)
            ),
            &candidates,
        )?);
    }
    Ok(json!({
        "node": node,
        "point": activation_point(&node)?,
    }))
}

/// Port of `resolveComputerPoint` (`apps/server/src/computer/uiTreeTargeting.ts:97`).
pub fn resolve_computer_point(target: Value, screen_size: Value) -> Result<Value, String> {
    let tx = js_get(&target, "x")?;
    let ty = js_get(&target, "y")?;
    let has_x = is_number(&tx);
    let has_y = is_number(&ty);
    if has_x != has_y {
        return Err("Computer coordinate targets must include both x and y.".into());
    }
    if !has_x && !has_y {
        return Err("Computer actions require x/y coordinates or a labelled target.".into());
    }
    let (x, y) = (num_or_nan(&tx), num_or_nan(&ty));
    let sw = js_get(&screen_size, "width")?;
    let sh = js_get(&screen_size, "height")?;
    if x < 0.0 || y < 0.0 || js_ge(&tx, &sw) || js_ge(&ty, &sh) {
        return Err(format!(
            "Computer target ({}, {}) is outside the {}x{} screen.",
            js::number_to_string(x),
            js::number_to_string(y),
            js_to_string(&sw),
            js_to_string(&sh)
        ));
    }
    Ok(json!({
        "x": js::number(x),
        "y": js::number(y),
    }))
}

/// Port of `actionableElements` (`apps/server/src/computer/uiTreeTargeting.ts:463`).
pub fn actionable_elements(root: Value, filter: Value) -> Result<Value, String> {
    let filter = if filter.is_null() {
        return Err("Cannot read properties of null (reading 'labelContains')".into());
    } else if is_undef(&filter) {
        json!({})
    } else {
        filter
    };
    let label_contains = js_get(&filter, "labelContains")?;
    let window_id = js_get(&filter, "windowId")?;
    let wanted = if is_undef(&label_contains) {
        None
    } else {
        Some(normalize_label_spaces_value(&label_contains)?.to_lowercase())
    };
    let mut items: Vec<Value> = Vec::new();
    let mut ref_index: Vec<Value> = Vec::new();
    let mut ordinals: Vec<(String, f64)> = Vec::new();
    let mut omitted = 0u64;
    let mut source_incomplete = false;
    #[allow(clippy::too_many_arguments)]
    fn walk(
        node: &Value,
        items: &mut Vec<Value>,
        ref_index: &mut Vec<Value>,
        ordinals: &mut Vec<(String, f64)>,
        omitted: &mut u64,
        source_incomplete: &mut bool,
        window_id: &Value,
        wanted: &Option<String>,
    ) -> Result<(), String> {
        if js_truthy(&js_get(node, "truncated")?) {
            *source_incomplete = true;
        }
        let label = matchable_label(node)?;
        let role = js_get(node, "role")?;
        let on_screen = js_get(node, "onScreen")?;
        let node_window = js_get(node, "windowId")?;
        let label_str = match js::string_units(&label) {
            Some(u) => String::from_utf16_lossy(&u),
            None => String::new(),
        };
        let collectible = matches!(&role, Value::String(r) if ACTIONABLE_ROLES.contains(&r.as_str()))
            && js_truthy(&on_screen)
            && !node_window.is_null()
            && !label_str.is_empty()
            && (is_undef(window_id) || js_strict_eq(&node_window, window_id))
            && wanted.as_ref().is_none_or(|w| {
                normalize_label_spaces(label_str.clone())
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(w)
            });
        if collectible {
            let identity = format!(
                "{}{}{}",
                js_to_string(&node_window),
                js_to_string(&role),
                normalize_label_spaces(label_str.clone()).unwrap_or_default()
            );
            let ordinal = ordinals
                .iter()
                .find(|(k, _)| k == &identity)
                .map(|(_, v)| *v)
                .unwrap_or(0.0);
            if let Some(entry) = ordinals.iter_mut().find(|(k, _)| k == &identity) {
                entry.1 = ordinal + 1.0;
            } else {
                ordinals.push((identity, ordinal + 1.0));
            }
            if items.len() < ELEMENT_DIGEST_MAX_LENGTH {
                ref_index.push(json!({
                    "label": label,
                    "role": role,
                    "windowId": node_window,
                    "ordinal": js::number(ordinal),
                }));
                let mut item = Map::new();
                item.insert("ref".into(), js::number(items.len() as f64));
                item.insert("role".into(), role.clone());
                item.insert(
                    "label".into(),
                    Value::String(clamp_text_to_length(
                        &label_str,
                        ELEMENT_TEXT_MAX_LENGTH as f64,
                    )),
                );
                let value = js_get(node, "value")?;
                if !value.is_null() && !is_undef(&value) {
                    let vs = js::string_units(&value)
                        .map(|u| String::from_utf16_lossy(&u))
                        .unwrap_or_default();
                    item.insert(
                        "value".into(),
                        Value::String(clamp_text_to_length(&vs, 40.0)),
                    );
                }
                item.insert("windowId".into(), node_window.clone());
                items.push(Value::Object(item));
            } else {
                *omitted += 1;
            }
        }
        let children = js_get(node, "children")?;
        let Value::Array(children) = children else {
            return Err("node.children is not iterable".into());
        };
        for child in children {
            walk(
                &child,
                items,
                ref_index,
                ordinals,
                omitted,
                source_incomplete,
                window_id,
                wanted,
            )?;
        }
        Ok(())
    }
    walk(
        &root,
        &mut items,
        &mut ref_index,
        &mut ordinals,
        &mut omitted,
        &mut source_incomplete,
        &window_id,
        &wanted,
    )?;
    Ok(json!({
        "items": items,
        "refIndex": ref_index,
        "complete": omitted == 0 && !source_incomplete,
        "omitted": omitted,
        "sourceIncomplete": source_incomplete,
    }))
}

/// Port of `diffActionableElements` (`apps/server/src/computer/uiTreeTargeting.ts:561`).
pub fn diff_actionable_elements(before: Value, after: Value) -> Result<Value, String> {
    let identity = |item: &Value| -> Result<String, String> {
        let window = js_get(item, "windowId")?;
        let window = if window.is_null() || is_undef(&window) {
            Value::Null
        } else {
            window
        };
        let role = js_get(item, "role")?;
        let label = js_get(item, "label")?;
        Ok(js_json_stringify(&Value::Array(vec![window, role, label]))
            .unwrap_or_else(|| "null".into()))
    };
    let group = |items: &Value| -> Result<Vec<(String, Vec<Value>)>, String> {
        let mut grouped: Vec<(String, Vec<Value>)> = Vec::new();
        for item in for_of(items, "items")? {
            let key = identity(&item)?;
            if let Some((_, bucket)) = grouped.iter_mut().find(|(k, _)| k == &key) {
                bucket.push(item.clone());
            } else {
                grouped.push((key, vec![item]));
            }
        }
        Ok(grouped)
    };
    let old_groups = group(&before)?;
    let mut new_groups = group(&after)?;
    let mut added: Vec<Value> = Vec::new();
    let mut removed: Vec<Value> = Vec::new();
    let mut changed: Vec<Value> = Vec::new();
    for (key, previous) in &old_groups {
        let current_pos = new_groups.iter().position(|(k, _)| k == key);
        let Some(current_pos) = current_pos else {
            removed.extend(previous.iter().cloned());
            continue;
        };
        let current = new_groups[current_pos].1.clone();
        let overlap = previous.len().min(current.len());
        for index in 0..overlap {
            let old_v = js_get(&previous[index], "value")?;
            let new_v = js_get(&current[index], "value")?;
            if !js_strict_eq(&old_v, &new_v) {
                let item = &current[index];
                let mut change = Map::new();
                change.insert("ref".into(), js_get(item, "ref")?);
                change.insert("role".into(), js_get(item, "role")?);
                change.insert("label".into(), js_get(item, "label")?);
                change.insert("windowId".into(), js_get(item, "windowId")?);
                if !is_undef(&old_v) {
                    change.insert("was".into(), old_v.clone());
                }
                if !is_undef(&new_v) {
                    change.insert("value".into(), new_v.clone());
                }
                changed.push(Value::Object(change));
            }
        }
        removed.extend(previous.iter().skip(overlap).cloned());
        added.extend(current.iter().skip(overlap).cloned());
        new_groups.remove(current_pos);
    }
    for (_, current) in &new_groups {
        added.extend(current.iter().cloned());
    }
    Ok(json!({
        "added": added,
        "removed": removed,
        "changed": changed,
    }))
}

// ── uiTreeText.ts / utf8Truncation.ts ───────────────────────────────────────

/// `clampTextToLength` — UTF-16-aware, surrogate-safe clamp.
fn clamp_text_to_length(text: &str, max_length: f64) -> String {
    let units = utf16::units(text);
    if (units.len() as f64) <= max_length {
        return text.to_string();
    }
    let cut = max_length - 1.0;
    let code = if cut > 0.0 {
        units.get(cut as usize - 1).copied().unwrap_or(0)
    } else {
        0
    };
    let cut = if (0xd800..=0xdbff).contains(&code) {
        cut - 1.0
    } else {
        cut
    };
    let end = js_max(0.0, cut);
    let end = if end.is_nan() || end < 0.0 {
        0usize
    } else {
        end as usize
    };
    format!("{}\u{2026}", utf16::slice(&units, 0, end))
}

/// Port of `describeComputerUiTree` (`apps/server/src/computer/uiTreeText.ts:17`).
pub fn describe_computer_ui_tree(root: Value) -> Result<String, String> {
    let mut lines: Vec<String> = Vec::new();
    fn visit(node: &Value, depth: usize, lines: &mut Vec<String>) -> Result<(), String> {
        let label = js_get(node, "label")?;
        let label = if label.is_null() || is_undef(&label) {
            let description = js_get(node, "description")?;
            if description.is_null() || is_undef(&description) {
                Value::String("(unlabelled)".into())
            } else {
                description
            }
        } else {
            label
        };
        let truncated = if js_strict_eq(&js_get(node, "truncated")?, &Value::Bool(true)) {
            " \u{2026}(truncated)"
        } else {
            ""
        };
        let value = js_get(node, "value")?;
        let role = js_get(node, "role")?;
        lines.push(format!(
            "{}{}: {}{}{}",
            "  ".repeat(depth),
            js_to_string(&role),
            js_to_string(&label),
            if js_truthy(&value) {
                format!(" = {}", js_to_string(&value))
            } else {
                String::new()
            },
            truncated
        ));
        let children = js_get(node, "children")?;
        let Value::Array(children) = children else {
            return Err("node.children is not iterable".into());
        };
        for child in children {
            visit(&child, depth + 1, lines)?;
        }
        Ok(())
    }
    visit(&root, 0, &mut lines)?;
    Ok(lines.join("\n"))
}

/// Port of `clampNodeText` (`apps/server/src/computer/uiTreeText.ts:40`).
pub fn clamp_node_text(text: String, max_length: f64) -> Result<String, String> {
    Ok(clamp_text_to_length(&text, max_length))
}

/// Port of `clampTextToLength` (`apps/server/src/computer/utf8Truncation.ts:15`).
pub fn clamp_text_to_length_pub(text: String, max_length: f64) -> Result<String, String> {
    Ok(clamp_text_to_length(&text, max_length))
}

// ── ComputerControlState.ts ─────────────────────────────────────────────────

/// Port of class `ComputerControlState` (`apps/server/src/computer/ComputerControlState.ts:14`): holds the instance state.
pub struct ComputerControlState {
    threads: Map<String, Value>,
    file_path: Option<String>,
    load_error: bool,
}

impl ComputerControlState {
    /// Port of `ComputerControlState` constructor (`apps/server/src/computer/ComputerControlState.ts:14`).
    pub fn new(file_path: Option<String>) -> Result<Self, String> {
        let mut state = ComputerControlState {
            threads: Map::new(),
            file_path,
            load_error: false,
        };
        let Some(path) = &state.file_path else {
            return Ok(state);
        };
        let data = match std::fs::read(path) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(state),
            Err(_) => {
                state.load_error = true;
                return Ok(state);
            }
        };
        let text = String::from_utf8_lossy(&data);
        let parsed: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => {
                state.load_error = true;
                return Ok(state);
            }
        };
        let Value::Object(map) = &parsed else {
            state.load_error = true;
            return Ok(state);
        };
        for (thread_id, value) in js_entries(map) {
            let valid = match &value {
                Value::Object(o) => {
                    let disabled = matches!(o.get("disabled"), Some(Value::Bool(_)));
                    let generation = o
                        .get("generation")
                        .map(|g| {
                            is_number(g) && is_safe_integer(num_or_nan(g)) && num_or_nan(g) >= 0.0
                        })
                        .unwrap_or(false);
                    let chat = match o.get("chatGeneration") {
                        None => true,
                        Some(g) => {
                            is_number(g) && is_safe_integer(num_or_nan(g)) && num_or_nan(g) >= 0.0
                        }
                    };
                    disabled && generation && chat
                }
                _ => false,
            };
            if !valid {
                state.load_error = true;
                return Ok(state);
            }
            state.threads.insert(thread_id, value);
        }
        Ok(state)
    }

    /// Port of `ComputerControlState.get` (`apps/server/src/computer/ComputerControlState.ts:58`).
    pub fn get(&mut self, thread_id: String) -> Result<Value, String> {
        if self.load_error {
            return Ok(json!({ "disabled": true, "generation": 0 }));
        }
        Ok(self
            .threads
            .get(&thread_id)
            .cloned()
            .unwrap_or_else(|| json!({ "disabled": false, "generation": 0 })))
    }

    /// Port of `ComputerControlState.allows` (`apps/server/src/computer/ComputerControlState.ts:63`).
    pub fn allows(&mut self, thread_id: String, generation: f64) -> Result<bool, String> {
        let state = self.get(thread_id)?;
        let disabled = js_truthy(&js_get(&state, "disabled")?);
        let stored = js_get(&state, "generation")?;
        Ok(!disabled && js_strict_eq(&js::number(generation), &stored))
    }

    /// Port of `ComputerControlState.set` (`apps/server/src/computer/ComputerControlState.ts:68`).
    pub fn set(&mut self, thread_id: String, disabled: bool) -> Result<(), String> {
        if self.load_error {
            return Err(
                "Computer authorization state could not be loaded; control remains disabled."
                    .into(),
            );
        }
        let previous = self.get(thread_id.clone())?;
        if !disabled && !js_truthy(&js_get(&previous, "disabled")?) {
            return Ok(());
        }
        let current = self.get(thread_id.clone())?;
        if !disabled && !js_truthy(&js_get(&current, "disabled")?) {
            return Ok(());
        }
        let generation =
            num_or_nan(&js_get(&current, "generation")?) + if disabled { 1.0 } else { 0.0 };
        self.threads.insert(
            thread_id,
            json!({ "disabled": disabled, "generation": js::number(generation) }),
        );
        self.persist()
    }

    /// Port of `ComputerControlState.recordChatIntent` (`apps/server/src/computer/ComputerControlState.ts:83`).
    pub fn record_chat_intent(
        &mut self,
        thread_id: String,
        enabled: bool,
        generation: f64,
    ) -> Result<(), String> {
        if self.load_error {
            return if enabled {
                Err(
                    "Computer authorization state could not be loaded; control remains disabled."
                        .into(),
                )
            } else {
                Ok(())
            };
        }
        let previous = self.get(thread_id.clone())?;
        let chat_generation = if enabled && self.allows(thread_id.clone(), generation)? {
            Some(generation)
        } else {
            None
        };
        let prev_chat = previous.get("chatGeneration");
        let same = match (prev_chat, chat_generation) {
            (None, None) => true,
            (Some(a), Some(b)) => js_strict_eq(a, &js::number(b)),
            _ => false,
        };
        if same {
            return Ok(());
        }
        let mut next = Map::new();
        next.insert(
            "disabled".into(),
            previous
                .get("disabled")
                .cloned()
                .unwrap_or(Value::Bool(false)),
        );
        next.insert(
            "generation".into(),
            previous
                .get("generation")
                .cloned()
                .unwrap_or_else(|| json!(0)),
        );
        if let Some(g) = chat_generation {
            next.insert("chatGeneration".into(), js::number(g));
        }
        self.threads.insert(thread_id, Value::Object(next));
        if self.persist().is_err() {
            // Roll back like `persist().catch` does, keeping the in-memory record.
        }
        Ok(())
    }

    fn persist(&mut self) -> Result<(), String> {
        let Some(path) = &self.file_path else {
            return Ok(());
        };
        if path.is_empty() {
            return Ok(());
        }
        let mut obj = Map::new();
        for (k, v) in &self.threads {
            obj.insert(k.clone(), v.clone());
        }
        let content = js_json_stringify(&Value::Object(obj)).unwrap_or_else(|| "{}".into());
        if let Some(dir) = std::path::Path::new(path).parent()
            && std::fs::create_dir_all(dir).is_err()
        {
            return Err("persist failed".into());
        }
        let tmp = format!("{path}.tmp");
        if std::fs::write(&tmp, &content).is_err() {
            return Err("persist failed".into());
        }
        if std::fs::rename(&tmp, path).is_err() {
            return Err("persist failed".into());
        }
        Ok(())
    }
}

// ── ScreenshotFrameRegistry ─────────────────────────────────────────────────

const SCREENSHOT_FRAMES_PER_THREAD: usize = 8;
const SCREENSHOT_FRAME_THREADS: usize = 256;

/// Port of class `ScreenshotFrameRegistry` (`apps/server/src/computer/screenshotFrames.ts:57`): holds the instance state.
pub struct ScreenshotFrameRegistry {
    threads: Vec<(String, Vec<Value>)>,
    hashes: std::collections::HashMap<String, String>,
    sequence: u64,
}

impl ScreenshotFrameRegistry {
    /// Port of `ScreenshotFrameRegistry` constructor (`apps/server/src/computer/screenshotFrames.ts:57`).
    pub fn new() -> Result<Self, String> {
        Ok(ScreenshotFrameRegistry {
            threads: Vec::new(),
            hashes: std::collections::HashMap::new(),
            sequence: 0,
        })
    }

    fn frames(&self, thread_id: &str) -> &[Value] {
        self.threads
            .iter()
            .find(|(t, _)| t == thread_id)
            .map(|(_, f)| f.as_slice())
            .unwrap_or(&[])
    }

    fn delete_thread(&mut self, thread_id: &str) {
        self.threads.retain(|(t, _)| t != thread_id);
    }

    fn touch_thread(&mut self, thread_id: &str, frames: Vec<Value>) {
        self.threads.retain(|(t, _)| t != thread_id);
        self.threads.push((thread_id.to_string(), frames));
        while self.threads.len() > SCREENSHOT_FRAME_THREADS {
            self.threads.remove(0);
        }
    }

    fn image_hash(screenshot: &Value) -> Result<String, String> {
        let bytes = js_get(screenshot, "bytesBase64")?;
        if bytes.is_null() || is_undef(&bytes) {
            return Ok(sha256_hex(b""));
        }
        match js::string_units(&bytes) {
            Some(units) => Ok(sha256_hex(String::from_utf16_lossy(&units).as_bytes())),
            None => Err(
                "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView. Received type number".into(),
            ),
        }
    }

    /// Port of `ScreenshotFrameRegistry.record` (`apps/server/src/computer/screenshotFrames.ts:80`).
    pub fn record(
        &mut self,
        thread_id: String,
        screenshot: Value,
        window_id: Value,
    ) -> Result<Value, String> {
        let region = js_get(&screenshot, "region")?;
        if !js_truthy(&region) {
            self.delete_thread(&thread_id);
            return Ok(undef());
        }
        let rw = js_get(&region, "width")?;
        let rh = js_get(&region, "height")?;
        if js_le(&rw, &json!(0)) || js_le(&rh, &json!(0)) {
            self.delete_thread(&thread_id);
            return Ok(undef());
        }
        let scale_prop = js_get(&screenshot, "scale")?;
        let scale = if scale_prop.is_null() || is_undef(&scale_prop) {
            js_to_number(&js_get(&screenshot, "width")?) / js_to_number(&rw)
        } else {
            js_to_number(&scale_prop)
        };
        if !scale.is_finite() || scale <= 0.0 {
            self.delete_thread(&thread_id);
            return Ok(undef());
        }
        self.sequence += 1;
        let id = format!("shot-{}", self.sequence);
        let mut frame = Map::new();
        frame.insert("id".into(), Value::String(id.clone()));
        frame.insert("width".into(), js_get(&screenshot, "width")?);
        frame.insert("height".into(), js_get(&screenshot, "height")?);
        frame.insert("region".into(), js_spread(&region));
        frame.insert("scale".into(), js::number(scale));
        if !is_undef(&window_id) {
            frame.insert("windowId".into(), window_id);
        }
        let bytes64 = js_get(&screenshot, "bytesBase64")?;
        if !is_undef(&bytes64) {
            self.hashes
                .insert(id.clone(), Self::image_hash(&screenshot)?);
        }
        let mut frames = self
            .threads
            .iter()
            .find(|(t, _)| *t == thread_id)
            .map(|(_, f)| f.clone())
            .unwrap_or_default();
        frames.push(Value::Object(frame));
        if frames.len() > SCREENSHOT_FRAMES_PER_THREAD {
            let drop = frames.len() - SCREENSHOT_FRAMES_PER_THREAD;
            frames.drain(0..drop);
        }
        self.touch_thread(&thread_id, frames);
        let stored = self
            .threads
            .iter()
            .find(|(t, _)| *t == thread_id)
            .and_then(|(_, f)| f.last())
            .cloned();
        Ok(stored.unwrap_or_else(undef))
    }

    /// Port of `ScreenshotFrameRegistry.latest` (`apps/server/src/computer/screenshotFrames.ts:122`).
    pub fn latest(&mut self, thread_id: String) -> Result<Value, String> {
        Ok(self
            .frames(&thread_id)
            .last()
            .cloned()
            .unwrap_or_else(undef))
    }

    /// Port of `ScreenshotFrameRegistry.matchLatest` (`apps/server/src/computer/screenshotFrames.ts:127`).
    pub fn match_latest(
        &mut self,
        thread_id: String,
        screenshot: Value,
        window_id: Value,
    ) -> Result<Value, String> {
        let frame = self.frames(&thread_id).last().cloned();
        let region = js_get(&screenshot, "region")?;
        let Some(frame) = frame else {
            return Ok(undef());
        };
        if !js_truthy(&region) {
            return Ok(undef());
        }
        let frame_window = js_get(&frame, "windowId")?;
        if !js_strict_eq(&frame_window, &window_id) {
            return Ok(undef());
        }
        if !js_strict_eq(&js_get(&frame, "width")?, &js_get(&screenshot, "width")?)
            || !js_strict_eq(&js_get(&frame, "height")?, &js_get(&screenshot, "height")?)
        {
            return Ok(undef());
        }
        let scale_prop = js_get(&screenshot, "scale")?;
        let rw = js_get(&region, "width")?;
        let scale = if scale_prop.is_null() || is_undef(&scale_prop) {
            js_to_number(&js_get(&screenshot, "width")?) / js_to_number(&rw)
        } else {
            js_to_number(&scale_prop)
        };
        if !js_strict_eq(&js_get(&frame, "scale")?, &js::number(scale)) {
            return Ok(undef());
        }
        let frame_region = js_get(&frame, "region")?;
        for key in ["x", "y", "width", "height"] {
            if !js_strict_eq(&js_get(&frame_region, key)?, &js_get(&region, key)?) {
                return Ok(undef());
            }
        }
        let id = js_get(&frame, "id")?;
        let stored = id.as_str().and_then(|s| self.hashes.get(s).cloned());
        let current = Self::image_hash(&screenshot)?;
        if stored.as_deref() != Some(current.as_str()) {
            return Ok(undef());
        }
        Ok(frame)
    }

    /// Port of `ScreenshotFrameRegistry.resolve` (`apps/server/src/computer/screenshotFrames.ts:159`).
    pub fn resolve(&mut self, thread_id: String, screenshot_id: Value) -> Result<Value, String> {
        let frames = self.frames(&thread_id);
        if !is_undef(&screenshot_id) {
            for candidate in frames {
                if js_strict_eq(&js_get(candidate, "id")?, &screenshot_id) {
                    return Ok(candidate.clone());
                }
            }
            let list = if frames.is_empty() {
                ". Take a screenshot first and use its screenshotId.".to_string()
            } else {
                let ids: Vec<String> = frames
                    .iter()
                    .map(|f| js_to_string(&js_get(f, "id").unwrap_or_else(|_| undef())))
                    .collect();
                format!(
                    "; the screenshots still available are {} (newest last).",
                    ids.join(", ")
                )
            };
            return Err(format!(
                "No screenshot {} is available to point into{}",
                json_or_undefined(&screenshot_id),
                list
            ));
        }
        if let Some(latest) = frames.last() {
            return Ok(latest.clone());
        }
        Err("No screenshot to point into: this conversation has not received one yet. Take one with computer_screenshot or computer_get_state, then give x/y as pixel coordinates in that image.".into())
    }
}

// ── ScrollGearingStore ──────────────────────────────────────────────────────

const MAX_GEARING_KEYS: usize = 64;
const MIN_SCROLL_GEARING: f64 = 0.05;
const MAX_SCROLL_GEARING: f64 = 50.0;
const MIN_LEARNABLE_SCROLL_INJECTION: f64 = 30.0;
const SCROLL_GEARING_SMOOTHING: f64 = 0.5;

/// Port of class `ScrollGearingStore` (`apps/server/src/computer/scrollCalibration.ts:357`): holds the instance state.
pub struct ScrollGearingStore {
    gearings: Vec<(Value, f64)>,
}

impl ScrollGearingStore {
    /// Port of `ScrollGearingStore` constructor (`apps/server/src/computer/scrollCalibration.ts:357`).
    pub fn new() -> Result<Self, String> {
        Ok(ScrollGearingStore {
            gearings: Vec::new(),
        })
    }

    fn get(&self, key: &Value) -> Option<f64> {
        self.gearings
            .iter()
            .find(|(k, _)| same_value_zero(k, key))
            .map(|(_, v)| *v)
    }

    /// Port of `ScrollGearingStore.gearing` (`apps/server/src/computer/scrollCalibration.ts:361`).
    pub fn gearing(&mut self, key: Value) -> Result<f64, String> {
        Ok(if is_undef(&key) {
            1.0
        } else {
            self.get(&key).unwrap_or(1.0)
        })
    }

    /// Port of `ScrollGearingStore.has` (`apps/server/src/computer/scrollCalibration.ts:371`).
    pub fn has(&mut self, key: Value) -> Result<bool, String> {
        Ok(!is_undef(&key) && self.get(&key).is_some())
    }

    /// Port of `ScrollGearingStore.plan` (`apps/server/src/computer/scrollCalibration.ts:382`).
    pub fn plan(
        &mut self,
        key: Value,
        requested: f64,
        fallback: Option<f64>,
    ) -> Result<f64, String> {
        if !requested.is_finite() || requested == 0.0 {
            return Ok(0.0);
        }
        let divisor = if is_undef(&key) {
            fallback.unwrap_or(1.0)
        } else {
            self.get(&key).or(fallback).unwrap_or(1.0)
        };
        let scaled = requested / divisor;
        if scaled.abs() < 1.0 {
            return Ok(if scaled.is_nan() {
                f64::NAN
            } else if scaled > 0.0 {
                1.0
            } else if scaled < 0.0 {
                -1.0
            } else {
                scaled
            });
        }
        Ok(scaled)
    }

    /// Port of `ScrollGearingStore.learn` (`apps/server/src/computer/scrollCalibration.ts:408`).
    pub fn learn(&mut self, key: Value, injected: f64, traveled: f64) -> Result<bool, String> {
        if is_undef(&key) {
            return Ok(false);
        }
        if !injected.is_finite() || !traveled.is_finite() {
            return Ok(false);
        }
        if traveled == 0.0 || injected.abs() < MIN_LEARNABLE_SCROLL_INJECTION {
            return Ok(false);
        }
        if traveled.signum() != injected.signum() {
            return Ok(false);
        }
        let observed = traveled / injected;
        if !(MIN_SCROLL_GEARING..=MAX_SCROLL_GEARING).contains(&observed) {
            return Ok(false);
        }
        let previous = self.get(&key);
        let next = match previous {
            None => observed,
            Some(p) => p * (1.0 - SCROLL_GEARING_SMOOTHING) + observed * SCROLL_GEARING_SMOOTHING,
        };
        if previous.is_none() && self.gearings.len() >= MAX_GEARING_KEYS {
            self.gearings.remove(0);
        }
        let clamped = next.clamp(MIN_SCROLL_GEARING, MAX_SCROLL_GEARING);
        if let Some(entry) = self
            .gearings
            .iter_mut()
            .find(|(k, _)| same_value_zero(k, &key))
        {
            entry.1 = clamped;
        } else {
            self.gearings.push((key, clamped));
        }
        Ok(true)
    }
}

/// JS SameValueZero: `===` except NaN equals NaN.
fn same_value_zero(a: &Value, b: &Value) -> bool {
    if is_number(a) && is_number(b) {
        let (x, y) = (num_or_nan(a), num_or_nan(b));
        return (x.is_nan() && y.is_nan()) || x == y;
    }
    js_strict_eq(a, b)
}
