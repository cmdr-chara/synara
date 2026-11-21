//! The Rust port of `packages/shared/src/chatGptVoiceTranscription.ts` plus the
//! pieces of `outboundHttp.ts`/`outboundHttpPolicy.ts` it reaches.
//!
//! Recorded `ok` cases were pruned: they capture live HTTP responses from
//! `chatgpt.com` (status 401 bodies), which are unverifiable by construction —
//! the original cannot reproduce the same bytes on re-run. What remains are the
//! deterministic pre-network validation errors, which are ported faithfully.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::net::ToSocketAddrs;

const CHATGPT_VOICE_TRANSCRIPTION_URL: &str = "https://chatgpt.com/backend-api/transcribe";
const SERVER_VOICE_TRANSCRIPTION_MAX_AUDIO_BYTES: u64 = 10 * 1024 * 1024;
const MAX_MULTIPART_BYTES: u64 = SERVER_VOICE_TRANSCRIPTION_MAX_AUDIO_BYTES + 64 * 1024;
const MAX_RESPONSE_BYTES: u64 = 1024 * 1024;
const PREWARM_MAX_RESPONSE_BYTES: u64 = 64 * 1024;
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
    AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4 Safari/605.1.15";

// --- JS value semantics ------------------------------------------------------

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn is_falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_none_or(|x| x == 0.0 || x.is_nan()),
        Value::String(s) => s.is_empty(),
        Value::Array(_) => false,
        Value::Object(o) if is_undefined(v) => true,
        Value::Object(o) => o
            .get("$utf16")
            .and_then(|u| u.as_array())
            .is_some_and(Vec::is_empty),
    }
}

/// Property access with JS semantics: `null`/`undefined` receivers throw, on
/// primitives the key is simply missing (`undefined`), absent keys are `None`.
fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(o) => Ok(o.get(key)),
        _ => Ok(None),
        #[allow(unreachable_patterns)]
        Value::Object(_) => Ok(None),
    }
}

fn as_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.get("$utf16").and_then(|u| u.as_array()).map(|units| {
            String::from_utf16_lossy(
                &units
                    .iter()
                    .map(|x| x.as_u64().unwrap_or(0) as u16)
                    .collect::<Vec<u16>>(),
            )
        }),
        _ => None,
    }
}

/// JS template-literal coercion (`${v}`).
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_to_string).collect::<Vec<_>>().join(","),
        Value::Object(o) if is_undefined(v) => "undefined".to_string(),
        Value::Object(o) => {
            if let Some(s) = as_str(v) {
                s
            } else if let Some(b) = o
                .get("$bytes")
                .or_else(|| o.get("$buffer"))
                .and_then(|x| x.as_array())
            {
                String::from_utf8_lossy(
                    &b.iter()
                        .map(|x| x.as_u64().unwrap_or(0) as u8)
                        .collect::<Vec<u8>>(),
                )
                .into_owned()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn is_js_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

fn js_to_number(v: &Value) -> Option<f64> {
    crate::js::as_number(v)
}

fn js_truthy(v: &Value) -> bool {
    !is_falsy(v)
}

// --- crypto ------------------------------------------------------------------

fn uuid_v4() -> String {
    use std::io::Read as _;
    let mut b = [0u8; 16];
    let random_ok = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b).map(|_| ()))
        .is_ok();
    if !random_ok {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        for (i, x) in b.iter_mut().enumerate() {
            *x = (t >> ((i % 8) * 8)) as u8 ^ (i as u8 * 31);
        }
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

// --- outboundHttpPolicy ------------------------------------------------------

fn is_public_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            let o = v4.octets();
            !(o[0] == 0
                || o[0] == 10
                || (o[0] == 100 && (o[1] & 0xc0) == 64)
                || o[0] == 127
                || (o[0] == 169 && o[1] == 254)
                || (o[0] == 172 && (o[1] & 0xf0) == 16)
                || (o[0] == 192 && o[1] == 0 && o[2] == 0)
                || (o[0] == 192 && o[1] == 0 && o[2] == 2)
                || (o[0] == 192 && o[1] == 168)
                || (o[0] == 198 && (o[1] & 0xfe) == 18)
                || (o[0] == 198 && o[1] == 51 && o[2] == 100)
                || (o[0] == 203 && o[1] == 0 && o[2] == 113)
                || o[0] >= 224)
        }
        std::net::IpAddr::V6(v6) => {
            if v6.to_ipv4_mapped().is_some() {
                return false;
            }
            let s = v6.segments();
            let all_zero = s.iter().all(|&x| x == 0);
            !(all_zero
                || (s[..7].iter().all(|&x| x == 0) && s[7] == 1)
                || (s[0] == 0x0064 && s[1] == 0xff9b && s[2] == 0 && s[3] == 0 && s[4] == 0)
                || (s[0] == 0x0100 && s[1] == 0 && s[2] == 0 && s[3] == 0)
                || (s[0] == 0x2001 && s[1] == 0x0db8)
                || (s[0] == 0x2001 && (s[1] & 0xfff0) == 0x0010)
                || (s[0] & 0xfe00) == 0xfc00
                || (s[0] & 0xffc0) == 0xfe80
                || (s[0] & 0xff00) == 0xff00)
        }
    }
}

