//! Clean-room port of `packages/contracts/src/hubWork.ts`,
//! written from recorded behavior only.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use crate::js::{is_undefined, number_to_string, undefined, utf16_string};

const FIELDS: [&str; 12] = [
    "id",
    "sourceThreadId",
    "sourceMessageId",
    "title",
    "workerThreadId",
    "state",
    "queueReason",
    "resultSummary",
    "progress",
    "revision",
    "createdAt",
    "updatedAt",
];

fn single<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object()
        .filter(|o| o.len() == 1)
        .and_then(|o| o.get(key))
}

fn single_marker(key: &str, v: Value) -> Value {
    let mut o = Map::new();
    o.insert(key.to_string(), v);
    Value::Object(o)
}

fn marker_units(v: &Value) -> Option<Vec<u16>> {
    v.as_array().map(|a| {
        a.iter()
            .filter_map(|n| n.as_f64().map(|x| (x as i64).rem_euclid(65536) as u16))
            .collect()
    })
}

fn byte_array(v: &Value) -> Option<Vec<u8>> {
    v.as_array().map(|a| {
        a.iter()
            .filter_map(|n| n.as_f64().map(|x| (x as i64).rem_euclid(256) as u8))
            .collect()
    })
}

/// ms since epoch -> `YYYY-MM-DDTHH:MM:SS.sssZ`, or `None` when not a finite
/// instant JS Date can represent (encoded as `"Invalid"`).
fn ms_to_iso(t: f64) -> Option<String> {
    if !t.is_finite() || t.abs() > 8.64e15 {
        return None;
    }
    let ms = t.trunc() as i64;
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    let hh = rem / 3_600_000;
    let mm = rem % 3_600_000 / 60_000;
    let ss = rem % 60_000 / 1000;
    let frac = rem % 1000;
    let year = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else {
        format!("{}{y:06}", if y < 0 { "-" } else { "+" })
    };
    Some(format!(
        "{year}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{frac:03}Z"
    ))
}

fn iso_to_ms(s: &str) -> Option<f64> {
    let (date, time) = s.split_once('T')?;
    let mut dp = date.splitn(4, '-');
    let y: i64 = dp.next()?.parse().ok()?;
    let m: i64 = dp.next()?.parse().ok()?;
    let d: i64 = dp.next()?.parse().ok()?;
    let (hhmmss, frac) = time.split_once('.').unwrap_or((time, ""));
    let frac = frac.trim_end_matches('Z');
    let ms: i64 = if frac.is_empty() {
        0
    } else {
        let mut f = frac.to_string();
        f.truncate(3);
        while f.len() < 3 {
            f.push('0');
        }
        f.parse().ok()?
    };
    let mut tp = hhmmss.splitn(3, ':');
    let hh: i64 = tp.next()?.parse().ok()?;
    let mm: i64 = tp.next()?.parse().ok()?;
    let ss: i64 = tp.next()?.trim_end_matches('Z').parse().ok()?;
    Some(
        (days_from_civil(y, m, d) * 86_400_000 + hh * 3_600_000 + mm * 60_000 + ss * 1000 + ms)
            as f64,
    )
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Canonicalize a structured value the way the JS revive + re-encode does.
fn normalize(v: &Value) -> Value {
    if let Some(u) = single(v, "$undefined") {
        let _ = u;
        return single_marker("$undefined", Value::Bool(true));
    }
    if let Some(u) = single(v, "$utf16")
        && let Some(units) = marker_units(u)
    {
        return utf16_string(&units);
    }
    if let Some(n) = single(v, "$num") {
        let x: f64 = n.as_str().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN);
        if x.is_finite() && !x.is_sign_negative() || x == 0.0 && x.is_sign_positive() {
            return serde_json::json!(x);
        }
        let tag = if x == 0.0 {
            "-0"
        } else if x.is_nan() {
            "NaN"
        } else if x > 0.0 {
            "Infinity"
        } else {
            "-Infinity"
        };
        return single_marker("$num", Value::String(tag.to_string()));
    }
    if let Some(b) = single(v, "$bigint") {
        return single_marker("$bigint", b.clone());
    }
    if let Some(d) = single(v, "$date") {
        let out = match d {
            Value::Number(n) => ms_to_iso(n.as_f64().unwrap_or(f64::NAN)),
            Value::String(s) if s == "Invalid" => None,
            Value::String(s) => iso_to_ms(s).and_then(ms_to_iso),
            _ => None,
        };
        return single_marker(
            "$date",
            Value::String(out.unwrap_or_else(|| "Invalid".to_string())),
        );
    }
    if let Some(b) = single(v, "$bytes")
        && let Some(bytes) = byte_array(b)
    {
        return single_marker("$bytes", serde_json::json!(bytes));
    }
    if let Some(b) = single(v, "$buffer")
        && let Some(bytes) = byte_array(b)
    {
        return single_marker("$buffer", serde_json::json!(bytes));
    }
    if let Some(m) = single(v, "$map")
        && let Some(pairs) = m.as_array()
    {
        let out: Vec<Value> = pairs
            .iter()
            .filter_map(|p| p.as_array())
            .map(|p| Value::Array(p.iter().map(normalize).collect::<Vec<Value>>()))
            .collect();
        return single_marker("$map", Value::Array(out));
    }
    if let Some(s) = single(v, "$set")
        && let Some(items) = s.as_array()
    {
        return single_marker("$set", Value::Array(items.iter().map(normalize).collect()));
    }
    match v {
        Value::Array(a) => Value::Array(a.iter().map(normalize).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, x)| (k.clone(), normalize(x))).collect())
        }
        _ => v.clone(),
    }
}

