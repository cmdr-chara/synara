//! Remote resource gateway, port of upstream remoteTransport/resourceGateway.ts
//! and resourceProxy.ts. Remote devices fetch host resources (attachments,
//! workspace previews, favicons, voice upload) through the tunnel as plain
//! inner HTTP requests carrying a DPoP-bound session credential. The resource
//! manifest is the only route selector — there is no caller-supplied URL.
//!
//! Principled deviation: upstream re-bridges into the local server with a
//! short-lived bearer lease because the tunnel terminated in a separate
//! process context from session auth. Here the tunnel terminates inside the
//! server itself, and the loopback surface is already trusted, so the gateway
//! forwards the byte stream to 127.0.0.1 with strictly rebuilt headers —
//! the same header discipline, one fewer credential hop.

use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const REMOTE_RESOURCE_PATH: &str = "/v2/resource";
pub const MAX_REMOTE_RESOURCE_METADATA_BYTES: usize = 16 * 1024;
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 25 * 1024 * 1024;
const MAX_VOICE_BYTES: u64 = 10 * 1024 * 1024;

fn encode_uri_component(value: &str) -> String {
    // JavaScript encodeURIComponent semantics: unreserved characters plus
    // ! ~ * ' ( ) survive; everything else is UTF-8 percent-encoded.
    let mut out = String::new();
    for byte in value.bytes() {
        let keep = matches!(byte,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn presentation_url(environment_id: &str, encoded_reference: &str) -> String {
    format!(
        "synara://remote/{}{}/{}",
        encode_uri_component(environment_id),
        REMOTE_RESOURCE_PATH,
        encode_uri_component(encoded_reference)
    )
}

fn url_encode_param(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        let keep =
            matches!(byte, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~');
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

pub struct ResourceRoute {
    pub path: String,
    pub method: &'static str,
    pub max_body_bytes: u64,
}

/// The manifest mapping, exactly upstream's remoteResourceRoute. Returns the
/// loopback path (with query), method and upload body cap for a resource.
pub fn resource_route(resource: &Value) -> Result<ResourceRoute, String> {
    let str_of = |key: &str| resource.get(key).and_then(Value::as_str).unwrap_or("");
    let kind = str_of("kind");
    let mut max_body_bytes = 0u64;
    let mut path = match kind {
        "attachment" => {
            let id = str_of("attachmentId");
            if id.is_empty() {
                return Err("attachment id missing".to_string());
            }
            format!("/attachments/{}", url_encode_param(id))
        }
        "attachment-upload" => {
            let attachment_type = str_of("type");
            max_body_bytes = if attachment_type == "image" {
                MAX_IMAGE_BYTES
            } else {
                MAX_FILE_BYTES
            };
            format!(
                "/api/attachments/upload?threadId={}&type={}&name={}&mimeType={}",
                url_encode_param(str_of("threadId")),
                url_encode_param(attachment_type),
                url_encode_param(str_of("name")),
                url_encode_param(str_of("mimeType")),
            )
        }
        "attachment-cancel" => {
            max_body_bytes = 4096;
            "/api/attachments/cancel".to_string()
        }
        "workspace-preview" => {
            let mut query = format!("path={}", url_encode_param(str_of("path")));
            for key in ["cwd", "grant"] {
                let value = str_of(key);
                if !value.is_empty() {
                    query.push_str(&format!("&{key}={}", url_encode_param(value)));
                }
            }
            if let Some(revision) = resource.get("revision").and_then(Value::as_u64) {
                query.push_str(&format!("&v={revision}"));
            }
            if resource.get("download").and_then(Value::as_bool) == Some(true) {
                query.push_str("&download=1");
            }
            format!("/api/local-image?{query}")
        }
        "thread-export" => format!(
            "/api/thread-export?threadId={}",
            url_encode_param(str_of("threadId"))
        ),
        "project-favicon" => format!(
            "/api/project-favicon?cwd={}&fallback=none",
            url_encode_param(str_of("cwd"))
        ),
        "site-favicon" => format!(
            "/api/site-favicon?domain={}",
            url_encode_param(str_of("domain"))
        ),
        "editor-icon" => format!(
            "/api/editor-icon?id={}",
            url_encode_param(str_of("editorId"))
        ),
        "voice-upload" => {
            max_body_bytes = MAX_VOICE_BYTES;
            let mut query = format!(
                "provider={}&cwd={}&mimeType={}&sampleRateHz={}&durationMs={}",
                url_encode_param(str_of("provider")),
                url_encode_param(str_of("cwd")),
                url_encode_param(str_of("mimeType")),
                url_encode_param(str_of("sampleRateHz")),
                url_encode_param(str_of("durationMs")),
            );
            let instance = str_of("providerInstanceId");
            if !instance.is_empty() {
                query.push_str(&format!(
                    "&providerInstanceId={}",
                    url_encode_param(instance)
                ));
            }
            let thread = str_of("threadId");
            if !thread.is_empty() {
                query.push_str(&format!("&threadId={}", url_encode_param(thread)));
            }
            format!("/api/voice/transcribe?{query}")
        }
        _ => return Err("unknown resource kind".to_string()),
    };
    let method: &'static str = if max_body_bytes > 0 { "POST" } else { "GET" };
    if kind == "attachment" || kind == "workspace-preview" {
        // GET routes
    }
    let _ = &mut path;
    Ok(ResourceRoute {
        path,
        method,
        max_body_bytes,
    })
}

pub struct InnerRequestHead {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
}

impl InnerRequestHead {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Reads an inner HTTP request head from the TLS stream. Returns None when the
/// stream ends or the head is malformed beyond salvage.
pub async fn read_inner_head<S>(stream: &mut S) -> Option<InnerRequestHead>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut head = Vec::with_capacity(1024);
    let mut byte = [0u8; 1];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        if stream.read_exact(&mut byte).await.is_err() {
            return None;
        }
        head.push(byte[0]);
        if head.len() > 64 * 1024 {
            return None;
        }
    }
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.lines();
    let request_line = lines.next()?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.push((name.trim().to_string(), value.trim().to_string()));
        }
    }
    Some(InnerRequestHead {
        method,
        path,
        headers,
    })
}

/// Validates a resource request and returns the decoded reference JSON plus
/// the exact encoded reference string as it appeared in the query. Mirrors
/// upstream: single reference parameter, no foreign keys, metadata cap.
fn extract_reference(path: &str) -> Result<(String, Value), String> {
    let (pathname, query) = path.split_once('?').unwrap_or((path, ""));
    if pathname != REMOTE_RESOURCE_PATH {
        return Err("invalid resource endpoint".to_string());
    }
    let mut reference: Option<String> = None;
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        if key != "reference" {
            return Err("invalid resource endpoint".to_string());
        }
        if reference.is_some() {
            return Err("invalid resource endpoint".to_string());
        }
        reference = Some(percent_decode(value));
    }
    let encoded = reference.ok_or_else(|| "invalid resource endpoint".to_string())?;
    if encoded.len() > MAX_REMOTE_RESOURCE_METADATA_BYTES {
        return Err("resource metadata exceeds limit".to_string());
    }
    let parsed: Value =
        serde_json::from_str(&encoded).map_err(|_| "resource metadata is not JSON".to_string())?;
    Ok((encoded, parsed))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() + 1 && index + 2 <= bytes.len() - 1 + 1 {
            if index + 2 < bytes.len() + 1 {
                let hex = std::str::from_utf8(&bytes[index + 1..bytes.len().min(index + 3)]);
                if let Ok(hex) = hex {
                    if let Ok(number) = u8::from_str_radix(hex, 16) {
                        out.push(number);
                        index += 3;
                        continue;
                    }
                }
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

/// Rebuilds request headers across the trust boundary, exactly upstream's
/// resourceRequestHeaders: only range, content-length and content-type may
/// cross; cookies and credentials never do.
fn rebuild_headers(
    head: &InnerRequestHead,
    max_body_bytes: u64,
) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    if let Some(range) = head.header("range") {
        let valid = {
            let body = range.strip_prefix("bytes=").unwrap_or("");
            let mut halves = body.split('-');
            let start = halves.next().unwrap_or("");
            let end = halves.next().unwrap_or("");
            halves.next().is_none()
                && (start.chars().all(|c| c.is_ascii_digit()) || start.is_empty())
                && (end.chars().all(|c| c.is_ascii_digit()) || end.is_empty())
                && !(start.is_empty() && end.is_empty())
        };
        if max_body_bytes > 0 || !valid {
            return Err("only one valid byte range is supported".to_string());
        }
        out.push(("range".to_string(), range.to_string()));
    }
    if let Some(length) = head.header("content-length") {
        let parsed: u64 = length
            .parse()
            .map_err(|_| "resource request body exceeds limit".to_string())?;
        if parsed > max_body_bytes {
            return Err("resource request body exceeds limit".to_string());
        }
        out.push(("content-length".to_string(), length.to_string()));
    }
    if max_body_bytes > 0 {
        let content_type = head
            .header("content-type")
            .ok_or_else(|| "resource content type is required".to_string())?;
        if content_type.len() > 256 || content_type.contains(['\r', '\n']) {
            return Err("resource content type is required".to_string());
        }
        out.push(("content-type".to_string(), content_type.to_string()));
    }
    Ok(out)
}

async fn write_refusal<S>(stream: &mut S)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let body = "Remote resource request refused";
    let response = format!(
        "HTTP/1.1 403 Forbidden\r\nCache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes()).await;
}

/// Serves one inner `/v2/resource` request: validate, verify the credential,
/// check trust, then stream the exchange with the loopback surface. Returns
/// after the response is fully relayed. Any refusal answers 403 like upstream.
pub async fn serve_resource<S>(stream: &mut S, head: InnerRequestHead, listening_port: u16)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let _ = serve_resource_inner(stream, head, listening_port).await;
}

async fn serve_resource_inner<S>(
    stream: &mut S,
    head: InnerRequestHead,
    listening_port: u16,
) -> Result<(), String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let validated = (|| -> Result<(ResourceRoute, String), String> {
        let (encoded, reference) = extract_reference(&head.path)?;
        let state_dir = crate::todo::state_dir();
        let secrets_dir = format!("{state_dir}/secrets");
        let environment_id = crate::server_env::environment_id(&state_dir).unwrap_or_default();
        let claimed_env = reference
            .get("environmentId")
            .and_then(Value::as_str)
            .unwrap_or("");
        if claimed_env != environment_id {
            return Err("wrong resource environment".to_string());
        }
        let resource = reference
            .get("resource")
            .cloned()
            .ok_or_else(|| "resource missing".to_string())?;
        let route = resource_route(&resource)?;
        if head.method != route.method {
            return Err("wrong resource method".to_string());
        }
        let authorization = head
            .header("authorization")
            .ok_or_else(|| "missing resource authorization".to_string())?;
        let credential = authorization
            .strip_prefix("DPoP ")
            .ok_or_else(|| "missing resource authorization".to_string())?;
        let dpop = head
            .header("dpop")
            .ok_or_else(|| "missing resource authorization".to_string())?;
        let identity_path = crate::host_identity::host_identity_path(&secrets_dir);
        let identity = crate::host_identity::read_host_identity(&identity_path)?
            .ok_or_else(|| "host identity missing".to_string())?;
        let htu = presentation_url(&environment_id, &encoded);
        let verified = crate::host_auth_verify::verify_session_credential(
            credential,
            dpop,
            &identity,
            &environment_id,
            1,
            &htu,
            route.method,
            None,
        )?;
        let conn = crate::remote_pairing::open()?;
        let scope = crate::remote_pairing::local_scope(&secrets_dir, &environment_id)
            .ok_or_else(|| "scope unavailable".to_string())?;
        if !crate::remote_pairing::authorize_device(&conn, &scope, &verified.device_jkt)? {
            return Err("device trust is not current".to_string());
        }
        Ok((route, verified.device_jkt))
    })();

    let (route, device_jkt) = match validated {
        Ok(value) => value,
        Err(_) => {
            write_refusal(stream).await;
            return Ok(());
        }
    };

    let mut headers = match rebuild_headers(&head, route.max_body_bytes) {
        Ok(value) => value,
        Err(_) => {
            write_refusal(stream).await;
            return Ok(());
        }
    };
    // The credential has already been verified and the caller's own
    // authorization/cookie headers are stripped. This internal principal is
    // what keeps one remote device from cancelling another device's staging.
    headers.push(("x-synara-remote-device-jkt".to_string(), device_jkt));

    let mut loopback = match tokio::net::TcpStream::connect(("127.0.0.1", listening_port)).await {
        Ok(value) => value,
        Err(_) => {
            write_refusal(stream).await;
            return Ok(());
        }
    };

    let mut forwarded = format!(
        "{} {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n",
        head.method, route.path
    );
    for (name, value) in &headers {
        forwarded.push_str(&format!("{name}: {value}\r\n"));
    }
    forwarded.push_str("\r\n");
    if loopback.write_all(forwarded.as_bytes()).await.is_err() {
        write_refusal(stream).await;
        return Ok(());
    }

    // Stream the bounded upload body across, then relay the response verbatim.
    let content_length: u64 = head
        .header("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut remaining = content_length;
    let mut chunk = [0u8; 16 * 1024];
    while remaining > 0 {
        let take = remaining.min(chunk.len() as u64) as usize;
        match stream.read_exact(&mut chunk[..take]).await {
            Ok(_) => {
                if loopback.write_all(&chunk[..take]).await.is_err() {
                    return Ok(());
                }
            }
            Err(_) => return Ok(()),
        }
        remaining -= take as u64;
    }

    let mut response_buf = [0u8; 16 * 1024];
    loop {
        match loopback.read(&mut response_buf).await {
            Ok(0) => break,
            Ok(count) => {
                if stream.write_all(&response_buf[..count]).await.is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_uri_component_matches_javascript() {
        assert_eq!(encode_uri_component("a b"), "a%20b");
        assert_eq!(encode_uri_component("{\"a\":1}"), "%7B%22a%22%3A1%7D");
        assert_eq!(encode_uri_component("!'()*-._~"), "!'()*-._~");
        assert_eq!(encode_uri_component("/path?x=1"), "%2Fpath%3Fx%3D1");
    }

    #[test]
    fn presentation_url_shape() {
        let url = presentation_url("env-1", "{\"environmentId\":\"env-1\"}");
        assert!(url.starts_with("synara://remote/env-1/v2/resource/"));
        assert!(url.contains("%7B"));
    }

    #[test]
    fn extract_reference_validates_endpoint() {
        let path = "/v2/resource?reference=%7B%22environmentId%22%3A%22e%22%7D";
        let (decoded, parsed) = extract_reference(path).expect("valid");
        assert_eq!(decoded, "{\"environmentId\":\"e\"}");
        assert_eq!(parsed["environmentId"], "e");
        assert!(extract_reference("/v2/resource").is_err());
        assert!(extract_reference("/v2/resource?reference=%7B%7D&other=1").is_err());
        assert!(extract_reference("/elsewhere?reference=%7B%7D").is_err());
    }

    #[test]
    fn route_manifest_covers_every_kind() {
        let attachment = serde_json::json!({"kind": "attachment", "attachmentId": "a1"});
        let route = resource_route(&attachment).expect("attachment");
        assert_eq!(route.path, "/attachments/a1");
        assert_eq!(route.method, "GET");
        assert_eq!(route.max_body_bytes, 0);

        let upload = serde_json::json!({
            "kind": "attachment-upload", "threadId": "t", "type": "image",
            "name": "n", "mimeType": "image/png"
        });
        let route = resource_route(&upload).expect("upload");
        assert_eq!(route.method, "POST");
        assert_eq!(route.max_body_bytes, MAX_IMAGE_BYTES);

        let file_upload = serde_json::json!({
            "kind": "attachment-upload", "threadId": "t", "type": "file",
            "name": "n", "mimeType": "text/plain"
        });
        assert_eq!(
            resource_route(&file_upload).expect("upload").max_body_bytes,
            MAX_FILE_BYTES
        );

        let voice = serde_json::json!({
            "kind": "voice-upload", "provider": "p", "cwd": "/x", "mimeType": "audio/wav",
            "sampleRateHz": "24000", "durationMs": "100"
        });
        let route = resource_route(&voice).expect("voice");
        assert_eq!(route.max_body_bytes, MAX_VOICE_BYTES);
        assert!(route.path.starts_with("/api/voice/transcribe?"));

        assert!(resource_route(&serde_json::json!({"kind": "bogus"})).is_err());
    }

    #[test]
    fn rebuild_headers_strips_credentials() {
        let head = InnerRequestHead {
            method: "GET".to_string(),
            path: "/v2/resource?reference=x".to_string(),
            headers: vec![
                ("authorization".to_string(), "DPoP secret".to_string()),
                ("cookie".to_string(), "session=abc".to_string()),
                ("range".to_string(), "bytes=0-100".to_string()),
            ],
        };
        let rebuilt = rebuild_headers(&head, 0).expect("headers");
        assert_eq!(rebuilt.len(), 1);
        assert_eq!(rebuilt[0].0, "range");
        assert!(
            rebuilt
                .iter()
                .all(|(n, _)| n != "cookie" && n != "authorization")
        );
    }

    #[test]
    fn rebuild_headers_rejects_oversized_body() {
        let head = InnerRequestHead {
            method: "POST".to_string(),
            path: String::new(),
            headers: vec![
                ("content-length".to_string(), "99999999".to_string()),
                ("content-type".to_string(), "text/plain".to_string()),
            ],
        };
        assert!(rebuild_headers(&head, 4096).is_err());
    }
}
