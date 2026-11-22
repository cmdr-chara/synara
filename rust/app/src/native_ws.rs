//! Native implementation of `GET /ws/negotiate` — the WebSocket compatibility
//! handshake from `apps/server/src/wsCompatibility.ts` +
//! `makeWsNegotiateHttpRouteLayer` in `wsRpc.ts`.
//!
//! Reproduces the exact wire contract:
//! - `Cache-Control: no-store` on every response.
//! - 403 `Forbidden` (text) when the request's `Origin` header is present but
//!   does not normalize to a trusted origin.
//! - 426 JSON `WsCompatibilityError` on invalid parameters or failed
//!   negotiation.
//! - 200 JSON `WsBootstrapNegotiateResult` on success.
//! - `Access-Control-Allow-Origin` is reflected only for trusted app origins,
//!   always with `Vary: Origin`.
//!
//! Server config fields consumed (same env as `apps/server/src/main.ts`):
//! `SYNARA_HOST`, `SYNARA_PUBLIC_URL`, `VITE_DEV_SERVER_URL`.

use std::sync::OnceLock;

use axum::Router;
use axum::http::{HeaderMap, HeaderValue, Request, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use serde_json::{Value, json};

use crate::original::Original;

const WS_NEGOTIATE_HTTP_PATH: &str = "/ws/negotiate";

const PROTOCOL_EPOCH: i64 = 1;
const MIN_REVISION: i64 = 3;
const MAX_REVISION: i64 = 3;
const SERVER_BUILD: &str = env!("SYNARA_SERVER_BUILD");

/// `WS_SERVER_CAPABILITIES` from `packages/contracts/src/wsCompatibility.ts`,
/// order preserved.
const SERVER_CAPABILITIES: &[&str] = &[
    "agent-gateway.remote-v1",
    "orchestration.cursor-safe-streams",
    "orchestration.thread-detail-snapshot",
    "rpc.typed-errors",
    "git.worktree-setup-progress",
    "projects.github-provisioning",
    "projects.file-watch",
    "orchestration.turn-dispatch-settlement",
    "orchestration.thread-goal-visible-start",
    "orchestration.usage-limit-recovery",
    "git.action-recovery",
    "server.runtime-status",
    "transport.http-negotiate",
];

const DESKTOP_APP_CORS_ORIGINS: &[&str] = &[
    "synara://app",
    "synara-canary://app",
    "synara-cua://app",
    "synara-beta://app",
    "synara-remote://app",
];

struct Config {
    host: Option<String>,
    public_url: Option<String>,
    dev_url: Option<String>,
}

fn config() -> &'static Config {
    static CONFIG: OnceLock<Config> = OnceLock::new();
    CONFIG.get_or_init(|| Config {
        host: std::env::var("SYNARA_HOST")
            .ok()
            .filter(|v| !v.trim().is_empty()),
        public_url: std::env::var("SYNARA_PUBLIC_URL")
            .ok()
            .filter(|v| !v.trim().is_empty()),
        dev_url: std::env::var("VITE_DEV_SERVER_URL")
            .ok()
            .filter(|v| !v.trim().is_empty()),
    })
}

fn server_instance_id() -> &'static str {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(new_uuid_v4)
}

fn new_uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut bytes);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// `isWildcardHost` — "0.0.0.0" | "::" | "[::]".
fn is_wildcard_host(host: Option<&str>) -> bool {
    matches!(host, Some("0.0.0.0") | Some("::") | Some("[::]"))
}

/// `isLoopbackHost` — undefined counts as loopback; brackets stripped.
fn is_loopback_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return true;
    };
    let normalized = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    normalized == "localhost" || normalized == "127.0.0.1" || normalized == "::1"
}

fn normalize_host(host: &str) -> String {
    host.strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host)
        .to_lowercase()
}

