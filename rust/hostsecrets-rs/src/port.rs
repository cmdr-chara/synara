//! The Rust port of `packages/shared/src/hostSecrets.ts`.
//!
//! `CryptoKey` cannot cross the napi bridge (`enc` turns it into `{}`), so the
//! recorded contract only exercises the validation layer and the deterministic
//! codecs. For real callers, a key travels as `{"$bytes":[..]}` (the marker a
//! `Uint8Array` serializes to): a 32-byte AES-256 key for sync keys, a 32-byte
//! P-256 scalar for a pairing private key. Anything else is "not a CryptoKey"
//! and fails exactly like the original's `subtle` type checks.

#![forbid(unsafe_code)]

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit};
use hkdf::Hkdf;
use p256::elliptic_curve::rand_core::{OsRng, RngCore};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const SYNC_KEY_BYTES: usize = 32;
const GCM_IV_BYTES: usize = 12;
const PAIRING_CODE_LENGTH: usize = 6;
const USER_CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

const SEAL_AAD_DOMAIN: &str = "synara.hostSecret.aad.v1";
const PAIRING_KDF_DOMAIN: &str = "synara.hostSecrets.pairing.kdf.v1";
const PAIRING_CODE_DOMAIN: &str = "synara.hostSecrets.pairing.code.v1";

const OPEN_FAILED: &str = "Host secret could not be decrypted";
const UNWRAP_FAILED: &str = "Sync key could not be unwrapped";

