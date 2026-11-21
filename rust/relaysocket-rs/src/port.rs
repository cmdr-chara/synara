//! The Rust port of `apps/server/src/relaySocket.ts`.
//!
//! `data` reaches the port as a JSON value: `RawData`'s `Buffer` crosses as
//! `{"$buffer": [..]}` (revived by `Buffer.from(v)`), a `Uint8Array` as
//! `{"$bytes": [..]}` (`Uint8Array.from(v)`), and other JS values carry their
//! usual markers (`$utf16`, `$num`, `$bigint`, `$date`, `$map`, `$set`,
//! `$undefined`).

#![forbid(unsafe_code)]

use crate::js::{number_to_string, string_units, utf16_string};
use serde_json::{Value, json};

const FROM_ARG: &str = "The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object. Received ";
const NULL_TOSTRING: &str = "Cannot read properties of null (reading 'toString')";
const UNDEF_TOSTRING: &str = "Cannot read properties of undefined (reading 'toString')";
const NOT_ITERABLE: &str =
    "object null is not iterable (cannot read property Symbol(Symbol.iterator))";
const BIGINT_TO_NUMBER: &str = "Cannot convert a BigInt value to a number";

/// Port of `normalizeRelayFrame` (`apps/server/src/relaySocket.ts:22`).
pub fn normalize_relay_frame(
    data: serde_json::Value,
    binary: bool,
) -> Result<serde_json::Value, String> {
    if !binary {
        // `data.toString()` on every kind of RawData input.
        return Ok(utf16_string(&to_units(&data)?));
    }
    // `Buffer.isBuffer(data)`: the `{"$buffer": v}` marker, revived by
    // `Buffer.from(v)`, so the echo is the coerced bytes, not the raw payload.
    if let Some(p) = marker(&data, "$buffer") {
        return Ok(buffer_json(buffer_from(p)?));
    }
    if let Value::Array(list) = &data {
        return Ok(buffer_json(concat(list)?));
    }
    Ok(buffer_json(buffer_from(&data)?))
}

/// A single-key marker object (`{"$buffer": v}` and friends), as `index.cjs`
/// writes them.
fn marker<'v>(v: &'v Value, key: &str) -> Option<&'v Value> {
    v.as_object()
        .filter(|o| o.len() == 1)
        .and_then(|o| o.get(key))
}

fn buffer_json(bytes: Vec<u8>) -> Value {
    json!({ "$buffer": bytes })
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// `data.toString()` as UTF-16 units: the result can keep lone surrogates that
/// arrived in `{"$utf16": ...}` strings.
fn to_units(v: &Value) -> Result<Vec<u16>, String> {
    match v {
        Value::Null => Err(NULL_TOSTRING.into()),
        Value::Bool(b) => Ok(utf16(if *b { "true" } else { "false" })),
        Value::Number(n) => Ok(utf16(&number_to_string(n.as_f64().unwrap_or(f64::NAN)))),
        Value::String(_) => Ok(string_units(v).unwrap_or_default()),
        Value::Array(items) => {
            let mut out = Vec::new();
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push(u16::from(b','));
                }
                // Array.prototype.join substitutes "" for null and undefined.
                if x.is_null() || marker(x, "$undefined").is_some() {
                    continue;
                }
                out.extend(to_units(x)?);
            }
            Ok(out)
        }
        Value::Object(_) => {
            if let Some(u) = string_units(v) {
                return Ok(u);
            }
            if marker(v, "$undefined").is_some() {
                return Err(UNDEF_TOSTRING.into());
            }
            if let Some(p) = marker(v, "$num") {
                return Ok(utf16(&number_to_string(marker_num(p)?)));
            }
            if let Some(p) = marker(v, "$bigint") {
                return Ok(utf16(&bigint_str(p)));
            }
            if let Some(p) = marker(v, "$buffer") {
                return Ok(String::from_utf8_lossy(&buffer_from(p)?)
                    .encode_utf16()
                    .collect());
            }
            if let Some(p) = marker(v, "$bytes") {
                return Ok(utf16(&u8_list(p)?));
            }
            if let Some(p) = marker(v, "$date") {
                return Ok(utf16(&date_string(p)));
            }
            Ok(utf16(match v {
                _ if marker(v, "$map").is_some() => "[object Map]",
                _ if marker(v, "$set").is_some() => "[object Set]",
                _ => "[object Object]",
            }))
        }
    }
}

