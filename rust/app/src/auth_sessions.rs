//! Session credential tokens, port of upstream auth/utils.ts +
//! SessionCredentialService token core. Exact wire shapes: session claims
//! `{v:1,kind:"session",sid,sub,role,method,iat,exp}` and websocket tickets
//! `{v:2,kind:"websocket",sid,jti,iat,exp}` with epoch MILLIS, serialized as
//! `{base64url(json)}.{base64url(hmac-sha256)}` with the server signing key
//! from the secret store. TTLs are exact: 30 days sessions, 5 minutes tickets.

use serde_json::{json, Value};
use sha2::Digest;

use crate::server_secrets;

const SIGNING_SECRET_NAME: &str = "server-signing-key";
pub const SESSION_TTL_MS: u64 = 30 * 24 * 60 * 60 * 1000;
pub const WEBSOCKET_TOKEN_TTL_MS: u64 = 5 * 60 * 1000;
pub const MAX_OUTSTANDING_WEBSOCKET_TICKETS_PER_SESSION: usize = 16;

// Tickets are bounded per session and pruned by expiry. Like upstream, this
// allowlist is deliberately process-local: a restart invalidates outstanding
// tickets instead of making old signed values replayable.
static OUTSTANDING_WEBSOCKET_TICKETS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<String, (String, u64)>>,
> = std::sync::OnceLock::new();

fn outstanding_tickets() -> &'static std::sync::Mutex<std::collections::HashMap<String, (String, u64)>> {
    OUTSTANDING_WEBSOCKET_TICKETS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

pub fn base64url_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        }
    }
    out
}

pub fn base64url_decode(input: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    }
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let mut n: u32 = 0;
        let mut len = 0;
        for (i, &c) in chunk.iter().enumerate() {
            n |= val(c)? << (18 - 6 * i);
            len += 1;
        }
        out.push((n >> 16) as u8);
        if len > 2 {
            out.push((n >> 8) as u8);
        }
        if len > 3 {
            out.push(n as u8);
        }
    }
    Some(out)
}

fn hmac_sha256(key: &[u8], message: &[u8]) -> Vec<u8> {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let mut h = sha2::Sha256::new();
        h.update(key);
        let digest = h.finalize();
        key_block[..32].copy_from_slice(&digest);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0u8; 64];
    let mut opad = [0u8; 64];
    for i in 0..64 {
        ipad[i] = key_block[i] ^ 0x36;
        opad[i] = key_block[i] ^ 0x5c;
    }
    let mut inner = sha2::Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();
    let mut outer = sha2::Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    outer.finalize().to_vec()
}

fn timing_safe_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

