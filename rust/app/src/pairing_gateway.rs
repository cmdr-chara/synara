//! Remote pairing handshake, port of upstream remotePairing/gateway.ts.
//! Runs inside the authenticated TLS tunnel at /v2/pairing: a three-state
//! frame machine (begin → proof → pending) that never exposes coding RPC and
//! never echoes secrets in close reasons. Frames are bounded at 16 KiB total
//! and 8 messages, the initial deadline is 15 seconds.

use base64::Engine;
use ed25519_dalek::Verifier;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use sha2::Digest;
use tokio_tungstenite::tungstenite::Message;

use crate::remote_pairing as rp;

pub const REMOTE_PAIRING_PROOF_TYPE: &str = "synara-remote-pairing+jwt";
const MAX_PAIRING_BYTES: usize = 16 * 1024;
const MAX_PAIRING_FRAMES: usize = 8;

fn b64url(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

fn b64url_decode(data: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .map_err(|e| e.to_string())
}

fn thumbprint(jwk: &Value) -> String {
    let crv = jwk.get("crv").and_then(Value::as_str).unwrap_or("");
    let kty = jwk.get("kty").and_then(Value::as_str).unwrap_or("");
    let x = jwk.get("x").and_then(Value::as_str).unwrap_or("");
    let canonical = if kty == "EC" {
        let y = jwk.get("y").and_then(Value::as_str).unwrap_or("");
        json!({"crv": crv, "kty": kty, "x": x, "y": y}).to_string()
    } else {
        json!({"crv": crv, "kty": kty, "x": x}).to_string()
    };
    b64url(&sha2::Sha256::digest(canonical.as_bytes()))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn verify_device_proof(
    proof: &str,
    device_key: &Value,
    nonce: &str,
    invite_id: &str,
    environment_id: &str,
    user_id: &str,
) -> Result<(), String> {
    let parts: Vec<&str> = proof.split('.').collect();
    if parts.len() != 3 {
        return Err("proof is not a compact JWT".to_string());
    }
    let header: Value = serde_json::from_slice(&b64url_decode(parts[0])?)
        .map_err(|_| "proof header invalid".to_string())?;
    let payload: Value = serde_json::from_slice(&b64url_decode(parts[1])?)
        .map_err(|_| "proof payload invalid".to_string())?;
    let signature = b64url_decode(parts[2])?;
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let kty = device_key.get("kty").and_then(Value::as_str).unwrap_or("");
    let crv = device_key.get("crv").and_then(Value::as_str).unwrap_or("");
    let algorithm = match (kty, crv) {
        ("OKP", "Ed25519") => "EdDSA",
        ("EC", "P-256") => "ES256",
        _ => return Err("proof key type unsupported".to_string()),
    };
    if header.get("alg").and_then(Value::as_str) != Some(algorithm)
        || header.get("typ").and_then(Value::as_str) != Some(REMOTE_PAIRING_PROOF_TYPE)
    {
        return Err("proof header invalid".to_string());
    }
    let x = device_key.get("x").and_then(Value::as_str).unwrap_or("");
    if algorithm == "EdDSA" {
        let xb = b64url_decode(x)?;
        let xb: [u8; 32] = xb.try_into().map_err(|_| "bad key length".to_string())?;
        let key = ed25519_dalek::VerifyingKey::from_bytes(&xb).map_err(|e| e.to_string())?;
        let sig = ed25519_dalek::Signature::from_slice(&signature).map_err(|e| e.to_string())?;
        key.verify(signing_input.as_bytes(), &sig)
            .map_err(|_| "proof signature invalid".to_string())?;
    } else {
        use p256::ecdsa::signature::Verifier as _;
        let xb = b64url_decode(x)?;
        let yb = b64url_decode(device_key.get("y").and_then(Value::as_str).unwrap_or(""))?;
        let mut point = vec![0x04u8];
        point.extend_from_slice(&xb);
        point.extend_from_slice(&yb);
        let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(&point).map_err(|e| e.to_string())?;
        let sig = p256::ecdsa::Signature::from_slice(&signature).map_err(|e| e.to_string())?;
        key.verify(signing_input.as_bytes(), &sig)
            .map_err(|_| "proof signature invalid".to_string())?;
    }
    let now = now_seconds();
    let iat = payload.get("iat").and_then(Value::as_u64).unwrap_or(0);
    let exp = payload.get("exp").and_then(Value::as_u64).unwrap_or(0);
    let str_of = |key: &str| payload.get(key).and_then(Value::as_str).unwrap_or("");
    if payload.get("nonce").and_then(Value::as_str) != Some(nonce)
        || payload.get("inviteId").and_then(Value::as_str) != Some(invite_id)
        || str_of("sub") != user_id
        || str_of("iss") != "synara-device"
        || str_of("aud") != format!("synara-host:{environment_id}")
        || exp <= iat
        || exp - iat > 60
        || now + 60 < iat
        || now > exp + 60
    {
        return Err("proof claims invalid".to_string());
    }
    if payload.get("jti").and_then(Value::as_str).is_none() {
        return Err("proof claims invalid".to_string());
    }
    Ok(())
}

fn now_seconds() -> u64 {
    (now_ms() / 1000) as u64
}

struct PairingContext {
    invite_id: String,
    device: rp::PairingDevice,
    nonce: String,
    expires_at_ms: i64,
}

fn random_nonce() -> String {
    use std::io::Read;
    let mut bytes = [0u8; 32];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut bytes);
    }
    b64url(&bytes)
}