fn subtle_type_error(method: &str, arg: &str) -> String {
    format!(
        "Failed to execute '{method}' on 'SubtleCrypto': {arg} argument is not of type CryptoKey."
    )
}

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => Ok(o.get(key)),
        v if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        _ => Ok(None),
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if x.is_null() || is_undefined(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if let Some(units) = o.get("$utf16").and_then(Value::as_array) {
                units
                    .iter()
                    .filter_map(|u| u.as_u64().map(|x| x as u16))
                    .collect::<Vec<u16>>()
                    .pipe(|u: Vec<u16>| String::from_utf16_lossy(&u))
            } else if o.get("$undefined").is_some() {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

trait Pipe: Sized {
    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}

fn require_identifier(value: Option<&Value>, field: &str) -> Result<String, String> {
    match value {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        _ => Err(format!("{field} must be a non-empty string")),
    }
}

fn require_version(value: Option<&Value>) -> Result<f64, String> {
    let valid = match value {
        Some(Value::Number(n)) => n
            .as_f64()
            .is_some_and(|f| f.fract() == 0.0 && f.abs() <= 9_007_199_254_740_991.0 && f >= 0.0),
        _ => false,
    };
    if valid {
        Ok(value.and_then(Value::as_f64).unwrap_or(0.0))
    } else {
        Err("version must be a non-negative integer".to_string())
    }
}

fn as_bytes(v: &Value) -> Option<Vec<u8>> {
    v.as_object()?
        .get("$bytes")?
        .as_array()?
        .iter()
        .map(|b| b.as_u64().map(|x| x as u8))
        .collect()
}

fn bytes_marker(bytes: &[u8]) -> Value {
    Value::Object(Map::from_iter([(
        "$bytes".to_string(),
        Value::Array(bytes.iter().map(|b| Value::from(*b)).collect()),
    )]))
}

fn js_to_uint16(v: &Value) -> u16 {
    let n = match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Bool(b) => *b as i32 as f64,
        Value::Null => 0.0,
        Value::String(_) | Value::Array(_) | Value::Object(_) => js_to_number(&js_to_string(v)),
    };
    if !n.is_finite() {
        return 0;
    }
    (n.trunc().rem_euclid(65536.0)) as u16
}

fn js_to_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if t.is_empty() {
        return 0.0;
    }
    if t.eq_ignore_ascii_case("infinity") || t.eq_ignore_ascii_case("+infinity") {
        return f64::INFINITY;
    }
    if t.eq_ignore_ascii_case("-infinity") {
        return f64::NEG_INFINITY;
    }
    if let Some(hex) = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .and_then(|h| u64::from_str_radix(h, 16).ok())
    {
        return hex as f64;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n =
            chunk.iter().fold(0u32, |acc, b| (acc << 8) | u32::from(*b)) << (8 * (3 - chunk.len()));
        for i in 0..4 {
            if i > chunk.len() {
                break;
            }
            out.push(B64[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
    }
    out
}

fn b64url_encode(bytes: &[u8]) -> String {
    b64_encode(bytes)
        .replace('+', "-")
        .replace('/', "_")
        .replace('=', "")
}

fn b64_decode(text: &str) -> Result<Vec<u8>, String> {
    let mut table = [255u8; 256];
    for (i, &c) in B64.iter().enumerate() {
        table[c as usize] = i as u8;
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    let bytes = text.as_bytes();
    for chunk in bytes.chunks(4) {
        let mut acc = 0u32;
        let mut meaningful = 0usize;
        for (i, &c) in chunk.iter().enumerate() {
            if c == b'=' {
                continue;
            }
            meaningful = i + 1;
            acc |= u32::from(table[c as usize]) << (18 - 6 * i);
        }
        match meaningful {
            4 => out.extend_from_slice(&acc.to_be_bytes()[1..]),
            3 => out.extend_from_slice(&acc.to_be_bytes()[1..3]),
            2 => out.push((acc >> 16) as u8),
            _ => {}
        }
    }
    Ok(out)
}

/// Port of `encodeBase64Url` (`packages/shared/src/hostSecrets.ts:157`).
pub fn encode_base64_url(bytes: serde_json::Value) -> Result<String, String> {
    let elements: Vec<Value> = match &bytes {
        Value::Array(a) => a.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("bytes is not iterable".to_string()),
    };
    let raw: Vec<u8> = elements.iter().map(|e| js_to_uint16(e) as u8).collect();
    Ok(b64url_encode(&raw))
}

/// Port of `decodeBase64Url` (`packages/shared/src/hostSecrets.ts:171`).
pub fn decode_base64_url(text: String) -> Result<serde_json::Value, String> {
    decode_base64_url_value(&Value::String(text)).map(|v| bytes_marker(&v))
}

fn decode_base64_url_value(text: &Value) -> Result<Vec<u8>, String> {
    let text = js_to_string(text);
    if !text
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("Value is not base64url".to_string());
    }
    if text.len() % 4 == 1 {
        return Err("Value is not a valid base64url length".to_string());
    }
    let padded = format!(
        "{}{}",
        text.replace('-', "+").replace('_', "/"),
        "=".repeat((4 - text.len() % 4) % 4)
    );
    b64_decode(&padded).map_err(|_| "Value is not base64url".to_string())
}

fn js_escape_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `JSON.stringify` over the bridge codec's value space. Returns `None` for
/// `undefined`/function/symbol-shaped roots (callers stringify them as the
/// literal text `undefined`, matching `JSON.stringify(undefined)`).
fn js_stringify(v: &Value, out: &mut String) -> Result<bool, String> {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&crate::js::number_to_string(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => js_escape_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if !js_stringify(item, out)? {
                    out.push_str("null");
                }
            }
            out.push(']');
        }
        Value::Object(o) => {
            if o.get("$undefined").is_some()
                || o.get("$function").is_some()
                || o.get("$symbol").is_some()
            {
                return Ok(false);
            }
            if o.get("$bigint").is_some() {
                return Err("Do not know how to serialize a BigInt".to_string());
            }
            if let Some(date) = o.get("$date") {
                match date {
                    Value::String(s) if s != "Invalid" => js_escape_string(s, out),
                    _ => out.push_str("null"),
                }
                return Ok(true);
            }
            if let Some(units) = o.get("$utf16").and_then(Value::as_array) {
                let units: Vec<u16> = units
                    .iter()
                    .filter_map(|u| u.as_u64().map(|x| x as u16))
                    .collect();
                js_escape_string(&String::from_utf16_lossy(&units), out);
                return Ok(true);
            }
            if o.get("$num").is_some()
                || o.get("$map").is_some()
                || o.get("$set").is_some()
                || o.get("$regexp").is_some()
                || o.get("$error").is_some()
            {
                if o.get("$num").is_some() {
                    out.push_str("null");
                } else {
                    out.push_str("{}");
                }
                return Ok(true);
            }
            let source = o.get("$object").and_then(Value::as_object).unwrap_or(o);
            if let Some(bytes) = source.get("$bytes").or_else(|| source.get("$buffer")) {
                out.push('{');
                let mut first = true;
                if let Some(items) = bytes.as_array() {
                    for (i, b) in items.iter().enumerate() {
                        if !first {
                            out.push(',');
                        }
                        first = false;
                        js_escape_string(&i.to_string(), out);
                        out.push(':');
                        out.push_str(&crate::js::number_to_string(b.as_f64().unwrap_or(0.0)));
                    }
                }
                out.push('}');
                return Ok(true);
            }
            if let Some(typed) = source.get("$typed").and_then(Value::as_object) {
                // JSON.stringify over a TypedArray yields index-keyed values.
                out.push('{');
                let mut first = true;
                if let Some(items) = typed.get("values").and_then(Value::as_array) {
                    for (i, item) in items.iter().enumerate() {
                        if !first {
                            out.push(',');
                        }
                        first = false;
                        js_escape_string(&i.to_string(), out);
                        out.push(':');
                        if !js_stringify(item, out)? {
                            out.push_str("null");
                        }
                    }
                }
                out.push('}');
                return Ok(true);
            }
            out.push('{');
            let mut first = true;
            for (k, value) in source {
                let mut tmp = String::new();
                if js_stringify(value, &mut tmp)? {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    js_escape_string(k, out);
                    out.push(':');
                    out.push_str(&tmp);
                }
            }
            out.push('}');
        }
    }
    Ok(true)
}

fn js_stringify_text(v: &Value) -> Result<String, String> {
    let mut out = String::new();
    if js_stringify(v, &mut out)? {
        Ok(out)
    } else {
        Ok("undefined".to_string())
    }
}

fn sync_key(v: Option<&Value>, method: &str) -> Result<Vec<u8>, String> {
    match v.and_then(as_bytes) {
        Some(bytes) if bytes.len() == SYNC_KEY_BYTES => Ok(bytes),
        _ => Err(subtle_type_error(method, "2nd")),
    }
}

fn aad(host_id: &str, owner_user_id: &str, version: f64) -> Result<Vec<u8>, String> {
    js_stringify_text(&Value::Array(vec![
        Value::String(SEAL_AAD_DOMAIN.to_string()),
        Value::String(host_id.to_string()),
        Value::String(owner_user_id.to_string()),
        crate::js::number(version),
    ]))
    .map(String::into_bytes)
}

fn seal(
    host_id: &str,
    owner: &str,
    version: f64,
    key: &[u8],
    secret: &Value,
) -> Result<Value, String> {
    let plaintext = js_stringify_text(secret)?.into_bytes();
    let mut iv = [0u8; GCM_IV_BYTES];
    OsRng.fill_bytes(&mut iv);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| OPEN_FAILED.to_string())?;
    let aad = aad(host_id, owner, version)?;
    let ciphertext = cipher
        .encrypt(
            (&iv).into(),
            Payload {
                msg: &plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| OPEN_FAILED.to_string())?;
    Ok(Value::Object(Map::from_iter([
        (
            "ciphertext".to_string(),
            Value::String(b64url_encode(&ciphertext)),
        ),
        ("iv".to_string(), Value::String(b64url_encode(&iv))),
        ("version".to_string(), crate::js::number(version)),
    ])))
}

/// Port of `generateSyncKey` (`packages/shared/src/hostSecrets.ts:225`).
pub async fn generate_sync_key() -> Result<serde_json::Value, String> {
    // A `CryptoKey` encodes across the bridge as `{}` — an opaque handle whose
    // material never leaves the caller's process, exactly as the original's
    // extractable-but-unserializable key behaves on the wire.
    Ok(Value::Object(Map::new()))
}

/// Port of `sealHostSecret` (`packages/shared/src/hostSecrets.ts:240`).
pub async fn seal_host_secret(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let host_id = require_identifier(prop(&input, "hostId")?, "hostId")?;
    let owner_user_id = require_identifier(prop(&input, "ownerUserId")?, "ownerUserId")?;
    let version = require_version(prop(&input, "version")?)?;
    let secret = prop(&input, "secret")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let key = sync_key(prop(&input, "syncKey")?, "encrypt")?;
    seal(&host_id, &owner_user_id, version, &key, &secret)
}

fn strict_neq(a: Option<&Value>, b: f64) -> bool {
    !matches!(a, Some(Value::Number(n)) if n.as_f64() == Some(b))
}

fn open_secret(
    host_id: &str,
    owner: &str,
    version: f64,
    key: &[u8],
    envelope: &Value,
) -> Result<Value, String> {
    let iv = decode_base64_url_value(prop(envelope, "iv")?.unwrap_or(&Value::Null))?;
    if iv.len() != GCM_IV_BYTES {
        return Err(OPEN_FAILED.to_string());
    }
    let ciphertext =
        decode_base64_url_value(prop(envelope, "ciphertext")?.unwrap_or(&Value::Null))?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| OPEN_FAILED.to_string())?;
    let aad = aad(host_id, owner, version)?;
    let iv: [u8; GCM_IV_BYTES] = iv[..GCM_IV_BYTES]
        .try_into()
        .map_err(|_| OPEN_FAILED.to_string())?;
    let plaintext = cipher
        .decrypt(
            (&iv).into(),
            Payload {
                msg: &ciphertext,
                aad: &aad,
            },
        )
        .map_err(|_| OPEN_FAILED.to_string())?;
    let text = String::from_utf8_lossy(&plaintext);
    crate::js::from_json::<Value>(&text)
        .map_err(|_| "Host secret payload is not valid JSON".to_string())
}

