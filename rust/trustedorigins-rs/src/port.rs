//! The Rust port of `apps/server/src/trustedOrigins.ts`.
//!
//! URL parsing uses the WHATWG `url` crate, matching `new URL()`.

#![forbid(unsafe_code)]

use serde_json::Value;

const DESKTOP_APP_CORS_ORIGINS: [&str; 5] = [
    "synara://app",
    "synara-canary://app",
    "synara-cua://app",
    "synara-beta://app",
    "synara-remote://app",
];

fn js_ws(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}'
}

/// `v.key` in JS: throws on null/undefined receivers, otherwise
/// property lookup (undefined for anything not present).
fn prop(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if crate::js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        _ if crate::js::is_undefined(v) => false,
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        _ => match crate::js::as_number(v) {
            Some(n) => n != 0.0 && !n.is_nan(),
            None => true,
        },
    }
}

/// JS `ToString` for the values the recorder can emit.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if x.is_null() || crate::js::is_undefined(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ if crate::js::is_undefined(v) => "undefined".to_string(),
        _ => {
            if let Some(n) = crate::js::as_number(v) {
                crate::js::number_to_string(n)
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// JS `===` on wire values (objects never equal distinct objects).
fn strict_eq(a: &Value, b: &Value) -> bool {
    if crate::js::is_undefined(a) || crate::js::is_undefined(b) {
        return crate::js::is_undefined(a) && crate::js::is_undefined(b);
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => match (crate::js::as_number(a), crate::js::as_number(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
    }
}

fn is_loopback_host(host: &Value) -> Result<bool, String> {
    if !truthy(host) {
        return Ok(true);
    }
    let Value::String(h) = host else {
        return Err("host.startsWith is not a function".to_string());
    };
    let normalized = h
        .strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .unwrap_or(h.as_str());
    Ok(normalized == "localhost" || normalized == "127.0.0.1" || normalized == "::1")
}

fn is_wildcard_host(host: &Value) -> bool {
    matches!(
        host,
        Value::String(h) if h == "0.0.0.0" || h == "::" || h == "[::]"
    )
}

fn normalize_host_for_comparison(host: &Value) -> Result<String, String> {
    let Value::String(h) = host else {
        return Err("host.startsWith is not a function".to_string());
    };
    Ok(h.strip_prefix('[')
        .and_then(|x| x.strip_suffix(']'))
        .unwrap_or(h.as_str())
        .to_lowercase())
}

/// `new URL(x)` — WHATWG parse of the JS `ToString` of the value.
fn parse_url(v: &Value) -> Option<url::Url> {
    url::Url::parse(&js_to_string(v)).ok()
}

/// `new URL(x).origin` — "null" for opaque origins.
fn url_origin(v: &Value) -> Option<String> {
    parse_url(v).map(|u| u.origin().ascii_serialization())
}

/// `new URL(x).hostname` — empty for hostless URLs.
fn url_hostname(v: &Value) -> Option<String> {
    parse_url(v).map(|u| u.host_str().unwrap_or("").to_string())
}

/// `v?.origin` — undefined on null/undefined, a plain property read
/// otherwise (undefined for anything without an `origin` member).
fn opt_origin(v: &Value) -> Value {
    match v {
        Value::Object(m) => m
            .get("origin")
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    }
}

/// Port of `normalizeCorsOrigin` (`apps/server/src/trustedOrigins.ts:27`).
pub fn normalize_cors_origin(raw_origin: serde_json::Value) -> Result<serde_json::Value, String> {
    let value = match &raw_origin {
        Value::Array(a) if a.len() != 1 => return Ok(Value::Null),
        Value::Array(a) => a[0].clone(),
        _ => raw_origin,
    };
    let trimmed = match &value {
        Value::String(s) => Some(s.trim_matches(js_ws).to_string()),
        Value::Null => None,
        _ if crate::js::is_undefined(&value) => None,
        _ => return Err("value?.trim is not a function".to_string()),
    };
    let Some(trimmed) = trimmed else {
        return Ok(Value::Null);
    };
    if trimmed.is_empty() || trimmed == "null" {
        return Ok(Value::Null);
    }
    let normalized_desktop = trimmed.trim_end_matches('/');
    if DESKTOP_APP_CORS_ORIGINS.contains(&normalized_desktop) {
        return Ok(Value::String(normalized_desktop.to_string()));
    }
    match url_origin(&Value::String(trimmed)) {
        Some(o) if o != "null" => Ok(Value::String(o)),
        _ => Ok(Value::Null),
    }
}

/// `isTrustedRequestOriginHost` (`apps/server/src/trustedOrigins.ts:53`).
fn is_trusted_request_origin_host(request_origin: &Value, config: &Value) -> Result<bool, String> {
    if truthy(&prop(config, "publicUrl")?) && !is_loopback_host(&prop(config, "host")?)? {
        return Ok(false);
    }
    let Some(request_host) = url_hostname(request_origin) else {
        return Ok(false);
    };
    if is_loopback_host(&Value::String(request_host.clone()))? {
        return Ok(true);
    }
    let host = prop(config, "host")?;
    if !truthy(&host) {
        return Ok(false);
    }
    if is_wildcard_host(&host) {
        return Ok(true);
    }
    Ok(normalize_host_for_comparison(&Value::String(request_host))?
        == normalize_host_for_comparison(&host)?)
}

/// Port of `isTrustedAppOrigin` (`apps/server/src/trustedOrigins.ts:78`).
pub fn is_trusted_app_origin(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let origin = prop(&input, "origin")?;
    let request_origin = prop(&input, "requestOrigin")?;
    let config = prop(&input, "config")?;
    // `||` short-circuits each property read, matching JS laziness.
    let trusted = !truthy(&origin)
        || strict_eq(&origin, &opt_origin(&prop(&config, "publicUrl")?))
        || (strict_eq(&origin, &request_origin)
            && is_trusted_request_origin_host(&request_origin, &config)?)
        || strict_eq(&origin, &opt_origin(&prop(&config, "devUrl")?))
        || matches!(&origin, Value::String(o) if DESKTOP_APP_CORS_ORIGINS.contains(&o.as_str()));
    Ok(Value::Bool(trusted))
}

/// Port of `shouldRejectUntrustedRequestOrigin` (`apps/server/src/trustedOrigins.ts:96`).
pub fn should_reject_untrusted_request_origin(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let raw_origin = prop(&input, "rawOrigin")?;
    if crate::js::is_undefined(&raw_origin) {
        return Ok(Value::Bool(false));
    }
    let origin = normalize_cors_origin(raw_origin)?;
    let reject = !truthy(&origin)
        || !truthy(&is_trusted_app_origin(serde_json::json!({
            "origin": origin,
            "requestOrigin": prop(&input, "requestOrigin")?,
            "config": prop(&input, "config")?,
        }))?);
    Ok(Value::Bool(reject))
}

/// Port of `shouldRejectAuthMutationOrigin` (`apps/server/src/trustedOrigins.ts:115`).
pub fn should_reject_auth_mutation_origin(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let raw_origin = prop(&input, "rawOrigin")?;
    if crate::js::is_undefined(&raw_origin) {
        let credential_source = prop(&input, "credentialSource")?;
        return Ok(Value::Bool(!strict_eq(
            &credential_source,
            &Value::String("bearer".to_string()),
        )));
    }
    should_reject_untrusted_request_origin(input)
}

/// Port of `requiresWebSocketAuthentication` (`apps/server/src/trustedOrigins.ts:128`).
pub fn requires_web_socket_authentication(config: serde_json::Value) -> Result<bool, String> {
    Ok(truthy(&prop(&config, "authToken")?)
        || truthy(&prop(&config, "publicUrl")?)
        || !is_loopback_host(&prop(&config, "host")?)?
        || !crate::js::is_undefined(&prop(&config, "sshForwardPort")?))
}
