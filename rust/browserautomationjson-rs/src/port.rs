//! Clean-room port of `packages/contracts/src/browserAutomationJson.ts`,
//! written from recorded behavior only.
use serde_json::Value;

const MAX_DEPTH: f64 = 21.0;

/// Recreated `Reflect.ownKeys` order: array-index keys ascending first,
/// then the rest in insertion order.
fn own_keys(map: &serde_json::Map<String, Value>) -> Vec<&String> {
    let mut ints: Vec<(u64, &String)> = Vec::new();
    let mut rest: Vec<&String> = Vec::new();
    for k in map.keys() {
        match array_index(k) {
            Some(n) => ints.push((n, k)),
            None => rest.push(k),
        }
    }
    ints.sort_by_key(|(n, _)| *n);
    ints.into_iter().map(|(_, k)| k).chain(rest).collect()
}

/// A JS "array index" key: canonical integer in `[0, 2^32-1)`.
fn array_index(k: &str) -> Option<u64> {
    if k.is_empty() || k.len() > 10 || !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u64 = k.parse().ok()?;
    (n < 4_294_967_295 && n.to_string() == k).then_some(n)
}

/// Enumerable own values of the revived value — what `Object.values` yields.
fn children(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(items) => items.clone(),
        Value::Object(map) => {
            if crate::js::is_undefined(v) {
                return Vec::new();
            }
            if marker(v, "$utf16").is_some() {
                return Vec::new(); // $utf16 string
            }
            if let Some(m) = marker(v, "$num")
                .or_else(|| marker(v, "$bigint"))
                .or_else(|| marker(v, "$date"))
            {
                let _ = m;
                return Vec::new(); // number, bigint, Date: leaves
            }
            if let Some(b) = marker(v, "$bytes").or_else(|| marker(v, "$buffer")) {
                // Typed array: enumerable index properties.
                return b.as_array().cloned().unwrap_or_default();
            }
            if marker(v, "$map").is_some() || marker(v, "$set").is_some() {
                return Vec::new(); // Map/Set have no enumerable own properties
            }
            own_keys(map).into_iter().map(|k| map[k].clone()).collect()
        }
        _ => Vec::new(),
    }
}

fn marker<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    v.as_object()
        .filter(|m| m.len() == 1)
        .and_then(|m| m.get(name))
}

fn depth_of(v: &Value, d: f64) -> f64 {
    if d >= MAX_DEPTH {
        return d;
    }
    children(v)
        .iter()
        .map(|c| depth_of(c, d + 1.0))
        .fold(d, f64::max)
}

/// Port of `browserJsonDepth`: measured JSON nesting on top of `depth`
/// (default 0), with recursion cut off at `MAX_DEPTH`.
pub fn browser_json_depth(value: Value, depth: Option<f64>) -> Result<f64, String> {
    Ok(depth_of(&value, depth.unwrap_or(0.0)))
}

/// `JSON.stringify` string escaping on UTF-16 units; lone surrogates
/// are escaped, paired ones emitted literally.
fn quote_units(units: &[u16], out: &mut String) {
    out.push('"');
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        match u {
            0x22 => out.push_str("\\\""),
            0x5C => out.push_str("\\\\"),
            0x08 => out.push_str("\\b"),
            0x09 => out.push_str("\\t"),
            0x0A => out.push_str("\\n"),
            0x0C => out.push_str("\\f"),
            0x0D => out.push_str("\\r"),
            0x00..=0x1F => out.push_str(&format!("\\u{:04x}", u)),
            0xD800..=0xDBFF if i + 1 < units.len() && (0xDC00..=0xDFFF).contains(&units[i + 1]) => {
                // Paired surrogate: emit the astral char literally.
                if let Ok(s) = String::from_utf16(&units[i..i + 2]) {
                    out.push_str(&s);
                }
                i += 1;
            }
            0xD800..=0xDFFF => out.push_str(&format!("\\u{:04x}", u)),
            _ => {
                if let Some(c) = char::from_u32(u as u32) {
                    out.push(c);
                }
            }
        }
        i += 1;
    }
    out.push('"');
}

fn quote(s: &str, out: &mut String) {
    quote_units(&s.encode_utf16().collect::<Vec<_>>(), out)
}

/// JS `JSON.stringify` over a revived wire value. `Ok(false)` = `undefined`
/// (dropped by objects, `null` in arrays); `Err(())` = a thrown TypeError.
fn stringify(v: &Value, out: &mut String) -> Result<bool, ()> {
    if crate::js::is_undefined(v) {
        return Ok(false);
    }
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&crate::js::number_to_string(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => quote(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if !stringify(item, out)? {
                    out.push_str("null");
                }
            }
            out.push(']');
        }
        Value::Object(map) => {
            if let Some(n) = marker(v, "$num").and_then(|m| m.as_str()) {
                out.push_str(match n {
                    "NaN" | "Infinity" | "-Infinity" => "null",
                    _ => "0", // -0
                });
                return Ok(true);
            }
            if let Some(units) = marker(v, "$utf16").and_then(|m| m.as_array()) {
                let units: Vec<u16> = units
                    .iter()
                    .filter_map(|u| u.as_u64().map(|x| x as u16))
                    .collect();
                quote_units(&units, out);
                return Ok(true);
            }
            if marker(v, "$bigint").is_some() {
                return Err(()); // TypeError: Do not know how to serialize a BigInt
            }
            if let Some(d) = marker(v, "$date").and_then(|m| m.as_str()) {
                if d == "Invalid" {
                    out.push_str("null")
                } else {
                    quote(d, out)
                }
                return Ok(true);
            }
            if marker(v, "$map").is_some() || marker(v, "$set").is_some() {
                out.push_str("{}");
                return Ok(true);
            }
            if let Some(b) = marker(v, "$bytes").and_then(|m| m.as_array()) {
                let obj: serde_json::Map<String, Value> = b
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (i.to_string(), x.clone()))
                    .collect();
                return stringify(&Value::Object(obj), out);
            }
            if let Some(b) = marker(v, "$buffer").and_then(|m| m.as_array()) {
                let mut obj = serde_json::Map::new();
                obj.insert("type".into(), Value::String("Buffer".into()));
                obj.insert("data".into(), Value::Array(b.clone()));
                return stringify(&Value::Object(obj), out);
            }
            out.push('{');
            let mut first = true;
            for k in own_keys(map) {
                let mut buf = String::new();
                if !stringify(&map[k], &mut buf)? {
                    continue; // undefined-valued property is dropped
                }
                if !first {
                    out.push(',');
                }
                first = false;
                quote(k, out);
                out.push(':');
                out.push_str(&buf);
            }
            out.push('}');
        }
    }
    Ok(true)
}

/// Port of `browserJsonBytes`: UTF-8 byte length of `JSON.stringify(value)`;
/// `undefined`-serializing inputs measure 0, a serialization throw yields
/// `Infinity`.
pub fn browser_json_bytes(value: Value) -> Result<f64, String> {
    let mut out = String::new();
    Ok(match stringify(&value, &mut out) {
        Err(()) => f64::INFINITY,
        Ok(false) => 0.0,
        Ok(true) => out.len() as f64,
    })
}
