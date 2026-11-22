//! The Rust port of `apps/api/src/billing/`.
//!
//! Derived from `cases.jsonl` plus the declared unit interface (clean-room):
//! remote admission needs enforcement + a fresh granted status; a public
//! profile stays visible while its stored grant is unrevoked or within the
//! 24h renewal grace. `SYNARA_STATE_NOW_MS` freezes the clock for replay.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js;

const REMOTE_BILLING_MAX_STATUS_AGE_MS: f64 = 60_000.0;
const PROFILE_RENEWAL_GRACE_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;

fn real_now_ms() -> f64 {
    std::env::var("SYNARA_STATE_NOW_MS")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0)
        })
}

/// `v ?? default`: null, `undefined` and a missing key all take the default;
/// every other value passes through (a non-number then fails `isSafeInteger`).
fn number_or_default(v: Option<&Value>, default: f64) -> f64 {
    match v {
        None | Some(Value::Null) => default,
        Some(v) if js::is_undefined(v) => default,
        Some(v) => js::as_number(v).unwrap_or(f64::NAN),
    }
}

fn is_safe_integer(x: f64) -> bool {
    x.is_finite() && x.fract() == 0.0 && x.abs() <= 9_007_199_254_740_991.0
}

/// `Date.parse` subset: full ISO forms plus the bare shapes the corpus sends.
/// Non-string inputs and anything else are NaN.
fn date_parse(v: &Value) -> f64 {
    let Some(s) = v.as_str() else {
        return f64::NAN;
    };
    date_parse_iso(s.trim()).unwrap_or(f64::NAN)
}

fn date_parse_iso(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let num = |i: usize, n: usize| -> Option<i64> {
        (0..n).try_fold(0i64, |acc, j| {
            let d = *b.get(i + j)?;
            d.is_ascii_digit().then(|| acc * 10 + (d - b'0') as i64)
        })
    };
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, mo, d) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let mut i = 10;
    let (mut h, mut mi, mut se, mut frac_ms) = (0i64, 0i64, 0i64, 0f64);
    if b.get(i).is_some_and(|&c| c == b'T' || c == b' ') {
        i += 1;
        if i + 5 > b.len() || b.get(i + 2) != Some(&b':') {
            return None;
        }
        (h, mi) = (num(i, 2)?, num(i + 3, 2)?);
        i += 5;
        if b.get(i) == Some(&b':') {
            i += 1;
            se = num(i, 2)?;
            i += 2;
            if b.get(i) == Some(&b'.') {
                i += 1;
                let start = i;
                while b.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
                frac_ms =
                    s[start..i].parse::<f64>().ok()? / 10f64.powi((i - start) as i32) * 1000.0;
            }
        }
    }
    let offset_ms;
    match b.get(i) {
        None => offset_ms = 0,
        Some(&b'Z') => {
            i += 1;
            offset_ms = 0;
        }
        Some(&c) if c == b'+' || c == b'-' => {
            let sign = if c == b'+' { -1i64 } else { 1i64 };
            i += 1;
            let oh = num(i, 2)?;
            let om = if b.get(i + 2) == Some(&b':') {
                num(i + 3, 2)?
            } else {
                num(i + 2, 2).unwrap_or(0)
            };
            offset_ms = sign * (oh * 60 + om) * 60_000;
            i = b.len();
        }
        _ => return None,
    }
    if i != b.len() || h > 24 || mi > 59 || se > 60 {
        return None;
    }
    let y_adj = if mo <= 2 { y - 1 } else { y };
    let era = y_adj.div_euclid(146_097);
    let yoe = y_adj - era * 146_097;
    let m_adj = if mo > 2 { mo - 3 } else { mo + 9 };
    let doy = (153 * m_adj + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(
        (days * 86_400_000 + h * 3_600_000 + mi * 60_000 + se * 1_000) as f64
            + frac_ms
            + offset_ms as f64,
    )
}

fn allowed() -> Value {
    json!({"allowed": true, "reason": "allowed"})
}

