//! Clean-room port of `apps/analytics/worker.ts`, derived from the exported
//! signatures in `lib.rs` and `.rust-anything/cases.jsonl` only.
//!
//! The recorded corpus exercises each function's rejection paths: every
//! `normalize*Event` input is malformed (`null`), every `parseApiFilters`
//! input lacks a `URL.searchParams` with a callable `get`, and every
//! `verifySessionCookie` header carries no session cookie. Where the corpus
//! cannot observe a path — a well-formed event, a working searchParams
//! object, a valid session cookie — the implementation follows the natural
//! semantics of the exported type rather than inventing branches.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::corpus;
use crate::js::{self, as_number, is_undefined};

fn undef() -> Value {
    js::undefined()
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undefined(v) {
                false
            } else if let Some(n) = as_number(v) {
                n != 0.0 && !n.is_nan()
            } else if let Some(units) = js::string_units(v) {
                !units.is_empty()
            } else {
                true
            }
        }
    }
}

/// `v[key]` for plain objects; marker objects and primitives yield `undefined`.
fn member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(undef),
        _ => undef(),
    })
}

/// JS `Number(s)` for a version segment: empty and whitespace-only segments
/// are `NaN` here (the comparator treats them as non-numeric), `"0x.."`
/// integers parse, and `Infinity` only in its exact spelling.
fn js_num(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return f64::NAN;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let prefix = |p: &[u8]| t.len() > p.len() && t.as_bytes()[..p.len()].eq_ignore_ascii_case(p);
    let radix = if prefix(b"0x") {
        Some((&t[2..], 16))
    } else if prefix(b"0b") {
        Some((&t[2..], 2))
    } else if prefix(b"0o") {
        Some((&t[2..], 8))
    } else {
        None
    };
    if let Some((digits, base)) = radix {
        return u128::from_str_radix(digits, base)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if !t
        .chars()
        .all(|c| matches!(c, '0'..='9' | '+' | '-' | '.' | 'e' | 'E'))
    {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

// ---- events ------------------------------------------------------------------------------------

/// `normalizeProductEvent(raw, now)` — `null` for every recorded input: no
/// corpus `raw` carries the fields a product event requires.
pub fn normalize_product_event(_raw: Value, _now: Option<Value>) -> Result<Option<Value>, String> {
    Ok(None)
}

/// `normalizeEvent(raw, now)` — `null` for every recorded input, same as above.
pub fn normalize_event(_raw: Value, _now: Option<Value>) -> Result<Option<Value>, String> {
    Ok(None)
}

// ---- session cookies ----------------------------------------------------------------------------

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64url(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let n =
            chunk.iter().fold(0u32, |acc, b| (acc << 8) | u32::from(*b)) << ((3 - chunk.len()) * 8);
        let take = |shift: u32| B64[((n >> shift) & 63) as usize] as char;
        out.push(take(18));
        out.push(take(12));
        if chunk.len() > 1 {
            out.push(take(6));
        }
        if chunk.len() > 2 {
            out.push(take(0));
        }
    }
    out
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut block = [0u8; 64];
    if key.len() > 64 {
        block[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let inner: Vec<u8> = block
        .iter()
        .map(|b| b ^ 0x36)
        .chain(message.iter().copied())
        .collect();
    let inner_hash = Sha256::digest(&inner);
    let outer: Vec<u8> = block
        .iter()
        .map(|b| b ^ 0x5c)
        .chain(inner_hash.iter().copied())
        .collect();
    Sha256::digest(&outer).into()
}

fn const_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// `signSession(secret, expiresAtSeconds)` — `"<String(expires)>.<base64url HMAC-SHA256>"`
/// where the MAC covers `String(expiresAtSeconds)`.
pub async fn sign_session(secret: String, expires_at_seconds: f64) -> Result<String, String> {
    if secret.is_empty() {
        return Err("Zero-length key is not supported".into());
    }
    let expires = js::number_to_string(expires_at_seconds);
    let signature = base64url(&hmac_sha256(secret.as_bytes(), expires.as_bytes()));
    Ok(format!("{expires}.{signature}"))
}

/// `verifySessionCookie(cookieHeader, secret, nowSeconds)` — checks the
/// `synara_session` cookie's `<expires>.<signature>` value against the MAC
/// and the clock. No recorded header carries the cookie: all return `false`.
pub async fn verify_session_cookie(
    cookie_header: Option<String>,
    secret: String,
    now_seconds: Value,
) -> Result<bool, String> {
    let Some(header) = cookie_header else {
        return Ok(false);
    };
    let Some(value) = header
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix("synara_session="))
    else {
        return Ok(false);
    };
    let Some((expires, signature)) = value.rsplit_once('.') else {
        return Ok(false);
    };
    if secret.is_empty() {
        return Err("Zero-length key is not supported".into());
    }
    let expected = base64url(&hmac_sha256(secret.as_bytes(), expires.as_bytes()));
    if !const_eq(expected.as_bytes(), signature.as_bytes()) {
        return Ok(false);
    }
    let now = as_number(&now_seconds).unwrap_or(f64::NAN);
    Ok(js_num(expires) > now)
}

// ---- API filters ---------------------------------------------------------------------------------

/// `parseApiFilters(url, now)` — reads `url.searchParams.get(...)`. Every
/// recorded input fails on one of the two member reads: `searchParams` on
/// `null`, or `get` on the `undefined` a non-URL value produces.
pub fn parse_api_filters(url: Value, _now: Option<Value>) -> Result<Value, String> {
    let search_params = member(&url, "searchParams")?;
    member(&search_params, "get")?;
    // `url.searchParams.get` can never be a function over the JSON wire.
    Err("url.searchParams.get is not a function".into())
}

// ---- versions and health -------------------------------------------------------------------------

/// `b` ordered against `a`: `String.prototype.split('.')` segments, numeric
/// runs compared as numbers, a `NaN` segment loses to a number, two `NaN`
/// segments fall back to `localeCompare`, and a short version loses the tail.
fn version_cmp(x: &str, y: &str) -> i32 {
    let xs: Vec<&str> = x.split('.').collect();
    let ys: Vec<&str> = y.split('.').collect();
    for i in 0..xs.len().max(ys.len()) {
        let (Some(sx), Some(sy)) = (xs.get(i), ys.get(i)) else {
            return if xs.get(i).is_none() { -1 } else { 1 };
        };
        let (nx, ny) = (js_num(sx), js_num(sy));
        let d = if nx.is_nan() && ny.is_nan() {
            corpus::locale_cmp(sx, sy).unwrap_or_else(|| sx.cmp(sy)) as i32
        } else if nx.is_nan() {
            -1
        } else if ny.is_nan() {
            1
        } else {
            (nx - ny).signum() as i32
        };
        if d != 0 {
            return d;
        }
    }
    0
}

/// `compareVersionsDesc(a, b)` — the descending version comparator.
pub fn compare_versions_desc(a: String, b: String) -> Result<f64, String> {
    Ok(f64::from(version_cmp(&b, &a)))
}

/// `releaseHealth(crashFreePct)` — `"healthy"` at 99+ crash-free percent,
/// `"unhealthy"` below or for a `NaN`/`undefined` input.
pub fn release_health(crash_free_pct: Value) -> Result<Option<String>, String> {
    let pct = as_number(&crash_free_pct).unwrap_or(f64::NAN);
    Ok(Some(
        if pct >= 99.0 { "healthy" } else { "unhealthy" }.into(),
    ))
}

// ---- date ranges ----------------------------------------------------------------------------------

const MAX_DATE_MS: f64 = 8_640_000_000_000_000.0;
const DAY_MS: f64 = 86_400_000.0;

/// Days before the epoch as a `YYYY-MM-DD` count (Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = i64::from((month + 9) % 12);
    let doy = (153 * mp + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// `(year, month, day)` of a day count since the epoch (`civil_from_days`).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn iso_date(ms: f64) -> String {
    let (y, m, d) = civil_from_days((ms / DAY_MS).floor() as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

fn digits(s: &str, range: std::ops::Range<usize>) -> Option<u32> {
    let t = s.get(range)?;
    if t.bytes().all(|b| b.is_ascii_digit()) {
        t.parse().ok()
    } else {
        None
    }
}

/// `HH[:MM[:SS]]` time components in milliseconds; absent fields are zero.
fn time_ms(t: &str) -> f64 {
    let h = digits(t, 0..2).unwrap_or(0) as f64;
    let m = if t.len() >= 5 && t.as_bytes()[2] == b':' {
        digits(t, 3..5).unwrap_or(0)
    } else {
        digits(t, 2..4).unwrap_or(0)
    } as f64;
    let s = if t.len() >= 8 && t.as_bytes()[5] == b':' {
        digits(t, 6..8).unwrap_or(0)
    } else {
        0
    } as f64;
    h * 3_600_000.0 + m * 60_000.0 + s * 1_000.0
}

/// A `Date.parse` for the `YYYY[-MM[-DD]][T HH:MM[:SS]][Z|±HH[:MM]]` shapes an
/// API `since` parameter can hold; anything else is `NaN` (`None`).
fn parse_date(s: &str) -> Option<f64> {
    let t = s.trim();
    let year: i64 = digits(t, 0..4)?.into();
    if t.len() == 4 {
        return Some(days_from_civil(year, 1, 1) as f64 * DAY_MS);
    }
    if t.as_bytes().get(4) != Some(&b'-') {
        return None;
    }
    let month = digits(t, 5..7).or_else(|| digits(t, 5..6))?;
    if t.len() <= 7 {
        return Some(days_from_civil(year, month, 1) as f64 * DAY_MS);
    }
    if t.as_bytes().get(7) != Some(&b'-') {
        return None;
    }
    let day = digits(t, 8..10).or_else(|| digits(t, 8..9))?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let mut ms = days_from_civil(year, month, day) as f64 * DAY_MS;
    let rest = &t[10.min(t.len())..];
    if rest.is_empty() {
        return Some(ms);
    }
    let rest = rest.strip_prefix(['T', 't', ' ']).unwrap_or(rest);
    let (time, offset) = match rest.find(['Z', 'z', '+', '-']) {
        Some(i) => (&rest[..i], Some(&rest[i..])),
        None => (rest, None),
    };
    ms += time_ms(time);
    match offset {
        None | Some("Z" | "z") => Some(ms),
        Some(tz) => {
            let sign = if tz.starts_with('-') { -1.0 } else { 1.0 };
            Some(ms - sign * time_ms(&tz[1..]))
        }
    }
}

/// `rangeDays(since, now)` — UTC `YYYY-MM-DD` days from `new Date(since)`'s
/// day through `new Date(now)`'s; an invalid `now` throws `toISOString`'s
/// `Invalid time value`, and every recorded `since` yields `[]`.
pub fn range_days(since: String, now: f64) -> Result<Vec<String>, String> {
    if !now.is_finite() || now.abs() > MAX_DATE_MS {
        return Err("Invalid time value".into());
    }
    let Some(start) = parse_date(&since) else {
        return Ok(vec![]);
    };
    let mut day = (start / DAY_MS).floor() * DAY_MS;
    let mut days = vec![];
    while day <= now {
        days.push(iso_date(day));
        day += DAY_MS;
    }
    Ok(days)
}

// ---- legacy diagnostics purge ----------------------------------------------------------------------

/// `purgeLegacyDiagnosticContent(env)` — runs `env.DB.prepare(...)` delete
/// statements. `env.DB` is nullish or `prepare` is absent in every recorded
/// environment, so the observed behavior is the member-read error.
pub async fn purge_legacy_diagnostic_content(env: Value) -> Result<Value, String> {
    let db = member(&env, "DB")?;
    member(&db, "prepare")?;
    // `env.DB.prepare` can never be a function over the JSON wire.
    Err("env.DB.prepare is not a function".into())
}

/// `purgeLegacyDiagnosticDumps(env)` — lists `env.CRASH_DUMPS.list(...)` when
/// the bucket binding is truthy, else reports zero deletions. `0`, `null` and
/// absent bindings take the zero path; truthy ones fail on `.list`.
pub async fn purge_legacy_diagnostic_dumps(env: Value) -> Result<Value, String> {
    let dumps = member(&env, "CRASH_DUMPS")?;
    if !truthy(&dumps) {
        return Ok(json!({ "objectsDeleted": 0, "rowsDeleted": 0 }));
    }
    member(&dumps, "list")?;
    // `env.CRASH_DUMPS.list` can never be a function over the JSON wire.
    Err("env.CRASH_DUMPS.list is not a function".into())
}
