//! The Rust port of `packages/shared/src/outboundHttp.ts` (bridged slice).
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) if !js::is_undefined(v) => o.get(key),
        _ => None,
    }
}

fn body_bytes(v: &Value) -> Result<Vec<u8>, String> {
    match prop(v, "$bytes").or_else(|| prop(v, "$buffer")) {
        Some(Value::Array(a)) => Ok(a
            .iter()
            .map(|x| x.as_u64().unwrap_or(0) as u8)
            .collect()),
        _ => Err(
            "The \"list\" argument must be an instance of SharedArrayBuffer, ArrayBuffer or ArrayBufferView."
                .to_string(),
        ),
    }
}

fn random_uuid() -> String {
    let mut bytes = [0u8; 16];
    // `/dev/urandom` keeps this dependency-free; a failed read falls back to a
    // time-seeded mix so the shape stays a valid v4 UUID either way.
    let filled = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut bytes))
        .is_ok();
    if !filled {
        let mut x = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9e3779b97f4a7c15);
        for b in &mut bytes {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *b = x as u8;
        }
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

fn is_safe_integer(v: &Value) -> bool {
    js::as_number(v)
        .is_some_and(|n| n.is_finite() && n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0)
}

// INFALLIBLE: NaN is falsy in JS, so `x != 0.0` would be wrong here.
#[allow(clippy::float_cmp, clippy::double_comparisons)]
fn truthy(v: Option<&Value>) -> bool {
    match v {
        None => false,
        Some(v) if js::is_undefined(v) => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x > 0.0 || x < 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Object(o)) if o.get("$num").is_some() => {
            js::as_number(v.unwrap()).is_some_and(|x| x > 0.0 || x < 0.0)
        }
        Some(_) => true,
    }
}

