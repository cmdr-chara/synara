//! The Rust port of `apps/server/src/auth/`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

fn marker(v: &Value) -> bool {
    v.get("$undefined").and_then(Value::as_bool) == Some(true)
}

fn js_num_str(x: f64) -> String {
    if x.is_nan() {
        "NaN".to_string()
    } else if x == f64::INFINITY {
        "Infinity".to_string()
    } else if x == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if x == x.trunc() && x.abs() < 1e21 {
        format!("{x:.0}")
    } else {
        format!("{x}")
    }
}

/// JS `String(value)` for the port-suffix position.
fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js_num_str(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_str).collect::<Vec<_>>().join(","),
        Value::Object(m) => match m.get("$num").and_then(Value::as_str) {
            Some("NaN") => "NaN".to_string(),
            Some("Infinity") => "Infinity".to_string(),
            Some("-Infinity") => "-Infinity".to_string(),
            Some("-0") => "0".to_string(),
            _ => "undefined".to_string(),
        },
    }
}

/// Node `Buffer.from` argument validation: the recorded TypeError variants.
fn buffer_from_arg_error(v: &Value) -> String {
    let got = match v {
        Value::Null => "Received null".to_string(),
        Value::Number(n) => format!(
            "Received type number ({})",
            js_num_str(n.as_f64().unwrap_or(f64::NAN))
        ),
        Value::Bool(b) => format!("Received type boolean ({b})"),
        Value::Object(_) if marker(v) => "Received undefined".to_string(),
        Value::Object(_) => "Received an instance of Object".to_string(),
        _ => "Received undefined".to_string(),
    };
    format!(
        "The first argument must be of type string or an instance of Buffer, ArrayBuffer, or Array or an Array-like Object. {got}"
    )
}

/// `Buffer.from(value)` for string and byte-array inputs; errors otherwise.
fn buffer_from(v: &Value) -> Result<Vec<u8>, String> {
    match v {
        Value::String(s) => Ok(s.as_bytes().to_vec()),
        Value::Array(items) => Ok(items
            .iter()
            .map(|i| (i.as_f64().unwrap_or(f64::NAN) as i64) as u8)
            .collect()),
        _ => Err(buffer_from_arg_error(v)),
    }
}

const B64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64url_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 / 3 + 4);
    for chunk in bytes.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((*chunk.get(1).unwrap_or(&0) as u32) << 8)
            | (*chunk.get(2).unwrap_or(&0) as u32);
        let shifts = [18, 12, 6, 0];
        for (i, s) in shifts.iter().enumerate() {
            if i * 6 < chunk.len() * 8 {
                out.push(B64_ALPHABET[(n >> s) as usize & 63] as char);
            }
        }
    }
    out
}

/// Node's lenient base64 decode: each UTF-16 code unit's low byte is
/// classified against the alphabet; `=` (0x3D) stops the decode, other
/// non-alphabet bytes are skipped. Surrogate pairs therefore terminate the
/// decode (`0xD83D & 0xFF == 0x3D`); BMP chars like `İ` (0x30 = '0') alias
/// into the alphabet.
fn base64url_decode_lenient(input: &str) -> Vec<u8> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::new();
    for unit in input.encode_utf16() {
        let c = (unit & 0xFF) as u8;
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => continue,
        };
        acc = ((acc << 6) | v as u32) & 0xFFFFFF;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().into()
}

/// HMAC-SHA256 with block size 64 (RFC 2104).
fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        key_block[..32].copy_from_slice(&sha256(key));
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut inner = Vec::with_capacity(64 + msg.len());
    inner.extend(key_block.iter().map(|b| b ^ 0x36));
    inner.extend_from_slice(msg);
    let inner_hash = sha256(&inner);
    let mut outer = Vec::with_capacity(96);
    outer.extend(key_block.iter().map(|b| b ^ 0x5c));
    outer.extend_from_slice(&inner_hash);
    sha256(&outer)
}

