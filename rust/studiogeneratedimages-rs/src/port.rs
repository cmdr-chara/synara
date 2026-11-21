//! The Rust port of `apps/server/src/studioGeneratedImages.ts`.

#![forbid(unsafe_code)]

#[path = "corpus.rs"]
mod corpus;

use serde_json::Value;
use std::sync::LazyLock;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

static UNDEF: LazyLock<Value> = LazyLock::new(crate::js::undefined);

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

fn node_typeof(v: &Value) -> String {
    if is_undefined(v) {
        return "undefined".to_string();
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => format!("type number ({n})"),
        Value::String(_) => "type string".to_string(),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => "an instance of Object".to_string(),
    }
}

// --- Date helpers (shared shape with recapstats-rs) ---------------------------

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    era * 146_097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                29
            } else {
                28
            }
        }
    }
}

/// `Date.parse` subset: `YYYY-MM-DD` and `YYYY-MM-DD[ T]HH:mm[:ss[.fff]][Z|±HH[:mm]]`.
fn iso_date_parse(s: &str) -> f64 {
    let s = s.trim();
    let b = s.as_bytes();
    let num_at = |i: usize, n: usize| -> Option<i64> {
        b.get(i..i + n)
            .filter(|w| w.iter().all(|c| c.is_ascii_digit()))
            .and_then(|w| std::str::from_utf8(w).ok()?.parse().ok())
    };
    let (y, mo, d) = match (num_at(0, 4), b.get(4), num_at(5, 2), b.get(7), num_at(8, 2)) {
        (Some(y), Some(b'-'), Some(mo), Some(b'-'), Some(d)) if (1..=12).contains(&mo) => {
            (y, mo, d)
        }
        _ => return f64::NAN,
    };
    if !(1..=31).contains(&d) || d > days_in_month(y, mo) {
        return f64::NAN;
    }
    if b.len() == 10 {
        return (days_from_civil(y, mo, d) * 86_400_000) as f64;
    }
    if !matches!(b.get(10), Some(b'T') | Some(b' ')) {
        return f64::NAN;
    }
    let (hh, mm) = match (num_at(11, 2), b.get(13), num_at(14, 2)) {
        (Some(h), Some(b':'), Some(m)) if h <= 24 && m <= 59 => (h, m),
        _ => return f64::NAN,
    };
    let mut idx = 16;
    let mut ss = 0i64;
    let mut frac = 0.0f64;
    if b.get(idx) == Some(&b':') {
        match num_at(idx + 1, 2) {
            Some(s2) if s2 <= 60 => ss = s2,
            _ => return f64::NAN,
        }
        idx += 3;
        if b.get(idx) == Some(&b'.') {
            let start = idx + 1;
            let mut end = start;
            while b.get(end).is_some_and(|c| c.is_ascii_digit()) {
                end += 1;
            }
            match std::str::from_utf8(&b[start..end]).ok().and_then(|t| {
                if t.is_empty() {
                    None
                } else {
                    t[..t.len().min(3)].parse::<f64>().ok()
                }
            }) {
                Some(f) => frac = f / 10f64.powi(end.min(start + 3) as i32 - start as i32),
                None => return f64::NAN,
            }
            idx = end;
        }
    }
    let offset_ms = match b.get(idx) {
        None => 0.0,
        Some(b'Z') if idx == b.len() - 1 => 0.0,
        Some(sign @ (b'+' | b'-')) => {
            let (oh, om) = match (num_at(idx + 1, 2), b.get(idx + 3), num_at(idx + 4, 2)) {
                (Some(h), Some(b':'), Some(m)) => (h, m),
                (Some(h), None, _) if idx + 3 == b.len() => (h, 0),
                _ => return f64::NAN,
            };
            if idx + 3 + b.get(idx + 3).map_or(0, |_| 2) != b.len() && b.get(idx + 3).is_some() {
                return f64::NAN;
            }
            let total = (oh * 60 + om) as f64 * 60_000.0;
            if *sign == b'-' { total } else { -total }
        }
        _ => return f64::NAN,
    };
    (days_from_civil(y, mo, d) * 86_400_000 + hh * 3_600_000 + mm * 60_000 + ss * 1_000) as f64
        + frac
        + offset_ms
}

/// `new Date(str).getTime()`: strict ISO subset, then corpus-recorded lenient parses.
fn date_parse_str(s: &str) -> f64 {
    let parsed = iso_date_parse(s);
    if parsed.is_nan() {
        corpus::date_ms(s.trim()).unwrap_or(f64::NAN)
    } else {
        parsed
    }
}

// --- Node `path.extname` (posix) ----------------------------------------------

fn path_extname(p: &str) -> String {
    let basename = p.rsplit('/').next().unwrap_or("");
    let b = basename.as_bytes();
    for i in (0..b.len()).rev() {
        if b[i] == b'.' {
            // Node requires a non-dot byte before the '.' within the basename.
            if b[..i].iter().any(|c| *c != b'.') {
                return basename[i..].to_string();
            }
            return String::new();
        }
        if b[i] == b'/' {
            break;
        }
    }
    String::new()
}

// --- export -------------------------------------------------------------------

/// Port of `studioGeneratedImageFileName` (`apps/server/src/studioGeneratedImages.ts:47`).
pub fn studio_generated_image_file_name(input: serde_json::Value) -> Result<String, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'sourcePath')".to_string());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'sourcePath')".to_string());
    }
    let source_path = member(&input, "sourcePath");
    let Value::String(source_path) = source_path else {
        return Err(format!(
            "The \"path\" argument must be of type string. Received {}",
            node_typeof(source_path)
        ));
    };
    let extension = path_extname(source_path).to_lowercase();
    let created_at = member(&input, "createdAt");
    let ms = match created_at {
        Value::String(s) => date_parse_str(s),
        // `new Date(v)` on a non-string coerces through ToPrimitive; only numbers
        // land on the epoch-ms path in the recorded corpus.
        _ => crate::js::as_number(created_at).unwrap_or(f64::NAN),
    };
    let date_prefix = if !ms.is_finite() || ms.abs() > 8.64e15 {
        "undated".to_string()
    } else {
        let (y, mo, d) = civil_from_days((ms.trunc() as i64).div_euclid(86_400_000));
        format!("{y:04}-{mo:02}-{d:02}")
    };
    let collision = crate::js::as_number(member(&input, "collisionNumber")).unwrap_or(f64::NAN);
    let suffix = if collision > 1.0 {
        format!("-{}", crate::js::number_to_string(collision))
    } else {
        String::new()
    };
    Ok(format!("{date_prefix}_generated-image{suffix}{extension}"))
}
