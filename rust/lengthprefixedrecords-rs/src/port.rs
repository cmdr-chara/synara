//! The Rust port of `packages/shared/src/lengthPrefixedRecords.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const DEFAULT_MAX_RECORD_BYTES: f64 = 8.0 * 1024.0 * 1024.0;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn marker_bytes(v: &Value) -> Option<Vec<u8>> {
    let o = v.as_object()?;
    if o.len() != 1 {
        return None;
    }
    let arr = o.get("$bytes").or_else(|| o.get("$buffer"))?.as_array()?;
    Some(arr.iter().map(js_to_uint8).collect())
}

/// JS `ToNumber`, markers included (`$num`, `$utf16`, `$undefined`, `$bigint` → error string).
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
        Value::String(s) => js_parse_number(s),
        Value::Array(a) => {
            js_parse_number(&a.iter().map(js_to_string).collect::<Vec<_>>().join(","))
        }
        _ if is_undefined(v) => f64::NAN,
        _ if crate::js::as_number(v).is_some() => crate::js::as_number(v).unwrap_or(f64::NAN),
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => crate::js::string_units(v)
            .and_then(|u| String::from_utf16(&u).ok())
            .map(|s| js_parse_number(&s))
            .unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// JS `ToString`, markers included.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if matches!(x, Value::Null) || is_undefined(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ if is_undefined(v) => "undefined".to_string(),
        _ if crate::js::as_number(v).is_some() => {
            crate::js::number_to_string(crate::js::as_number(v).unwrap_or(f64::NAN))
        }
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => crate::js::string_units(v)
            .and_then(|u| String::from_utf16(&u).ok())
            .unwrap_or_default(),
        Value::Object(o) if o.len() == 1 && o.contains_key("$bigint") => {
            o["$bigint"].as_str().unwrap_or("0").to_string() + "n"
        }
        _ => "[object Object]".to_string(),
    }
}

/// JS `String -> Number` (the `+x`/`Number(x)` coercion).
fn js_parse_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if t.is_empty() {
        return 0.0;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(oct) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return u64::from_str_radix(oct, 8)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(bin) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return u64::from_str_radix(bin, 2)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    match t {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => t.parse::<f64>().unwrap_or(f64::NAN),
    }
}

/// `ToUint8`/`value & 0xff` on a JSON value (array elements of `Buffer.from` / `Uint8Array.from`).
fn js_to_uint8(v: &Value) -> u8 {
    let n = js_to_number(v);
    if !n.is_finite() {
        return 0;
    }
    let m = n.trunc() % 256.0;
    if m < 0.0 { (m + 256.0) as u8 } else { m as u8 }
}

/// `util.inspect`-style integer rendering: digits grouped by three with underscores.
fn inspect_num(n: f64) -> String {
    if n.is_finite() && n.fract() == 0.0 && n.abs() >= 10_000.0 {
        let neg = n < 0.0;
        let digits = format!("{}", n.abs() as u128);
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push('_');
            }
            out.push(c);
        }
        if neg { format!("-{out}") } else { out }
    } else {
        crate::js::number_to_string(n)
    }
}

/// The class name a `Buffer.from`-rejecting value reports ("an instance of X") or
/// ("type X (...)") for primitives.
fn buffer_arg_error(v: &Value) -> String {
    const HEAD: &str = "The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object. Received ";
    if let Some(n) = crate::js::as_number(v) {
        return format!("{HEAD}type number ({})", inspect_num(n));
    }
    match v {
        Value::Number(n) => format!(
            "{HEAD}type number ({})",
            inspect_num(n.as_f64().unwrap_or(f64::NAN))
        ),
        Value::Bool(b) => format!("{HEAD}type boolean ({b})"),
        Value::Object(o) if o.len() == 1 => {
            let cls = if o.contains_key("$bigint") {
                return format!(
                    "{HEAD}type bigint ({}n)",
                    o["$bigint"].as_str().unwrap_or("0")
                );
            } else if o.contains_key("$date") {
                "Date"
            } else if o.contains_key("$map") {
                "Map"
            } else if o.contains_key("$set") {
                "Set"
            } else {
                "Object"
            };
            format!("{HEAD}an instance of {cls}")
        }
        _ => format!("{HEAD}an instance of Object"),
    }
}