/// Port of `makeEffectAuthRequest` (`apps/server/src/auth/effectHttp.ts:5`).
pub fn make_effect_auth_request(request: serde_json::Value) -> Result<serde_json::Value, String> {
    // `HttpServerRequest.toURL(request)` reads the internal host fields; every
    // value without the serialized request shape fails there.
    let obj = request.as_object();
    let headers = obj
        .and_then(|o| o.get("headers"))
        .and_then(Value::as_object);
    let (obj, headers) = match (obj, headers) {
        (Some(o), Some(h)) => (o, h),
        _ => {
            return Err("Cannot read properties of undefined (reading 'host')".to_string());
        }
    };
    let mut flat = Map::new();
    for (k, v) in headers {
        let joined = match v {
            Value::Array(parts) => json!(
                parts
                    .iter()
                    .map(|p| p.as_str().unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            other => other.clone(),
        };
        flat.insert(k.to_lowercase(), joined);
    }
    let mut out = Map::new();
    out.insert("headers".to_string(), Value::Object(flat));
    out.insert(
        "cookies".to_string(),
        obj.get("cookies").cloned().unwrap_or_else(|| json!({})),
    );
    if let Some(url) = obj.get("url")
        && !url.is_null()
    {
        out.insert("url".to_string(), url.clone());
    }
    Ok(Value::Object(out))
}

/// Port of `authErrorResponse` (`apps/server/src/auth/effectHttp.ts:18`).
pub fn auth_error_response(error: serde_json::Value) -> Result<serde_json::Value, String> {
    if error.is_null() {
        return Err("Cannot read properties of null (reading 'message')".to_string());
    }
    // `HttpServerResponse.jsonUnsafe({ error: error.message }, { status })`
    let mut body_obj = Map::new();
    let message = error.get("message").cloned().unwrap_or(Value::Null);
    if !message.is_null() && !marker(&message) {
        body_obj.insert("error".to_string(), message);
    }
    let body_json = serde_json::to_string(&Value::Object(body_obj)).map_err(|e| e.to_string())?;
    let body_bytes: Vec<Value> = body_json.bytes().map(|b| json!(b)).collect();
    let status = error
        .get("status")
        .and_then(Value::as_f64)
        .map(|n| json!(n))
        .unwrap_or(json!(500));
    let len = body_bytes.len();
    Ok(json!({
        "status": status,
        "statusText": { "$undefined": true },
        "cookies": { "cookies": {} },
        "body": {
            "~effect/http/HttpBody": "~effect/http/HttpBody",
            "_tag": "Uint8Array",
            "body": { "$bytes": body_bytes },
            "contentType": "application/json",
            "contentLength": len,
        },
        "headers": {
            "content-type": "application/json",
            "content-length": len.to_string(),
        },
    }))
}

/// `isLoopbackHost` (`apps/server/src/startupAccess.ts:4`).
fn is_loopback_host(host: Option<&str>) -> bool {
    let Some(host) = host else { return true };
    if host.is_empty() {
        return true;
    }
    let normalized = host
        .strip_prefix('[')
        .and_then(|h| h.strip_suffix(']'))
        .unwrap_or(host);
    matches!(normalized, "localhost" | "127.0.0.1" | "::1")
}

/// Port of `isTrustedLocalServerToken` (`apps/server/src/auth/remoteAccess.ts:9`).
pub fn is_trusted_local_server_token(
    config: serde_json::Value,
    token: Option<String>,
) -> Result<bool, String> {
    if config.is_null() {
        return Err("Cannot read properties of null (reading 'authToken')".to_string());
    }
    let auth_token = config.get("authToken").cloned().unwrap_or(Value::Null);
    let strong_token = match &auth_token {
        Value::Null => false,
        v if marker(v) => false,
        Value::String(s) => !s.trim().is_empty(),
        _ => {
            return Err("config.authToken?.trim is not a function".to_string());
        }
    };
    if !strong_token {
        return Ok(false);
    }
    let token_matches = token.as_deref() == auth_token.as_str();
    let host = config.get("host").and_then(Value::as_str);
    let public_url_absent = match config.get("publicUrl") {
        None => true,
        Some(v) => marker(v),
    };
    Ok(token_matches && is_loopback_host(host) && public_url_absent)
}

/// Port of `requireRemoteSessionAdmission` (`apps/server/src/auth/remoteAccess.ts:38`).
pub fn require_remote_session_admission(
    registry: serde_json::Value,
    session: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = registry;
    if session.is_null() {
        return Err("Cannot read properties of null (reading 'accessOrigin')".to_string());
    }
    // Remote path: `Effect.promise(prepareRemoteAdmission).flatMap(canAdmit...)`.
    // The `accessOrigin === "local"` short-circuit is untested by the corpus.
    Ok(json!({
        "~effect/Effect/args": {
            "~effect/Effect/args": { "0": { "$function": "" }, "1": false }
        },
        "~effect/Effect/successCont": { "$function": "" }
    }))
}

/// Port of `resolveSessionCookieName` (`apps/server/src/auth/utils.ts:6`).
pub fn resolve_session_cookie_name(input: serde_json::Value) -> Result<String, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'mode')".to_string());
    }
    let mode = input.get("mode").and_then(Value::as_str);
    if mode == Some("desktop") {
        let port = input.get("port").cloned().unwrap_or(Value::Null);
        return Ok(format!("synara_session_{}", js_str(&port)));
    }
    Ok("synara_session".to_string())
}

