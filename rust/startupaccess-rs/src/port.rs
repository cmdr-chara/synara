//! The Rust port of `apps/server/src/startupAccess.ts`.

#![forbid(unsafe_code)]

use crate::js;

/// Port of `isWildcardHost` (`apps/server/src/startupAccess.ts:4`).
pub fn is_wildcard_host(host: Option<String>) -> Result<bool, String> {
    Ok(matches!(
        host.as_deref(),
        Some("0.0.0.0") | Some("::") | Some("[::]")
    ))
}

/// Port of `isLoopbackHost` (`apps/server/src/startupAccess.ts:7`).
pub fn is_loopback_host(host: Option<String>) -> Result<bool, String> {
    let Some(host) = host.as_deref() else {
        return Ok(true);
    };
    if host.is_empty() {
        return Ok(true);
    }
    let normalized = if host.starts_with('[') && host.ends_with(']') && host.len() >= 2 {
        &host[1..host.len() - 1]
    } else {
        host
    };
    Ok(matches!(normalized, "localhost" | "127.0.0.1" | "::1"))
}

/// Port of `formatHostForUrl` (`apps/server/src/startupAccess.ts:13`).
pub fn format_host_for_url(host: String) -> Result<String, String> {
    Ok(if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host
    })
}

/// Port of `resolveListeningPort` (`apps/server/src/startupAccess.ts:16`).
pub fn resolve_listening_port(
    address: serde_json::Value,
    fallback_port: f64,
) -> Result<f64, String> {
    // typeof address === "object" && address !== null && "port" in address &&
    // typeof address.port === "number" — serde objects and arrays are both
    // "object"; a named "port" lookup on an array misses either way.
    if !matches!(
        address,
        serde_json::Value::Object(_) | serde_json::Value::Array(_)
    ) {
        return Ok(fallback_port);
    }
    match address.get("port").and_then(js::as_number) {
        Some(port) => Ok(port),
        None => Ok(fallback_port),
    }
}