/// Origin part of a URL string (scheme://authority), without pulling in a URL
/// crate. Returns `None` when the value cannot parse as an absolute URL with a
/// scheme and authority.
fn url_origin(url: &str) -> Option<String> {
    let scheme_end = url.find("://")?;
    let scheme = &url[..scheme_end];
    if scheme.is_empty()
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return None;
    }
    let rest = &url[scheme_end + 3..];
    let authority_end = rest
        .find(|c| matches!(c, '/' | '?' | '#'))
        .unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() {
        return None;
    }
    Some(format!("{}://{}", scheme, authority))
}

/// Hostname portion of an absolute URL string.
fn url_hostname(url: &str) -> Option<String> {
    let rest = &url[url.find("://")? + 3..];
    let authority = rest
        .split(|c| matches!(c, '/' | '?' | '#'))
        .next()
        .unwrap_or("");
    // Strip userinfo if present.
    let hostport = authority.rsplit('@').next().unwrap_or(authority);
    if hostport.starts_with('[') {
        let end = hostport.find(']')?;
        return Some(hostport[..=end].to_string());
    }
    Some(hostport.split(':').next().unwrap_or(hostport).to_string())
}

/// `normalizeCorsOrigin`: multi-valued Origin headers never normalize; empty
/// and `null` become `None`; desktop app origins pass through after stripping
/// trailing slashes; everything else reduces to `new URL(value).origin`.
fn normalize_cors_origin(raw: Option<&str>) -> Option<String> {
    let trimmed = raw?.trim();
    if trimmed.is_empty() || trimmed == "null" {
        return None;
    }
    let normalized = trimmed.trim_end_matches('/').to_string();
    if DESKTOP_APP_CORS_ORIGINS.contains(&normalized.as_str()) {
        return Some(normalized);
    }
    let origin = url_origin(trimmed)?;
    if origin == "null" {
        return None;
    }
    Some(origin)
}

/// `isTrustedRequestOriginHost`.
fn is_trusted_request_origin_host(request_origin: &str, cfg: &Config) -> bool {
    if cfg.public_url.is_some() && !is_loopback_host(cfg.host.as_deref()) {
        return false;
    }
    let Some(request_host) = url_hostname(request_origin) else {
        return false;
    };
    if is_loopback_host(Some(request_host.as_str())) {
        return true;
    }
    let Some(host) = cfg.host.as_deref() else {
        return false;
    };
    if is_wildcard_host(Some(host)) {
        return true;
    }
    normalize_host(&request_host) == normalize_host(host)
}

/// `isTrustedAppOrigin`.
fn is_trusted_app_origin(origin: Option<&str>, request_origin: &str, cfg: &Config) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    if cfg.public_url.as_deref().and_then(url_origin).as_deref() == Some(origin) {
        return true;
    }
    if origin == request_origin && is_trusted_request_origin_host(request_origin, cfg) {
        return true;
    }
    if cfg.dev_url.as_deref().and_then(url_origin).as_deref() == Some(origin) {
        return true;
    }
    DESKTOP_APP_CORS_ORIGINS.contains(&origin)
}

/// `shouldRejectUntrustedRequestOrigin`: an absent Origin header is allowed
/// (CLI/non-browser); a present-but-invalid or unrelated one rejects.
fn should_reject_untrusted_request_origin(
    raw_origin: Option<&str>,
    request_origin: &str,
    cfg: &Config,
) -> bool {
    if raw_origin.is_none() {
        return false;
    }
    let origin = normalize_cors_origin(raw_origin);
    match origin {
        None => true,
        Some(ref o) => !is_trusted_app_origin(Some(o.as_str()), request_origin, cfg),
    }
}

