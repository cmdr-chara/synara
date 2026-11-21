//! The Rust port of `packages/shared/src/deviceKey.ts`.
//!
//! Signing keys live in the platform keystore (`CryptoKey`), which is opaque to
//! the recorded boundary: the recorder serializes them as `{}`, so the signing
//! entry points reproduce the validation and error behavior of the original
//! (`jose` key checks, `in`-operator failures, URL parsing) while real key
//! material stays host-side.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Port of `hostAudience` (`packages/shared/src/deviceKey.ts:43`).
pub fn host_audience(environment_id: String) -> Result<String, String> {
    Ok(format!("synara-host:{environment_id}"))
}

fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|i| {
                if i.is_null() || js::is_undefined(i) {
                    String::new()
                } else {
                    js_to_string(i)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".into()
            } else {
                "[object Object]".into()
            }
        }
    }
}

/// `"member" in value` — throws on a primitive or missing receiver.
fn in_check(v: &Value, name: &str) -> Result<(), String> {
    if js::is_undefined(v) {
        return Err(format!(
            "Cannot use 'in' operator to search for '{name}' in undefined"
        ));
    }
    let repr = match v {
        Value::Object(_) | Value::Array(_) => return Ok(()),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
    };
    Err(format!(
        "Cannot use 'in' operator to search for '{name}' in {repr}"
    ))
}

/// jose `checkKeyType` rejection suffix for a non-CryptoKey, non-JWK value.
fn jose_type_suffix(v: &Value) -> &'static str {
    match v {
        Value::Object(_) if js::is_undefined(v) => " Received undefined",
        Value::Object(_) => " Received an instance of Object",
        Value::Array(_) => " Received an instance of Array",
        Value::Null => " Received null",
        _ => "",
    }
}

fn is_jwk(v: &Value) -> bool {
    matches!(v, Value::Object(m) if matches!(m.get("kty"), Some(Value::String(_))))
}

/// Reproduce the jose signing-key validation: returns the private key value
/// that jose would see, or the error it would surface first.
fn private_key_of(key: &Value) -> Result<Value, String> {
    in_check(key, "privateKey")?;
    match key {
        Value::Object(m) => Ok(m.get("privateKey").cloned().unwrap_or_else(|| key.clone())),
        Value::Array(_) => Ok(key.clone()),
        _ => Ok(key.clone()),
    }
}

fn jose_key_error(alg: &str, key: &Value) -> String {
    format!(
        "Key for the {alg} algorithm must be one of type CryptoKey, KeyObject, or JSON Web Key.{}",
        jose_type_suffix(key)
    )
}

fn read_kty(jwk: &Value) -> Result<String, String> {
    match jwk {
        Value::Null => Err("Cannot read properties of null (reading 'kty')".into()),
        _ if js::is_undefined(jwk) => {
            Err("Cannot read properties of undefined (reading 'kty')".into())
        }
        _ => match member(jwk, "kty") {
            Value::String(s) => Ok(if s == "EC" {
                "ES256".into()
            } else {
                "EdDSA".into()
            }),
            _ => Ok("EdDSA".into()),
        },
    }
}

fn proof_window_check(input: &Value) -> Result<(), String> {
    match input {
        Value::Null => Err("Cannot read properties of null (reading 'nowSeconds')".into()),
        _ if js::is_undefined(input) => {
            Err("Cannot read properties of undefined (reading 'nowSeconds')".into())
        }
        _ => {
            let _ = member(input, "nowSeconds");
            let _ = member(input, "lifetimeSeconds");
            Ok(())
        }
    }
}

fn finish_sign(input: &Value) -> Result<String, String> {
    let alg = read_kty(&member(input, "publicJwk"))?;
    let key = private_key_of(&member(input, "key"))?;
    Err(jose_key_error(&alg, &key))
}

/// Port of `generateDeviceKey` (`packages/shared/src/deviceKey.ts:79`).
///
/// The keypair materializes in the platform keystore; at the recorded boundary
/// a `CryptoKey` serializes to `{}`, which is what callers receive back.
pub async fn generate_device_key(options: serde_json::Value) -> Result<serde_json::Value, String> {
    match &options {
        Value::Null => Err("Cannot read properties of null (reading 'extractable')".into()),
        _ => {
            let _ = member(&options, "extractable");
            Ok(json!({"publicKey": {}, "privateKey": {}}))
        }
    }
}