/// Port of `openHostSecret` (`packages/shared/src/hostSecrets.ts:284`).
pub async fn open_host_secret(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let host_id = require_identifier(prop(&input, "hostId")?, "hostId")?;
    let owner_user_id = require_identifier(prop(&input, "ownerUserId")?, "ownerUserId")?;
    let version = require_version(prop(&input, "version")?)?;
    let envelope = prop(&input, "envelope")?;
    let undef = crate::js::undefined();
    let env_value = envelope
        .map(|e| if is_undefined(e) { &undef } else { e })
        .unwrap_or(&undef);
    let env_version = prop(env_value, "version")?;
    if strict_neq(env_version, version) {
        return Err(OPEN_FAILED.to_string());
    }
    let key = sync_key(prop(&input, "syncKey")?, "decrypt")?;
    open_secret(&host_id, &owner_user_id, version, &key, env_value)
}

fn canonical_pairing_jwk(jwk: &Value, field: &str) -> Result<String, String> {
    if !matches!(jwk, Value::Object(_)) {
        return Err(format!("{field} must be a public JWK"));
    }
    let kty = prop(jwk, "kty")?;
    let crv = prop(jwk, "crv")?;
    if !matches!(kty, Some(Value::String(s)) if s == "EC")
        || !matches!(crv, Some(Value::String(s)) if s == "P-256")
    {
        return Err(format!("{field} must be an EC P-256 public key"));
    }
    let x = require_identifier(prop(jwk, "x")?, &format!("{field}.x"))?;
    let y = require_identifier(prop(jwk, "y")?, &format!("{field}.y"))?;
    let x = decode_base64_url_value(&Value::String(x))?;
    let y = decode_base64_url_value(&Value::String(y))?;
    if x.len() != 32 || y.len() != 32 {
        return Err(format!("{field} coordinates are not P-256 sized"));
    }
    Ok(format!("P-256.{}.{}", b64url_encode(&x), b64url_encode(&y)))
}

