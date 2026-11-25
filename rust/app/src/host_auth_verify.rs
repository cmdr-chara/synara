//! Device session credential verification, port of upstream
//! hostAuth/credential.ts + replayCache.ts. A session credential is an EdDSA
//! JWT minted by the host identity; the DPoP proof binds it to the device key
//! and the exact request (htm/htu/ath), with a jti replay cache retained past
//! the verifier's clock tolerance.

use std::collections::HashMap;
use std::sync::Mutex;

use base64::Engine;
use ed25519_dalek::Verifier;
use serde_json::{json, Value};
use sha2::Digest;

use crate::host_identity::HostIdentity;

pub const SESSION_CREDENTIAL_JWT_TYP: &str = "synara-session-credential+jwt";
pub const DPOP_JWT_TYP: &str = "dpop+jwt";
pub const SESSION_CREDENTIAL_MAX_AGE_SECONDS: u64 = 60 * 60;
pub const JWT_CLOCK_TOLERANCE_SECONDS: u64 = 60;
pub const HOST_CONNECT_SCOPE: &str = "host:connect";
pub const SYNARA_SESSION_AUDIENCE: &str = "synara-session";

fn b64url_decode(data: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .map_err(|e| format!("base64url decode failed: {e}"))
}

fn b64url(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn sha256_bytes(data: &[u8]) -> Vec<u8> {
    sha2::Sha256::digest(data).to_vec()
}

/// RFC 7638 thumbprint: lexicographically ordered required members, no
/// whitespace — the exact bytes jose hashes upstream.
fn jwk_thumbprint(jwk: &Value) -> String {
    let crv = jwk.get("crv").and_then(Value::as_str).unwrap_or("");
    let kty = jwk.get("kty").and_then(Value::as_str).unwrap_or("");
    let x = jwk.get("x").and_then(Value::as_str).unwrap_or("");
    let canonical = if kty == "EC" {
        let y = jwk.get("y").and_then(Value::as_str).unwrap_or("");
        json!({"crv": crv, "kty": kty, "x": x, "y": y}).to_string()
    } else {
        json!({"crv": crv, "kty": kty, "x": x}).to_string()
    };
    b64url(&sha256_bytes(canonical.as_bytes()))
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn split_jwt(jwt: &str) -> Result<(Value, Value, Vec<u8>, String), String> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err("not a compact JWT".to_string());
    }
    let header: Value = serde_json::from_slice(&b64url_decode(parts[0])?)
        .map_err(|e| format!("header is not json: {e}"))?;
    let payload: Value = serde_json::from_slice(&b64url_decode(parts[1])?)
        .map_err(|e| format!("payload is not json: {e}"))?;
    let signature = b64url_decode(parts[2])?;
    Ok((header, payload, signature, format!("{}.{}", parts[0], parts[1])))
}

fn verify_ed25519(signing_input: &str, signature: &[u8], x_b64: &str) -> Result<(), String> {
    let x = b64url_decode(x_b64)?;
    let x: [u8; 32] = x.try_into().map_err(|_| "ed25519 public key is not 32 bytes".to_string())?;
    let key = ed25519_dalek::VerifyingKey::from_bytes(&x)
        .map_err(|e| format!("ed25519 key invalid: {e}"))?;
    let sig = ed25519_dalek::Signature::from_slice(signature)
        .map_err(|e| format!("signature malformed: {e}"))?;
    key.verify(signing_input.as_bytes(), &sig)
        .map_err(|_| "signature verification failed".to_string())
}

fn verify_p256(signing_input: &str, signature: &[u8], x_b64: &str, y_b64: &str) -> Result<(), String> {
    use p256::ecdsa::signature::Verifier as _;
    let x = b64url_decode(x_b64)?;
    let y = b64url_decode(y_b64)?;
    if x.len() != 32 || y.len() != 32 {
        return Err("p256 coordinates are not 32 bytes".to_string());
    }
    let mut point = vec![0x04u8];
    point.extend_from_slice(&x);
    point.extend_from_slice(&y);
    let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(&point)
        .map_err(|e| format!("p256 key invalid: {e}"))?;
    // JOSE ECDSA signatures are raw R||S, not DER.
    let sig = p256::ecdsa::Signature::from_slice(signature)
        .map_err(|e| format!("signature malformed: {e}"))?;
    key.verify(signing_input.as_bytes(), &sig)
        .map_err(|_| "signature verification failed".to_string())
}