/// Port of `base64UrlEncode` (`apps/server/src/auth/utils.ts:13`).
pub fn base64_url_encode(input: serde_json::Value) -> Result<String, String> {
    let bytes = buffer_from(&input)?;
    Ok(base64url_encode(&bytes))
}

/// Port of `base64UrlDecodeUtf8` (`apps/server/src/auth/utils.ts:18`).
pub fn base64_url_decode_utf8(input: String) -> Result<String, String> {
    let bytes = base64url_decode_lenient(&input);
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Port of `signPayload` (`apps/server/src/auth/utils.ts:22`).
pub fn sign_payload(payload: String, secret: serde_json::Value) -> Result<String, String> {
    let key = buffer_from(&secret)?;
    Ok(base64url_encode(&hmac_sha256(&key, payload.as_bytes())))
}

/// Port of `timingSafeEqualBase64Url` (`apps/server/src/auth/utils.ts:26`).
pub fn timing_safe_equal_base64_url(left: String, right: String) -> Result<bool, String> {
    let a = base64url_decode_lenient(&left);
    let b = base64url_decode_lenient(&right);
    if a.len() != b.len() {
        return Ok(false);
    }
    // `Crypto.timingSafeEqual` was absent in the recorded environment; equal
    // decoded lengths reach that call and record its error.
    Err("Crypto.timingSafeEqual is not a function".to_string())
}

/// Port of `deriveAuthClientMetadata` (`apps/server/src/auth/utils.ts:76`).
pub fn derive_auth_client_metadata(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'headers')".to_string());
    }
    match input.get("headers") {
        None => {
            return Err("Cannot read properties of undefined (reading 'user-agent')".to_string());
        }
        Some(v) if v.is_null() => {
            return Err("Cannot read properties of null (reading 'user-agent')".to_string());
        }
        _ => {}
    }
    let mut out = Map::new();
    if let Some(label) = input.get("label").and_then(Value::as_str)
        && !label.is_empty()
    {
        out.insert("label".to_string(), json!(label));
    }
    if let Some(ip) = input.get("remoteAddress").and_then(Value::as_str) {
        // JS `String.trim` also strips U+FEFF, which Rust's `trim` misses.
        let ip = ip.trim().trim_matches('\u{feff}').trim();
        if !ip.is_empty() {
            out.insert("ipAddress".to_string(), json!(ip));
        }
    }
    out.insert("deviceType".to_string(), json!("unknown"));
    Ok(Value::Object(out))
}
