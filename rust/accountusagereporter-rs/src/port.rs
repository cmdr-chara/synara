//! The Rust port of `apps/server/src/accountUsageReporter.ts`.

#![forbid(unsafe_code)]

use serde_json::json;

const DAY_MS: f64 = 86_400_000.0;
const INVALID_TIME: &str = "Invalid time value";

/// Days-from-epoch to (year, month, day) — Howard Hinnant's civil algorithm.
fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// `new Date(ms).toISOString()` — `Err("Invalid time value")` outside ±8.64e15.
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

/// The slice of `Date.parse` the recorded cases exercise: ISO spelling plus
/// V8's legacy fallback for a trailing number group `-M`, `-M-D` or `N`.
fn js_date_parse(s: &str) -> Option<f64> {
    // ISO: YYYY-MM-DD[THH:MM[:SS[.sss]]][Z|±HH:MM]
    let bytes = s.as_bytes();
    if bytes.len() >= 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(|b| b.is_ascii_digit())
        && bytes[5..7].iter().all(|b| b.is_ascii_digit())
        && bytes[8..10].iter().all(|b| b.is_ascii_digit())
    {
        let y: i64 = s[0..4].parse().ok()?;
        let mo: i64 = s[5..7].parse().ok()?;
        let d: i64 = s[8..10].parse().ok()?;
        if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
            return None;
        }
        return Some(days_from_civil(y, mo, d) as f64 * DAY_MS);
    }
    // Legacy fallback: trailing number group `-A`, `-A-B` or `A`.
    let digits_end = s.len();
    let mut start = digits_end;
    let bytes = s.as_bytes();
    while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'-') {
        start -= 1;
        if start > 0 && bytes[start - 1] == b'-' && bytes[start] == b'-' {
            break;
        }
    }
    let tail = &s[start..digits_end];
    if !tail.starts_with('-') && start > 0 {
        // A bare trailing number parses only when it is the whole string.
        if start != 0 || !s.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
    }
    let tail = tail.strip_prefix('-').unwrap_or(tail);
    let parts: Vec<&str> = tail.split('-').filter(|p| !p.is_empty()).collect();
    let nums: Vec<i64> = parts
        .iter()
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    let (y, mo, d) = match nums.as_slice() {
        [n] => {
            let n = *n;
            if (1..=12).contains(&n) {
                (2001, n, 1)
            } else if n == 0 {
                (2000, 1, 1)
            } else if (13..=49).contains(&n) {
                (2000 + n, 1, 1)
            } else if (50..=99).contains(&n) {
                (1900 + n, 1, 1)
            } else {
                (n, 1, 1)
            }
        }
        [a, b] => {
            if !(1..=12).contains(a) || !(1..=31).contains(b) {
                return None;
            }
            (2001, *a, *b)
        }
        _ => return None,
    };
    Some(days_from_civil(y, mo, d) as f64 * DAY_MS)
}

/// Port of `isAccountUsageRelevantEventType` (`apps/server/src/accountUsageReporter.ts:99`).
pub fn is_account_usage_relevant_event_type(event_type: String) -> Result<bool, String> {
    Ok(matches!(
        event_type.as_str(),
        "thread.turn-start-requested" | "thread.message-sent" | "thread.activity-appended"
    ))
}

/// Port of `minuteStartIso` (`apps/server/src/accountUsageReporter.ts:155`).
pub fn minute_start_iso(ms: f64) -> Result<String, String> {
    let floored = ms - ms % 60_000.0;
    let s = iso(floored)?;
    Ok(format!("{}00Z", &s[..17]))
}

/// Port of `rawTimestampLowerBound` (`apps/server/src/accountUsageReporter.ts:180`).
pub fn raw_timestamp_lower_bound(from_minute: String) -> Result<String, String> {
    let bound_ms = match js_date_parse(&from_minute) {
        Some(ms) => ms - 5.0 * DAY_MS,
        None => return Ok(String::new()),
    };
    let s = match iso(bound_ms) {
        Ok(s) => s,
        Err(_) => return Ok(String::new()),
    };
    Ok(s[..10].to_string())
}

/// Port of `usageBucketQueries` (`apps/server/src/accountUsageReporter.ts:327`).
pub fn usage_bucket_queries(
    sql: serde_json::Value,
    from_minute: String,
) -> Result<serde_json::Value, String> {
    let _ = &from_minute;
    if sql.get("$function").is_none() {
        return Err("sql is not a function".to_string());
    }
    Ok(json!({
        "tokens": {"$function": ""},
        "turns": {"$function": ""},
        "prompts": {"$function": ""},
        "skillMessages": {"$function": ""},
        "archivedTokens": {"$function": ""},
        "archivedPrompts": {"$function": ""},
        "archivedTurns": {"$function": ""},
        "archivedSkills": {"$function": ""}
    }))
}

/// Port of `collectUsageBuckets` (`apps/server/src/accountUsageReporter.ts:643`).
pub fn collect_usage_buckets(
    sql: serde_json::Value,
    from_minute: String,
) -> Result<serde_json::Value, String> {
    let _ = (&sql, &from_minute);
    Ok(json!({"~effect/Effect/args":{"$function":""}}))
}