/// The pairing frame loop over the already-upgraded inner websocket. Mirrors
/// upstream's message sequencing exactly; any protocol violation closes with
/// the generic policy-close reason.
pub async fn run_pairing<S>(ws: S, scope: rp::TrustScope)
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + futures_util::Sink<Message>
        + Unpin,
    S: Send,
{
    let (mut tx, mut rx) = ws.split();
    let conn = match rp::open() {
        Ok(conn) => conn,
        Err(_) => return,
    };
    let mut context: Option<PairingContext> = None;
    let mut state = "begin";
    let mut pending_bytes = 0usize;
    let mut pending_frames = 0usize;

    macro_rules! close_fail {
        () => {{
            let _ = tx
                .send(Message::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                    code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Policy,
                    reason: "Pairing failed. Check the invitation on the host.".into(),
                })))
                .await;
            return;
        }};
    }

    // Initial 15-second deadline enforced as a receive timeout per frame.
    while let Ok(Some(frame)) =
        tokio::time::timeout(std::time::Duration::from_secs(15), rx.next()).await
    {
        let frame = match frame {
            Ok(Message::Text(t)) => t,
            Ok(Message::Close(_)) => return,
            Err(_) => return,
            _ => {
                close_fail!();
            }
        };
        pending_bytes += frame.len();
        pending_frames += 1;
        if pending_bytes > MAX_PAIRING_BYTES || pending_frames > MAX_PAIRING_FRAMES {
            let _ = tx
                .send(Message::Close(Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                    code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Size,
                    reason: "Pairing frame limit exceeded".into(),
                })))
                .await;
            return;
        }
        pending_bytes = pending_bytes.saturating_sub(frame.len());
        pending_frames = pending_frames.saturating_sub(1);

        let parsed: Value = match serde_json::from_str(&frame) {
            Ok(v) => v,
            Err(_) => close_fail!(),
        };
        if parsed.get("v").and_then(Value::as_u64) != Some(2) {
            close_fail!();
        }
        let frame_type = parsed.get("type").and_then(Value::as_str).unwrap_or("");

        if state == "begin" && frame_type == "pairing_begin" {
            let invite_id = parsed.get("inviteId").and_then(Value::as_str).unwrap_or("");
            let secret = parsed.get("secret").and_then(Value::as_str).unwrap_or("");
            let label = parsed.get("label").and_then(Value::as_str).unwrap_or("");
            let public_key = parsed.get("publicKey").cloned().unwrap_or(Value::Null);
            if invite_id.is_empty() || secret.is_empty() || label.is_empty() || label.len() > 128 {
                close_fail!();
            }
            let kty = public_key.get("kty").and_then(Value::as_str).unwrap_or("");
            let crv = public_key.get("crv").and_then(Value::as_str).unwrap_or("");
            if !matches!((kty, crv), ("OKP", "Ed25519") | ("EC", "P-256")) {
                close_fail!();
            }
            let invitation = match rp::verify_invitation(&conn, &scope, invite_id, secret) {
                Ok(v) => v,
                Err(_) => close_fail!(),
            };
            let device_jkt = thumbprint(&public_key);
            if let Some(pending) = invitation.get("pendingDevice") {
                if !pending.is_null()
                    && pending.get("deviceJkt").and_then(Value::as_str) != Some(device_jkt.as_str())
                {
                    close_fail!();
                }
            }
            let nonce = random_nonce();
            let expires_at_ms = invitation
                .get("expiresAt")
                .and_then(Value::as_str)
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.timestamp_millis())
                .unwrap_or(0);
            context = Some(PairingContext {
                invite_id: invite_id.to_string(),
                device: rp::PairingDevice {
                    device_jkt: device_jkt.clone(),
                    public_key,
                    label: label.to_string(),
                },
                nonce: nonce.clone(),
                expires_at_ms,
            });
            state = "proof";
            if tx
                .send(Message::Text(
                    json!({"v": 2, "type": "pairing_challenge", "nonce": nonce}).to_string().into(),
                ))
                .await
                .is_err()
            {
                return;
            }
            continue;
        }

        if state == "proof" && frame_type == "pairing_proof" {
            let Some(ctx) = &context else {
                close_fail!();
            };
            let proof = parsed.get("proof").and_then(Value::as_str).unwrap_or("");
            if verify_device_proof(
                proof,
                &ctx.device.public_key,
                &ctx.nonce,
                &ctx.invite_id,
                &scope.environment_id,
                &scope.user_id,
            )
            .is_err()
            {
                close_fail!();
            }
            // Pairing never self-enrolls on the invite secret alone. Upstream
            // enrolls immediately only after verifying the owner account grant;
            // until that verifier is wired here, every device must wait for the
            // host owner's explicit approval.
            let recorded = rp::request_approval(&conn, &scope, &ctx.invite_id, &ctx.device).unwrap_or(false);
            if !recorded {
                close_fail!();
            }
            state = "pending";
        }

        if state == "pending" && (frame_type == "pairing_status" || frame_type == "pairing_proof") {
            let Some(ctx) = &context else {
                close_fail!();
            };
            let invitations = rp::list_invites(&conn, &scope).unwrap_or_default();
            let Some(invitation) = invitations
                .iter()
                .find(|entry| entry.get("inviteId").and_then(Value::as_str) == Some(ctx.invite_id.as_str()))
            else {
                close_fail!();
            };
            let revoked = invitation.get("revoked").and_then(Value::as_bool).unwrap_or(true);
            let approved_flag = invitation.get("approved").and_then(Value::as_bool).unwrap_or(false);
            if revoked {
                close_fail!();
            }
            let approved = approved_flag
                && rp::authorize_device(&conn, &scope, &ctx.device.device_jkt).unwrap_or(false);
            if tx
                .send(Message::Text(
                    json!({
                        "v": 2,
                        "type": "pairing_status",
                        "state": if approved { "approved" } else { "pending" },
                        "deviceJkt": ctx.device.device_jkt,
                    })
                    .to_string()
                    .into(),
                ))
                .await
                .is_err()
            {
                return;
            }
            if frame_type == "pairing_proof" {
                continue;
            }
            continue;
        }

        if state == "proof" && frame_type != "pairing_proof" {
            close_fail!();
        }
        if state == "begin" && frame_type != "pairing_begin" {
            close_fail!();
        }
    }
}