/// `Buffer.concat(list)`: each item must be a Buffer (`$buffer`) or a
/// Uint8Array (`$bytes`); a bad marker payload fails while `index.cjs` revives
/// it, so its `Buffer.from`/`Uint8Array.from` error replaces the `list[i]` one.
fn concat(list: &[Value]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    for (i, x) in list.iter().enumerate() {
        let bytes = if let Some(p) = marker(x, "$buffer") {
            buffer_from(p)?
        } else if let Some(p) = marker(x, "$bytes") {
            u8_array_from(p)?
        } else {
            return Err(format!(
                "The \"list[{i}]\" argument must be an instance of Buffer or Uint8Array. Received {}",
                received(x)
            ));
        };
        out.extend_from_slice(&bytes);
    }
    Ok(out)
}

/// `Buffer.from(v)` on the JS value a JSON argument stands for.
fn buffer_from(v: &Value) -> Result<Vec<u8>, String> {
    if let Some(u) = string_units(v) {
        return Ok(String::from_utf16_lossy(&u).into_bytes());
    }
    if let Value::Array(items) = v {
        return items.iter().map(to_u8).collect();
    }
    if let Value::Object(_) = v {
        if let Some(p) = marker(v, "$buffer") {
            return buffer_from(p);
        }
        if let Some(p) = marker(v, "$bytes") {
            return u8_array_from(p);
        }
        if let Some(p) = marker(v, "$date") {
            return Ok(date_string(p).into_bytes());
        }
    }
    Err(format!("{FROM_ARG}{}", received(v)))
}

/// `Uint8Array.from(v)`: strings and byte sequences are iterable; `null` and
/// `undefined` throw, and non-iterables read as array-likes.
fn u8_array_from(v: &Value) -> Result<Vec<u8>, String> {
    if let Some(u) = string_units(v) {
        return String::from_utf16_lossy(&u)
            .chars()
            .map(|c| to_u8(&Value::String(c.to_string())))
            .collect();
    }
    match v {
        Value::Array(items) => items.iter().map(to_u8).collect(),
        Value::Null => Err(NOT_ITERABLE.into()),
        Value::Object(_) => {
            if marker(v, "$undefined").is_some() {
                return Err(NOT_ITERABLE.into());
            }
            if let Some(p) = marker(v, "$buffer") {
                return buffer_from(p);
            }
            if let Some(p) = marker(v, "$map").or_else(|| marker(v, "$set")) {
                // Maps iterate [k, v] entries, Sets their values.
                return p
                    .as_array()
                    .map(|a| a.iter().map(to_u8).collect())
                    .unwrap_or_else(|| Ok(Vec::new()));
            }
            if let Some(p) = marker(v, "$bytes") {
                return u8_array_from(p);
            }
            // Non-iterable array-likes without a `length` read as empty.
            Ok(vec![
                0;
                v.get("length")
                    .and_then(Value::as_u64)
                    .map_or(0, |n| n.min(1 << 24) as usize)
            ])
        }
        _ => Ok(Vec::new()),
    }
}

/// `Uint8Array.prototype.toString()`: elements joined with `,`.
fn u8_list(v: &Value) -> Result<String, String> {
    Ok(u8_array_from(v)?
        .iter()
        .map(u8::to_string)
        .collect::<Vec<_>>()
        .join(","))
}

/// `ToUint8(ToNumber(v))`: truncate, then take the value mod 256.
fn to_u8(v: &Value) -> Result<u8, String> {
    let n = to_number(v)?;
    Ok(if n.is_finite() {
        n.trunc().rem_euclid(256.0) as u8
    } else {
        0
    })
}

/// JS `Number(v)`.
fn to_number(v: &Value) -> Result<f64, String> {
    Ok(match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_num(s),
        Value::Array(items) => js_num(&join_string(items)?),
        Value::Object(_) => {
            if marker(v, "$bigint").is_some() {
                return Err(BIGINT_TO_NUMBER.into());
            }
            if let Some(p) = marker(v, "$num") {
                marker_num(p)?
            } else if let Some(u) = string_units(v) {
                js_num(&String::from_utf16_lossy(&u))
            } else if let Some(p) = marker(v, "$buffer") {
                js_num(&String::from_utf8_lossy(&buffer_from(p)?))
            } else if let Some(p) = marker(v, "$bytes") {
                js_num(&u8_list(p)?)
            } else if let Some(p) = marker(v, "$date") {
                date_ms(p)
            } else {
                f64::NAN
            }
        }
    })
}

