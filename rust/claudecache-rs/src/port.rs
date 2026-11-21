//! The Rust port of `packages/shared/src/claudeCache.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let yoe = y.rem_euclid(400);
    let doy = (153 * (m + 9).rem_euclid(12) + 2) / 5 + d - 1;
    y.div_euclid(400) * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

/// America/Los_Angeles offset in hours (this machine's TZ): -7 in PDT, -8 otherwise.
fn pacific_offset_hours(y: i64, m: i64, d: i64, h: i64) -> f64 {
    let sun = |mo: i64, n: i64| {
        let f = days_from_civil(y, mo, 1);
        if n > 0 {
            (7 - f.rem_euclid(7)).rem_euclid(7) + 7 * n - 6
        } else {
            let dim = days_from_civil(y, mo + 1, 1) - f;
            dim - (f + dim - 1).rem_euclid(7)
        }
    };
    let (sm, sd, em, ed) = if y >= 2007 {
        (3, sun(3, 2), 11, sun(11, 1))
    } else if y >= 1987 {
        (4, sun(4, 1), 10, sun(10, -1))
    } else {
        (4, sun(4, -1), 10, sun(10, -1))
    };
    let t = days_from_civil(y, m, d) * 24 + h;
    let pdt = t >= days_from_civil(y, sm, sd) * 24 + 2 && t < days_from_civil(y, em, ed) * 24 + 2;
    if pdt { -7.0 } else { -8.0 }
}

fn civil_ms(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: f64) -> f64 {
    (days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60) as f64 * 1000.0 + s * 1000.0
}

fn month_name(s: &str) -> Option<i64> {
    const M: &str =
        "january february march april may june july august september october november december";
    let l = s.to_ascii_lowercase();
    (l.len() >= 3).then_some(&l).and_then(|l| {
        M.split(' ')
            .position(|m| m.starts_with(l.as_str()) || l.starts_with(&m[..3]))
            .map(|i| i as i64 + 1)
    })
}

fn time_part(s: &str) -> Option<(i64, i64, f64)> {
    let p: Vec<&str> = s.split(':').collect();
    let h = p.first()?.parse().ok()?;
    let mi = p.get(1)?.parse().ok()?;
    let sec = p.get(2).map_or(Ok(0.0), |x| x.parse()).ok()?;
    Some((h, mi, sec))
}

/// JS `Date.parse` subset for the shapes callers actually send.
fn js_date_parse(s: &str) -> f64 {
    let s = s.trim();
    // ISO: YYYY[-MM[-DD]][T HH:MM[:SS[.fff]]][offset]; date-only is UTC, time without offset is local.
    let (date_s, rest) = s.split_once(['T', 't', ' ']).unwrap_or((s, ""));
    let p: Vec<&str> = date_s.split('-').collect();
    let iso = (!p.is_empty() && p.len() <= 3 && p[0].len() == 4)
        .then_some(p[0].parse::<i64>().ok())
        .flatten()
        .and_then(|y| {
            Some((
                y,
                p.get(1).map_or(Some(1), |x| x.parse().ok())?,
                p.get(2).map_or(Some(1), |x| x.parse().ok())?,
            ))
        });
    if let Some((y, mo, d)) = iso.filter(|&(_, m, d)| m != 0 && d != 0) {
        if s.len() == date_s.len() {
            return civil_ms(y, mo, d, 0, 0, 0.0);
        }
        let (time_s, off_s) = match rest.split_once(['Z', 'z', '+', '-']) {
            Some((t, _)) => (t, Some(&rest[t.len()..])),
            None => (rest, None),
        };
        let Some((h, mi, sec)) = time_part(time_s) else {
            return f64::NAN;
        };
        let off = match off_s {
            None => pacific_offset_hours(y, mo, d, h),
            Some(o) if o.eq_ignore_ascii_case("z") => 0.0,
            Some(o) => {
                let (sgn, o) = (&o[..1], o[1..].replace(':', ""));
                if o.len() != 4 || !o.bytes().all(|b| b.is_ascii_digit()) {
                    return f64::NAN;
                }
                (o[..2].parse::<f64>().unwrap_or(0.0) + o[2..].parse::<f64>().unwrap_or(0.0) / 60.0)
                    * if sgn == "-" { -1.0 } else { 1.0 }
            }
        };
        return civil_ms(y, mo, d, h, mi, sec) - off * 3_600_000.0;
    }
    // US numeric: MM/DD/YYYY [HH:MM[:SS]] — local.
    let (date_s, time_s) = s.split_once(' ').unwrap_or((s, ""));
    let p: Vec<&str> = date_s.split('/').collect();
    if p.len() == 3
        && let (Ok(mo), Ok(d), Ok(y)) = (
            p[0].parse::<i64>(),
            p[1].parse::<i64>(),
            p[2].parse::<i64>(),
        )
    {
        let tp = if time_s.is_empty() {
            Some((0, 0, 0.0))
        } else {
            time_part(time_s)
        };
        let Some((h, mi, sec)) = tp else {
            return f64::NAN;
        };
        return civil_ms(y, mo, d, h, mi, sec) - pacific_offset_hours(y, mo, d, h) * 3_600_000.0;
    }
    // English: "Mon DD [YYYY] [time]" or "DD Mon YYYY" — local; JSC falls back to year 2001.
    let t: Vec<&str> = s.split_whitespace().collect();
    let Some(mo_i) = t.iter().take(2).position(|x| month_name(x).is_some()) else {
        return f64::NAN;
    };
    let mo = month_name(t[mo_i]).unwrap_or(1);
    let d: i64 = t
        .get(1 - mo_i)
        .and_then(|x| x.trim_end_matches(',').parse().ok())
        .unwrap_or(-1);
    let mut y: i64 = 2001;
    let mut ti = 2;
    if let Some(yy) = t.get(2).and_then(|x| x.parse().ok()) {
        y = yy;
        ti = 3;
    }
    let Some((h, mi, sec)) = (match t.get(ti) {
        Some(ts) => time_part(ts),
        None => Some((0, 0, 0.0)),
    }) else {
        return f64::NAN;
    };
    civil_ms(y, mo, d, h, mi, sec) - pacific_offset_hours(y, mo, d, h) * 3_600_000.0
}

