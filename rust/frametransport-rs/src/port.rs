//! The Rust port of `packages/shared/src/frameTransport.ts`.
//!
//! `FrameTransport` and `makeFrameSink` stay in TypeScript: they are class /
//! callback-shaped APIs that do not cross the bridge.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use crate::js;

const FRAME_HEADER_FIXED_BYTES: usize = 17;
const FRAME_MAX_STREAM_ID_BYTES: f64 = 255.0;
const FRAME_FLAG_KEYFRAME: u8 = 0b1;
const FRAME_FLAG_CODEC_CONFIG: u8 = 0b10;

fn bytes_of(value: &Value) -> Option<Vec<u8>> {
    value.get("$bytes")?.as_array().map(|a| {
        a.iter()
            .map(|x| x.as_u64().unwrap_or(0) as u8)
            .collect::<Vec<u8>>()
    })
}

fn bytes_value(bytes: Vec<u8>) -> Value {
    let mut m = Map::new();
    m.insert(
        "$bytes".to_string(),
        Value::Array(bytes.into_iter().map(Value::from).collect()),
    );
    Value::Object(m)
}

fn js_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        _ => jscore::js_to_string(value),
    }
}

fn decode_fail(reason: &str) -> Result<serde_json::Value, String> {
    let mut m = Map::new();
    m.insert("ok".to_string(), Value::Bool(false));
    m.insert("reason".to_string(), Value::String(reason.to_string()));
    Ok(Value::Object(m))
}

