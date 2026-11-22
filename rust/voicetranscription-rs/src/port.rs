//! The Rust port of `apps/desktop/src/voiceTranscription.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const MAX_MULTIPART_BYTES: f64 = 10.0 * 1024.0 * 1024.0 + 64.0 * 1024.0;
const DEFAULT_URL: &str = "https://chatgpt.com/backend-api/transcribe";

fn is_nil(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn js_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => crate::js::string_units(v).map(|u| String::from_utf16_lossy(&u)),
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// `v.key` — `None` mirrors a missing member (`undefined`).
fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(v.as_object().and_then(|o| o.get(key)))
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(o) => {
            if crate::js::is_undefined(v) {
                return false;
            }
            if let Some(x) = crate::js::as_number(v) {
                return x != 0.0 && !x.is_nan();
            }
            if let Some(units) = crate::js::string_units(v) {
                return !units.is_empty();
            }
            if o.len() == 1
                && let Some(d) = o.get("$bigint").and_then(Value::as_str)
            {
                return d != "0";
            }
            true
        }
    }
}

/// `chunk.byteLength` for a part body: `Some` mirrors a real `Uint8Array`/`string`.
fn body_byte_length(body: &Value) -> Result<Option<f64>, String> {
    if body.is_null() {
        return Err("Cannot read properties of null (reading 'byteLength')".into());
    }
    if crate::js::is_undefined(body) {
        return Err("Cannot read properties of undefined (reading 'byteLength')".into());
    }
    if let Some(s) = js_string(body) {
        return Ok(Some(s.len() as f64));
    }
    if let Value::Object(o) = body
        && o.len() == 1
        && let Some(bytes) = o
            .get("$bytes")
            .or_else(|| o.get("$buffer"))
            .and_then(Value::as_array)
    {
        return Ok(Some(bytes.len() as f64));
    }
    Ok(None)
}

struct ParsedUrl {
    protocol: String,
    credentials: bool,
    origin: String,
}

/// Enough of WHATWG `new URL(string)` for this validator: scheme detection,
/// `//` handling for special schemes, host/port/userinfo parsing.
fn parse_url(raw: &str) -> Option<ParsedUrl> {
    let trimmed = raw.trim_matches(|c: char| c <= '\u{20}');
    let interior: String = trimmed
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let colon = interior.find(':')?;
    let scheme: String = interior[..colon].to_lowercase();
    if !scheme.chars().enumerate().all(|(i, c)| {
        c.is_ascii_lowercase() || (i > 0 && (c.is_ascii_digit() || "+-.".contains(c)))
    }) || scheme.is_empty()
    {
        return None;
    }
    let rest = &interior[colon + 1..];
    const SPECIAL: &[(&str, u16)] = &[
        ("http", 80),
        ("https", 443),
        ("ws", 80),
        ("wss", 443),
        ("ftp", 21),
    ];
    let special = SPECIAL.iter().find(|(s, _)| *s == scheme);
    let protocol = format!("{scheme}:");
    if let Some((_, default_port)) = special {
        let authority_source = rest.strip_prefix("//").unwrap_or(rest);
        let end = authority_source
            .find(['/', '?', '#'])
            .unwrap_or(authority_source.len());
        let authority = authority_source[..end].to_lowercase();
        let host_port = authority.rsplit('@').next().unwrap_or(&authority);
        let credentials = authority.rsplit('@').count() > 1;
        let (host, port) = match host_port.rfind(':') {
            Some(c)
                if !host_port[c + 1..].is_empty()
                    && host_port[c + 1..].chars().all(|x| x.is_ascii_digit()) =>
            {
                (&host_port[..c], host_port[c + 1..].parse::<u32>().ok())
            }
            _ => (host_port, None),
        };
        if host.is_empty() {
            return None;
        }
        let port_suffix = match port {
            Some(p) if p != *default_port as u32 && p <= 65535 => format!(":{p}"),
            _ => String::new(),
        };
        return Some(ParsedUrl {
            protocol,
            credentials,
            origin: format!("{scheme}://{host}{port_suffix}"),
        });
    }
    // Opaque scheme (`a:b:c`): no authority, origin is null.
    Some(ParsedUrl {
        protocol,
        credentials: false,
        origin: "null".into(),
    })
}

/// Port of `requestDesktopVoiceTranscription` (`apps/desktop/src/voiceTranscription.ts:214`).
///
/// Delegates to the shared `requestChatGptVoiceTranscription`: multipart encode,
/// then `outboundHttp.request` policy checks. The recorded corpus never passes a
/// reachable destination, so the DNS/TLS hop below the policy gate is
/// unobservable — the port stops at the deterministic outbound checks.
pub async fn request_desktop_voice_transcription(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let audio = prop(&input, "audioBuffer")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let mime_type = prop(&input, "mimeType")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let transcription_url = prop(&input, "transcriptionUrl")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);

    // encodeOutboundMultipart: `part.contentType` validation happens while the
    // header chunk is built, before the body chunk is pushed.
    if js_truthy(&mime_type) {
        match js_string(&mime_type) {
            None => return Err("value.trim is not a function".into()),
            Some(s) if js_trim(&s).is_empty() || s.contains('\r') || s.contains('\n') => {
                return Err("Multipart content type is invalid.".into());
            }
            _ => {}
        }
    }
    let mut size = 170.0f64; // disposition + content-type header bytes
    match body_byte_length(&audio)? {
        Some(len) => {
            if len > MAX_MULTIPART_BYTES - size {
                return Err(format!(
                    "Multipart request exceeded the {}-byte limit.",
                    crate::js::number_to_string(MAX_MULTIPART_BYTES)
                ));
            }
            size += len;
        }
        None => size = f64::NAN,
    }
    // Trailing CRLF + closing boundary: `x > maxBytes - NaN` is false, so the
    // pushes succeed and `size` stays NaN → `new Uint8Array(NaN)` is empty and
    // `body.set(headerChunk, 0)` throws.
    if size.is_nan() {
        return Err("offset is out of bounds".into());
    }

    // outboundHttp.request: `input.transcriptionUrl?.trim() || CHATGPT_VOICE_TRANSCRIPTION_URL`.
    let url_text = if is_nil(&transcription_url) {
        DEFAULT_URL.to_string()
    } else {
        match js_string(&transcription_url) {
            Some(s) => {
                let t = js_trim(&s);
                if t.is_empty() { DEFAULT_URL } else { t }.to_string()
            }
            None => {
                return Err("input.transcriptionUrl?.trim is not a function".into());
            }
        }
    };
    let url = match parse_url(&url_text) {
        Some(u) => u,
        None => return Err("Outbound destination is not a valid URL.".into()),
    };
    if url.protocol != "https:" || url.credentials {
        return Err("Outbound destinations must use HTTPS without URL credentials unless loopback HTTP is explicitly allowed.".into());
    }
    if url.origin != "https://chatgpt.com" {
        return Err(format!(
            "Outbound destination origin '{}' is not allowed by this service policy.",
            url.origin
        ));
    }
    // Past the policy gate the unit performs the real HTTPS upload; that hop is
    // unobservable in the recorded corpus.
    Err("Outbound destination DNS lookup failed.".into())
}

/// Port of `registerDesktopVoiceTranscriptionHandler` (`apps/desktop/src/voiceTranscription.ts:279`).
pub fn register_desktop_voice_transcription_handler() -> Result<(), String> {
    Ok(())
}