/// JS `Number(v)`.
fn js_number(v: &Value) -> f64 {
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => i32::from(*b) as f64,
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                return 0.0;
            }
            let (neg, t) = t
                .strip_prefix('-')
                .map(|r| (true, r))
                .unwrap_or((false, t.strip_prefix('+').unwrap_or(t)));
            let n = if t.eq_ignore_ascii_case("infinity") {
                f64::INFINITY
            } else if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                i64::from_str_radix(h, 16).map_or(f64::NAN, |x| x as f64)
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            };
            if neg { -n } else { n }
        }
        Value::Array(a) => match a.len() {
            0 => 0.0,
            1 => js_number(&a[0]),
            _ => f64::NAN,
        },
        _ => f64::NAN,
    }
}

/// Port of `assessClaudeCache` (`packages/shared/src/claudeCache.ts:10`).
pub fn assess_claude_cache(
    observation: Option<serde_json::Value>,
    now_ms: f64,
) -> Result<serde_json::Value, String> {
    let unknown = || Ok(json!({"state": "unknown", "requiresConfirmation": false}));
    let Some(obs) = observation.as_ref().filter(|o| o.is_object()) else {
        return unknown();
    };
    let get = |k: &str| obs.get(k).unwrap_or(&Value::Null);
    let date = |v: &Value| v.as_str().map(js_date_parse);
    let (Some(obs_ms), Some(resp_ms)) = (date(get("observedAt")), date(get("lastResponseAt")))
    else {
        return unknown();
    };
    let ref_ms = match obs.get("cacheReferenceAt") {
        None | Some(Value::Null) => resp_ms,
        v => match v.and_then(|v| v.as_str().map(js_date_parse)) {
            Some(m) => m,
            None => return unknown(),
        },
    };
    if [obs_ms, resp_ms, ref_ms].iter().any(|m| m.is_nan())
        || !(ref_ms <= resp_ms && resp_ms <= obs_ms && obs_ms <= now_ms)
    {
        return unknown();
    }
    let idle = ((now_ms - resp_ms) / 1000.0).floor();
    let requires_confirmation = idle >= 4200.0 && js_number(get("contextTokens")) >= 100_000.0;
    let state = if get("state") == "likely-expired" {
        "likely-expired"
    } else {
        match obs.get("ttlSeconds") {
            None => "unknown",
            Some(raw) => {
                let elapsed = ((now_ms - ref_ms) / 1000.0).floor();
                let partial = obs
                    .get("partialTtlSeconds")
                    .map_or(f64::INFINITY, js_number);
                if elapsed >= js_number(raw) {
                    "likely-expired"
                } else if elapsed >= partial {
                    "unknown"
                } else {
                    "likely-warm"
                }
            }
        }
    };
    Ok(json!({
        "state": state,
        "idleSeconds": crate::js::number(idle),
        "requiresConfirmation": requires_confirmation,
    }))
}