fn quote_multipart_token(value: &Value, label: &str) -> Result<String, String> {
    if !truthy(Some(value)) {
        return Err(format!("Multipart {label} is invalid."));
    }
    let s = value
        .as_str()
        .ok_or_else(|| format!("Multipart {label} is invalid."))?;
    if s.contains('\r') || s.contains('\n') {
        return Err(format!("Multipart {label} is invalid."));
    }
    Ok(s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Port of `encodeOutboundMultipart` (`packages/shared/src/outboundHttp.ts:112`).
pub fn encode_outbound_multipart(parts: Vec<Value>, options: Value) -> Result<Value, String> {
    let max_bytes = match prop(&options, "maxBytes") {
        Some(v) if is_safe_integer(v) => js::as_number(v).unwrap_or(0.0),
        _ => {
            return Err("Multipart byte limit must be a positive integer.".to_string());
        }
    };
    if max_bytes <= 0.0 {
        return Err("Multipart byte limit must be a positive integer.".to_string());
    }
    let list = &parts;
    let boundary = format!("Synara-{}", random_uuid());
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut size: f64 = 0.0;
    let mut push = |chunk: &[u8]| -> Result<(), String> {
        if (chunk.len() as f64) > max_bytes - size {
            return Err(format!(
                "Multipart request exceeded the {}-byte limit.",
                js::number_to_string(max_bytes)
            ));
        }
        chunks.push(chunk.to_vec());
        size += chunk.len() as f64;
        Ok(())
    };

    for part in list {
        let name = quote_multipart_token(prop(part, "name").unwrap_or(&Value::Null), "field name")?;
        let mut disposition = format!("form-data; name=\"{name}\"");
        if truthy(prop(part, "filename")) {
            disposition.push_str(&format!(
                "; filename=\"{}\"",
                quote_multipart_token(prop(part, "filename").unwrap_or(&Value::Null), "filename")?
            ));
        }
        let mut header = format!("--{boundary}\r\nContent-Disposition: {disposition}\r\n");
        if truthy(prop(part, "contentType")) {
            let ct = prop(part, "contentType").unwrap_or(&Value::Null);
            let ct_str = ct
                .as_str()
                .ok_or_else(|| "Multipart content type is invalid.".to_string())?;
            if ct_str.trim().is_empty() || ct_str.contains('\r') || ct_str.contains('\n') {
                return Err("Multipart content type is invalid.".to_string());
            }
            header.push_str(&format!("Content-Type: {ct_str}\r\n"));
        }
        header.push_str("\r\n");
        push(header.as_bytes())?;
        match prop(part, "body") {
            Some(Value::String(s)) => push(s.as_bytes())?,
            Some(b) => push(&body_bytes(b)?)?,
            None => push(&body_bytes(&Value::Null)?)?,
        }
        push(b"\r\n")?;
    }
    push(format!("--{boundary}--\r\n").as_bytes())?;

    let mut body = Vec::with_capacity(size as usize);
    for chunk in chunks {
        body.extend_from_slice(&chunk);
    }
    Ok(json!({
        "body": { "$bytes": body },
        "contentType": format!("multipart/form-data; boundary={boundary}"),
    }))
}

/// Port of `decodeOutboundText` (`packages/shared/src/outboundHttp.ts:631`).
pub fn decode_outbound_text(response: Value) -> Result<String, String> {
    if response.is_null() || js::is_undefined(&response) {
        return Err("Cannot read properties of null (reading 'body')".to_string());
    }
    match prop(&response, "body") {
        None => Ok(String::new()),
        Some(v) if js::is_undefined(v) => Ok(String::new()),
        Some(v) => Ok(String::from_utf8_lossy(&body_bytes(v)?).into_owned()),
    }
}

/// `assertJsonWithinLimits` from `./outboundHttpPolicy`.
fn assert_json_within_limits(value: &Value, limits: &Value) -> Result<(), String> {
    let max_depth = prop(limits, "maxDepth").and_then(js::as_number);
    let max_nodes = prop(limits, "maxNodes").and_then(js::as_number);
    let mut stack: Vec<(&Value, f64)> = vec![(value, 0.0)];
    let mut nodes: u64 = 0;
    while let Some((current, depth)) = stack.pop() {
        nodes += 1;
        if let Some(max) = max_nodes
            && nodes as f64 > max
        {
            return Err(format!(
                "Outbound JSON exceeded the {}-node limit.",
                js::number_to_string(max)
            ));
        }
        if let Some(max) = max_depth
            && depth > max
        {
            return Err(format!(
                "Outbound JSON exceeded the depth limit of {}.",
                js::number_to_string(max)
            ));
        }
        match current {
            Value::Array(a) => {
                for child in a {
                    stack.push((child, depth + 1.0));
                }
            }
            Value::Object(o) if !js::is_undefined(current) => {
                for child in o.values() {
                    stack.push((child, depth + 1.0));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Re-encode non-finite / negative-zero numbers as the wire's `$num` markers.
// INFALLIBLE: NaN is falsy in JS, so `f != 0.0` would be wrong here.
#[allow(clippy::float_cmp, clippy::double_comparisons)]
fn normalize_numbers(v: Value) -> Value {
    match v {
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.is_finite() && (f > 0.0 || f < 0.0 || !f.is_sign_negative()) => {
                Value::Number(n)
            }
            Some(f) => js::number(f),
            None => Value::Number(n),
        },
        Value::Array(a) => Value::Array(a.into_iter().map(normalize_numbers).collect()),
        Value::Object(o) => Value::Object(
            o.into_iter()
                .map(|(k, v)| (k, normalize_numbers(v)))
                .collect(),
        ),
        other => other,
    }
}

/// Port of `decodeOutboundJson` (`packages/shared/src/outboundHttp.ts:635`).
pub fn decode_outbound_json(response: Value, limits: Value) -> Result<Value, String> {
    let text = decode_outbound_text(response)
        .map_err(|_| "Outbound response was not valid JSON.".to_string())?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|_| "Outbound response was not valid JSON.".to_string())?;
    assert_json_within_limits(&value, &limits)?;
    Ok(normalize_numbers(value))
}