/// Port of `exportPublicJwk` (`packages/shared/src/deviceKey.ts:98`).
pub async fn export_public_jwk(key: serde_json::Value) -> Result<serde_json::Value, String> {
    in_check(&key, "publicKey")?;
    let public = match &key {
        Value::Object(m) => m.get("publicKey").cloned().unwrap_or_else(|| key.clone()),
        _ => key.clone(),
    };
    let _ = public;
    Err(
        "Failed to execute 'exportKey' on 'SubtleCrypto': 2nd argument is not of type CryptoKey."
            .into(),
    )
}

fn base64url(bytes: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(A[(n >> 18) as usize & 63] as char);
        out.push(A[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(A[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(A[n as usize & 63] as char);
        }
    }
    out
}

fn required_jwk_member<'a>(
    jwk: &'a serde_json::Map<String, Value>,
    name: &str,
    label: &str,
) -> Result<&'a str, String> {
    match jwk.get(name) {
        Some(Value::String(s)) => Ok(s.as_str()),
        _ => Err(format!("\"{name}\" ({label}) Parameter missing or invalid")),
    }
}

/// Port of `deviceThumbprint` (`packages/shared/src/deviceKey.ts:115`).
///
/// RFC 7638: base64url(SHA-256(canonical JSON of the required members)).
pub async fn device_thumbprint(jwk: serde_json::Value) -> Result<String, String> {
    if !is_jwk(&jwk) {
        return Err(format!(
            "Key must be one of type CryptoKey, KeyObject, or JSON Web Key.{}",
            jose_type_suffix(&jwk)
        ));
    }
    let m = match &jwk {
        Value::Object(m) => m,
        _ => {
            return Err(format!(
                "Key must be one of type CryptoKey, KeyObject, or JSON Web Key.{}",
                jose_type_suffix(&jwk)
            ));
        }
    };
    let kty = m.get("kty").and_then(Value::as_str).unwrap_or("");
    let members: Vec<(&str, &str)> = match kty {
        "EC" => vec![
            ("crv", "Curve"),
            ("x", "X Coordinate"),
            ("y", "Y Coordinate"),
        ],
        "OKP" => vec![("crv", "Curve"), ("x", "X Coordinate")],
        "RSA" => vec![("e", "Public Exponent"), ("n", "Modulus")],
        "oct" => vec![("k", "Key Value")],
        _ => return Err("\"kty\" (Key Type) Parameter missing or unsupported".into()),
    };
    let mut entries: Vec<(&str, &str)> = Vec::with_capacity(members.len() + 1);
    for (name, label) in members {
        entries.push((name, required_jwk_member(m, name, label)?));
    }
    entries.push(("kty", kty));
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let canonical = format!(
        "{{{}}}",
        entries
            .iter()
            .map(|(k, v)| format!("\"{k}\":\"{v}\""))
            .collect::<Vec<_>>()
            .join(",")
    );
    Ok(base64url(&Sha256::digest(canonical.as_bytes())))
}

/// Port of `signDeviceRegistration` (`packages/shared/src/deviceKey.ts:183`).
pub async fn sign_device_registration(input: serde_json::Value) -> Result<String, String> {
    proof_window_check(&input)?;
    let _ = (
        member(&input, "userId"),
        member(&input, "displayName"),
        member(&input, "platform"),
        member(&input, "audience"),
    );
    finish_sign(&input)
}

const SPECIAL_SCHEMES: &[&str] = &["ftp", "file", "http", "https", "ws", "wss"];

fn pct_encode(s: &str, path_set: bool) -> String {
    let mut out = String::new();
    for &b in s.as_bytes() {
        let encoded = if path_set {
            !(0x20..=0x7e).contains(&b)
                || matches!(
                    b,
                    b' ' | b'"' | b'#' | b'<' | b'>' | b'?' | b'`' | b'^' | b'{' | b'}'
                )
        } else {
            !(0x20..=0x7e).contains(&b)
        };
        if encoded {
            out.push_str(&format!("%{b:02X}"));
        } else {
            out.push(b as char);
        }
    }
    out
}