/// `v.join(",")` the way `Number(array)` needs it: element `toString`s joined,
/// null and undefined as empty strings.
fn join_string(items: &[Value]) -> Result<String, String> {
    let mut out = String::new();
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        if !x.is_null() && marker(x, "$undefined").is_none() {
            out.push_str(&String::from_utf16_lossy(&to_units(x)?));
        }
    }
    Ok(out)
}

/// `Number(str)`: whitespace-trimmed decimal, the `Infinity` spellings and the
/// `0x`/`0o`/`0b` integer forms; everything else is NaN.
fn js_num(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    match t {
        "" => return 0.0,
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (p, b) in [
        ("0x", 16),
        ("0o", 8),
        ("0b", 2),
        ("0X", 16),
        ("0O", 8),
        ("0B", 2),
    ] {
        if let Some(d) = t.strip_prefix(p) {
            return u64::from_str_radix(d, b).map_or(f64::NAN, |v| v as f64);
        }
    }
    // Rust's float parser accepts "inf"/"infinity"; JS only allows `e` exponents.
    if t.bytes()
        .any(|b| b.is_ascii_alphabetic() && !matches!(b, b'e' | b'E'))
    {
        return f64::NAN;
    }
    t.parse().unwrap_or(f64::NAN)
}

/// `Number(marker_payload)`: a `$num` payload names a special value, anything
/// else goes through `ToNumber`.
fn marker_num(p: &Value) -> Result<f64, String> {
    if let Value::String(s) = p {
        return Ok(match s.as_str() {
            "NaN" => f64::NAN,
            "Infinity" => f64::INFINITY,
            "-Infinity" => f64::NEG_INFINITY,
            "-0" => -0.0,
            _ => js_num(s),
        });
    }
    to_number(p)
}

/// `BigInt(payload).toString()`; the marker normally carries the digits.
fn bigint_str(p: &Value) -> String {
    p.as_str().map_or_else(
        || (to_number(p).unwrap_or(0.0).trunc() as i64).to_string(),
        String::from,
    )
}

/// `new Date(p).getTime()`: the marker holds a `toISOString` result or
/// `"Invalid"`.
fn date_ms(p: &Value) -> f64 {
    match p {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => iso_ms(s).unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// `new Date(p).toString()`. The real method uses the host timezone, which the
/// port cannot see; the UTC spelling matches hosts running with `TZ=UTC`.
fn date_string(p: &Value) -> String {
    const WDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let ms = date_ms(p);
    if !ms.is_finite() {
        return "Invalid Date".into();
    }
    let days = (ms / 86_400_000.0).floor() as i64;
    let tod = (ms % 86_400_000.0).rem_euclid(86_400_000.0) as i64;
    // The civil date a day count falls on (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let (y, doy) = (yoe + era * 400, doe - (365 * yoe + yoe / 4 - yoe / 100));
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (
        doy - (153 * mp + 2) / 5 + 1,
        if mp < 10 { mp + 3 } else { mp - 9 },
    );
    format!(
        "{} {} {d:02} {:04} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        WDAYS[(days + 4).rem_euclid(7) as usize],
        MONTHS[(m - 1) as usize],
        if m <= 2 { y + 1 } else { y },
        tod / 3_600_000,
        tod / 60_000 % 60,
        tod / 1_000 % 60,
    )
}

/// Milliseconds since the epoch for a `toISOString` timestamp
/// (`YYYY-MM-DDTHH:MM:SS(.sss)?Z`); anything else is an invalid date.
fn iso_ms(s: &str) -> Option<f64> {
    if s.len() < 20 || !s.ends_with('Z') {
        return None;
    }
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, se) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let millis = match s.get(20..s.len() - 1).unwrap_or("").strip_prefix('.') {
        Some(f) if !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()) => {
            format!("{f}00")[..3].parse().ok()?
        }
        Some(_) => return None,
        None => 0,
    };
    // Days since the epoch for the civil date (Howard Hinnant's algorithm).
    let y = if mo <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let days =
        era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * ((mo + 9) % 12) + 2) / 5 + d
            - 1
            - 719_468;
    Some((days * 86_400_000 + h * 3_600_000 + mi * 60_000 + se * 1_000 + millis) as f64)
}