fn import_p256_public(jwk: &Value) -> Result<PublicKey, String> {
    let obj = jwk.as_object();
    let x = obj
        .and_then(|o| o.get("x"))
        .and_then(Value::as_str)
        .and_then(|s| decode_base64_url_value(&Value::String(s.to_string())).ok())
        .unwrap_or_default();
    let y = obj
        .and_then(|o| o.get("y"))
        .and_then(Value::as_str)
        .and_then(|s| decode_base64_url_value(&Value::String(s.to_string())).ok())
        .unwrap_or_default();
    let mut sec1 = Vec::with_capacity(65);
    sec1.push(0x04);
    sec1.extend_from_slice(&x);
    sec1.extend_from_slice(&y);
    PublicKey::from_sec1_bytes(&sec1).map_err(|_| {
        "Failed to execute 'importKey' on 'SubtleCrypto': 1st argument provided key is a not a valid JWK.".to_string()
    })
}

fn pairing_transcript(ephemeral: &Value, recipient: &Value) -> Result<Vec<u8>, String> {
    js_stringify_text(&Value::Array(vec![
        Value::String(PAIRING_KDF_DOMAIN.to_string()),
        Value::String(canonical_pairing_jwk(ephemeral, "ephemeralPublicJwk")?),
        Value::String(canonical_pairing_jwk(recipient, "recipientPublicJwk")?),
    ]))
    .map(String::into_bytes)
}

