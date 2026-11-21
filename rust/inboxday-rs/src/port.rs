//! The Rust port of `packages/shared/src/inboxDay.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const DAY_MS: f64 = 86_400_000.0;
const HALF_DAY_MS: f64 = 43_200_000.0;
const HOUR_MS: f64 = 3_600_000.0;
const INVALID_TIME: &str = "Invalid time value";

/// `Math.floor((now - 12h) / 24h) * 24h + 12h` — the noon-to-noon day start.
fn day_start(now_ms: f64) -> f64 {
    if !now_ms.is_finite() || now_ms.abs() > 8_640_000_000_000_000.0 {
        return f64::NAN;
    }
    ((now_ms - HALF_DAY_MS) / DAY_MS).floor() * DAY_MS + HALF_DAY_MS
}

/// `{"$num": ...}` or a plain number, as the bridge encodes it.
fn num(v: f64) -> Value {
    crate::js::number(v)
}

fn num_field(v: &Value) -> f64 {
    crate::js::as_number(v).unwrap_or(f64::NAN)
}

fn slot(id: &str, label: &str, from_ms: f64, to_ms: f64) -> Value {
    json!({
        "id": id,
        "label": label,
        "fromMs": num(from_ms),
        "toMs": num(to_ms),
    })
}

/// The shape `{fromMs, toMs, slots, hours}` shared by the current and
/// previous day.
fn day(now_ms: f64) -> Value {
    let start = day_start(now_ms);
    let slots = json!([
        slot("morning", "Morning", start, start + 28_800_000.0),
        slot(
            "afternoon",
            "Afternoon",
            start + 28_800_000.0,
            start + 50_400_000.0
        ),
        slot("evening", "Evening", start + 50_400_000.0, start + DAY_MS),
    ]);
    let mut hours = Vec::new();
    if start.is_finite() {
        for i in 0..24 {
            let h = start + (i as f64) * HOUR_MS;
            hours.push(json!({"fromMs": num(h), "toMs": num(h + HOUR_MS)}));
        }
    }
    json!({
        "fromMs": num(start),
        "toMs": num(start + DAY_MS),
        "slots": slots,
        "hours": hours,
    })
}

/// Port of `resolveActivityDayStartMs`.
pub fn resolve_activity_day_start_ms(now_ms: f64) -> Result<f64, String> {
    Ok(day_start(now_ms))
}

/// Port of `resolveInboxDay`.
pub fn resolve_inbox_day(now_ms: f64) -> Result<Value, String> {
    let mut d = day(now_ms);
    let to_morning = num_field(&d["slots"][0]["toMs"]);
    let to_afternoon = num_field(&d["slots"][1]["toMs"]);
    let current = if now_ms < to_morning {
        "morning"
    } else if now_ms < to_afternoon {
        "afternoon"
    } else {
        "evening"
    };
    d["currentSlotId"] = json!(current);
    d["previousDay"] = day(now_ms - DAY_MS);
    Ok(d)
}

/// Days-from-epoch to (year, month, day) — Howard Hinnant's civil algorithm.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `new Date(ms).toISOString()` — `Err("Invalid time value")` outside the
/// `±8.64e15` range JS accepts.
fn iso(ms: f64) -> Result<String, String> {
    if !ms.is_finite() || ms.abs() > 8_640_000_000_000_000.0 {
        return Err(INVALID_TIME.to_string());
    }
    let ms = ms.trunc();
    let days = (ms / DAY_MS).floor() as i64;
    let rem = (ms - days as f64 * DAY_MS) as i64;
    let (y, mo, d) = civil(days);
    let (hh, mm, ss, mss) = (
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1_000 % 60,
        rem % 1_000,
    );
    let year = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else if y < 0 {
        format!("-{:06}", -y)
    } else {
        format!("+{y:06}")
    };
    Ok(format!(
        "{year}-{mo:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{mss:03}Z"
    ))
}

/// Port of `recapInputForRange`.
pub fn recap_input_for_range(range: Value) -> Result<Value, String> {
    if range.is_null() {
        return Err("Cannot read properties of null (reading 'fromMs')".to_string());
    }
    let from_ms = num_field(range.get("fromMs").unwrap_or(&Value::Null));
    let to_ms = num_field(range.get("toMs").unwrap_or(&Value::Null));
    let from = iso(from_ms)?;
    let to = iso(to_ms)?;
    let hours = match range.get("hours") {
        None | Some(Value::Null) => {
            return Err("Cannot read properties of undefined (reading 'slice')".to_string());
        }
        Some(Value::Array(h)) => h.clone(),
        Some(other) => {
            return Err(format!(
                "Cannot read properties of {} (reading 'slice')",
                match other {
                    Value::Bool(_) | Value::Number(_) => "number",
                    _ => "string",
                }
            ));
        }
    };
    let mut bounds = Vec::new();
    for h in hours.iter().skip(1) {
        let s = iso(h.get("fromMs").map_or(f64::NAN, num_field))?;
        if !bounds.contains(&s) {
            bounds.push(s);
        }
    }
    Ok(json!({"from": from, "to": to, "slotBoundaries": bounds}))
}