/// `inspect(n)` inside an error message: `-0` keeps its sign, everything else
/// prints as `Number.prototype.toString` does.
fn inspect_num(n: f64) -> String {
    if n == 0.0 && n.is_sign_negative() {
        "-0".into()
    } else {
        number_to_string(n)
    }
}

/// The string form inside `Received type string (...)`: `'raw'` unless the
/// string holds a `'`, then `JSON.stringify`'s double-quoted escaping.
fn inspect_str(s: &str) -> String {
    if s.contains('\'') {
        serde_json::to_string(s).unwrap_or_default()
    } else {
        format!("'{s}'")
    }
}

/// The `Received ...` tail of Node's `ERR_INVALID_ARG_TYPE`.
fn received(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => format!(
            "type number ({})",
            inspect_num(n.as_f64().unwrap_or(f64::NAN))
        ),
        Value::String(s) => format!("type string ({})", inspect_str(s)),
        Value::Array(_) => "an instance of Array".into(),
        Value::Object(_) => {
            if marker(v, "$undefined").is_some() {
                "undefined".into()
            } else if let Some(p) = marker(v, "$num") {
                format!(
                    "type number ({})",
                    inspect_num(marker_num(p).unwrap_or(f64::NAN))
                )
            } else if let Some(p) = marker(v, "$bigint") {
                format!("type bigint ({}n)", bigint_str(p))
            } else if let Some(u) = string_units(v) {
                format!(
                    "type string ({})",
                    inspect_str(&String::from_utf16_lossy(&u))
                )
            } else {
                for (key, name) in [
                    ("$map", "Map"),
                    ("$set", "Set"),
                    ("$date", "Date"),
                    ("$buffer", "Buffer"),
                    ("$bytes", "Uint8Array"),
                ] {
                    if marker(v, key).is_some() {
                        return format!("an instance of {name}");
                    }
                }
                "an instance of Object".into()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn text_frames_to_string() {
        for (data, want) in [
            (json!("x"), "x"),
            (json!(0), "0"),
            (json!(true), "true"),
            (json!([]), ""),
            (json!(["a", 1]), "a,1"),
            (json!({}), "[object Object]"),
            (json!({"a": "b"}), "[object Object]"),
            (json!({"$bytes": [104, 105]}), "104,105"),
            (json!({"$buffer": [104, 105]}), "hi"),
            (json!([{"$buffer": [104]}, null]), "h,"),
        ] {
            assert_eq!(normalize_relay_frame(data, false), Ok(json!(want)));
        }
        assert_eq!(
            normalize_relay_frame(json!(null), false),
            Err(NULL_TOSTRING.to_string())
        );
        assert_eq!(
            normalize_relay_frame(json!({"$undefined": true}), false),
            Err(UNDEF_TOSTRING.to_string())
        );
    }

    #[test]
    fn binary_frames() {
        assert_eq!(
            normalize_relay_frame(json!("x"), true),
            Ok(json!({"$buffer": [120]}))
        );
        assert_eq!(
            normalize_relay_frame(json!([]), true),
            Ok(json!({"$buffer": []}))
        );
        assert_eq!(
            normalize_relay_frame(
                json!([{"$buffer": [65]}, {"$bytes": [66]}, {"$buffer": [300]}]),
                true
            ),
            Ok(json!({"$buffer": [65, 66, 44]}))
        );
    }

    #[test]
    fn binary_frame_errors() {
        for (data, want) in [
            (
                json!(["a", 1]),
                "The \"list[0]\" argument must be an instance of Buffer or Uint8Array. Received type string ('a')".to_string(),
            ),
            (json!({}), format!("{FROM_ARG}an instance of Object")),
            (json!(true), format!("{FROM_ARG}type boolean (true)")),
            (json!(null), format!("{FROM_ARG}null")),
            (json!(0), format!("{FROM_ARG}type number (0)")),
            (json!({"$undefined": true}), format!("{FROM_ARG}undefined")),
        ] {
            assert_eq!(normalize_relay_frame(data, true), Err(want));
        }
    }
}
