//! The Rust port of `packages/shared/src/kanban.ts`.

#![forbid(unsafe_code)]

use crate::js::{as_number, is_undefined};
use serde_json::Value;

const WARN_MS: f64 = 20.0 * 60_000.0;
const HARD_MS: f64 = 40.0 * 60_000.0;

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn field<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&Value::Null)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undefined(v) {
                return false;
            }
            !matches!(
                v.get("$num").and_then(|m| m.as_str()),
                Some("NaN") | Some("-0") | Some("0")
            )
        }
    }
}

fn flag(v: &Value, key: &str) -> bool {
    v.get(key) == Some(&Value::Bool(true))
}

/// JS `Number()` coercion used by arithmetic on untyped input.
fn num_coerce(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => i32::from(*b) as f64,
        Value::String(s) => {
            let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(a) if a.is_empty() => 0.0,
        _ => as_number(v).unwrap_or(f64::NAN),
    }
}

fn null_err(prop: &str) -> String {
    format!("Cannot read properties of null (reading '{prop}')")
}

fn undef_err(prop: &str) -> String {
    format!("Cannot read properties of undefined (reading '{prop}')")
}

fn check_t(t: &Value, prop: &str) -> Result<(), String> {
    if t.is_null() {
        return Err(null_err(prop));
    }
    if is_undefined(t) {
        return Err(undef_err(prop));
    }
    Ok(())
}

/// A session can still receive the answer to a pending request while its
/// status is outside the dead set {stopped, error}; no session → answerable.
fn can_answer(session: &Value) -> bool {
    if is_nullish(session) {
        return true;
    }
    !matches!(
        session.get("status"),
        Some(Value::String(s)) if s == "stopped" || s == "error"
    )
}

fn turn_settled(t: &Value) -> bool {
    let turn = field(t, "latestTurn");
    if is_nullish(turn) || !truthy(field(turn, "startedAt")) {
        return false;
    }
    if !truthy(field(turn, "completedAt")) {
        return false;
    }
    if matches!(field(turn, "state"), Value::String(s) if s == "interrupted" || s == "error") {
        return true;
    }
    let session = field(t, "session");
    if is_nullish(session) {
        return true;
    }
    if field(session, "status") == &Value::String("running".into()) {
        return false;
    }
    true
}

fn live_latest_turn(t: &Value) -> bool {
    let turn = field(t, "latestTurn");
    if is_nullish(turn) || !truthy(field(turn, "startedAt")) {
        return false;
    }
    !turn_settled(t)
}