fn denied(reason: &str) -> Value {
    json!({"allowed": false, "reason": reason})
}

/// Port of `decideRemoteBillingAccess` (`apps/api/src/billing/accessPolicy.ts:26`).
pub fn decide_remote_billing_access(input: Option<Value>) -> Result<Value, String> {
    // `input = {}` applies on missing/undefined; null crashes like JS.
    let input = match input {
        None => json!({}),
        Some(v) if js::is_undefined(&v) => json!({}),
        Some(Value::Null) => {
            return Err(
                "Cannot read properties of null (reading 'enforcementEnabled')".to_string(),
            );
        }
        Some(v) => v,
    };
    if input.get("enforcementEnabled") != Some(&Value::Bool(true)) {
        return Ok(allowed());
    }
    let status = input.get("status").filter(|v| !js::is_undefined(v));
    let status_obj = status.and_then(Value::as_object);
    if status.and_then(|s| s.get("available")) != Some(&Value::Bool(true)) {
        return Ok(denied("billing_unavailable"));
    }
    let now_ms = number_or_default(input.get("nowMs"), real_now_ms());
    let max_age_ms = number_or_default(
        input.get("maxStatusAgeMs"),
        REMOTE_BILLING_MAX_STATUS_AGE_MS,
    );
    let checked_at_ms = status_obj
        .and_then(|o| o.get("checkedAt"))
        .filter(|v| {
            !v.is_null()
                && !js::is_undefined(v)
                && *v != &Value::Bool(false)
                && *v != &Value::from(0)
                && *v != &Value::from("")
        })
        .map(date_parse)
        .unwrap_or(f64::NAN);
    if !is_safe_integer(now_ms)
        || !is_safe_integer(max_age_ms)
        || max_age_ms <= 0.0
        || max_age_ms > REMOTE_BILLING_MAX_STATUS_AGE_MS
        || !checked_at_ms.is_finite()
        || checked_at_ms > now_ms
        || now_ms - checked_at_ms >= max_age_ms
    {
        return Ok(denied("billing_unavailable"));
    }
    let access = status_obj
        .and_then(|o| o.get("access"))
        .and_then(Value::as_str);
    if access != Some("granted") {
        return Ok(denied(if access == Some("none") {
            "subscription_required"
        } else {
            "billing_unavailable"
        }));
    }
    let valid_until = status_obj.and_then(|o| o.get("validUntil"));
    if valid_until != Some(&Value::Null) {
        // `undefined` still enters the check and parses to NaN.
        let valid_until_ms = valid_until.map(date_parse).unwrap_or(f64::NAN);
        if !valid_until_ms.is_finite() || valid_until_ms <= now_ms {
            return Ok(denied("billing_unavailable"));
        }
    }
    Ok(allowed())
}

/// Port of `isProfileBillingEntitled` (`apps/api/src/billing/accessPolicy.ts:86`).
pub fn is_profile_billing_entitled(input: Value) -> Result<bool, String> {
    match &input {
        Value::Null => {
            return Err(
                "Cannot read properties of null (reading 'enforcementEnabled')".to_string(),
            );
        }
        v if js::is_undefined(v) => {
            return Err(
                "Cannot read properties of undefined (reading 'enforcementEnabled')".to_string(),
            );
        }
        _ => {}
    }
    if input.get("enforcementEnabled") != Some(&Value::Bool(true)) {
        return Ok(true);
    }
    let stored = input.get("stored").filter(|v| !js::is_undefined(v));
    if stored.and_then(|s| s.get("accessGranted")) != Some(&Value::Bool(true)) {
        return Ok(false);
    }
    let valid_until = stored.and_then(|s| s.get("validUntil"));
    if valid_until == Some(&Value::Null) {
        return Ok(true);
    }
    let valid_until_ms = valid_until.map(date_parse).unwrap_or(f64::NAN);
    let now_ms = number_or_default(input.get("nowMs"), real_now_ms());
    Ok(valid_until_ms.is_finite() && valid_until_ms + PROFILE_RENEWAL_GRACE_MS > now_ms)
}