/// `Number(str)` — what `ToNumber` does to a string.
fn parse_js_number(s: &str) -> f64 {
    let t = s.trim_matches(jscore::is_space);
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    let (sign, rest) = if let Some(r) = t.strip_prefix('-') {
        (-1.0, r)
    } else if let Some(r) = t.strip_prefix('+') {
        (1.0, r)
    } else {
        (1.0, t)
    };
    if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(|v| sign * v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(b) = rest.strip_prefix("0b").or_else(|| rest.strip_prefix("0B")) {
        return u64::from_str_radix(b, 2)
            .map(|v| sign * v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(o) = rest.strip_prefix("0o").or_else(|| rest.strip_prefix("0O")) {
        return u64::from_str_radix(o, 8)
            .map(|v| sign * v as f64)
            .unwrap_or(f64::NAN);
    }
    rest.parse::<f64>().map(|v| sign * v).unwrap_or(f64::NAN)
}

/// JS `ToNumber(value)`: null → +0, undefined → NaN, bool → 0/1, string → parse,
/// object → NaN, array → Number(join(",")).
fn to_number(value: Option<&Value>) -> f64 {
    match value {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => *b as u8 as f64,
        Some(Value::String(s)) => parse_js_number(s),
        Some(Value::Array(a)) => match a.len() {
            0 => 0.0,
            1 => match a.first() {
                Some(Value::Null) | None => 0.0,
                Some(v) if js::is_undefined(v) => 0.0,
                Some(v) => parse_js_number(&jscore::js_to_string(v)),
            },
            _ => f64::NAN,
        },
        Some(v) => js::as_number(v).unwrap_or(f64::NAN),
    }
}

/// `ToUint32`/`ToIntegerOrInfinity`-mod-2^32 as written by `setUint32(v >>> 0)`.
fn to_uint32(value: Option<&Value>) -> u32 {
    let n = to_number(value);
    if !n.is_finite() {
        return 0;
    }
    let t = n.trunc();
    (t - (t / 4294967296.0).floor() * 4294967296.0) as u64 as u32
}

fn to_number_or_zero(value: Option<&Value>) -> f64 {
    to_number(value)
}

/// Port of `encodeFrameEnvelope` (`packages/shared/src/frameTransport.ts:43`).
pub fn encode_frame_envelope(
    config: serde_json::Value,
    frame: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let max_stream_id_bytes = config
        .get("maxStreamIdBytes")
        .filter(|v| !v.is_null() && !js::is_undefined(v))
        .map(|v| js::as_number(v).unwrap_or(f64::NAN))
        .unwrap_or(FRAME_MAX_STREAM_ID_BYTES);
    let frame_label = config
        .get("frameLabel")
        .map(js_str)
        .unwrap_or_else(|| jscore::js_to_string(&js::undefined()));
    let stream_id_label = config
        .get("streamIdLabel")
        .map(js_str)
        .unwrap_or_else(|| jscore::js_to_string(&js::undefined()));

    let header = frame.get("header");
    let header = match header {
        None => {
            return Err("Cannot read properties of undefined (reading 'streamId')".to_string());
        }
        Some(h) if h.is_null() => {
            return Err("Cannot read properties of null (reading 'streamId')".to_string());
        }
        Some(h) => h,
    };
    let stream_id = jscore::js_to_string(
        &header
            .get("streamId")
            .cloned()
            .unwrap_or_else(js::undefined),
    );
    let stream_id_bytes = stream_id.as_bytes();
    if stream_id_bytes.is_empty() {
        return Err(format!(
            "{frame_label} frame header requires a non-empty {stream_id_label}"
        ));
    }
    if (stream_id_bytes.len() as f64) > max_stream_id_bytes {
        return Err(format!(
            "{frame_label} frame {stream_id_label} exceeds {} UTF-8 bytes",
            js::number_to_string(max_stream_id_bytes)
        ));
    }

    let payload = frame.get("payload");
    let payload_len = match payload {
        Some(p) if p.is_null() => {
            return Err("Cannot read properties of null (reading 'byteLength')".to_string());
        }
        Some(p) => match bytes_of(p) {
            Some(b) => b.len() as f64,
            None => p
                .get("byteLength")
                .and_then(js::as_number)
                .unwrap_or(f64::NAN),
        },
        None => {
            return Err("Cannot read properties of undefined (reading 'byteLength')".to_string());
        }
    };
    let total = 17.0 + stream_id_bytes.len() as f64 + payload_len;
    if !total.is_finite() || total < 0.0 || total.fract() != 0.0 {
        if total.is_nan() {
            return Err("Offset is outside the bounds of the DataView".to_string());
        }
        return Err("Invalid array buffer length".to_string());
    }
    let total = total as usize;
    if total < FRAME_HEADER_FIXED_BYTES + stream_id_bytes.len() {
        return Err("Invalid array buffer length".to_string());
    }

    let mut bytes = vec![0u8; total];
    let mut flags = 0u8;
    if jscore::js_truthy(header.get("keyframe").unwrap_or(&Value::Null)) {
        flags |= FRAME_FLAG_KEYFRAME;
    }
    if jscore::js_truthy(header.get("codecConfig").unwrap_or(&Value::Null)) {
        flags |= FRAME_FLAG_CODEC_CONFIG;
    }
    let magic = to_number_or_zero(config.get("magic"));
    let magic_u16 = (magic as i64 & 0xffff) as u16;
    bytes[0..2].copy_from_slice(&magic_u16.to_le_bytes());
    bytes[2] = (to_number_or_zero(config.get("version")) as i64 & 0xff) as u8;
    bytes[3] = flags;
    bytes[4..8].copy_from_slice(&to_uint32(header.get("sequence")).to_le_bytes());
    let timestamp = to_number(header.get("timestampMs"));
    bytes[8..16].copy_from_slice(&timestamp.to_le_bytes());
    bytes[16] = stream_id_bytes.len() as u8;
    bytes[FRAME_HEADER_FIXED_BYTES..FRAME_HEADER_FIXED_BYTES + stream_id_bytes.len()]
        .copy_from_slice(stream_id_bytes);

    // `bytes.set(payload, offset)` — array-like `.length` drives the copy.
    let payload_offset = FRAME_HEADER_FIXED_BYTES + stream_id_bytes.len();
    if let Some(p) = payload {
        if let Some(b) = bytes_of(p) {
            if payload_offset + b.len() > total {
                return Err("offset is out of bounds".to_string());
            }
            bytes[payload_offset..payload_offset + b.len()].copy_from_slice(&b);
        } else if let Some(len) = p.get("length").and_then(js::as_number) {
            let len = len.max(0.0) as usize;
            if payload_offset + len > total {
                return Err("offset is out of bounds".to_string());
            }
            for i in 0..len {
                let v = to_number(p.get(i.to_string()));
                bytes[payload_offset + i] = (v as i64 & 0xff) as u8;
            }
        }
    }
    Ok(bytes_value(bytes))
}

/// Port of `decodeFrameEnvelope` (`packages/shared/src/frameTransport.ts:79`).
pub fn decode_frame_envelope(
    config: serde_json::Value,
    bytes: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let byte_length = match &bytes {
        Value::Object(_) => bytes_of(&bytes)
            .map(|b| b.len() as f64)
            .or_else(|| bytes.get("byteLength").and_then(js::as_number)),
        _ => None,
    };
    let data = match byte_length {
        Some(n) if n < FRAME_HEADER_FIXED_BYTES as f64 => {
            return decode_fail("too-short");
        }
        Some(_) => match bytes_of(&bytes) {
            Some(b) => b,
            None => {
                return Err(
                    "First argument to DataView constructor must be an ArrayBuffer".to_string(),
                );
            }
        },
        None => {
            return Err(
                "First argument to DataView constructor must be an ArrayBuffer".to_string(),
            );
        }
    };
    let magic = u16::from_le_bytes([data[0], data[1]]);
    if Some(magic as f64) != config.get("magic").and_then(js::as_number) {
        return decode_fail("bad-magic");
    }
    if Some(data[2] as f64) != config.get("version").and_then(js::as_number) {
        return decode_fail("unsupported-version");
    }
    let flags = data[3];
    let stream_id_length = data[16] as usize;
    let payload_offset = FRAME_HEADER_FIXED_BYTES + stream_id_length;
    if stream_id_length == 0 || data.len() < payload_offset {
        return decode_fail("truncated-stream-id");
    }
    let stream_id = match String::from_utf8(data[FRAME_HEADER_FIXED_BYTES..payload_offset].to_vec())
    {
        Ok(s) => s,
        Err(_) => return decode_fail("invalid-stream-id"),
    };
    let mut header = Map::new();
    header.insert("streamId".to_string(), Value::String(stream_id));
    header.insert(
        "sequence".to_string(),
        js::number(u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as f64),
    );
    header.insert(
        "timestampMs".to_string(),
        js::number(f64::from_le_bytes(
            data[8..16].try_into().map_err(|_| "decode".to_string())?,
        )),
    );
    header.insert(
        "keyframe".to_string(),
        Value::Bool(flags & FRAME_FLAG_KEYFRAME != 0),
    );
    header.insert(
        "codecConfig".to_string(),
        Value::Bool(flags & FRAME_FLAG_CODEC_CONFIG != 0),
    );
    let mut frame = Map::new();
    frame.insert("header".to_string(), Value::Object(header));
    frame.insert(
        "payload".to_string(),
        bytes_value(data[payload_offset..].to_vec()),
    );
    let mut m = Map::new();
    m.insert("ok".to_string(), Value::Bool(true));
    m.insert("frame".to_string(), Value::Object(frame));
    Ok(Value::Object(m))
}

/// Port of `peekFrameHeader` (`packages/shared/src/frameTransport.ts:124`).
pub fn peek_frame_header(
    config: serde_json::Value,
    bytes: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let result = decode_frame_envelope(config, bytes)?;
    if result.get("ok") == Some(&Value::Bool(true)) {
        Ok(result.get("frame").and_then(|f| f.get("header")).cloned())
    } else {
        Ok(None)
    }
}

/// Port of `classifyByFrameFlags` (`packages/shared/src/frameTransport.ts:153`).
pub fn classify_by_frame_flags(frame: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(frame)
}

/// Port of `decodeFrameResyncRequest` (`packages/shared/src/frameTransport.ts:423`).
pub fn decode_frame_resync_request(
    message: serde_json::Value,
    resync_message: String,
    max_bytes: serde_json::Value,
) -> Result<serde_json::Value, String> {
    const ERR_PREFIX: &str = "The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object.";
    let text = match &message {
        Value::String(s) => s.clone(),
        Value::Null => return Err(format!("{ERR_PREFIX} Received null")),
        Value::Bool(b) => return Err(format!("{ERR_PREFIX} Received type boolean ({b})")),
        Value::Number(_) => {
            let n = js::as_number(&message).unwrap_or(f64::NAN);
            return Err(format!(
                "{ERR_PREFIX} Received type number ({})",
                js::number_to_string(n)
            ));
        }
        Value::Object(_) if js::is_undefined(&message) => {
            return Err(format!("{ERR_PREFIX} Received undefined"));
        }
        Value::Object(_) => {
            if let Some(b) = bytes_of(&message) {
                String::from_utf8_lossy(&b).into_owned()
            } else if let Some(len) = message.get("length").and_then(js::as_number) {
                let len = len.max(0.0) as usize;
                let bytes: Vec<u8> = (0..len)
                    .map(|i| (to_number(message.get(i.to_string())) as i64 & 0xff) as u8)
                    .collect();
                String::from_utf8_lossy(&bytes).into_owned()
            } else {
                return Err(format!("{ERR_PREFIX} Received an instance of Object"));
            }
        }
        Value::Array(a) => {
            let bytes: Vec<u8> = a
                .iter()
                .map(|v| (to_number(Some(v)) as i64 & 0xff) as u8)
                .collect();
            String::from_utf8_lossy(&bytes).into_owned()
        }
    };
    let max_bytes = if js::is_undefined(&max_bytes) {
        1024.0
    } else {
        to_number(Some(&max_bytes))
    };
    let text_len = text.encode_utf16().count() as f64;
    if text_len > max_bytes {
        return Ok(Value::Null);
    }
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(p) => p,
        Err(_) => return Ok(Value::Null),
    };
    if parsed.is_object() && parsed.get("type") == Some(&Value::String(resync_message)) {
        return Ok(Value::String("resync".to_string()));
    }
    Ok(Value::Null)
}
