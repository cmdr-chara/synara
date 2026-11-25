//! Automation schedule evaluation, ported from upstream `automation/schedule.ts`.
//! Times are epoch milliseconds; ISO strings use the upstream JS shape
//! (`2026-03-09T06:30:00.000Z`). Timezone-aware wall-clock slots verify the
//! round-trip so DST gaps are skipped instead of firing at a surprising
//! nearby instant, and the fall-back duplicate hour fires exactly once.

use chrono::{DateTime, Datelike, TimeZone, Timelike};
use serde_json::Value;

const MINUTE_MS: i64 = 60 * 1000;
const DAY_MS: i64 = 24 * 60 * MINUTE_MS;
const MAX_CRON_SEARCH_DAYS: i64 = 366;

fn iso(ms: i64) -> String {
    crate::claude_monitor::iso_from_ms(ms)
}

fn parse_iso(value: &str, label: &str) -> Result<i64, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|d| d.timestamp_millis())
        .map_err(|_| format!("Invalid automation schedule {label}: {value}"))
}

fn sha256_remainder(install_salt: &str, automation_id: &str) -> i64 {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut hasher, format!("{install_salt}:{automation_id}").as_bytes());
    let digest = hasher.finalize();
    let mut remainder: i64 = 0;
    for byte in digest {
        remainder = (remainder * 256 + byte as i64) % 120;
    }
    remainder
}

pub fn deterministic_jitter_seconds(install_salt: &str, automation_id: &str) -> i64 {
    sha256_remainder(install_salt, automation_id)
}

fn jitter_ms(schedule_type: &str, jitter_context: Option<(&str, &str)>) -> i64 {
    let Some((salt, id)) = jitter_context else { return 0 };
    match schedule_type {
        "daily" | "weekdays" | "weekly" | "cron" => sha256_remainder(salt, id) * 1000,
        _ => 0,
    }
}

fn parse_time_of_day(value: &str) -> (i64, i64) {
    let mut parts = value.split(':');
    let hours = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let minutes = parts.next().unwrap_or("0").parse().unwrap_or(0);
    (hours, minutes)
}

#[derive(Clone, Copy)]
struct LocalParts {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
}

fn parse_tz(timezone: &str) -> Result<chrono_tz::Tz, String> {
    timezone
        .parse::<chrono_tz::Tz>()
        .map_err(|_| format!("Invalid automation schedule timezone: {timezone}"))
}

fn local_parts_for(ms: i64, timezone: &str) -> Result<LocalParts, String> {
    let tz = parse_tz(timezone)?;
    let utc = chrono::Utc
        .timestamp_millis_opt(ms)
        .single()
        .ok_or_else(|| format!("Invalid timestamp: {ms}"))?;
    let local = utc.with_timezone(&tz);
    Ok(LocalParts {
        year: local.year() as i64,
        month: local.month() as i64,
        day: local.day() as i64,
        hour: local.hour() as i64,
        minute: local.minute() as i64,
        second: local.second() as i64,
    })
}

fn date_utc_ms(year: i64, month: i64, day: i64, hour: i64, minute: i64, second: i64) -> i64 {
    chrono::Utc
        .with_ymd_and_hms(
            year as i32,
            month as u32,
            day as u32,
            hour as u32,
            minute as u32,
            second as u32,
        )
        .single()
        .map(|d| d.timestamp_millis())
        .unwrap_or(0)
}

fn timezone_offset_ms(ms: i64, timezone: &str) -> Result<i64, String> {
    let parts = local_parts_for(ms, timezone)?;
    let as_utc = date_utc_ms(parts.year, parts.month, parts.day, parts.hour, parts.minute, parts.second);
    Ok(as_utc - ms)
}

/// Resolve a local wall-clock slot in an IANA timezone to UTC. The round-trip
/// verification skips DST gaps instead of scheduling at a surprising instant.
fn zoned_wall_clock_to_utc(
    timezone: &str,
    year: i64,
    month: i64,
    day: i64,
    time_of_day: &str,
) -> Result<Option<i64>, String> {
    let (hours, minutes) = parse_time_of_day(time_of_day);
    let naive_utc_ms = date_utc_ms(year, month, day, hours, minutes, 0);
    let first_pass = naive_utc_ms - timezone_offset_ms(naive_utc_ms, timezone)?;
    let second_pass = naive_utc_ms - timezone_offset_ms(first_pass, timezone)?;
    let round_trip = local_parts_for(second_pass, timezone)?;
    Ok(
        if round_trip.year == year
            && round_trip.month == month
            && round_trip.day == day
            && round_trip.hour == hours
            && round_trip.minute == minutes
        {
            Some(second_pass)
        } else {
            None
        },
    )
}