fn signing_key() -> Option<Vec<u8>> {
    server_secrets::get_or_create_secret(SIGNING_SECRET_NAME).ok()
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn sign_payload(payload: &str, secret: &[u8]) -> String {
    base64url_encode(&hmac_sha256(secret, payload.as_bytes()))
}

/// Issues a session token with the exact upstream claims shape.
pub fn issue_session_token(session_id: &str, subject: &str, role: &str, method: &str) -> Option<String> {
    let secret = signing_key()?;
    let now = now_millis();
    let claims = json!({
        "v": 1,
        "kind": "session",
        "sid": session_id,
        "sub": subject,
        "role": role,
        "method": method,
        "iat": now,
        "exp": now + SESSION_TTL_MS,
    });
    let payload = base64url_encode(claims.to_string().as_bytes());
    let signature = sign_payload(&payload, &secret);
    Some(format!("{payload}.{signature}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WebSocketTicketIssueError {
    SigningUnavailable,
    Capacity { active: usize },
}

/// Issues a websocket ticket through the same in-memory allowlist upstream
/// uses. The signed token is necessary but not sufficient: upgrade consumes
/// its `jti` exactly once, capacity is bounded per session, and expiry is
/// capped by the underlying session expiry.
pub fn issue_websocket_ticket(
    session_id: &str,
    session_expires_at_ms: u64,
) -> Result<(String, u64), WebSocketTicketIssueError> {
    let secret = signing_key().ok_or(WebSocketTicketIssueError::SigningUnavailable)?;
    let now = now_millis();
    let expires_at = (now + WEBSOCKET_TOKEN_TTL_MS).min(session_expires_at_ms);
    let mut tickets = outstanding_tickets()
        .lock()
        .map_err(|_| WebSocketTicketIssueError::SigningUnavailable)?;
    tickets.retain(|_, (_, expires)| *expires > now);
    let active = tickets
        .values()
        .filter(|(ticket_session, _)| ticket_session == session_id)
        .count();
    if active >= MAX_OUTSTANDING_WEBSOCKET_TICKETS_PER_SESSION {
        return Err(WebSocketTicketIssueError::Capacity { active });
    }
    let ticket_id = crate::orchestration_engine::uuid_v4();
    let claims = json!({
        "v": 2,
        "kind": "websocket",
        "sid": session_id,
        "jti": ticket_id,
        "iat": now,
        "exp": expires_at,
    });
    let payload = base64url_encode(claims.to_string().as_bytes());
    let signature = sign_payload(&payload, &secret);
    tickets.insert(ticket_id, (session_id.to_string(), expires_at));
    Ok((format!("{payload}.{signature}"), expires_at))
}

pub struct VerifiedClaims {
    pub session_id: String,
    pub subject: Option<String>,
    pub role: Option<String>,
    pub method: Option<String>,
    pub kind: String,
    pub ticket_id: Option<String>,
    pub expires_at_ms: u64,
}

/// Verifies either token kind: signature must match with a timing-safe
/// compare, expiry must be in the future, kind/version must be exact.
pub fn verify_token(token: &str) -> Option<VerifiedClaims> {
    let secret = signing_key()?;
    let (payload, signature) = token.split_once('.')?;
    let expected = sign_payload(payload, &secret);
    let sig_bytes = base64url_decode(signature)?;
    let expected_bytes = base64url_decode(&expected)?;
    if !timing_safe_equal(&sig_bytes, &expected_bytes) {
        return None;
    }
    let json_bytes = base64url_decode(payload)?;
    let claims: Value = serde_json::from_slice(&json_bytes).ok()?;
    let kind = claims.get("kind")?.as_str()?.to_string();
    let expected_version = if kind == "session" { 1 } else if kind == "websocket" { 2 } else { return None };
    if claims.get("v")?.as_u64()? != expected_version {
        return None;
    }
    let exp = claims.get("exp")?.as_u64()?;
    if exp <= now_millis() {
        return None;
    }
    Some(VerifiedClaims {
        session_id: claims.get("sid")?.as_str()?.to_string(),
        subject: claims.get("sub").and_then(Value::as_str).map(str::to_string),
        role: claims.get("role").and_then(Value::as_str).map(str::to_string),
        method: claims.get("method").and_then(Value::as_str).map(str::to_string),
        kind,
        ticket_id: claims.get("jti").and_then(Value::as_str).map(str::to_string),
        expires_at_ms: exp,
    })
}

/// Consume the process-local ticket lease after the signed claims verify.
/// Like upstream, the jti is removed before the session-row check, so every
/// upgrade attempt is single-use even when the underlying session has since
/// been revoked.
pub fn consume_websocket_ticket(claims: &VerifiedClaims) -> bool {
    if claims.kind != "websocket" {
        return false;
    }
    let Some(ticket_id) = claims.ticket_id.as_deref() else { return false };
    let now = now_millis();
    let Ok(mut tickets) = outstanding_tickets().lock() else { return false };
    tickets.retain(|_, (_, expires)| *expires > now);
    let Some((session_id, expires_at)) = tickets.remove(ticket_id) else { return false };
    session_id == claims.session_id
        && expires_at == claims.expires_at_ms
        && claims.expires_at_ms > now
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_roundtrips() {
        assert_eq!(base64url_encode(b"hello"), "aGVsbG8");
        assert_eq!(base64url_decode("aGVsbG8"), Some(b"hello".to_vec()));
        assert_eq!(base64url_encode(b""), "");
    }

    #[test]
    fn hmac_matches_rfc4231_vector() {
        // RFC 4231 test case 2: key "Jefe", data "what do ya want for nothing?"
        let sig = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        let hex: String = sig.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
    }


    #[test]
    fn ticket_consumption_requires_exact_live_lease() {
        let now = now_millis();
        let ticket_id = crate::orchestration_engine::uuid_v4();
        outstanding_tickets()
            .lock()
            .unwrap()
            .insert(ticket_id.clone(), ("session-a".to_string(), now + 1000));
        let claims = VerifiedClaims {
            session_id: "session-a".to_string(),
            subject: None,
            role: None,
            method: None,
            kind: "websocket".to_string(),
            ticket_id: Some(ticket_id),
            expires_at_ms: now + 1000,
        };
        assert!(consume_websocket_ticket(&claims));
        assert!(!consume_websocket_ticket(&claims));
    }

    #[test]
    fn ticket_consumption_rejects_wrong_session_or_expiry() {
        let now = now_millis();
        let ticket_id = crate::orchestration_engine::uuid_v4();
        outstanding_tickets()
            .lock()
            .unwrap()
            .insert(ticket_id.clone(), ("session-a".to_string(), now + 1000));
        let mut claims = VerifiedClaims {
            session_id: "session-b".to_string(),
            subject: None,
            role: None,
            method: None,
            kind: "websocket".to_string(),
            ticket_id: Some(ticket_id),
            expires_at_ms: now + 1000,
        };
        assert!(!consume_websocket_ticket(&claims));
        claims.session_id = "missing".to_string();
        assert!(!consume_websocket_ticket(&claims));
    }
}