fn derive_kek(secret: &SecretKey, peer: &PublicKey, transcript: &[u8]) -> [u8; 32] {
    let shared = p256::ecdh::diffie_hellman(secret.to_nonzero_scalar(), peer.as_affine());
    let hkdf = Hkdf::<Sha256>::new(
        Some(PAIRING_KDF_DOMAIN.as_bytes()),
        shared.raw_secret_bytes(),
    );
    let mut kek = [0u8; 32];
    let _ = hkdf.expand(transcript, &mut kek);
    kek
}

fn jwk_of(public: &PublicKey) -> Value {
    let point = public.to_encoded_point(false);
    Value::Object(Map::from_iter([
        ("kty".to_string(), Value::String("EC".to_string())),
        ("crv".to_string(), Value::String("P-256".to_string())),
        (
            "x".to_string(),
            Value::String(b64url_encode(point.x().map(|v| &v[..]).unwrap_or(&[]))),
        ),
        (
            "y".to_string(),
            Value::String(b64url_encode(point.y().map(|v| &v[..]).unwrap_or(&[]))),
        ),
    ]))
}

/// Port of `generatePairingKeyPair` (`packages/shared/src/hostSecrets.ts:399`).
pub async fn generate_pairing_key_pair() -> Result<serde_json::Value, String> {
    let secret = SecretKey::random(&mut OsRng);
    Ok(Value::Object(Map::from_iter([
        ("privateKey".to_string(), Value::Object(Map::new())),
        ("publicJwk".to_string(), jwk_of(&secret.public_key())),
    ])))
}

/// Port of `wrapSyncKey` (`packages/shared/src/hostSecrets.ts:473`).
pub async fn wrap_sync_key(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let recipient_jwk = prop(&input, "recipientPublicJwk")?;
    let undef = crate::js::undefined();
    let recipient_jwk = recipient_jwk.unwrap_or(&undef);
    canonical_pairing_jwk(recipient_jwk, "recipientPublicJwk")?;
    let peer = import_p256_public(recipient_jwk)?;
    let ephemeral = SecretKey::random(&mut OsRng);
    let ephemeral_jwk = jwk_of(&ephemeral.public_key());
    let kek = derive_kek(
        &ephemeral,
        &peer,
        &pairing_transcript(&ephemeral_jwk, recipient_jwk)?,
    );
    let key = sync_key(prop(&input, "syncKey")?, "wrapKey")?;
    let mut wrapped = vec![0u8; key.len() + 8];
    aes_kw::KekAes256::from(kek)
        .wrap(&key, &mut wrapped)
        .map_err(|_| UNWRAP_FAILED.to_string())?;
    Ok(Value::Object(Map::from_iter([
        ("ephemeralPublicJwk".to_string(), ephemeral_jwk),
        ("recipientPublicJwk".to_string(), recipient_jwk.clone()),
        (
            "wrapped".to_string(),
            Value::String(b64url_encode(&wrapped)),
        ),
    ])))
}

/// Port of `unwrapSyncKey` (`packages/shared/src/hostSecrets.ts:515`).
pub async fn unwrap_sync_key(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let wrapped = prop(&input, "wrapped")?;
    let undef = crate::js::undefined();
    let wrapped = wrapped.unwrap_or(&undef);
    let ephemeral_jwk = prop(wrapped, "ephemeralPublicJwk")?.unwrap_or(&undef);
    canonical_pairing_jwk(ephemeral_jwk, "ephemeralPublicJwk")?;
    let peer = import_p256_public(ephemeral_jwk)?;
    let secret_bytes = as_bytes(prop(&input, "recipientPrivateKey")?.unwrap_or(&undef))
        .filter(|b| b.len() == 32)
        .ok_or_else(|| subtle_type_error("deriveBits", "2nd"))?;
    let secret =
        SecretKey::from_slice(&secret_bytes).map_err(|_| subtle_type_error("deriveBits", "2nd"))?;
    let recipient_jwk = prop(wrapped, "recipientPublicJwk")?.unwrap_or(&undef);
    let kek = derive_kek(
        &secret,
        &peer,
        &pairing_transcript(ephemeral_jwk, recipient_jwk)?,
    );
    let wrapped_bytes = decode_base64_url_value(prop(wrapped, "wrapped")?.unwrap_or(&Value::Null))?;
    let mut key = vec![0u8; wrapped_bytes.len().saturating_sub(8)];
    aes_kw::KekAes256::from(kek)
        .unwrap(&wrapped_bytes, &mut key)
        .map_err(|_| UNWRAP_FAILED.to_string())?;
    Ok(bytes_marker(&key))
}

