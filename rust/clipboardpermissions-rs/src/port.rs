//! The Rust port of `apps/desktop/src/clipboardPermissions.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

const TRUSTED_SCHEMES: [&str; 5] = [
    "synara",
    "synara-canary",
    "synara-cua",
    "synara-beta",
    "synara-remote",
];

fn member<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => js::is_undefined(v),
        Value::Number(_) => js::as_number(v).is_some_and(|n| n == 0.0 || n.is_nan()),
    }
}

/// `new URL(value)` — coerces its argument to a string first.
fn parse_url(v: &Value) -> Option<url::Url> {
    let text = match v {
        Value::String(s) => s.clone(),
        Value::Number(_) => js::number_to_string(js::as_number(v).unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        _ => return None,
    };
    url::Url::parse(&text).ok()
}

/// WHATWG origin of a parsed URL; opaque-scheme URLs serialize as "null".
fn url_origin(u: &url::Url) -> String {
    match u.origin() {
        url::Origin::Tuple(scheme, host, port) => format!("{scheme}://{host}:{port}"),
        _ => "null".to_string(),
    }
}

fn allowed_inner(
    requester: &Value,
    details: &Value,
    requesting_origin: &Option<String>,
) -> Result<bool, String> {
    // `requester.isDestroyed()` runs outside the try block; a missing method
    // throws and propagates out of the function.
    let is_destroyed = member(requester, "isDestroyed");
    match is_destroyed {
        Some(Value::Bool(b)) => {
            if *b {
                return Ok(false);
            }
        }
        Some(_) | None => {
            return Err("requester.isDestroyed is not a function".to_string());
        }
    }
    if member(details, "isMainFrame") == Some(&Value::Bool(false)) {
        return Ok(false);
    }
    // try { ... } catch { return false; }
    let page = match member(requester, "getURL") {
        Some(Value::String(s)) => match url::Url::parse(s) {
            Ok(u) => u,
            Err(_) => return Ok(false),
        },
        Some(_) => return Ok(false), // getURL() non-string -> URL ctor throws -> catch
        None => return Ok(false),    // getURL is not a function -> TypeError -> catch
    };
    let trusted_scheme = TRUSTED_SCHEMES.contains(&page.scheme());
    if trusted_scheme
        && (page.host_str() != Some("app")
            || !page.username().is_empty()
            || !page.password().is_none_or(|p| p.is_empty()))
    {
        return Ok(false);
    }
    let localhost_http = page.scheme() == "http"
        && matches!(
            page.host_str(),
            Some("localhost") | Some("127.0.0.1") | Some("[::1]")
        );
    if page.scheme() != "https" && !trusted_scheme && !localhost_http {
        return Ok(false);
    }
    let mut origins: Vec<&Value> = Vec::new();
    let requesting = requesting_origin
        .as_ref()
        .map(|s| Value::String(s.clone()))
        .unwrap_or_else(js::undefined);
    origins.push(Box::leak(Box::new(requesting)));
    if let Some(v) = member(details, "requestingUrl") {
        origins.push(v);
    }
    if let Some(v) = member(details, "embeddingOrigin") {
        origins.push(v);
    }
    for origin in origins {
        if falsy(origin) {
            continue;
        }
        let source = match parse_url(origin) {
            Some(u) => u,
            None => return Ok(false),
        };
        let ok = if trusted_scheme {
            source.scheme() == page.scheme()
                && source.host_str() == page.host_str()
                && source.port_or_known_default() == page.port_or_known_default()
                && source.username().is_empty()
                && source.password().is_none_or(|p| p.is_empty())
        } else {
            url_origin(&source) == url_origin(&page)
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Port of `isClipboardWritePermission` (`apps/desktop/src/clipboardPermissions.ts:11`).
pub fn is_clipboard_write_permission(
    requester: Option<serde_json::Value>,
    permission: String,
    details: serde_json::Value,
    requesting_origin: Option<String>,
) -> Result<bool, String> {
    if permission != "clipboard-sanitized-write" {
        return Ok(false);
    }
    let requester = match &requester {
        Some(v) if !falsy(v) => v,
        _ => return Ok(false),
    };
    allowed_inner(requester, &details, &requesting_origin)
}