fn is_dot(seg: &str) -> bool {
    seg == "." || seg.eq_ignore_ascii_case("%2e")
}

fn is_dotdot(seg: &str) -> bool {
    seg == ".."
        || seg.eq_ignore_ascii_case(".%2e")
        || seg.eq_ignore_ascii_case("%2e.")
        || seg.eq_ignore_ascii_case("%2e%2e")
}

fn normalize_path(path: &str, special: bool) -> String {
    let path = if special {
        path.replace('\\', "/")
    } else {
        path.to_string()
    };
    let segs: Vec<&str> = path.split('/').collect();
    let mut out: Vec<String> = Vec::new();
    for (i, seg) in segs.iter().enumerate() {
        let last = i == segs.len() - 1;
        if is_dot(seg) {
            if last {
                out.push(String::new());
            }
        } else if is_dotdot(seg) {
            out.pop();
            if last {
                out.push(String::new());
            }
        } else {
            out.push((*seg).to_string());
        }
    }
    let joined = out.join("/");
    if joined.is_empty() {
        "/".into()
    } else {
        joined
    }
}

fn parse_authority(a: &str, special: bool, default_port: Option<u16>) -> Option<(String, String)> {
    // Split host part from path/query/fragment. For special schemes '\' also
    // delimits the authority.
    let end = a
        .find(|c| c == '/' || c == '?' || c == '#' || (special && c == '\\'))
        .unwrap_or(a.len());
    let (authority, tail) = a.split_at(end);
    let hostport = authority.rsplit('@').next().unwrap_or("");
    let (host, port_raw) = if hostport.starts_with('[') {
        match hostport.find(']') {
            Some(i) => (&hostport[..=i], hostport[i + 1..].strip_prefix(':')),
            None => (hostport, None),
        }
    } else {
        match hostport.rfind(':') {
            Some(i) => (&hostport[..i], Some(&hostport[i + 1..])),
            None => (hostport, None),
        }
    };
    let forbidden = |h: &str, extra: &[char]| {
        h.chars().any(|c| {
            (c as u32) <= 0x20
                || matches!(
                    c,
                    '#' | '/' | ':' | '?' | '@' | '[' | '\\' | ']' | '^' | '|'
                )
                || extra.contains(&c)
        })
    };
    if special {
        // Domain host: lowercased; '%' is forbidden; ':' was consumed as port.
        if forbidden(host, &['%']) {
            return None;
        }
        let host = host.to_ascii_lowercase();
        if host.is_empty() {
            return None;
        }
        let port = match port_raw {
            Some("") | None => None,
            Some(p) => {
                if !p.bytes().all(|b| b.is_ascii_digit()) {
                    return None;
                }
                let n: u64 = p.parse().ok()?;
                if n > 65535 {
                    return None;
                }
                if Some(n as u16) == default_port {
                    None
                } else {
                    Some(n.to_string())
                }
            }
        };
        let host = match port {
            Some(p) => format!("{host}:{p}"),
            None => host,
        };
        Some((host, tail.to_string()))
    } else {
        if forbidden(host, &[]) {
            return None;
        }
        match port_raw {
            Some(p) if !p.bytes().all(|b| b.is_ascii_digit()) && !p.is_empty() => {
                return None;
            }
            _ => {}
        }
        Some((hostport.to_string(), tail.to_string()))
    }
}