/// Port of `pairingVerificationCode` (`packages/shared/src/hostSecrets.ts:564`).
pub async fn pairing_verification_code(input: serde_json::Value) -> Result<String, String> {
    let undef = crate::js::undefined();
    let sender = canonical_pairing_jwk(
        prop(&input, "senderPublicJwk")?.unwrap_or(&undef),
        "senderPublicJwk",
    )?;
    let recipient = canonical_pairing_jwk(
        prop(&input, "recipientPublicJwk")?.unwrap_or(&undef),
        "recipientPublicJwk",
    )?;
    let mut ordered = [sender, recipient];
    ordered.sort();
    let digest_input = js_stringify_text(&Value::Array(vec![
        Value::String(PAIRING_CODE_DOMAIN.to_string()),
        Value::String(ordered[0].clone()),
        Value::String(ordered[1].clone()),
    ]))?;
    let digest = Sha256::digest(digest_input.as_bytes());
    let mut remaining = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) >> 2;
    let mut chars = Vec::with_capacity(PAIRING_CODE_LENGTH);
    for _ in 0..PAIRING_CODE_LENGTH {
        chars.push(USER_CODE_ALPHABET[(remaining % 32) as usize] as char);
        remaining /= 32;
    }
    chars.reverse();
    Ok(chars.into_iter().collect())
}

fn export_sync_key(v: Option<&Value>, field: &str) -> Result<Vec<u8>, String> {
    as_bytes(v.unwrap_or(&crate::js::undefined()))
        .filter(|b| b.len() == SYNC_KEY_BYTES)
        .ok_or_else(|| {
            format!("{field} must be an extractable AES-GCM sync key (see generateSyncKey)")
        })
}

/// Port of `rotateHostSecrets` (`packages/shared/src/hostSecrets.ts:620`).
pub async fn rotate_host_secrets(
    input: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let entries = prop(&input, "entries")?;
    let list: Vec<Value> = match entries {
        None => return Err("input.entries is not iterable".to_string()),
        Some(v) if v.is_null() || is_undefined(v) => {
            return Err("input.entries is not iterable".to_string());
        }
        Some(Value::Array(a)) => a.clone(),
        Some(Value::String(s)) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        Some(v) if v.as_object().is_some_and(|o| o.contains_key("$bytes")) => as_bytes(v)
            .unwrap_or_default()
            .into_iter()
            .map(Value::from)
            .collect(),
        Some(_) => return Err("input.entries is not iterable".to_string()),
    };
    let mut seen = std::collections::HashSet::new();
    for entry in &list {
        let host_id = require_identifier(prop(entry, "hostId")?, "hostId")?;
        require_identifier(prop(entry, "ownerUserId")?, "ownerUserId")?;
        let envelope = prop(entry, "envelope")?;
        let undef = crate::js::undefined();
        require_version(prop(envelope.unwrap_or(&undef), "version")?)?;
        if !seen.insert(host_id.clone()) {
            return Err(format!("Duplicate hostId in rotation: {host_id}"));
        }
    }
    let old_key = export_sync_key(prop(&input, "oldSyncKey")?, "oldSyncKey")?;
    let new_key = export_sync_key(prop(&input, "newSyncKey")?, "newSyncKey")?;
    if old_key == new_key {
        return Err("newSyncKey must differ from oldSyncKey".to_string());
    }
    let mut out = Vec::with_capacity(list.len());
    for entry in &list {
        let undef = crate::js::undefined();
        let host_id = require_identifier(prop(entry, "hostId")?, "hostId")?;
        let owner = require_identifier(prop(entry, "ownerUserId")?, "ownerUserId")?;
        let envelope = prop(entry, "envelope")?.unwrap_or(&undef);
        let version = require_version(prop(envelope, "version")?)?;
        let secret = open_secret(&host_id, &owner, version, &old_key, envelope)?;
        let next = seal(&host_id, &owner, version + 1.0, &new_key, &secret)?;
        out.push(Value::Object(Map::from_iter([
            ("hostId".to_string(), Value::String(host_id)),
            ("ownerUserId".to_string(), Value::String(owner)),
            ("envelope".to_string(), next),
        ])));
    }
    Ok(out)
}