/// `negotiateWsCompatibility`: Ok = `WsBootstrapNegotiateResult`-shaped JSON,
/// Err = `WsCompatibilityError`-shaped JSON. Pure over the protocol inputs.
pub fn negotiate(
    epoch: i64,
    min_rev: i64,
    max_rev: i64,
    required: &[String],
) -> Result<Value, Value> {
    if min_rev > max_rev {
        return Err(error_body(
            "The client sent an invalid WebSocket protocol range.",
            "WS_PROTOCOL_INCOMPATIBLE",
            "reload",
        ));
    }
    if epoch != PROTOCOL_EPOCH {
        let action = if epoch < PROTOCOL_EPOCH {
            "update-client"
        } else {
            "update-server"
        };
        return Err(error_body(
            &format!(
                "WebSocket protocol epoch {epoch} is incompatible with server epoch {PROTOCOL_EPOCH}."
            ),
            "WS_PROTOCOL_INCOMPATIBLE",
            action,
        ));
    }
    let low = min_rev.max(MIN_REVISION);
    let negotiated = max_rev.min(MAX_REVISION);
    if negotiated < low {
        let action = if max_rev < MIN_REVISION {
            "update-client"
        } else {
            "update-server"
        };
        return Err(error_body(
            &format!(
                "WebSocket protocol revisions {min_rev}-{max_rev} do not overlap server revisions {MIN_REVISION}-{MAX_REVISION}."
            ),
            "WS_PROTOCOL_INCOMPATIBLE",
            action,
        ));
    }
    let missing: Vec<&str> = required
        .iter()
        .filter(|c| !SERVER_CAPABILITIES.contains(&c.as_str()))
        .map(|c| c.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(error_body(
            &format!(
                "The server is missing required WebSocket capabilities: {}.",
                missing.join(", ")
            ),
            "WS_CAPABILITIES_INCOMPATIBLE",
            "update-server",
        ));
    }
    Ok(json!({
        "protocolEpoch": PROTOCOL_EPOCH,
        "negotiatedRevision": negotiated,
        "serverBuild": SERVER_BUILD,
        "serverInstanceId": server_instance_id(),
        "capabilities": SERVER_CAPABILITIES,
    }))
}

fn error_body(message: &str, code: &str, action: &str) -> Value {
    json!({
        "_tag": "WsCompatibilityError",
        "message": message,
        "code": code,
        "retryable": false,
        "action": action,
        "serverBuild": SERVER_BUILD,
        "protocolEpoch": PROTOCOL_EPOCH,
        "minRevision": MIN_REVISION,
        "maxRevision": MAX_REVISION,
    })
}

fn error_response(status: StatusCode, err: Value, cors: Option<&str>) -> Response {
    let mut headers = HeaderMap::new();
    headers.insert("cache-control", HeaderValue::from_static("no-store"));
    if let Some(origin) = cors {
        if let Ok(v) = HeaderValue::from_str(origin) {
            headers.insert("access-control-allow-origin", v);
        }
        headers.insert("vary", HeaderValue::from_static("Origin"));
    }
    (status, headers, axum::Json(err)).into_response()
}

fn request_origin(headers: &HeaderMap) -> String {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    format!("http://{host}")
}