fn live_work(t: &Value) -> bool {
    if (flag(t, "hasPendingApprovals") || flag(t, "hasPendingUserInput"))
        && can_answer(field(t, "session"))
    {
        return true;
    }
    if flag(t, "hasLiveTailWork") {
        return true;
    }
    let turn = field(t, "latestTurn");
    if !is_nullish(turn) && field(turn, "state") == &Value::String("running".into()) {
        return true;
    }
    if live_latest_turn(t) {
        return true;
    }
    matches!(
        field(field(t, "session"), "status"),
        Value::String(s) if s == "starting" || s == "running"
    )
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// `Date.parse` subset: ISO calendar dates, `YYYY-MM-DD` (UTC) and
/// `YYYY-MM-DD[ T]HH:mm[:ss[.fff]][Z|±HH[:mm]]` — anything else is NaN.
fn date_parse(v: &Value) -> f64 {
    let s = match v.as_str() {
        Some(s) => s.trim(),
        None => return f64::NAN,
    };
    let b = s.as_bytes();
    let num = |i: usize, n: usize| -> Option<i64> {
        b.get(i..i + n)
            .filter(|w| w.iter().all(|c| c.is_ascii_digit()))
            .and_then(|w| std::str::from_utf8(w).ok()?.parse().ok())
    };
    let y = num(0, 4);
    let (y, mo, d) = match (y, b.get(4), num(5, 2), b.get(7), num(8, 2)) {
        (Some(y), Some(b'-'), Some(mo), Some(b'-'), Some(d)) if (1..=12).contains(&mo) => {
            (y, mo, d)
        }
        _ => return f64::NAN,
    };
    if !(1..=31).contains(&d) {
        return f64::NAN;
    }
    let rest = &s[10.min(s.len())..];
    let rest_b = rest.as_bytes();
    if b.len() == 10 {
        return (days_from_civil(y, mo, d) * 86_400_000) as f64;
    }
    if !matches!(rest_b.first(), Some(b'T') | Some(b' ')) {
        return f64::NAN;
    }
    let hh = num(11, 2);
    let mm = if b.get(13) == Some(&b':') {
        num(14, 2)
    } else {
        None
    };
    let (hh, mm) = match (hh, b.get(13), mm) {
        (Some(h), Some(b':'), Some(m)) if h <= 24 && m <= 59 => (h, m),
        _ => return f64::NAN,
    };
    let mut idx = 16;
    let mut ss = 0i64;
    let mut frac = 0.0f64;
    if b.get(idx) == Some(&b':') {
        match num(idx + 1, 2) {
            Some(s2) if s2 <= 60 => ss = s2,
            _ => return f64::NAN,
        }
        idx += 3;
        if b.get(idx) == Some(&b'.') {
            let start = idx + 1;
            let mut end = start;
            while end < b.len() && b[end].is_ascii_digit() {
                end += 1;
            }
            if end == start || end - start > 3 {
                return f64::NAN;
            }
            let digits = &s[start..end];
            frac = digits.parse::<f64>().unwrap_or(0.0) / 10f64.powi(digits.len() as i32);
            idx = end;
        }
    }
    let tz = &b[idx.min(b.len())..];
    let offset_min: i64 = if tz.is_empty() || tz == b"Z" {
        0
    } else if matches!(tz.first(), Some(b'+') | Some(b'-')) && tz.len() >= 3 {
        let sign = if tz[0] == b'+' { 1i64 } else { -1i64 };
        let rest: String = tz[1..]
            .iter()
            .filter(|c| **c != b':')
            .map(|c| *c as char)
            .collect();
        let rb = rest.as_bytes();
        let oh: Option<i64> = rb
            .get(0..2)
            .and_then(|w| std::str::from_utf8(w).ok()?.parse().ok());
        let om = if rb.len() == 2 {
            Some(0i64)
        } else if rb.len() == 4 {
            rb.get(2..4)
                .and_then(|w| std::str::from_utf8(w).ok()?.parse().ok())
        } else {
            None
        };
        match (oh, om) {
            (Some(h), Some(m)) if h <= 23 && m <= 59 => sign * (h * 60 + m),
            _ => return f64::NAN,
        }
    } else {
        return f64::NAN;
    };
    let ms = (days_from_civil(y, mo, d) * 86_400_000) as f64
        + (hh * 3_600_000 + mm * 60_000 + ss * 1000) as f64
        + frac
        - (offset_min * 60_000) as f64;
    if ms.abs() > 8_640_000_000_000_000.0 {
        f64::NAN
    } else {
        ms
    }
}

/// Effective heartbeat: the latest finite stamp among session.updatedAt,
/// threadUpdatedAt and lastActivityTimestampMs; null when none parses.
fn heartbeat(t: &Value) -> Option<f64> {
    let session = field(t, "session");
    let session_obs = if is_nullish(session) {
        f64::NAN
    } else {
        date_parse(field(session, "updatedAt"))
    };
    let thread_obs = date_parse(field(t, "threadUpdatedAt"));
    let activity_obs = match field(t, "lastActivityTimestampMs") {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        _ => f64::NAN,
    };
    let best = [session_obs, thread_obs, activity_obs]
        .into_iter()
        .filter(|x| x.is_finite())
        .fold(f64::NEG_INFINITY, f64::max);
    if best.is_finite() { Some(best) } else { None }
}

/// `heartbeat !== null ? Math.max(0, opts.now - heartbeat) : null`.
fn heartbeat_age(t: &Value, opts: &Value) -> Result<Option<f64>, String> {
    let Some(hb) = heartbeat(t) else {
        return Ok(None);
    };
    let now = if opts.is_null() {
        return Err(null_err("now"));
    } else if is_undefined(opts) {
        return Err(undef_err("now"));
    } else {
        num_coerce(field(opts, "now"))
    };
    Ok(Some((now - hb).max(0.0)))
}

fn hard_stuck(t: &Value, opts: &Value) -> Result<bool, String> {
    Ok(heartbeat_age(t, opts)?
        .map(|a| a >= HARD_MS)
        .unwrap_or(false))
}

fn warn_stuck(t: &Value, opts: &Value) -> Result<bool, String> {
    Ok(heartbeat_age(t, opts)?
        .map(|a| a >= WARN_MS)
        .unwrap_or(false))
}

fn awaiting_reason(t: &Value, opts: &Value) -> Result<Option<&'static str>, String> {
    check_t(t, "latestTurn")?;
    let turn = field(t, "latestTurn");
    if field(turn, "state") == &Value::String("error".into()) {
        return Ok(Some("failed"));
    }
    let session = field(t, "session");
    if !is_nullish(session)
        && (field(session, "status") == &Value::String("error".into())
            || !is_nullish(field(session, "lastError")))
    {
        return Ok(Some("failed"));
    }
    if can_answer(session) {
        if flag(t, "hasPendingApprovals") {
            return Ok(Some("pending-approval"));
        }
        if flag(t, "hasPendingUserInput") {
            return Ok(Some("pending-input"));
        }
    }
    if live_work(t) && hard_stuck(t, opts)? {
        return Ok(Some("stuck"));
    }
    Ok(None)
}