// ---- JS ToString on structured values, in UTF-16 units ----------------------------------------

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

const WD: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MO: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `new Date(ms).toString()` for the Pacific timezone the recordings were made in.
fn date_to_string_units(ms: f64) -> Vec<u16> {
    let Some(_iso) = ms_to_iso(ms) else {
        return units("Invalid Date");
    };
    // Pacific: PDT (UTC-7) from 2nd Sun Mar 02:00 local to 1st Sun Nov 02:00 local.
    let ms_i = ms.trunc() as i64;
    let (y_guess, _, _) = civil_from_days(ms_i.div_euclid(86_400_000));
    let nth_sun = |y: i64, mon: i64, n: i64| -> i64 {
        let z0 = days_from_civil(y, mon, 1);
        let w0 = (z0 + 4).rem_euclid(7); // 1970-01-01 was Thursday
        days_from_civil(y, mon, 1 + (7 - w0).rem_euclid(7) + 7 * (n - 1))
    };
    let dst_start = nth_sun(y_guess, 3, 2) * 86_400_000 + 10 * 3_600_000;
    let dst_end = nth_sun(y_guess, 11, 1) * 86_400_000 + 9 * 3_600_000;
    let dst = ms_i >= dst_start && ms_i < dst_end;
    let off = if dst { -7 } else { -8 };
    let local = ms_i + off * 3_600_000;
    let days = local.div_euclid(86_400_000);
    let rem = local.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    let wday = (days + 4).rem_euclid(7) as usize;
    let (hh, mm, ss) = (
        rem / 3_600_000,
        rem % 3_600_000 / 60_000,
        rem % 60_000 / 1000,
    );
    let tz = if dst {
        "GMT-0700 (Pacific Daylight Time)"
    } else {
        "GMT-0800 (Pacific Standard Time)"
    };
    units(&format!(
        "{} {} {:02} {} {:02}:{:02}:{:02} {tz}",
        WD[wday],
        MO[(m - 1) as usize],
        d,
        y,
        hh,
        mm,
        ss
    ))
}

fn join_units(parts: &[Vec<u16>]) -> Vec<u16> {
    let mut out = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.push(b',' as u16);
        }
        out.extend_from_slice(p);
    }
    out
}