fn add_local_days(parts: LocalParts, days: i64) -> LocalParts {
    // Date arithmetic at UTC noon avoids every DST edge.
    let noon = date_utc_ms(parts.year, parts.month, parts.day, 12, 0, 0) + days * DAY_MS;
    let dt = chrono::Utc.timestamp_millis_opt(noon).single().unwrap();
    LocalParts {
        year: dt.year() as i64,
        month: dt.month() as i64,
        day: dt.day() as i64,
        hour: 0,
        minute: 0,
        second: 0,
    }
}

fn day_of_week_for_local_date(year: i64, month: i64, day: i64) -> i64 {
    // JS getUTCDay: 0=Sunday..6=Saturday. chrono num_days_from_sunday matches.
    let noon = date_utc_ms(year, month, day, 12, 0, 0);
    let dt = chrono::Utc.timestamp_millis_opt(noon).single().unwrap();
    dt.weekday().num_days_from_sunday() as i64
}

fn compute_next_zoned_wall_clock_run_at(schedule: &Value, from_ms: i64) -> Result<Option<i64>, String> {
    let schedule_type = schedule.get("type").and_then(Value::as_str).unwrap_or("");
    let time_of_day = schedule.get("timeOfDay").and_then(Value::as_str).unwrap_or("0:0");
    let timezone = schedule.get("timezone").and_then(Value::as_str);
    let day_of_week = schedule.get("dayOfWeek").and_then(Value::as_i64).unwrap_or(0);

    let Some(tz) = timezone else {
        // UTC path: direct slot arithmetic.
        let (hours, minutes) = parse_time_of_day(time_of_day);
        let day_start = from_ms - from_ms.rem_euclid(DAY_MS);
        let mut candidate = day_start + hours * 3_600_000 + minutes * 60_000;
        match schedule_type {
            "daily" => {
                if candidate <= from_ms {
                    candidate += DAY_MS;
                }
                return Ok(Some(candidate));
            }
            "weekdays" => {
                while candidate <= from_ms || [0, 6].contains(&day_of_week_ms(candidate)) {
                    candidate += DAY_MS;
                }
                return Ok(Some(candidate));
            }
            _ => {
                // weekly
                let days_until = (day_of_week - day_of_week_ms(candidate) + 7) % 7;
                candidate += days_until * DAY_MS;
                if candidate <= from_ms {
                    candidate += 7 * DAY_MS;
                }
                return Ok(Some(candidate));
            }
        }
    };

    let local_start = local_parts_for(from_ms, tz)?;
    for offset in 0..=370 {
        let local_date = add_local_days(local_start, offset);
        let local_dow = day_of_week_for_local_date(local_date.year, local_date.month, local_date.day);
        if schedule_type == "weekdays" && (local_dow == 0 || local_dow == 6) {
            continue;
        }
        if schedule_type == "weekly" && local_dow != day_of_week {
            continue;
        }
        if let Some(candidate) = zoned_wall_clock_to_utc(
            tz,
            local_date.year,
            local_date.month,
            local_date.day,
            time_of_day,
        )? {
            if candidate > from_ms {
                return Ok(Some(candidate));
            }
        }
    }
    Ok(None)
}

fn day_of_week_ms(ms: i64) -> i64 {
    let dt = chrono::Utc.timestamp_millis_opt(ms).single().unwrap();
    dt.weekday().num_days_from_sunday() as i64
}

#[derive(Default)]
struct CronField {
    values: std::collections::HashSet<i64>,
    is_wildcard: bool,
}

fn sorted_cron_values(field: &CronField) -> Vec<i64> {
    let mut v: Vec<i64> = field.values.iter().copied().collect();
    v.sort();
    v
}

fn parse_cron_integer(raw: Option<&str>, name: &str, reason: &str) -> Result<i64, String> {
    match raw {
        Some(s) if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) => {
            Ok(s.parse().unwrap_or(0))
        }
        _ => Err(format!("Invalid cron {name}: {reason}")),
    }
}