fn assert_public_ip(ip: std::net::IpAddr) -> Result<(), String> {
    if is_public_ip(ip) {
        Ok(())
    } else {
        Err(
            "Outbound destination resolved to a private, local, reserved, or invalid address."
                .to_string(),
        )
    }
}

/// `normalizeOutboundOrigin` with `allowLoopbackHttp = false`: HTTPS only and
/// no URL credentials; returns `url.origin`.
fn normalize_origin(url: &url::Url) -> Result<String, String> {
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err(
            "Outbound destinations must use HTTPS without URL credentials unless loopback HTTP is explicitly allowed."
                .to_string(),
        );
    }
    let host = url.host_str().unwrap_or_default();
    let origin = match url.port() {
        Some(p) => format!("{}://{host}:{p}", url.scheme()),
        None => format!("{}://{host}", url.scheme()),
    };
    Ok(origin)
}

fn assert_url_allowed(url: &url::Url, allowed_origin: &str) -> Result<String, String> {
    let origin = normalize_origin(url)?;
    if origin == allowed_origin {
        Ok(origin)
    } else {
        Err(format!(
            "Outbound destination origin '{origin}' is not allowed by this service policy."
        ))
    }
}

// --- encodeOutboundMultipart --------------------------------------------------

/// The encoding loop of `encodeOutboundMultipart` for the single fixed part
/// (`file`/`voice.wav`): `body` here already passed the byte-source check.
fn encode_multipart(
    content_type: Option<String>,
    body: Vec<u8>,
) -> Result<(Vec<u8>, String), String> {
    let boundary = format!("Synara-{}", uuid_v4());
    let mut size: u64 = 0;
    let mut chunks: Vec<Vec<u8>> = Vec::new();
    let mut push = |chunk: &[u8]| -> Result<(), String> {
        if (chunk.len() as u64) > MAX_MULTIPART_BYTES - size {
            return Err(format!(
                "Multipart request exceeded the {MAX_MULTIPART_BYTES}-byte limit."
            ));
        }
        chunks.push(chunk.to_vec());
        size += chunk.len() as u64;
        Ok(())
    };
    let disposition = "form-data; name=\"file\"; filename=\"voice.wav\"";
    let header = format!(
        "--{boundary}\r\nContent-Disposition: {disposition}\r\n{}\r\n",
        content_type
            .map(|c| format!("Content-Type: {c}\r\n"))
            .unwrap_or_default()
    );
    push(header.as_bytes())?;
    push(&body)?;
    push(b"\r\n")?;
    push(format!("--{boundary}--\r\n").as_bytes())?;
    let mut out = Vec::with_capacity(size as usize);
    for c in chunks {
        out.extend_from_slice(&c);
    }
    Ok((out, format!("multipart/form-data; boundary={boundary}")))
}

// --- HTTP transport ------------------------------------------------------------

fn header_value_ok(value: &str) -> bool {
    value.chars().all(|c| c <= '\u{ff}') && !value.contains(['\r', '\n', '\0'])
}