fn num_units(v: &Value) -> Vec<u16> {
    let x = match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        _ => v
            .as_object()
            .and_then(|o| o.get("$num"))
            .and_then(|s| s.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(f64::NAN),
    };
    units(&number_to_string(x))
}

fn js_to_units(v: &Value, in_array: bool) -> Vec<u16> {
    if in_array && (v.is_null() || is_undefined(v)) {
        return Vec::new();
    }
    if is_undefined(v) {
        return units("undefined");
    }
    match v {
        Value::Null => units("null"),
        Value::Bool(b) => units(if *b { "true" } else { "false" }),
        Value::Number(_) => num_units(v),
        Value::String(s) => units(s),
        Value::Array(a) => join_units(&a.iter().map(|e| js_to_units(e, true)).collect::<Vec<_>>()),
        Value::Object(_) => {
            if let Some(n) = single(v, "$num") {
                let _ = n;
                return num_units(v);
            }
            if let Some(u) = single(v, "$utf16")
                && let Some(us) = marker_units(u)
            {
                return us;
            }
            if let Some(b) = single(v, "$bigint")
                && let Some(s) = b.as_str()
            {
                return units(s);
            }
            if let Some(b) = single(v, "$bytes")
                && let Some(bytes) = byte_array(b)
            {
                return join_units(
                    &bytes
                        .iter()
                        .map(|x| units(&x.to_string()))
                        .collect::<Vec<_>>(),
                );
            }
            if let Some(b) = single(v, "$buffer")
                && let Some(bytes) = byte_array(b)
            {
                return units(&String::from_utf8_lossy(&bytes));
            }
            if single(v, "$map").is_some() {
                return units("[object Map]");
            }
            if single(v, "$set").is_some() {
                return units("[object Set]");
            }
            if let Some(d) = single(v, "$date") {
                let ms = match d {
                    Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
                    Value::String(s) if s == "Invalid" => f64::NAN,
                    Value::String(s) => iso_to_ms(s).unwrap_or(f64::NAN),
                    _ => f64::NAN,
                };
                return date_to_string_units(ms);
            }
            units("[object Object]")
        }
    }
}

// ---- ports -----------------------------------------------------------------------------------

/// Port of `hubWorkItem`: a fixed-field projection; `null`/`undefined` throw.
pub fn hub_work_item(record: Value) -> Result<Value, String> {
    if record.is_null() {
        return Err("Cannot read properties of null (reading 'id')".to_string());
    }
    if is_undefined(&record) {
        return Err("Cannot read properties of undefined (reading 'id')".to_string());
    }
    let mut out = Map::new();
    for f in FIELDS {
        out.insert(
            f.to_string(),
            record.get(f).map_or_else(undefined, normalize),
        );
    }
    Ok(Value::Object(out))
}

/// Port of `hubWorkLogicalTaskKey`: `taskKey` verbatim when not nullish,
/// else `task:` + ToString(taskIndex).
pub fn hub_work_logical_task_key(record: Value) -> Result<Value, String> {
    if record.is_null() {
        return Err("Cannot read properties of null (reading 'taskKey')".to_string());
    }
    if is_undefined(&record) {
        return Err("Cannot read properties of undefined (reading 'taskKey')".to_string());
    }
    let task_key = record.get("taskKey");
    if let Some(k) = task_key.filter(|k| !k.is_null() && !is_undefined(k)) {
        return Ok(normalize(k));
    }
    let task_index = record.get("taskIndex").cloned().unwrap_or_else(undefined);
    let mut out = units("task:");
    out.extend_from_slice(&js_to_units(&task_index, false));
    Ok(utf16_string(&out))
}

/// Port of `hubWorkPositionalTaskKey`: `task:` + ToString(position).
pub fn hub_work_positional_task_key(position: Value) -> Result<Value, String> {
    let mut out = units("task:");
    out.extend_from_slice(&js_to_units(&position, false));
    Ok(utf16_string(&out))
}