fn parse_cron_dow_value(raw: Option<&str>, name: &str, reason: &str) -> Result<i64, String> {
    if let Some(s) = raw {
        let named = match s.to_lowercase().as_str() {
            "sun" => Some(0),
            "mon" => Some(1),
            "tue" => Some(2),
            "wed" => Some(3),
            "thu" => Some(4),
            "fri" => Some(5),
            "sat" => Some(6),
            _ => None,
        };
        if let Some(v) = named {
            return Ok(v);
        }
    }
    parse_cron_integer(raw, name, reason)
}

fn parse_cron_field(
    raw: &str,
    min: i64,
    max: i64,
    name: &str,
    dow: bool,
) -> Result<CronField, String> {
    let parse_value = |raw: Option<&str>, name: &str, reason: &str| -> Result<i64, String> {
        if dow {
            parse_cron_dow_value(raw, name, reason)
        } else {
            parse_cron_integer(raw, name, reason)
        }
    };
    let normalize = |value: i64| -> i64 {
        if dow && value == 7 {
            0
        } else {
            value
        }
    };
    let mut field = CronField::default();
    for token in raw.split(',') {
        let part = token.trim();
        if part.is_empty() {
            return Err(format!("Invalid cron {name}: empty token"));
        }
        let step_parts: Vec<&str> = part.split('/').collect();
        if step_parts.len() > 2 {
            return Err(format!("Invalid cron {name}: bad step"));
        }
        let range_part = step_parts[0];
        let step_part = step_parts.get(1);
        let step = match step_part {
            None => 1,
            Some(s) => {
                let v = parse_cron_integer(Some(s), name, "bad step")?;
                if v <= 0 {
                    return Err(format!("Invalid cron {name}: bad step"));
                }
                v
            }
        };
        // Only a literal `*` is unrestricted for DOM/DOW cron semantics;
        // `*/2` is still a filter.
        if range_part == "*" && step_part.is_none() {
            field.is_wildcard = true;
        }
        let range_parts: Vec<String> = if range_part == "*" {
            vec![min.to_string(), max.to_string()]
        } else {
            range_part.split('-').map(str::to_string).collect()
        };
        if range_parts.len() > 2 {
            return Err(format!("Invalid cron {name}: out of range"));
        }
        let start = parse_value(Some(&range_parts[0]), name, "out of range")?;
        let end = parse_value(
            Some(range_parts.get(1).map(String::as_str).unwrap_or(&range_parts[0])),
            name,
            "out of range",
        )?;
        if start < min || end > max || start > end {
            return Err(format!("Invalid cron {name}: out of range"));
        }
        let mut value = start;
        while value <= end {
            field.values.insert(normalize(value));
            value += step;
        }
    }
    Ok(field)
}

struct CronExpression {
    minute: CronField,
    hour: CronField,
    day_of_month: CronField,
    month: CronField,
    day_of_week: CronField,
}

fn parse_cron_expression(expression: &str) -> Result<CronExpression, String> {
    let fields: Vec<&str> = expression.split_whitespace().collect();
    if fields.len() != 5 {
        return Err("Cron schedules must use a constrained 5-field expression.".to_string());
    }
    Ok(CronExpression {
        minute: parse_cron_field(fields[0], 0, 59, "minute", false)?,
        hour: parse_cron_field(fields[1], 0, 23, "hour", false)?,
        day_of_month: parse_cron_field(fields[2], 1, 31, "day-of-month", false)?,
        month: parse_cron_field(fields[3], 1, 12, "month", false)?,
        day_of_week: parse_cron_field(fields[4], 0, 7, "day-of-week", true)?,
    })
}

fn cron_day_matches(cron: &CronExpression, year: i64, month: i64, day: i64) -> bool {
    let dom_matches = cron.day_of_month.values.contains(&day);
    let dow_matches = cron
        .day_of_week
        .values
        .contains(&day_of_week_for_local_date(year, month, day));
    if cron.day_of_month.is_wildcard || cron.day_of_week.is_wildcard {
        return dom_matches && dow_matches;
    }
    dom_matches || dow_matches
}

