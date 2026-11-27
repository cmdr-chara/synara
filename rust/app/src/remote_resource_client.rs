//! Outbound remote resource client, a compact port of upstream
//! `hostConnections/resourcePool.ts`. Local HTTP callers never see a remote
//! credential: this module validates the resource manifest, opens a pinned TLS
//! channel to the paired host, attaches the DPoP-bound session credential, and
//! returns only the bounded response headers/body.

use axum::body::Body;
use axum::extract::{Path, Request};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Semaphore;
use std::sync::{Arc, Mutex, OnceLock};

use crate::remote_resource::{resource_route, REMOTE_RESOURCE_PATH};

const MAX_METADATA_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_HEAD_BYTES: usize = 64 * 1024;
const MAX_RESPONSE_BODY_BYTES: usize = 32 * 1024 * 1024;

fn percent_encode(input: &str, component: bool) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        let ch = *byte as char;
        let unreserved = byte.is_ascii_alphanumeric()
            || if component {
                matches!(ch, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')')
            } else {
                matches!(ch, '*' | '-' | '.' | '_')
            };
        if unreserved {
            out.push(ch);
        } else if !component && *byte == b' ' {
            out.push('+');
        } else {
            const HEX: &[u8; 16] = b"0123456789ABCDEF";
            out.push('%');
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 15) as usize] as char);
        }
    }
    out
}

/// JavaScript `encodeURIComponent`, used in the DPoP presentation URL.
fn encode_uri_component(input: &str) -> String {
    percent_encode(input, true)
}

/// JavaScript `URLSearchParams` byte encoding, used in the actual HTTP query.
fn encode_query_param(input: &str) -> String {
    percent_encode(input, false)
}

fn presentation_url(environment_id: &str, encoded_reference: &str) -> String {
    format!(
        "synara://remote/{}{}/{}",
        encode_uri_component(environment_id),
        REMOTE_RESOURCE_PATH,
        encode_uri_component(encoded_reference),
    )
}

fn filtered_request_headers(headers: &HeaderMap, max_body_bytes: usize) -> Result<Vec<(String, String)>, StatusCode> {
    let mut result = Vec::new();
    if let Some(value) = headers.get(header::RANGE).and_then(|value| value.to_str().ok()) {
        let valid = max_body_bytes == 0
            && {
                let bytes = value.strip_prefix("bytes=").unwrap_or("");
                let mut parts = bytes.split('-');
                let start = parts.next().unwrap_or("");
                let end = parts.next().unwrap_or("");
                parts.next().is_none()
                    && (start.chars().all(|c| c.is_ascii_digit()) || start.is_empty())
                    && (end.chars().all(|c| c.is_ascii_digit()) || end.is_empty())
                    && !(start.is_empty() && end.is_empty())
            };
        if !valid {
            return Err(StatusCode::FORBIDDEN);
        }
        result.push(("range".to_string(), value.to_string()));
    }
    if max_body_bytes != 0 {
        let content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .filter(|value| value.len() <= 256 && !value.contains(['\r', '\n']))
            .ok_or(StatusCode::FORBIDDEN)?;
        result.push(("content-type".to_string(), content_type.to_string()));
    }
    Ok(result)
}

fn response_headers(remote_head: &str) -> HeaderMap {
    const ALLOWED: &[&str] = &[
        "content-type", "content-length", "content-range", "accept-ranges",
        "content-disposition", "etag", "last-modified", "cache-control",
        "content-security-policy", "x-content-type-options",
    ];
    let mut result = HeaderMap::new();
    for line in remote_head.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else { continue };
        let lower = name.trim().to_ascii_lowercase();
        if !ALLOWED.contains(&lower.as_str()) { continue }
        let Ok(name) = HeaderName::from_bytes(lower.as_bytes()) else { continue };
        let Ok(value) = HeaderValue::from_str(value.trim()) else { continue };
        result.insert(name, value);
    }
    result.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    result
}


fn host_semaphore(host_id: &str, transfer: bool) -> Arc<Semaphore> {
    static PENDING: OnceLock<Mutex<std::collections::HashMap<String, Arc<Semaphore>>>> = OnceLock::new();
    static TRANSFERS: OnceLock<Mutex<std::collections::HashMap<String, Arc<Semaphore>>>> = OnceLock::new();
    let (cell, permits) = if transfer { (&TRANSFERS, 2) } else { (&PENDING, 32) };
    let map = cell.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    map.lock().ok()
        .and_then(|mut entries| Some(entries.entry(host_id.to_string()).or_insert_with(|| Arc::new(Semaphore::new(permits))).clone()))
        .unwrap_or_else(|| Arc::new(Semaphore::new(permits)))
}

fn bad(status: StatusCode, text: &'static str) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")], text).into_response()
}

