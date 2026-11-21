//! The Rust port of `apps/api/src/clientIp.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::{js, utf16};

const DEFAULT_TRUSTED_PROXY_HOPS: f64 = 0.0;
const MAX_FORWARDED_USER_AGENT_LENGTH: usize = 256;

fn js_trim(s: &str) -> &str {
    s.trim_matches(jscore::is_space)
}

/// Port of `clientIp` (`apps/api/src/clientIp.ts:61`).
pub fn client_ip(c: serde_json::Value, trusted_proxy_hops: f64) -> Result<String, String> {
    let _ = &trusted_proxy_hops;
    if c.is_null() {
        return Err("Cannot read properties of null (reading 'req')".to_string());
    }
    if js::is_undefined(&c) {
        return Err("Cannot read properties of undefined (reading 'req')".to_string());
    }
    match c.get("req") {
        None => Err("Cannot read properties of undefined (reading 'header')".to_string()),
        Some(Value::Null) => Err("Cannot read properties of null (reading 'header')".to_string()),
        // `c.req.header` is a function in a real Hono context, which JSON inputs
        // cannot carry; any other value makes the call a TypeError.
        Some(_) => Err("c.req.header is not a function".to_string()),
    }
}

/// `Number.parseInt(value, 10)`: leading integer digits, `NaN` when none.
fn js_parse_int10(s: &str) -> f64 {
    let t = s.trim_start_matches(jscore::is_space);
    let mut end = 0;
    for (i, c) in t.char_indices() {
        if i == 0 && (c == '+' || c == '-') {
            end = 1;
            continue;
        }
        if c.is_ascii_digit() {
            end = i + 1;
        } else {
            break;
        }
    }
    let head = &t[..end];
    if head.is_empty() || head == "+" || head == "-" {
        f64::NAN
    } else {
        head.parse().unwrap_or(f64::NAN)
    }
}

/// Port of `resolveTrustedProxyHops` (`apps/api/src/clientIp.ts:85`).
pub fn resolve_trusted_proxy_hops(value: serde_json::Value) -> Result<f64, String> {
    if js::is_undefined(&value) {
        return Ok(DEFAULT_TRUSTED_PROXY_HOPS);
    }
    if value.is_null() {
        return Err("Cannot read properties of null (reading 'trim')".to_string());
    }
    let Value::String(s) = &value else {
        return Err("value.trim is not a function".to_string());
    };
    let trimmed = js_trim(s);
    if trimmed.is_empty() {
        return Ok(DEFAULT_TRUSTED_PROXY_HOPS);
    }
    let parsed = js_parse_int10(trimmed);
    if !parsed.is_finite()
        || parsed.fract() != 0.0
        || parsed < 0.0
        || jscore::number_to_string(parsed) != trimmed
    {
        return Ok(DEFAULT_TRUSTED_PROXY_HOPS);
    }
    Ok(parsed)
}

/// Port of `sanitizeForwardableIp` (`apps/api/src/clientIp.ts:101`).
pub fn sanitize_forwardable_ip(ip: String) -> Result<serde_json::Value, String> {
    if ip == "unknown" || utf16::units(&ip).len() > 45 {
        return Ok(js::undefined());
    }
    if !ip.is_empty()
        && ip
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
    {
        return Ok(Value::String(ip));
    }
    Ok(js::undefined())
}

/// Port of `sanitizeForwardableUserAgent` (`apps/api/src/clientIp.ts:111`).
pub fn sanitize_forwardable_user_agent(
    user_agent: Option<String>,
) -> Result<serde_json::Value, String> {
    let Some(ua) = user_agent else {
        return Ok(js::undefined());
    };
    let trimmed = js_trim(&ua);
    if trimmed.is_empty() {
        return Ok(js::undefined());
    }
    let units = utf16::units(trimmed);
    Ok(Value::String(utf16::slice(
        &units,
        0,
        MAX_FORWARDED_USER_AGENT_LENGTH.min(units.len()),
    )))
}