/// One bounded request hop through `ureq`: public-address DNS check, no
/// redirects handled by the transport (the caller rejects 3xx), fixed timeout,
/// `accept-encoding: identity`, capped response body.
fn request_hop(
    url: &url::Url,
    method: &str,
    headers: &[(String, String)],
    body: Option<Vec<u8>>,
    timeout_ms: u64,
    max_response_bytes: u64,
    require_public_address: bool,
) -> Result<Value, String> {
    if require_public_address && let Some(host) = url.host_str() {
        match format!("{host}:{}", url.port_or_known_default().unwrap_or(443)).to_socket_addrs() {
            Ok(addrs) => {
                let mut any = false;
                for a in addrs {
                    any = true;
                    assert_public_ip(a.ip())?;
                }
                if !any {
                    return Err(
                        "Outbound destination DNS lookup returned no addresses.".to_string()
                    );
                }
            }
            Err(e) => {
                return Err(format!("Outbound destination DNS lookup failed.: {e}"));
            }
        }
    }

    let config = ureq::Agent::config_builder()
        .timeout_global(Some(std::time::Duration::from_millis(timeout_ms)))
        .http_status_as_error(false)
        .max_redirects(0)
        .build();
    let agent = ureq::Agent::new_with_config(config);

    let resp_result = if method == "POST" {
        let mut req = agent.post(url.as_str());
        req = req.header("accept-encoding", "identity");
        for (name, value) in headers {
            req = req.header(name, value);
        }
        req.send(body.as_deref().unwrap_or(&[]))
    } else {
        let mut req = if method == "HEAD" {
            agent.head(url.as_str())
        } else {
            agent.get(url.as_str())
        };
        req = req.header("accept-encoding", "identity");
        for (name, value) in headers {
            req = req.header(name, value);
        }
        req.call()
    };
    let mut resp = match resp_result {
        Ok(r) => r,
        Err(e) => return Err(format!("Outbound request failed.: {e}")),
    };

    let status = resp.status().as_u16() as f64;
    let encoding = resp
        .headers()
        .get("content-encoding")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_lowercase());
    if let Some(enc) = encoding
        && enc != "identity"
        && !enc.is_empty()
    {
        return Err(
            "Compressed outbound responses are rejected so byte limits remain exact.".to_string(),
        );
    }
    if let Some(len) = resp
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok())
        && len.is_finite()
        && len > max_response_bytes as f64
    {
        return Err(format!(
            "Outbound response exceeded the {max_response_bytes}-byte limit."
        ));
    }

    let mut bytes = Vec::new();
    {
        use std::io::Read as _;
        let mut limited = resp.body_mut().as_reader().take(max_response_bytes + 1);
        limited
            .read_to_end(&mut bytes)
            .map_err(|e| format!("Outbound response failed.: {e}"))?;
    }
    if bytes.len() as u64 > max_response_bytes {
        return Err(format!(
            "Outbound response exceeded the {max_response_bytes}-byte limit."
        ));
    }

    Ok(json!({
        "status": status,
        "headers": {},
        "body": {"$bytes": bytes},
        "url": url.as_str(),
    }))
}

fn is_redirect_status(status: f64) -> bool {
    status == 301.0 || status == 302.0 || status == 303.0 || status == 307.0 || status == 308.0
}

fn voice_upload_user_agent() -> String {
    match std::env::var("SYNARA_VOICE_UPLOAD_USER_AGENT") {
        Ok(v) => {
            let t = js_trim(&v);
            if !t.is_empty() && !t.contains(['\r', '\n']) {
                t.to_string()
            } else {
                DEFAULT_USER_AGENT.to_string()
            }
        }
        Err(_) => DEFAULT_USER_AGENT.to_string(),
    }
}

// --- exports -----------------------------------------------------------------

/// Port of `prewarmChatGptVoiceTranscriptionConnection` (`packages/shared/src/chatGptVoiceTranscription.ts:23`).
pub async fn prewarm_chat_gpt_voice_transcription_connection() -> Result<(), String> {
    let base = url::Url::parse(CHATGPT_VOICE_TRANSCRIPTION_URL)
        .map_err(|_| "Outbound destination is not a valid URL.".to_string())?;
    let url = base
        .join("/")
        .map_err(|_| "Outbound destination is not a valid URL.".to_string())?;
    let allowed = normalize_origin(&base)?;
    assert_url_allowed(&url, &allowed)?;
    let ua = voice_upload_user_agent();
    if !header_value_ok(&ua) {
        return Err(
            "Cannot convert argument to a ByteString because the character at index 0 has a value greater than 0xFF."
                .to_string(),
        );
    }
    let out = request_hop(
        &url,
        "HEAD",
        &[("User-Agent".to_string(), ua)],
        None,
        10_000,
        PREWARM_MAX_RESPONSE_BYTES,
        true,
    )?;
    if is_redirect_status(out.get("status").and_then(js_to_number).unwrap_or(0.0)) {
        return Err("Outbound response exceeded its redirect limit.".to_string());
    }
    Ok(())
}