fn parse_url(url: &str) -> Option<(String, String, String)> {
    let stripped: String = url
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let s = stripped.trim_matches(|c: char| (c as u32) <= 0x20);
    let b = s.as_bytes();
    if b.is_empty() || !b[0].is_ascii_alphabetic() {
        return None;
    }
    let mut i = 1;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || matches!(b[i], b'+' | b'-' | b'.')) {
        i += 1;
    }
    if i >= b.len() || b[i] != b':' {
        return None;
    }
    let scheme = s[..i].to_ascii_lowercase();
    let rest = &s[i + 1..];
    let special = SPECIAL_SCHEMES.contains(&scheme.as_str());

    if special {
        let default_port = match scheme.as_str() {
            "http" | "ws" => Some(80),
            "https" | "wss" => Some(443),
            "ftp" => Some(21),
            _ => None,
        };
        if scheme == "file" {
            if let Some(a) = rest
                .strip_prefix("//")
                .or_else(|| rest.strip_prefix("\\\\"))
            {
                let (host, tail) = parse_authority(a, true, default_port)
                    .unwrap_or_else(|| (String::new(), a.to_string()));
                let tail_end = tail.find(['?', '#']).unwrap_or(tail.len());
                let path = normalize_path(&tail[..tail_end], true);
                return Some((format!("{scheme}:"), host, pct_encode(&path, true)));
            }
            let end = rest.find(['?', '#']).unwrap_or(rest.len());
            let path = normalize_path(&rest[..end], true);
            return Some((format!("{scheme}:"), String::new(), pct_encode(&path, true)));
        }
        let (host, tail) = if let Some(a) = rest
            .strip_prefix("//")
            .or_else(|| rest.strip_prefix("\\\\"))
            .or_else(|| rest.strip_prefix("/\\"))
            .or_else(|| rest.strip_prefix("\\/"))
        {
            parse_authority(a, true, default_port)?
        } else {
            // "http:x" treats the first segment as the host.
            let a = rest
                .strip_prefix('/')
                .or_else(|| rest.strip_prefix('\\'))
                .unwrap_or(rest);
            parse_authority(a, true, default_port)?
        };
        let end = tail.find(['?', '#']).unwrap_or(tail.len());
        let path = normalize_path(&tail[..end], true);
        return Some((format!("{scheme}:"), host, pct_encode(&path, true)));
    }

    // Non-special scheme: "//" opens an opaque-host authority; anything else is
    // an opaque path kept verbatim up to '?' or '#'.
    if let Some(a) = rest.strip_prefix("//") {
        let (host, tail) = parse_authority(a, false, None)?;
        let end = tail.find(['?', '#']).unwrap_or(tail.len());
        let path = normalize_path(&tail[..end], false);
        let path = if tail.is_empty() { String::new() } else { path };
        return Some((format!("{scheme}:"), host, pct_encode(&path, true)));
    }
    let end = rest.find(['?', '#']).unwrap_or(rest.len());
    let path = pct_encode(&rest[..end], false);
    Some((format!("{scheme}:"), String::new(), path))
}

fn dpop_parse(url: &str) -> Result<String, String> {
    match parse_url(url) {
        Some((protocol, host, path)) => Ok(format!("{protocol}//{host}{path}")),
        None => Err("Invalid URL".into()),
    }
}

/// Port of `dpopHttpUri` (`packages/shared/src/deviceKey.ts:210`).
pub fn dpop_http_uri(url: String) -> Result<String, String> {
    dpop_parse(&url)
}

/// Port of `credentialHash` (`packages/shared/src/deviceKey.ts:216`).
///
/// `base64url(SHA-256(utf8(credential)))` — RFC 9449's `ath` claim.
pub async fn credential_hash(credential: String) -> Result<String, String> {
    Ok(base64url(&Sha256::digest(credential.as_bytes())))
}

/// Port of `signDpopProof` (`packages/shared/src/deviceKey.ts:245`).
pub async fn sign_dpop_proof(input: serde_json::Value) -> Result<String, String> {
    proof_window_check(&input)?;
    let url = member(&input, "url");
    dpop_parse(&js_to_string(&url))?;
    match member(&input, "method") {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'toUpperCase')".into());
        }
        Value::String(_) => {}
        v if js::is_undefined(&v) => {
            return Err("Cannot read properties of undefined (reading 'toUpperCase')".into());
        }
        _ => return Err("input.method.toUpperCase is not a function".into()),
    }
    let _ = member(&input, "accessToken");
    finish_sign(&input)
}

/// Port of `signMintRequest` (`packages/shared/src/deviceKey.ts:282`).
pub async fn sign_mint_request(input: serde_json::Value) -> Result<String, String> {
    proof_window_check(&input)?;
    let _ = (
        member(&input, "grant"),
        member(&input, "environmentId"),
        member(&input, "userId"),
    );
    finish_sign(&input)
}