/// GET/POST `/api/remote/resource/{hostId}?reference=<json>`.
///
/// This is deliberately not a general HTTP proxy. The resource JSON is the
/// upstream manifest and `resource_route` is the only authority that maps it
/// to a loopback path on the host.
pub async fn handle_remote_resource(
    Path(host_id): Path<String>,
    request: Request,
) -> Response {
    let uri = request.uri().clone();
    let headers = request.headers().clone();
    if crate::ws_relay::query_value(&uri, "token").is_some() {
        return bad(StatusCode::BAD_REQUEST, "Invalid resource request");
    }
    let cfg = crate::native_ws::config();
    let request_origin = crate::native_ws::request_origin(&headers);
    let raw_origin = headers.get(header::ORIGIN).and_then(|value| value.to_str().ok());
    if crate::native_ws::should_reject_untrusted_request_origin(raw_origin, &request_origin, cfg) {
        return bad(StatusCode::FORBIDDEN, "Forbidden");
    }
    if let Err(status) = crate::ws_relay::remote_owner_authorized(&headers, &uri) {
        return bad(status, "Forbidden");
    }
    let method = request.method().clone();
    let body = match axum::body::to_bytes(request.into_body(), 32 * 1024 * 1024).await {
        Ok(body) => body,
        Err(_) => return bad(StatusCode::FORBIDDEN, "Remote resource unavailable"),
    };
    let Some(encoded_reference) = crate::ws_relay::query_value(&uri, "reference") else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    if encoded_reference.len() > MAX_METADATA_BYTES {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    }
    let Ok(reference) = serde_json::from_str::<Value>(&encoded_reference) else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    let Some((host, credential)) = crate::remote_connections::resource_credential(&host_id) else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    if reference.get("environmentId").and_then(Value::as_str) != Some(host.environment_id.as_str()) {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    }
    let Some(resource) = reference.get("resource") else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    let Ok(route) = resource_route(resource) else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    if method.as_str() != route.method {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    }
    if (route.max_body_bytes == 0 && !body.is_empty())
        || (route.max_body_bytes != 0 && body.len() > route.max_body_bytes as usize)
    {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    }
    let pending = host_semaphore(&host_id, false);
    let Ok(_pending_permit) = pending.try_acquire_owned() else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    let transfers = host_semaphore(&host_id, true);
    let Ok(_transfer_permit) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        transfers.acquire_owned(),
    ).await else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    let Ok(forward_headers) = filtered_request_headers(&headers, route.max_body_bytes as usize) else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    let Ok(endpoints) = crate::hosts::fresh_host_endpoints(&host_id, &host.environment_id) else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    let secrets_dir = std::path::PathBuf::from(format!("{}/secrets", crate::todo::state_dir()));
    let identity_path = secrets_dir.join("device-identity.json");
    let Ok(identity) = crate::device_identity::load_or_create(&identity_path) else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    let presentation = presentation_url(&host.environment_id, &encoded_reference);
    let Ok(dpop) = identity.sign_dpop(route.method, &presentation, Some(&credential)) else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    let target_path = format!(
        "{}?reference={}",
        REMOTE_RESOURCE_PATH,
        encode_query_param(&encoded_reference),
    );
    let mut head = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nAuthorization: DPoP {}\r\nDPoP: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        route.method,
        target_path,
        crate::remote_tls::remote_tls_server_name(&host.environment_id).unwrap_or_else(|_| "remote.invalid".to_string()),
        credential,
        dpop,
        body.len(),
    );
    for (name, value) in forward_headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let forwarded = async move {
        let (mut tls, _) = crate::remote_client::open_tls_channel_endpoints(
            endpoints,
            &host.root_certificate,
            &host.environment_id,
        ).await?;
        tokio::time::timeout(std::time::Duration::from_secs(15), tls.write_all(head.as_bytes()))
            .await
            .map_err(|_| "Remote resource write timed out".to_string())?
            .map_err(|error| format!("Remote resource write failed: {error}"))?;
        tls.write_all(&body).await.map_err(|error| format!("Remote resource write failed: {error}"))?;
        tls.flush().await.map_err(|error| format!("Remote resource write failed: {error}"))?;
        let mut raw = Vec::new();
        tokio::time::timeout(std::time::Duration::from_secs(15 * 60), async {
            let mut chunk = [0u8; 64 * 1024];
            loop {
                let read = tls.read(&mut chunk).await.map_err(|error| format!("Remote resource read failed: {error}"))?;
                if read == 0 { break; }
                raw.extend_from_slice(&chunk[..read]);
                if raw.len() > MAX_RESPONSE_BODY_BYTES {
                    return Err("Remote resource response exceeds limit".to_string());
                }
            }
            Ok::<(), String>(())
        }).await.map_err(|_| "Remote resource timed out".to_string())??;
        Ok::<Vec<u8>, String>(raw)
    }.await;
    let Ok(raw) = forwarded else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    let Some(split) = raw.windows(4).position(|window| window == b"\r\n\r\n") else {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    };
    if split > MAX_RESPONSE_HEAD_BYTES {
        return bad(StatusCode::FORBIDDEN, "Remote resource unavailable");
    }
    let remote_head = String::from_utf8_lossy(&raw[..split]).to_string();
    let status_code = remote_head
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .and_then(|value| StatusCode::from_u16(value).ok())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let status = if status_code.is_redirection() && status_code != StatusCode::NOT_MODIFIED {
        StatusCode::BAD_GATEWAY
    } else {
        status_code
    };
    let mut response = Response::new(Body::from(raw[split + 4..].to_vec()));
    *response.status_mut() = status;
    *response.headers_mut() = response_headers(&remote_head);
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presentation_shape_matches_upstream() {
        assert_eq!(
            presentation_url("env 1", "{\"environmentId\":\"env 1\"}"),
            "synara://remote/env%201/v2/resource/%7B%22environmentId%22%3A%22env%201%22%7D",
        );
        assert_eq!(encode_uri_component(" !~*()'"), "%20!~*()'");
        assert_eq!(encode_query_param(" !~*()'"), "+%21%7E*%28%29%27");
    }

    #[test]
    fn request_header_filter_rebounds_trust_boundary() {
        let mut headers = HeaderMap::new();
        headers.insert(header::RANGE, HeaderValue::from_static("bytes=1-2"));
        headers.insert(header::COOKIE, HeaderValue::from_static("secret=yes"));
        let filtered = filtered_request_headers(&headers, 0).unwrap();
        assert_eq!(filtered, vec![("range".to_string(), "bytes=1-2".to_string())]);
        assert!(filtered_request_headers(&headers, 4096).is_err());
    }
}
