//! The Rust port of `packages/shared/src/pendingInteractions.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

const GRACE_MS: f64 = 30_000.0;
const MS_PER_DAY: f64 = 86_400_000.0;

/// V8 `Date.parse` for ISO-8601 shapes: `[sign]YYYY[-MM[-DD]][T|␣HH:mm[:ss[.frac]]][Z|±HH[:mm]]`.
/// Out-of-range days normalize like V8 (Feb 30 → Mar 1); month 13 or hour > 24 is NaN.
fn date_parse(s: &str) -> Option<f64> {
    let s = s.trim();
    let (s, sign, signed, mut i) = if let Some(r) = s.strip_prefix(['+', '-']) {
        (r, if s.starts_with('-') { -1i64 } else { 1 }, true, 0usize)
    } else {
        (s, 1, false, 0)
    };
    let year_digits = if signed { 6 } else { 4 };
    let year: i64 = {
        let end = i + year_digits;
        if s.len() < end || !s.as_bytes()[i..end].iter().all(|b| b.is_ascii_digit()) {
            return None;
        }
        i = end;
        sign * s[i - year_digits..i].parse::<i64>().ok()?
    };
    let mut ymd = [1i64, 1, 1];
    let mut part = 1; // month then day
    while part <= 2 {
        if s.as_bytes().get(i) != Some(&b'-') {
            break;
        }
        let rest = &s[i + 1..];
        let digits: String = rest
            .bytes()
            .take_while(|b| b.is_ascii_digit())
            .map(|b| b as char)
            .collect();
        if digits.len() != 2 {
            return None;
        }
        i += 1 + digits.len();
        ymd[part] = digits.parse().ok()?;
        part += 1;
    }
    let (mut h, mut mi, mut sec, mut ms) = (0i64, 0, 0, 0f64);
    let mut offset_min: i64 = 0;
    if i < s.len() {
        if s.as_bytes()[i] != b'T' && s.as_bytes()[i] != b' ' {
            return None;
        }
        i += 1;
        let rest = &s[i..];
        // Split at the zone: Z or +/-hh[:mm]; the '-' here is a zone sign, not a separator.
        let (time_part, zone_part) = match rest.find(['Z', '+', '-']) {
            Some(z) => (&rest[..z], &rest[z..]),
            None => (rest, ""),
        };
        let tparts: Vec<&str> = time_part.split(':').collect();
        if tparts.len() < 2 || tparts.len() > 3 {
            return None;
        }
        for (idx, p) in tparts.iter().enumerate() {
            let digits = p
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>();
            if idx == 2 && digits != *p {
                let (si, fr) = p.split_once('.')?;
                if si.is_empty() || fr.is_empty() {
                    return None;
                }
                sec = si.parse().ok()?;
                let mut m = fr.chars().take(3).collect::<String>();
                while m.len() < 3 {
                    m.push('0');
                }
                if !fr.chars().all(|c| c.is_ascii_digit()) {
                    return None;
                }
                ms = m.parse().ok()?;
                continue;
            }
            if digits != *p || digits.is_empty() || digits.len() > 2 {
                return None;
            }
            match idx {
                0 => h = digits.parse().ok()?,
                1 => mi = digits.parse().ok()?,
                _ => sec = digits.parse().ok()?,
            }
        }
        if !zone_part.is_empty() && zone_part != "Z" {
            {
                let z = zone_part.strip_prefix(['+', '-'])?;
                let zsign = if zone_part.starts_with('-') { -1 } else { 1 };
                let zh: i64;
                let zm: i64;
                match z.split_once(':') {
                    Some((a, b)) => {
                        zh = a.parse().ok()?;
                        zm = b.parse().ok()?;
                    }
                    None => {
                        if z.len() == 4 {
                            zh = z[..2].parse().ok()?;
                            zm = z[2..].parse().ok()?;
                        } else if z.len() <= 2 {
                            zh = z.parse().ok()?;
                            zm = 0;
                        } else {
                            return None;
                        }
                    }
                }
                if zh > 23 || zm > 59 {
                    return None;
                }
                offset_min = zsign * (zh * 60 + zm);
            }
        }
    }
    if ymd[1] < 1 || ymd[1] > 12 || ymd[2] < 1 || ymd[2] > 31 || h > 24 || mi > 59 || sec > 59 {
        return None;
    }
    if h == 24 && (mi != 0 || sec != 0 || ms != 0.0) {
        return None;
    }
    let days = days_from_civil(year, ymd[1], ymd[2]);
    let t = days as f64 * MS_PER_DAY + ((h * 60 + mi - offset_min) * 60 + sec) as f64 * 1000.0 + ms;
    (t.abs() <= 8.64e15).then_some(t)
}