/// `Buffer.from(chunk)` byte materialization for `ByteAccumulator.append`.
fn buffer_from(v: &Value) -> Result<Vec<u8>, String> {
    if let Some(bytes) = marker_bytes(v) {
        return Ok(bytes);
    }
    match v {
        Value::String(s) => Ok(s.as_bytes().to_vec()),
        Value::Array(a) => Ok(a.iter().map(js_to_uint8).collect()),
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => {
            Ok(crate::js::string_units(v)
                .map(|u| String::from_utf16_lossy(&u).into_bytes())
                .unwrap_or_default())
        }
        _ => Err(buffer_arg_error(v)),
    }
}

fn byte_length_of(v: &Value) -> Option<f64> {
    match v {
        Value::Object(o) => {
            if o.len() == 1 {
                if o.contains_key("$bytes") || o.contains_key("$buffer") {
                    return o
                        .values()
                        .next()
                        .and_then(|a| a.as_array())
                        .map(|a| a.len() as f64);
                }
                return None;
            }
            o.get("byteLength").map(js_to_number)
        }
        _ => None,
    }
}

/// Port of `encodeLengthPrefixedRecord` (`packages/shared/src/lengthPrefixedRecords.ts:62`).
pub fn encode_length_prefixed_record(
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let byte_length: Option<Value> = match &payload {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'byteLength')".to_string());
        }
        _ if is_undefined(&payload) => {
            return Err("Cannot read properties of undefined (reading 'byteLength')".to_string());
        }
        Value::Object(o) => {
            if o.len() == 1 && (o.contains_key("$bytes") || o.contains_key("$buffer")) {
                // Uint8Array / Buffer: `byteLength` is the element count.
                let n = o
                    .values()
                    .next()
                    .and_then(|a| a.as_array())
                    .map_or(0, |a| a.len());
                Some(Value::from(n as u64))
            } else {
                o.get("byteLength").cloned()
            }
        }
        _ => None,
    };
    let bl = byte_length.unwrap_or_else(crate::js::undefined);

    // `4 + payload.byteLength` with JS `+` semantics (string concat on a string operand).
    let total: f64 = match &bl {
        Value::String(s) => {
            return Err(format!(
                "The \"size\" argument must be of type number. Received type string ('4{s}')"
            ));
        }
        _ if is_undefined(&bl) => f64::NAN,
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => {
            return Err(format!(
                "The \"size\" argument must be of type number. Received type string ('4{}')",
                crate::js::string_units(&bl)
                    .map(|u| String::from_utf16_lossy(&u))
                    .unwrap_or_default()
            ));
        }
        Value::Object(o) if o.len() == 1 && o.contains_key("$bigint") => {
            return Err("Cannot mix BigInt and other types, use explicit conversions".to_string());
        }
        Value::Object(o)
            if !(o.len() == 1 && (o.contains_key("$num") || o.contains_key("$undefined"))) =>
        {
            return Err(
                "The \"size\" argument must be of type number. Received type string ('4[object Object]')"
                    .to_string(),
            );
        }
        Value::Array(a) => {
            return Err(format!(
                "The \"size\" argument must be of type number. Received type string ('4{}')",
                a.iter()
                    .map(|x| {
                        if matches!(x, Value::Null) || is_undefined(x) {
                            String::new()
                        } else {
                            js_to_string(x)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        _ => 4.0 + js_to_number(&bl),
    };

    if !total.is_finite() || total.fract() != 0.0 {
        return Err(format!(
            "The value of \"size\" is out of range. It must be >= 0 && <= 9007199254740991. Received {}",
            inspect_num(total)
        ));
    }
    if !(0.0..=4294967296.0).contains(&total) {
        return Err(format!(
            "The value of \"size\" is out of range. It must be >= 0 && <= 4294967296. Received {}",
            inspect_num(total)
        ));
    }
    let n = js_to_number(&bl);
    if !n.is_finite() || n.fract() != 0.0 || !(0.0..=4294967295.0).contains(&n) {
        return Err(format!(
            "The value of \"value\" is out of range. It must be >= 0 and <= 4294967295. Received {}",
            inspect_num(n)
        ));
    }

    let payload_bytes: Vec<u8> = if let Some(b) = marker_bytes(&payload) {
        b
    } else {
        // `record.set(payload, 4)` on an array-like: element `length` slots, ToNumber each.
        let src_len = match &payload {
            Value::Object(o) => o.get("length").map(js_to_number).unwrap_or(0.0),
            _ => 0.0,
        };
        let take = if src_len.is_finite() && src_len > 0.0 {
            src_len.trunc() as usize
        } else {
            0
        };
        let mut out = Vec::with_capacity(take);
        if let Value::Object(o) = &payload {
            for i in 0..take {
                out.push(js_to_uint8(
                    o.get(i.to_string().as_str()).unwrap_or(&Value::Null),
                ));
            }
        }
        out
    };

    let mut record = Vec::with_capacity(4 + n as usize);
    record.extend_from_slice(&(n as u32).to_le_bytes());
    record.extend_from_slice(&payload_bytes);
    record.resize(4 + n as usize, 0);
    Ok(json!({ "$buffer": record }))
}

/// Port of class `LengthPrefixedRecordParser` (`packages/shared/src/lengthPrefixedRecords.ts:38`): holds the instance state.
pub struct LengthPrefixedRecordParser {
    pending: Vec<u8>,
    max_record_bytes: Value,
}

impl LengthPrefixedRecordParser {
    /// Port of `LengthPrefixedRecordParser` constructor (`packages/shared/src/lengthPrefixedRecords.ts:38`).
    pub fn new(max_record_bytes: serde_json::Value) -> Result<Self, String> {
        Ok(Self {
            pending: Vec::new(),
            max_record_bytes: if is_undefined(&max_record_bytes) {
                json!(DEFAULT_MAX_RECORD_BYTES)
            } else {
                max_record_bytes
            },
        })
    }

    /// Port of `LengthPrefixedRecordParser.push` (`packages/shared/src/lengthPrefixedRecords.ts:44`).
    pub fn push(&mut self, chunk: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
        // ByteAccumulator.append: `chunk.byteLength` first, then `Buffer.from`.
        match &chunk {
            Value::Null => {
                return Err("Cannot read properties of null (reading 'byteLength')".to_string());
            }
            _ if is_undefined(&chunk) => {
                return Err(
                    "Cannot read properties of undefined (reading 'byteLength')".to_string()
                );
            }
            _ => {}
        }
        if byte_length_of(&chunk) != Some(0.0) {
            let bytes = buffer_from(&chunk)?;
            if !bytes.is_empty() {
                self.pending.extend_from_slice(&bytes);
            }
        }

        let mut payloads = Vec::new();
        while self.pending.len() >= 4 {
            let length = u32::from_le_bytes(self.pending[..4].try_into().unwrap_or_default());
            if js_gt(length as f64, &self.max_record_bytes)? {
                return Err(format!(
                    "Length-prefixed record claims {} bytes, past the {} byte limit",
                    length,
                    js_to_string(&self.max_record_bytes)
                ));
            }
            if self.pending.len() < 4 + length as usize {
                break;
            }
            self.pending.drain(..4);
            let payload: Vec<u8> = self.pending.drain(..length as usize).collect();
            payloads.push(json!({ "$buffer": payload }));
        }
        Ok(payloads)
    }
}

/// JS `>` on `(u32 length, max)` — ToNumber coercion for the right operand.
fn js_gt(len: f64, max: &Value) -> Result<bool, String> {
    if matches!(max, Value::Object(o) if o.len() == 1 && o.contains_key("$bigint")) {
        return Err("Cannot convert a BigInt value to a number".to_string());
    }
    Ok(len > js_to_number(max))
}
