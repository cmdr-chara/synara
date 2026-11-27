//! Account-signed host grant verification, port of upstream
//! hostAuth/grantVerifier.ts. Grants are EdDSA JWTs signed by the account
//! service, bound to one host/environment/owner/device key and spent once per
//! jti across pairing and session minting.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use base64::Engine;
use serde_json::Value;

use crate::host_auth_verify::{split_jwt, verify_ed25519, JwtReplayCache};

const GRANT_JWT_TYP: &str = "synara-grant+jwt";
const GRANT_MAX_AGE_SECONDS: u64 = 60;
const SYNARA_RELAY_AUDIENCE: &str = "synara-relay";

pub struct VerifiedGrant {
    pub subject: String,
    pub jti: String,
    pub expires_at_seconds: u64,
}

fn replay_cache() -> &'static JwtReplayCache {
    static CACHE: OnceLock<JwtReplayCache> = OnceLock::new();
    CACHE.get_or_init(JwtReplayCache::new)
}

fn jwks_cache() -> &'static Mutex<HashMap<String, (u64, Value)>> {
    static CACHE: OnceLock<Mutex<HashMap<String, (u64, Value)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn fetch_jwks(account_url: &str) -> Result<Value, String> {
    let client = crate::account::AccountClient::new(account_url);
    let value = client
        .get_public("/api/v1/keys/jwks")
        .map_err(|error| crate::account::service_error(&error, "Remote authorization unavailable").0)?;
    if value.get("keys").and_then(Value::as_array).is_none() {
        return Err("Remote authorization unavailable".to_string());
    }
    Ok(value)
}

fn cached_jwks(account_url: &str, force_refresh: bool) -> Result<Value, String> {
    let now = now_seconds();
    if !force_refresh
        && let Ok(cache) = jwks_cache().lock()
        && let Some((fetched, value)) = cache.get(account_url)
        && now.saturating_sub(*fetched) < 5 * 60
    {
        return Ok(value.clone());
    }
    let value = fetch_jwks(account_url)?;
    if let Ok(mut cache) = jwks_cache().lock() {
        cache.insert(account_url.to_string(), (now, value.clone()));
    }
    Ok(value)
}

fn find_key<'a>(jwks: &'a Value, kid: &str) -> Option<&'a Value> {
    jwks.get("keys")?.as_array()?.iter().find(|key| {
        key.get("kid").and_then(Value::as_str) == Some(kid)
            && key.get("kty").and_then(Value::as_str) == Some("OKP")
            && key.get("crv").and_then(Value::as_str) == Some("Ed25519")
            && key.get("alg").and_then(Value::as_str) == Some("EdDSA")
            && key.get("use").and_then(Value::as_str) == Some("sig")
    })
}

/// Verifies the grant but does not consume it. Pairing/minting consume only
/// after local device authorization also succeeds, so a transient local denial
/// does not burn a legitimate one-shot grant.
pub fn verify_host_grant(
    grant_jwt: &str,
    account_url: &str,
    api_issuer: &str,
    environment_id: &str,
    host_id: &str,
    owner_user_id: &str,
    device_jkt: &str,
    subject: Option<&str>,
) -> Result<VerifiedGrant, String> {
    let now = now_seconds();
    let (header, claims, signature, signing_input) = split_jwt(grant_jwt)?;
    if header.get("alg").and_then(Value::as_str) != Some("EdDSA")
        || header.get("typ").and_then(Value::as_str) != Some(GRANT_JWT_TYP)
    {
        return Err("grant header invalid".to_string());
    }
    let kid = header.get("kid").and_then(Value::as_str).unwrap_or_default();
    if kid.is_empty() {
        return Err("grant header invalid".to_string());
    }
    let mut jwks = cached_jwks(account_url, false)?;
    if find_key(&jwks, kid).is_none() {
        jwks = cached_jwks(account_url, true)?;
    }
    let key = find_key(&jwks, kid).ok_or_else(|| "grant signing key unknown".to_string())?;
    let x = key.get("x").and_then(Value::as_str).unwrap_or_default();
    verify_ed25519(&signing_input, &signature, x).map_err(|_| "grant signature invalid".to_string())?;

    let text = |name: &str| claims.get(name).and_then(Value::as_str).unwrap_or("");
    let number = |name: &str| claims.get(name).and_then(Value::as_u64).unwrap_or(0);
    let scope_ok = claims
        .get("scope")
        .and_then(Value::as_array)
        .and_then(|scope| scope.first())
        .and_then(Value::as_str)
        == Some(crate::host_auth_verify::HOST_CONNECT_SCOPE);
    if text("iss") != api_issuer
        || text("aud") != SYNARA_RELAY_AUDIENCE
        || !scope_ok
        || text("hostId") != host_id
        || text("environmentId") != environment_id
        || text("sub") != owner_user_id
        || subject.is_some_and(|expected| text("sub") != expected)
        || claims.pointer("/cnf/jkt").and_then(Value::as_str) != Some(device_jkt)
    {
        return Err("grant is not bound to this host, user, and device key".to_string());
    }
    let (iat, exp) = (number("iat"), number("exp"));
    if exp <= iat
        || exp - iat > GRANT_MAX_AGE_SECONDS
        || iat > now + crate::host_auth_verify::JWT_CLOCK_TOLERANCE_SECONDS
        || exp < now
    {
        return Err("grant lifetime invalid".to_string());
    }
    let jti = text("jti");
    if jti.is_empty() {
        return Err("grant claims invalid".to_string());
    }
    Ok(VerifiedGrant { subject: text("sub").to_string(), jti: jti.to_string(), expires_at_seconds: exp })
}

pub fn consume_host_grant(grant: &VerifiedGrant) -> Result<(), String> {
    replay_cache().consume(&grant.jti, grant.expires_at_seconds)
}

pub fn _grant_wire_type_for_test() -> &'static str {
    GRANT_JWT_TYP
}

pub fn _b64_for_test(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