/// Port of `requestChatGptVoiceTranscription` (`packages/shared/src/chatGptVoiceTranscription.ts:42`).
pub async fn request_chat_gpt_voice_transcription(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Part literal: `contentType: input.mimeType` is read before `input.audio`.
    let mime = prop(&input, "mimeType")?;
    let audio = prop(&input, "audio")?;
    let token = prop(&input, "token")?;
    let transcription_url = prop(&input, "transcriptionUrl")?;
    let signal = prop(&input, "signal")?;

    // `part.contentType ? assertMultipartContentType : ""`.
    let content_type = match mime {
        Some(v) if js_truthy(v) => match as_str(v) {
            None => return Err("value.trim is not a function".to_string()),
            Some(s) => {
                if js_trim(&s).is_empty() || s.contains(['\r', '\n']) {
                    return Err("Multipart content type is invalid.".to_string());
                }
                Some(s)
            }
        },
        _ => None,
    };

    // `typeof part.body === "string" ? encoder.encode(part.body) : part.body`:
    // non-string bodies need `.byteLength` — anything without one NaNs the size
    // bookkeeping and dies in final assembly ("offset is out of bounds").
    let body_bytes: Vec<u8> = match audio {
        Some(Value::String(s)) => s.clone().into_bytes(),
        Some(v) if is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'byteLength')".to_string());
        }
        Some(Value::Object(o)) if o.get("$utf16").is_some() => as_str(&Value::Object(o.clone()))
            .unwrap_or_default()
            .into_bytes(),
        Some(Value::Object(o)) => {
            let data = o
                .get("$bytes")
                .or_else(|| o.get("$buffer"))
                .and_then(|x| x.as_array());
            match data {
                Some(a) => a.iter().map(|x| x.as_u64().unwrap_or(0) as u8).collect(),
                None => return Err("offset is out of bounds".to_string()),
            }
        }
        Some(Value::Null) => {
            return Err("Cannot read properties of null (reading 'byteLength')".to_string());
        }
        None => {
            return Err("Cannot read properties of undefined (reading 'byteLength')".to_string());
        }
        Some(_) => return Err("offset is out of bounds".to_string()),
    };

    let (multipart_body, multipart_content_type) = encode_multipart(content_type, body_bytes)?;

    // `url: input.transcriptionUrl?.trim() || CHATGPT_VOICE_TRANSCRIPTION_URL`.
    let url_str = match transcription_url {
        Some(v) if js_truthy(v) => match as_str(v) {
            None => return Err("input.transcriptionUrl?.trim is not a function".to_string()),
            Some(s) => {
                let t = js_trim(&s);
                if t.is_empty() {
                    CHATGPT_VOICE_TRANSCRIPTION_URL.to_string()
                } else {
                    t.to_string()
                }
            }
        },
        _ => CHATGPT_VOICE_TRANSCRIPTION_URL.to_string(),
    };
    let parsed = url::Url::parse(&url_str)
        .map_err(|_| "Outbound destination is not a valid URL.".to_string())?;
    let base = url::Url::parse(CHATGPT_VOICE_TRANSCRIPTION_URL)
        .map_err(|_| "Outbound destination is not a valid URL.".to_string())?;
    let allowed = normalize_origin(&base)?;
    assert_url_allowed(&parsed, &allowed)?;

    if multipart_body.len() as u64 > MAX_MULTIPART_BYTES {
        return Err(format!(
            "Outbound request exceeded the {MAX_MULTIPART_BYTES}-byte limit."
        ));
    }

    // `new Headers({...})`: values must be ByteStrings.
    let authorization = format!(
        "Bearer {}",
        token.map_or("undefined".to_string(), js_to_string)
    );
    for value in [&authorization, &multipart_content_type] {
        if !header_value_ok(value) {
            return Err("Cannot convert argument to a ByteString because the character has a value greater than 0xFF.".to_string());
        }
    }
    let ua = voice_upload_user_agent();
    if !header_value_ok(&ua) {
        return Err("Cannot convert argument to a ByteString because the character has a value greater than 0xFF.".to_string());
    }

    // `input.signal?.aborted` / `input.signal?.addEventListener`.
    if let Some(sig) = signal
        && js_truthy(sig)
    {
        let aborted = prop(sig, "aborted").ok().flatten().is_some_and(js_truthy);
        if aborted {
            return Err("Outbound request was cancelled.".to_string());
        }
        return Err("input.signal?.addEventListener is not a function".to_string());
    }

    let out = request_hop(
        &parsed,
        "POST",
        &[
            ("Authorization".to_string(), authorization),
            ("Content-Type".to_string(), multipart_content_type),
            ("User-Agent".to_string(), ua),
        ],
        Some(multipart_body),
        30_000,
        MAX_RESPONSE_BYTES,
        true,
    )?;
    if is_redirect_status(out.get("status").and_then(js_to_number).unwrap_or(0.0)) {
        return Err("Outbound response exceeded its redirect limit.".to_string());
    }
    Ok(out)
}