fn compute_next_cron_run_at(schedule: &Value, from_ms: i64) -> Result<Option<i64>, String> {
    let timezone = schedule
        .get("timezone")
        .and_then(Value::as_str)
        .ok_or_else(|| "Invalid automation schedule timezone: missing".to_string())?;
    parse_tz(timezone)?;
    let expression = schedule.get("expression").and_then(Value::as_str).unwrap_or("");
    let cron = parse_cron_expression(expression)?;
    let local_start = local_parts_for(from_ms, timezone)?;
    let hours = sorted_cron_values(&cron.hour);
    let minutes = sorted_cron_values(&cron.minute);

    // Search by local calendar slots instead of UTC minutes: sparse cron
    // expressions can be months away.
    for day_offset in 0..=MAX_CRON_SEARCH_DAYS {
        let local_date = add_local_days(local_start, day_offset);
        if !cron.month.values.contains(&local_date.month)
            || !cron_day_matches(&cron, local_date.year, local_date.month, local_date.day)
        {
            continue;
        }
        for hour in &hours {
            if day_offset == 0 && *hour < local_start.hour {
                continue;
            }
            for minute in &minutes {
                if day_offset == 0 && *hour == local_start.hour && *minute <= local_start.minute {
                    continue;
                }
                let tod = format!("{hour:02}:{minute:02}");
                if let Some(candidate) = zoned_wall_clock_to_utc(
                    timezone,
                    local_date.year,
                    local_date.month,
                    local_date.day,
                    &tod,
                )? {
                    if candidate > from_ms {
                        return Ok(Some(candidate));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// Next occurrence strictly after `from_iso`, or None for manual/project-event.
pub fn compute_next_run_at(
    schedule: &Value,
    from_iso: &str,
    jitter_context: Option<(&str, &str)>,
) -> Result<Option<String>, String> {
    let schedule_type = schedule.get("type").and_then(Value::as_str).unwrap_or("");
    if schedule_type == "manual" || schedule_type == "project-event" {
        return Ok(None);
    }
    let from_ms = parse_iso(from_iso, "timestamp")?;
    if schedule_type == "once" {
        let run_at = parse_iso(
            schedule.get("runAt").and_then(Value::as_str).unwrap_or(""),
            "runAt",
        )?;
        return Ok(if run_at > from_ms { Some(iso(run_at)) } else { None });
    }
    if schedule_type == "interval" {
        let every = schedule.get("everySeconds").and_then(Value::as_i64).unwrap_or(60);
        return Ok(Some(iso(from_ms + every * 1000)));
    }
    let jitter = jitter_ms(schedule_type, jitter_context);
    let base_from = from_ms - jitter;
    let next = if schedule_type == "cron" {
        compute_next_cron_run_at(schedule, base_from)?
    } else {
        compute_next_zoned_wall_clock_run_at(schedule, base_from)?
    };
    Ok(next.map(|ms| iso(ms + jitter)))
}

/// Next run strictly after `not_before_iso`, coalescing missed occurrences
/// after downtime into a single future slot instead of replaying every one.
pub fn compute_next_run_at_after(
    schedule: &Value,
    from_iso: &str,
    not_before_iso: &str,
    jitter_context: Option<(&str, &str)>,
) -> Result<Option<String>, String> {
    let schedule_type = schedule.get("type").and_then(Value::as_str).unwrap_or("");
    if schedule_type == "manual" || schedule_type == "project-event" || schedule_type == "once" {
        return Ok(None);
    }
    if schedule_type == "interval" {
        let from_ms = parse_iso(from_iso, "timestamp")?;
        let floor = parse_iso(not_before_iso, "timestamp").unwrap_or(from_ms);
        let step_ms = schedule.get("everySeconds").and_then(Value::as_i64).unwrap_or(60) * 1000;
        let mut next = from_ms + step_ms;
        if next <= floor {
            // Jump straight to the first slot after the floor rather than
            // looping per interval.
            let missed = (floor - next + 1 + step_ms - 1) / step_ms;
            next += missed * step_ms;
        }
        return Ok(Some(iso(next)));
    }
    compute_next_run_at(schedule, not_before_iso, jitter_context)
}

/// Estimate the spacing between the next two occurrences from a fixed point.
/// Used for policy validation, not dispatch.
pub fn compute_schedule_spacing_seconds(schedule: &Value, from_iso: &str) -> Result<Option<i64>, String> {
    let schedule_type = schedule.get("type").and_then(Value::as_str).unwrap_or("");
    if matches!(schedule_type, "manual" | "once" | "project-event") {
        return Ok(None);
    }
    let Some(first) = compute_next_run_at(schedule, from_iso, None)? else { return Ok(None) };
    let Some(second) = compute_next_run_at_after(schedule, &first, &first, None)? else {
        return Ok(None);
    };
    let spacing = parse_iso(&second, "timestamp")? - parse_iso(&first, "timestamp")?;
    Ok(if spacing > 0 { Some(spacing / 1000) } else { None })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn at(schedule: Value, from: &str) -> Option<String> {
        compute_next_run_at(&schedule, from, None).unwrap()
    }

    #[test]
    fn one_shot_future() {
        let s = json!({"type":"once","runAt":"2026-03-09T10:00:00.000Z"});
        assert_eq!(
            at(s.clone(), "2026-03-08T10:00:00.000Z").as_deref(),
            Some("2026-03-09T10:00:00.000Z")
        );
        assert_eq!(at(s, "2026-03-10T10:00:00.000Z"), None);
    }

    #[test]
    fn weekly_utc() {
        // 2026-03-09 is a Monday (dow 1).
        let s = json!({"type":"weekly","timeOfDay":"09:30","dayOfWeek":3});
        let next = at(s, "2026-03-09T10:00:00.000Z").unwrap();
        assert_eq!(next, "2026-03-11T09:30:00.000Z");
    }

    #[test]
    fn weekdays_skip_weekend() {
        // 2026-03-06 is a Friday; from Friday evening the next weekday slot is Monday.
        let s = json!({"type":"weekdays","timeOfDay":"08:00"});
        let next = at(s, "2026-03-06T20:00:00.000Z").unwrap();
        assert_eq!(next, "2026-03-09T08:00:00.000Z");
    }

    #[test]
    fn daily_spring_forward_gap_skipped() {
        // 2026-03-08 02:30 does not exist in America/New_York (spring forward);
        // the occurrence must land on 03-09 02:30 EDT = 06:30 UTC.
        let s = json!({"type":"daily","timeOfDay":"02:30","timezone":"America/New_York"});
        let next = at(s, "2026-03-07T12:00:00.000Z").unwrap();
        assert_eq!(next, "2026-03-09T06:30:00.000Z");
    }

    #[test]
    fn fall_back_fires_first_occurrence_only() {
        // 2026-11-01 01:30 happens twice in America/New_York; the fire must be
        // the FIRST one (EDT) = 05:30 UTC, and the next day must be 11-02.
        let s = json!({"type":"daily","timeOfDay":"01:30","timezone":"America/New_York"});
        let first = at(s.clone(), "2026-10-31T12:00:00.000Z").unwrap();
        assert_eq!(first, "2026-11-01T05:30:00.000Z");
        let after = compute_next_run_at_after(&s, &first, &first, None).unwrap().unwrap();
        assert_eq!(after, "2026-11-02T06:30:00.000Z");
    }

    #[test]
    fn cron_basic_and_or_semantics() {
        let s = json!({"type":"cron","expression":"30 9 * * 1","timezone":"UTC"});
        // Next Monday 09:30 UTC after Monday 08:00.
        let next = at(s, "2026-03-09T08:00:00.000Z").unwrap();
        assert_eq!(next, "2026-03-09T09:30:00.000Z");
        // DOM/DOW both restricted → OR semantics: day 15 OR any Monday.
        let s2 = json!({"type":"cron","expression":"0 12 15 * 1","timezone":"UTC"});
        let n2 = at(s2, "2026-03-10T00:00:00.000Z").unwrap();
        assert_eq!(n2, "2026-03-15T12:00:00.000Z");
    }

    #[test]
    fn cron_named_and_seven_sunday() {
        let s = json!({"type":"cron","expression":"0 10 * * sun","timezone":"UTC"});
        let next = at(s.clone(), "2026-03-09T00:00:00.000Z").unwrap();
        assert_eq!(next, "2026-03-15T10:00:00.000Z");
        let s7 = json!({"type":"cron","expression":"0 10 * * 7","timezone":"UTC"});
        assert_eq!(at(s7, "2026-03-09T00:00:00.000Z").unwrap(), "2026-03-15T10:00:00.000Z");
    }

    #[test]
    fn interval_coalesces_after_downtime() {
        let s = json!({"type":"interval","everySeconds":3600});
        let next = compute_next_run_at_after(
            &s,
            "2026-03-01T00:00:00.000Z",
            "2026-03-05T00:00:00.000Z",
            None,
        )
        .unwrap()
        .unwrap();
        // First hourly slot after the floor, not a replay of every missed one.
        assert_eq!(next, "2026-03-05T01:00:00.000Z");
    }

    #[test]
    fn jitter_deterministic() {
        let a = deterministic_jitter_seconds("install-1", "auto-1");
        let b = deterministic_jitter_seconds("install-1", "auto-1");
        let c = deterministic_jitter_seconds("install-1", "auto-2");
        assert_eq!(a, b);
        assert!((0..120).contains(&a));
        assert!((0..120).contains(&c));
    }
}