/// jti replay cache: entries live until exp plus the verifier tolerance, so a
/// token cannot replay inside the acceptance window.
pub struct JwtReplayCache {
    entries: Mutex<HashMap<String, u64>>,
}

impl Default for JwtReplayCache {
    fn default() -> Self {
        Self::new()
    }
}

impl JwtReplayCache {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn consume(&self, jti: &str, expires_at_seconds: u64) -> Result<(), String> {
        let now = now_seconds();
        let mut entries = self.entries.lock().map_err(|e| e.to_string())?;
        entries.retain(|_, expires_at| *expires_at >= now);
        if entries.contains_key(jti) {
            return Err("JWT jti has already been used".to_string());
        }
        entries.insert(jti.to_string(), expires_at_seconds + JWT_CLOCK_TOLERANCE_SECONDS);
        Ok(())
    }
}

pub struct VerifiedSessionCredential {
    pub trust_generation: u64,
    pub user_id: String,
    pub device_jkt: String,
    pub expires_at_seconds: u64,
}

/// Verifies the credential and its DPoP proof exactly as upstream: issuer,
/// audience, typ, lifetime bounds, key generation, scope, cnf binding, then
/// proof htm/htu/ath/freshness and replay consumption.
#[allow(clippy::too_many_arguments)]
pub fn verify_session_credential(
    credential: &str,
    dpop: &str,
    identity: &HostIdentity,
    environment_id: &str,
    key_generation: u64,
    expected_htu: &str,
    expected_htm: &str,
    replay_cache: Option<&JwtReplayCache>,
) -> Result<VerifiedSessionCredential, String> {
    let now = now_seconds();
    let (header, claims, signature, signing_input) = split_jwt(credential)?;
    if header.get("alg").and_then(Value::as_str) != Some("EdDSA")
        || header.get("typ").and_then(Value::as_str) != Some(SESSION_CREDENTIAL_JWT_TYP)
    {
        return Err("session credential header is invalid".to_string());
    }
    let host_x = identity
        .public_key_jwk
        .get("x")
        .and_then(Value::as_str)
        .ok_or_else(|| "host identity jwk incomplete".to_string())?;
    verify_ed25519(&signing_input, &signature, host_x)
        .map_err(|_| "session credential signature is invalid".to_string())?;

    let expected_iss = format!("synara-host:{environment_id}");
    let str_claim = |key: &str| claims.get(key).and_then(Value::as_str).unwrap_or("");
    let num_claim = |key: &str| claims.get(key).and_then(Value::as_u64).unwrap_or(0);
    if str_claim("iss") != expected_iss {
        return Err("session credential issuer is invalid".to_string());
    }
    let aud_matches = match claims.get("aud") {
        Some(Value::String(aud)) => aud == SYNARA_SESSION_AUDIENCE,
        Some(Value::Array(auds)) => auds.iter().any(|a| a.as_str() == Some(SYNARA_SESSION_AUDIENCE)),
        _ => false,
    };
    if !aud_matches {
        return Err("session credential audience is invalid".to_string());
    }
    let (iat, exp) = (num_claim("iat"), num_claim("exp"));
    if exp <= iat || exp - iat > SESSION_CREDENTIAL_MAX_AGE_SECONDS {
        return Err("session credential claims are invalid".to_string());
    }
    if now + JWT_CLOCK_TOLERANCE_SECONDS < iat || now > exp + JWT_CLOCK_TOLERANCE_SECONDS {
        return Err("session credential is not currently valid".to_string());
    }
    if num_claim("keyGeneration") != key_generation {
        return Err("session credential claims are invalid".to_string());
    }
    let scope_ok = claims
        .get("scope")
        .and_then(Value::as_array)
        .and_then(|s| s.first())
        .and_then(Value::as_str)
        == Some(HOST_CONNECT_SCOPE);
    if !scope_ok {
        return Err("session credential claims are invalid".to_string());
    }
    let cnf_jkt = claims
        .get("cnf")
        .and_then(|c| c.get("jkt"))
        .and_then(Value::as_str)
        .unwrap_or("");

    // ── DPoP proof ──────────────────────────────────────────────────────
    let (dheader, dclaims, dsignature, dsigning_input) = split_jwt(dpop)?;
    let jwk = dheader
        .get("jwk")
        .cloned()
        .ok_or_else(|| "DPoP protected header is invalid".to_string())?;
    let kty = jwk.get("kty").and_then(Value::as_str).unwrap_or("");
    let crv = jwk.get("crv").and_then(Value::as_str).unwrap_or("");
    let algorithm = match (kty, crv) {
        ("OKP", "Ed25519") => "EdDSA",
        ("EC", "P-256") => "ES256",
        _ => return Err("DPoP protected header is invalid".to_string()),
    };
    if dheader.get("typ").and_then(Value::as_str) != Some(DPOP_JWT_TYP)
        || dheader.get("alg").and_then(Value::as_str) != Some(algorithm)
    {
        return Err("DPoP protected header is invalid".to_string());
    }
    let jkt = jwk_thumbprint(&jwk);
    if jkt != cnf_jkt {
        return Err("DPoP key does not match credential cnf.jkt".to_string());
    }
    let x = jwk.get("x").and_then(Value::as_str).unwrap_or("");
    if algorithm == "EdDSA" {
        verify_ed25519(&dsigning_input, &dsignature, x)
            .map_err(|_| "DPoP signature is invalid".to_string())?;
    } else {
        let y = jwk.get("y").and_then(Value::as_str).unwrap_or("");
        verify_p256(&dsigning_input, &dsignature, x, y)
            .map_err(|_| "DPoP signature is invalid".to_string())?;
    }
    let dstr = |key: &str| dclaims.get(key).and_then(Value::as_str).unwrap_or("");
    let diat = dclaims.get("iat").and_then(Value::as_u64).unwrap_or(0);
    let expected_ath = b64url(&sha256_bytes(credential.as_bytes()));
    if dstr("htu") != expected_htu
        || dstr("htm") != expected_htm
        || diat.abs_diff(now) > JWT_CLOCK_TOLERANCE_SECONDS
        || dstr("ath") != expected_ath
    {
        return Err("DPoP claims are invalid".to_string());
    }
    if let Some(cache) = replay_cache {
        cache.consume(dstr("jti"), now + JWT_CLOCK_TOLERANCE_SECONDS)?;
    }
    Ok(VerifiedSessionCredential {
        trust_generation: claims
            .get("trustGeneration")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        user_id: str_claim("sub").to_string(),
        device_jkt: cnf_jkt.to_string(),
        expires_at_seconds: exp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_identity::identity_from_keys_for_test;
    use crate::orchestration_engine::uuid_v4;

    fn b64(data: &[u8]) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
    }

    fn sign_ed25519(key: &ed25519_dalek::SigningKey, header: Value, payload: Value) -> String {
        use ed25519_dalek::Signer;
        let h = b64(serde_json::to_string(&header).unwrap().as_bytes());
        let p = b64(serde_json::to_string(&payload).unwrap().as_bytes());
        let input = format!("{h}.{p}");
        let sig = key.sign(input.as_bytes());
        format!("{input}.{}", b64(sig.to_bytes().as_slice()))
    }

    struct Fixture {
        host: HostIdentity,
        device: ed25519_dalek::SigningKey,
        device_jwk: Value,
        jkt: String,
    }

    fn fixture() -> Fixture {
        let mut rng = rand_core::OsRng;
        let host = identity_from_keys_for_test(ed25519_dalek::SigningKey::generate(&mut rng));
        let device = ed25519_dalek::SigningKey::generate(&mut rng);
        let x = b64(device.verifying_key().as_bytes());
        let device_jwk = json!({"kty": "OKP", "crv": "Ed25519", "x": x});
        let jkt = jwk_thumbprint(&device_jwk);
        Fixture { host, device, device_jwk, jkt }
    }

    fn mint_credential(fx: &Fixture, now: u64) -> String {
        let header = json!({"alg": "EdDSA", "typ": SESSION_CREDENTIAL_JWT_TYP});
        let payload = json!({
            "iss": "synara-host:env-1",
            "sub": "user-1",
            "aud": SYNARA_SESSION_AUDIENCE,
            "iat": now,
            "exp": now + 900,
            "jti": uuid_v4(),
            "keyGeneration": 1,
            "trustGeneration": 1,
            "scope": [HOST_CONNECT_SCOPE],
            "cnf": {"jkt": fx.jkt},
        });
        sign_ed25519(&fx.host.signing_for_test(), header, payload)
    }

    fn mint_dpop(fx: &Fixture, credential: &str, htm: &str, htu: &str, now: u64) -> String {
        let header = json!({"alg": "EdDSA", "typ": DPOP_JWT_TYP, "jwk": fx.device_jwk});
        let payload = json!({
            "jti": uuid_v4(),
            "htm": htm,
            "htu": htu,
            "iat": now,
            "ath": b64(&sha256_bytes(credential.as_bytes())),
        });
        sign_ed25519(&fx.device, header, payload)
    }

    #[test]
    fn valid_credential_verifies() {
        let fx = fixture();
        let now = now_seconds();
        let credential = mint_credential(&fx, now);
        let dpop = mint_dpop(&fx, &credential, "GET", "https://host.example/resource", now);
        let cache = JwtReplayCache::new();
        let verified = verify_session_credential(
            &credential, &dpop, &fx.host, "env-1", 1, "https://host.example/resource", "GET", Some(&cache),
        )
        .unwrap();
        assert_eq!(verified.user_id, "user-1");
        assert_eq!(verified.device_jkt, fx.jkt);
    }

    #[test]
    fn wrong_htu_is_refused() {
        let fx = fixture();
        let now = now_seconds();
        let credential = mint_credential(&fx, now);
        let dpop = mint_dpop(&fx, &credential, "GET", "https://host.example/other", now);
        assert!(verify_session_credential(
            &credential, &dpop, &fx.host, "env-1", 1, "https://host.example/resource", "GET", None,
        ).is_err());
    }

    #[test]
    fn replay_is_refused() {
        let fx = fixture();
        let now = now_seconds();
        let credential = mint_credential(&fx, now);
        let dpop = mint_dpop(&fx, &credential, "GET", "https://host.example/resource", now);
        let cache = JwtReplayCache::new();
        assert!(verify_session_credential(
            &credential, &dpop, &fx.host, "env-1", 1, "https://host.example/resource", "GET", Some(&cache),
        ).is_ok());
        assert!(verify_session_credential(
            &credential, &dpop, &fx.host, "env-1", 1, "https://host.example/resource", "GET", Some(&cache),
        ).is_err());
    }

    #[test]
    fn wrong_key_generation_is_refused() {
        let fx = fixture();
        let now = now_seconds();
        let credential = mint_credential(&fx, now);
        let dpop = mint_dpop(&fx, &credential, "GET", "https://host.example/resource", now);
        assert!(verify_session_credential(
            &credential, &dpop, &fx.host, "env-1", 2, "https://host.example/resource", "GET", None,
        ).is_err());
    }

    #[test]
    fn foreign_device_key_is_refused() {
        let fx = fixture();
        let now = now_seconds();
        let credential = mint_credential(&fx, now);
        let mut rng = rand_core::OsRng;
        let stranger = ed25519_dalek::SigningKey::generate(&mut rng);
        // A proof signed by another key fails even with the right jwk pinned in
        // the credential, because the signature no longer matches.
        let header = json!({"alg": "EdDSA", "typ": DPOP_JWT_TYP, "jwk": fx.device_jwk});
        let payload = json!({
            "jti": uuid_v4(), "htm": "GET", "htu": "https://host.example/resource",
            "iat": now, "ath": b64(&sha256_bytes(credential.as_bytes())),
        });
        let dpop = sign_ed25519(&stranger, header, payload);
        assert!(verify_session_credential(
            &credential, &dpop, &fx.host, "env-1", 1, "https://host.example/resource", "GET", None,
        ).is_err());
    }

    #[test]
    fn thumbprint_matches_rfc7638_ordering() {
        // RFC 7638 ordering: crv, kty, x, (y) — lexicographic, compact json.
        let jwk = json!({"kty": "OKP", "crv": "Ed25519", "x": "abc"});
        let expected = b64(&sha256_bytes(br#"{"crv":"Ed25519","kty":"OKP","x":"abc"}"#));
        assert_eq!(jwk_thumbprint(&jwk), expected);
    }
}