/// `GET /ws/negotiate` handler.
async fn negotiate_route(req: Request<axum::body::Body>) -> Response {
    let cfg = config();
    let headers = req.headers().clone();
    let uri: Uri = req.uri().clone();
    let req_origin = request_origin(&headers);

    let raw_origin = headers.get("origin").and_then(|v| v.to_str().ok());
    if should_reject_untrusted_request_origin(raw_origin, &req_origin, cfg) {
        let mut h = HeaderMap::new();
        h.insert("cache-control", HeaderValue::from_static("no-store"));
        h.insert("vary", HeaderValue::from_static("Origin"));
        return (StatusCode::FORBIDDEN, h, "Forbidden").into_response();
    }

    let cors_origin = normalize_cors_origin(raw_origin)
        .filter(|o| is_trusted_app_origin(Some(o.as_str()), &req_origin, cfg));

    let query = uri.query().unwrap_or("");
    let mut epoch: Option<i64> = None;
    let mut min_rev: Option<i64> = None;
    let mut max_rev: Option<i64> = None;
    let mut client_build: Option<String> = None;
    let mut required: Vec<String> = Vec::new();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (k, v) = match pair.split_once('=') {
            Some(kv) => kv,
            None => (pair, ""),
        };
        let value = percent_decode(v);
        let int_ok = |s: &str| -> Option<i64> {
            if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            s.parse().ok()
        };
        match k {
            "x-synara-protocol-epoch" => epoch = int_ok(&value),
            "x-synara-protocol-min-revision" => min_rev = int_ok(&value),
            "x-synara-protocol-max-revision" => max_rev = int_ok(&value),
            "x-synara-client-build" => {
                client_build = {
                    let t = value.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                }
            }
            "x-synara-required-capability" => required.push(value),
            _ => {}
        }
    }

    let cors = cors_origin.as_deref();
    let bad_request_error = |msg: &str| -> Response {
        error_response(
            StatusCode::UPGRADE_REQUIRED,
            error_body(msg, "WS_NEGOTIATION_REQUIRED", "reload"),
            cors,
        )
    };

    let (Some(epoch), Some(min_rev), Some(max_rev), Some(_client_build)) =
        (epoch, min_rev, max_rev, client_build)
    else {
        return bad_request_error("WebSocket negotiation request is missing required parameters.");
    };

    if min_rev > max_rev {
        return bad_request_error("The client sent an invalid WebSocket protocol range.");
    }
    if epoch != PROTOCOL_EPOCH {
        let action = if epoch < PROTOCOL_EPOCH {
            "update-client"
        } else {
            "update-server"
        };
        let err = error_body(
            &format!(
                "WebSocket protocol epoch {epoch} is incompatible with server epoch {PROTOCOL_EPOCH}."
            ),
            "WS_PROTOCOL_INCOMPATIBLE",
            action,
        );
        return error_response(StatusCode::UPGRADE_REQUIRED, err, cors);
    }
    let low = min_rev.max(MIN_REVISION);
    let negotiated = max_rev.min(MAX_REVISION);
    if negotiated < low {
        let action = if max_rev < MIN_REVISION {
            "update-client"
        } else {
            "update-server"
        };
        let err = error_body(
            &format!(
                "WebSocket protocol revisions {min_rev}-{max_rev} do not overlap server revisions {MIN_REVISION}-{MAX_REVISION}."
            ),
            "WS_PROTOCOL_INCOMPATIBLE",
            action,
        );
        return error_response(StatusCode::UPGRADE_REQUIRED, err, cors);
    }
    let missing: Vec<&str> = required
        .iter()
        .filter(|c| !SERVER_CAPABILITIES.contains(&c.as_str()))
        .map(|c| c.as_str())
        .collect();
    if !missing.is_empty() {
        let err = error_body(
            &format!(
                "The server is missing required WebSocket capabilities: {}.",
                missing.join(", ")
            ),
            "WS_CAPABILITIES_INCOMPATIBLE",
            "update-server",
        );
        return error_response(StatusCode::UPGRADE_REQUIRED, err, cors);
    }

    let mut h = HeaderMap::new();
    h.insert("cache-control", HeaderValue::from_static("no-store"));
    if let Some(origin) = cors {
        if let Ok(v) = HeaderValue::from_str(origin) {
            h.insert("access-control-allow-origin", v);
        }
        h.insert("vary", HeaderValue::from_static("Origin"));
    }
    let body = json!({
        "protocolEpoch": PROTOCOL_EPOCH,
        "negotiatedRevision": negotiated,
        "serverBuild": SERVER_BUILD,
        "serverInstanceId": server_instance_id(),
        "capabilities": SERVER_CAPABILITIES,
    });
    (StatusCode::OK, h, axum::Json(body)).into_response()
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let h = (bytes[i + 1] as char).to_digit(16);
                let l = (bytes[i + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (h, l) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn router() -> Router<Original> {
    Router::new().route(WS_NEGOTIATE_HTTP_PATH, get(negotiate_route))
}