/// Port of `isKanbanTurnSettled` (`packages/shared/src/kanban.ts:96`).
pub fn is_kanban_turn_settled(t: serde_json::Value) -> Result<bool, String> {
    check_t(&t, "latestTurn")?;
    Ok(turn_settled(&t))
}

/// Port of `hasKanbanLiveLatestTurn` (`packages/shared/src/kanban.ts:119`).
pub fn has_kanban_live_latest_turn(t: serde_json::Value) -> Result<bool, String> {
    check_t(&t, "latestTurn")?;
    Ok(live_latest_turn(&t))
}

/// Port of `hasKanbanLiveWork` (`packages/shared/src/kanban.ts:129`).
pub fn has_kanban_live_work(t: serde_json::Value) -> Result<bool, String> {
    check_t(&t, "session")?;
    Ok(live_work(&t))
}

/// Port of `kanbanHeartbeatTimestampMs` (`packages/shared/src/kanban.ts:152`).
pub fn kanban_heartbeat_timestamp_ms(t: serde_json::Value) -> Result<Option<f64>, String> {
    check_t(&t, "session")?;
    Ok(heartbeat(&t))
}

/// Port of `deriveKanbanAwaitingYouReason` (`packages/shared/src/kanban.ts:197`).
pub fn derive_kanban_awaiting_you_reason(
    t: serde_json::Value,
    opts: serde_json::Value,
) -> Result<Option<String>, String> {
    Ok(awaiting_reason(&t, &opts)?.map(str::to_string))
}

/// Port of `deriveKanbanColumnV2` (`packages/shared/src/kanban.ts:229`).
pub fn derive_kanban_column_v2(
    t: serde_json::Value,
    opts: Option<serde_json::Value>,
) -> Result<String, String> {
    let opts = opts.unwrap_or(Value::Null);
    if !is_nullish(&opts)
        && let Some(now) = opts.get("now").filter(|v| !is_undefined(v))
        && awaiting_reason(&t, &serde_json::json!({ "now": now.clone() }))?.is_some()
    {
        return Ok("awaitingYou".into());
    }
    check_t(&t, "session")?;
    if live_work(&t) {
        return Ok("inProgress".into());
    }
    if is_nullish(field(&t, "latestTurn")) {
        return Ok("draft".into());
    }
    Ok("done".into())
}

/// Port of `deriveKanbanAttention` (`packages/shared/src/kanban.ts:252`).
pub fn derive_kanban_attention(
    t: serde_json::Value,
    opts: serde_json::Value,
) -> Result<Vec<String>, String> {
    let mut flags: Vec<String> = Vec::new();
    let reason = awaiting_reason(&t, &opts)?;
    match reason {
        Some("pending-approval") => flags.push("awaiting-approval".into()),
        Some("pending-input") => flags.push("awaiting-input".into()),
        Some("failed") => flags.push("failed".into()),
        Some("stuck") => flags.push("stuck".into()),
        _ => {}
    }
    check_t(&t, "session")?;
    if live_work(&t) && warn_stuck(&t, &opts)? && reason != Some("stuck") {
        flags.push("stuck".into());
    }
    if opts.is_null() {
        return Err(null_err("needsReview"));
    }
    if is_undefined(&opts) {
        return Err(undef_err("needsReview"));
    }
    if flag(&opts, "needsReview") {
        flags.push("needs-review".into());
    }
    Ok(flags)
}
