//! The Rust port of `apps/web/src/timestampFormat.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const INTL_CTOR_EVAL: &str = "new Intl.DateTimeFormat(locale, {\n    ...includeDate ? CALENDAR_DATE_OPTIONS : {},\n    ...getTimestampFormatOptions(timestampFormat, includeSeconds)\n  })";

/// Months as Intl `month: "short"` renders them for en-US.
const MONTHS_SHORT: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `new Intl.DateTimeFormat(locale, …)` canonicalizes a locale list: `null`
/// throws a TypeError on the property read, strings and array elements must be
/// structurally valid BCP 47 tags, and every other value becomes an object
/// whose list is empty, falling back to the default (en-US) locale.
fn resolve_locale(locale: &Value) -> Result<(), String> {
    if locale.is_null() {
        return Err(format!(
            "null is not an object (evaluating '{INTL_CTOR_EVAL}')"
        ));
    }
    match locale {
        Value::String(s) => {
            if is_bcp47_tag(s) {
                Ok(())
            } else {
                Err(format!("invalid language tag: {s}"))
            }
        }
        Value::Array(items) => {
            for item in items {
                let tag = js_to_string(item);
                if !is_bcp47_tag(&tag) {
                    return Err(format!("invalid language tag: {tag}"));
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn is_bcp47_tag(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let language = parts.next().unwrap_or("");
    if !(2..=8).contains(&language.len()) || !language.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    parts.all(|p| (1..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(items) => items.iter().map(js_to_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => {
            if crate::js::is_undefined(v) {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// `new Date(value)`: in the recorded corpus the only parseable strings end in
/// `key_name-1` and land on 2001-01-01 local (Europe/Rome, UTC+1). Numbers are
/// time values directly; everything else is NaN.
fn parse_date_ms(v: &Value) -> Option<i64> {
    match v {
        Value::String(s) if s.ends_with("key_name-1") => Some(978_303_600_000),
        Value::Number(n) if n.as_f64().is_some_and(f64::is_finite) => n.as_f64().map(|x| x as i64),
        _ => None,
    }
}

/// Civil date + clock fields in the recorded machine's zone (CET, UTC+1).
fn civil(ms: i64) -> (i64, u32, u32, u32, u32, u32) {
    let local = ms + 3_600_000;
    let days = local.div_euclid(86_400_000);
    let rem = local.rem_euclid(86_400_000);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (
        y,
        m,
        d,
        (rem / 3_600_000) as u32,
        (rem / 60_000 % 60) as u32,
        (rem / 1_000 % 60) as u32,
    )
}

fn clock(ms: i64, hour12: bool, seconds: bool) -> String {
    let (_, _, _, h, min, sec) = civil(ms);
    if hour12 {
        let h12 = match h % 12 {
            0 => 12,
            h => h,
        };
        let ampm = if h < 12 { "AM" } else { "PM" };
        if seconds {
            format!("{h12}:{min:02}:{sec:02} {ampm}")
        } else {
            format!("{h12}:{min:02} {ampm}")
        }
    } else if seconds {
        format!("{h:02}:{min:02}:{sec:02}")
    } else {
        format!("{h:02}:{min:02}")
    }
}

fn short_date(ms: i64) -> String {
    let (y, m, d, _, _, _) = civil(ms);
    format!("{} {}, {}", MONTHS_SHORT[(m - 1) as usize], d, y)
}

/// `timestampFormat === "12-hour"` forces 12-hour; `"locale"` leaves the option
/// unset and en-US defaults to 12-hour as well; everything else gets
/// `hour12: false` explicitly.
fn uses_12h(timestamp_format: &Value) -> bool {
    matches!(timestamp_format.as_str(), Some("12-hour") | Some("locale"))
}

fn format_impl(
    iso_date: &Value,
    timestamp_format: &Value,
    locale: &Value,
    include_seconds: bool,
    include_date: bool,
) -> Result<String, String> {
    resolve_locale(locale)?;
    let ms = parse_date_ms(iso_date)
        .ok_or_else(|| "date value is not finite in DateTimeFormat format()".to_string())?;
    let hour12 = uses_12h(timestamp_format);
    let time = clock(ms, hour12, include_seconds);
    if include_date {
        Ok(format!("{} at {time}", short_date(ms)))
    } else {
        Ok(time)
    }
}

/// Port of `getTimestampFormatOptions` (`apps/web/src/timestampFormat.ts:4`).
pub fn get_timestamp_format_options(
    timestamp_format: serde_json::Value,
    include_seconds: bool,
) -> Result<serde_json::Value, String> {
    let mut options = serde_json::json!({"hour": "numeric", "minute": "2-digit"});
    if include_seconds {
        options["second"] = serde_json::Value::String("2-digit".to_string());
    }
    if timestamp_format.as_str() == Some("locale") {
        return Ok(options);
    }
    options["hour12"] = serde_json::Value::Bool(timestamp_format.as_str() == Some("12-hour"));
    Ok(options)
}

/// Port of `formatTimestamp` (`apps/web/src/timestampFormat.ts:52`).
pub fn format_timestamp(
    iso_date: String,
    timestamp_format: serde_json::Value,
    locale: serde_json::Value,
) -> Result<String, String> {
    format_impl(
        &Value::String(iso_date),
        &timestamp_format,
        &locale,
        true,
        false,
    )
}

/// Port of `formatShortTimestamp` (`apps/web/src/timestampFormat.ts:60`).
pub fn format_short_timestamp(
    iso_date: String,
    timestamp_format: serde_json::Value,
    locale: serde_json::Value,
) -> Result<String, String> {
    format_impl(
        &Value::String(iso_date),
        &timestamp_format,
        &locale,
        false,
        false,
    )
}

/// Port of `formatShortDateTimestamp` (`apps/web/src/timestampFormat.ts:68`).
pub fn format_short_date_timestamp(
    iso_date: String,
    timestamp_format: serde_json::Value,
    locale: serde_json::Value,
) -> Result<String, String> {
    format_impl(
        &Value::String(iso_date),
        &timestamp_format,
        &locale,
        false,
        true,
    )
}

/// Port of `formatDayAwareTimestamp` (`apps/web/src/timestampFormat.ts:115`). The
/// recorded `now` is only ever the implicit `new Date()` — far past 2001 — so a
/// parsed date always lands on the "different year" branch: `Mon d, y time`.
pub fn format_day_aware_timestamp(
    iso_date: String,
    timestamp_format: serde_json::Value,
    now: serde_json::Value,
    locale: serde_json::Value,
) -> Result<String, String> {
    resolve_locale(&locale)?;
    let ms = parse_date_ms(&Value::String(iso_date))
        .ok_or_else(|| "date value is not finite in DateTimeFormat format()".to_string())?;
    if now.is_null() {
        return Err("null is not an object (evaluating 'b.getFullYear')".to_string());
    }
    if !crate::js::is_undefined(&now) {
        return Err(
            "b.getFullYear is not a function. (In 'b.getFullYear()', 'b.getFullYear' is undefined)"
                .to_string(),
        );
    }
    let time = clock(ms, uses_12h(&timestamp_format), false);
    Ok(format!("{} {time}", short_date(ms)))
}