/// V8's legacy fallback: a trailing `-N` (one or two digits) reads as a short
/// year — `0-49` → 2000+N, `50-99` → 1900+N — at local midnight (PST on the
/// recording box, UTC-8).
fn legacy_date_parse(s: &str) -> Option<f64> {
    let (head, n) = s.rsplit_once('-')?;
    if head.is_empty() || n.is_empty() || n.len() > 2 || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: i64 = n.parse().ok()?;
    let year = if n < 50 { 2000 + n } else { 1900 + n };
    let t = days_from_civil(year, 1, 1) as f64 * MS_PER_DAY + 480.0 * 60_000.0;
    (t.abs() <= 8.64e15).then_some(t)
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

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    (
        y + if mp < 10 { 0 } else { 1 },
        if mp < 10 { mp + 3 } else { mp - 9 },
        doy - (153 * mp + 2) / 5 + 1,
    )
}

/// `new Date(ms).toISOString()`.
fn ms_to_iso(t: f64) -> String {
    let days = (t / MS_PER_DAY).floor() as i64;
    let rem = (t - days as f64 * MS_PER_DAY) as i64;
    let (h, mi, s, ms) = (
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60,
        rem % 1000,
    );
    let (y, m, d) = civil_from_days(days);
    let ys = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else if y < 0 {
        format!("-{abs:06}", abs = -y)
    } else {
        format!("+{y:06}")
    };
    format!("{ys}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{ms:03}Z")
}

/// `Date.parse(v)` — strings parse, other values coerce then parse.
fn parse_arg(v: &Value) -> Option<f64> {
    let s = match v {
        Value::String(s) => s.clone(),
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => utf16_decode(&o["$utf16"]),
        _ => js_to_string(v),
    };
    date_parse(&s).or_else(|| legacy_date_parse(&s))
}

fn utf16_decode(v: &Value) -> String {
    let units: Vec<u16> = v
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_u64().map(|u| u as u16))
                .collect()
        })
        .unwrap_or_default();
    String::from_utf16_lossy(&units)
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        _ => "[object Object]".into(),
    }
}

fn reclaim(at: &Value, delta: f64) -> Value {
    match parse_arg(at) {
        Some(ms) => Value::String(ms_to_iso(ms + delta)),
        None => at.clone(),
    }
}

/// Port of `respondingInteractionReclaimCutoff` (`packages/shared/src/pendingInteractions.ts:90`).
pub fn responding_interaction_reclaim_cutoff(
    requested_at: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(reclaim(&requested_at, -GRACE_MS))
}

/// Port of `respondingInteractionReclaimAt` (`packages/shared/src/pendingInteractions.ts:98`).
pub fn responding_interaction_reclaim_at(
    response_requested_at: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(reclaim(&response_requested_at, GRACE_MS))
}

/// Port of `isPendingInteractionAwaitingResponse` (`packages/shared/src/pendingInteractions.ts:109`).
pub fn is_pending_interaction_awaiting_response(
    status: Option<serde_json::Value>,
) -> Result<bool, String> {
    Ok(matches!(status.as_ref(), Some(Value::String(s)) if s == "pending" || s == "retryable"))
}

/// `obj.key` — nullish receiver throws.
fn member(v: &Value, k: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{k}')")),
        _ if js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        )),
        Value::Object(o) => Ok(o.get(k).cloned().unwrap_or_else(js::undefined)),
        _ => Ok(js::undefined()),
    }
}

fn str_is(v: &Value, s: &str) -> bool {
    matches!(v, Value::String(x) if x == s)
}

/// JS `a <= b` (relational): string compare when both are strings, else numeric.
fn js_le(a: &Value, b: &Value) -> bool {
    if js::is_undefined(a) || js::is_undefined(b) {
        return false;
    }
    let sa = match a {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => {
            Some(utf16_decode(&o["$utf16"]))
        }
        _ => None,
    };
    let sb = match b {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => {
            Some(utf16_decode(&o["$utf16"]))
        }
        _ => None,
    };
    match (sa, sb) {
        (Some(x), Some(y)) => x.encode_utf16().le(&mut y.encode_utf16()),
        _ => {
            let x = to_num(a);
            let y = to_num(b);
            x <= y
        }
    }
}

fn to_num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::String(s) => s.trim().parse().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// Port of `isPendingInteractionResponseClaimable` (`packages/shared/src/pendingInteractions.ts:115`).
pub fn is_pending_interaction_response_claimable(input: serde_json::Value) -> Result<bool, String> {
    let status = member(&input, "status")?;
    if str_is(&status, "pending") || str_is(&status, "retryable") || str_is(&status, "uncertain") {
        return Ok(true);
    }
    if !str_is(&status, "responding") {
        return Ok(false);
    }
    let rra = member(&input, "responseRequestedAt")?;
    if rra.is_null() {
        return Ok(true);
    }
    let cutoff = reclaim(&member(&input, "requestedAt")?, -GRACE_MS);
    Ok(js_le(&rra, &cutoff))
}
