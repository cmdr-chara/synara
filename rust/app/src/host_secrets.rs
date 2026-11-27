//! Host Secrets Sync Key cryptography and durable pairing state.
//!
//! Port of `packages/shared/src/hostSecrets.ts` plus the local halves of
//! `hostSecrets/syncKeyStore.ts` and `hostSecrets/coordinator.ts`. The account
//! service only ever relays opaque envelopes and wraps; the key material below
//! stays in the owner-only secrets directory.

use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit as AesKeyInit};
use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit as GcmKeyInit, Nonce};
use base64::Engine as _;
use hkdf::Hkdf;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::{PublicKey, SecretKey};
use serde_json::{json, Value};
use sha2::Sha256;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use crate::todo::WsErr;

const SEAL_AAD_DOMAIN: &str = "synara.hostSecret.aad.v1";
const PAIRING_KDF_DOMAIN: &str = "synara.hostSecrets.pairing.kdf.v1";
const PAIRING_CODE_DOMAIN: &str = "synara.hostSecrets.pairing.code.v1";
const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const MAX_CONFIRMATION_ATTEMPTS: u32 = 3;

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

fn b64u(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn b64u_decode(text: &str) -> Result<Vec<u8>, WsErr> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| err("Value is not base64url"))
}

pub(crate) fn sync_key_path() -> PathBuf {
    PathBuf::from(crate::todo::state_dir())
        .join("secrets")
        .join("host-secrets-sync-key.json")
}

fn random_bytes(length: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; length];
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut bytes);
    bytes
}

pub(crate) fn generate_sync_key() -> [u8; 32] {
    let bytes = random_bytes(32);
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    key
}

fn host_secret_aad(host_id: &str, owner_user_id: &str, version: i64) -> Vec<u8> {
    json!([SEAL_AAD_DOMAIN, host_id, owner_user_id, version]).to_string().into_bytes()
}

pub(crate) fn seal_host_secret(
    sync_key: &[u8; 32],
    host_id: &str,
    owner_user_id: &str,
    version: i64,
    secret: &Value,
) -> Result<Value, WsErr> {
    if host_id.is_empty() || owner_user_id.is_empty() || version < 1 {
        return Err(err("invalid-input"));
    }
    let iv = random_bytes(12);
    let cipher = Aes256Gcm::new_from_slice(sync_key).map_err(|_| err("invalid-input"))?;
    let plaintext = secret.to_string();
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: plaintext.as_bytes(),
                aad: &host_secret_aad(host_id, owner_user_id, version),
            },
        )
        .map_err(|_| err("Host secret could not be encrypted"))?;
    Ok(json!({
        "ciphertext": b64u(&ciphertext),
        "iv": b64u(&iv),
        "version": version,
    }))
}

pub(crate) fn open_host_secret(
    sync_key: &[u8; 32],
    host_id: &str,
    owner_user_id: &str,
    version: i64,
    envelope: &Value,
) -> Result<Value, WsErr> {
    if envelope.get("version").and_then(Value::as_i64) != Some(version) {
        return Err(err("Host secret could not be decrypted"));
    }
    let iv = b64u_decode(envelope.get("iv").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| err("Host secret could not be decrypted"))?;
    if iv.len() != 12 {
        return Err(err("Host secret could not be decrypted"));
    }
    let ciphertext = b64u_decode(envelope.get("ciphertext").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| err("Host secret could not be decrypted"))?;
    let cipher = Aes256Gcm::new_from_slice(sync_key).map_err(|_| err("Host secret could not be decrypted"))?;
    let plaintext = cipher
        .decrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: &ciphertext,
                aad: &host_secret_aad(host_id, owner_user_id, version),
            },
        )
        .map_err(|_| err("Host secret could not be decrypted"))?;
    serde_json::from_slice(&plaintext).map_err(|_| err("Host secret payload is not valid JSON"))
}

fn canonical_pairing_jwk(jwk: &Value, field: &str) -> Result<String, WsErr> {
    if jwk.get("kty").and_then(Value::as_str) != Some("EC")
        || jwk.get("crv").and_then(Value::as_str) != Some("P-256")
    {
        return Err(err(format!("{field} must be an EC P-256 public key")));
    }
    let x = b64u_decode(jwk.get("x").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| err(format!("{field} must be an EC P-256 public key")))?;
    let y = b64u_decode(jwk.get("y").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| err(format!("{field} must be an EC P-256 public key")))?;
    if x.len() != 32 || y.len() != 32 {
        return Err(err(format!("{field} coordinates are not P-256 sized")));
    }
    Ok(format!("P-256.{}.{}", b64u(&x), b64u(&y)))
}

pub(crate) struct PairingKeyPair {
    pub(crate) private_key: SecretKey,
    pub(crate) public_jwk: Value,
}

pub(crate) fn generate_pairing_keypair() -> PairingKeyPair {
    let private_key = SecretKey::random(&mut rand_core::OsRng);
    let public_jwk = public_jwk(&private_key.public_key());
    PairingKeyPair { private_key, public_jwk }
}

fn public_jwk(public: &PublicKey) -> Value {
    let point = public.to_encoded_point(false);
    json!({
        "kty": "EC",
        "crv": "P-256",
        "x": b64u(point.x().map(|value| value.as_slice()).unwrap_or(&[])),
        "y": b64u(point.y().map(|value| value.as_slice()).unwrap_or(&[])),
    })
}

fn import_public(jwk: &Value) -> Result<PublicKey, WsErr> {
    let x = b64u_decode(jwk.get("x").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| err("Public key is not EC P-256"))?;
    let y = b64u_decode(jwk.get("y").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| err("Public key is not EC P-256"))?;
    if x.len() != 32 || y.len() != 32 {
        return Err(err("Public key is not EC P-256"));
    }
    let mut encoded = Vec::with_capacity(65);
    encoded.push(4u8);
    encoded.extend(x);
    encoded.extend(y);
    PublicKey::from_sec1_bytes(&encoded).map_err(|_| err("Public key is not EC P-256"))
}

fn derive_pairing_kek(
    private_key: &SecretKey,
    peer_public: &PublicKey,
    ephemeral_public_jwk: &Value,
    recipient_public_jwk: &Value,
) -> Result<[u8; 32], WsErr> {
    let shared = p256::ecdh::diffie_hellman(private_key.to_nonzero_scalar(), peer_public.as_affine());
    let transcript = json!([
        PAIRING_KDF_DOMAIN,
        canonical_pairing_jwk(ephemeral_public_jwk, "ephemeralPublicJwk")?,
        canonical_pairing_jwk(recipient_public_jwk, "recipientPublicJwk")?,
    ])
    .to_string();
    let hkdf = Hkdf::<Sha256>::new(Some(PAIRING_KDF_DOMAIN.as_bytes()), shared.raw_secret_bytes());
    let mut output = [0u8; 32];
    hkdf.expand(transcript.as_bytes(), &mut output)
        .map_err(|_| err("Pairing key derivation failed"))?;
    Ok(output)
}

fn xor_block_t(block: &mut [u8], t: u64) {
    let value = u64::from_be_bytes(block.try_into().unwrap_or([0; 8])) ^ t;
    block.copy_from_slice(&value.to_be_bytes());
}

fn aes256_encrypt_block(key: &[u8; 32], input: &[u8; 16]) -> [u8; 16] {
    let cipher = aes::Aes256::new_from_slice(key).expect("AES-256 key length");
    let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(input);
    cipher.encrypt_block(&mut block);
    let mut output = [0u8; 16];
    output.copy_from_slice(&block);
    output
}

fn aes256_decrypt_block(key: &[u8; 32], input: &[u8; 16]) -> [u8; 16] {
    let cipher = aes::Aes256::new_from_slice(key).expect("AES-256 key length");
    let mut block = aes::cipher::generic_array::GenericArray::clone_from_slice(input);
    cipher.decrypt_block(&mut block);
    let mut output = [0u8; 16];
    output.copy_from_slice(&block);
    output
}

/// RFC 3394 AES Key Wrap, used by WebCrypto's AES-KW upstream.
fn aes_key_wrap(kek: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, WsErr> {
    if plaintext.len() < 16 || plaintext.len() % 8 != 0 {
        return Err(err("invalid-input"));
    }
    let n = plaintext.len() / 8;
    let mut a = [0xa6u8; 8];
    let mut r: Vec<[u8; 8]> = plaintext
        .chunks_exact(8)
        .map(|chunk| <[u8; 8]>::try_from(chunk).unwrap_or([0; 8]))
        .collect();
    for j in 0..=5u64 {
        for i in 0..n {
            let mut input = [0u8; 16];
            input[..8].copy_from_slice(&a);
            input[8..].copy_from_slice(&r[i]);
            let encrypted = aes256_encrypt_block(kek, &input);
            a.copy_from_slice(&encrypted[..8]);
            xor_block_t(&mut a, (n as u64) * j + (i as u64) + 1);
            r[i].copy_from_slice(&encrypted[8..]);
        }
    }
    let mut output = Vec::with_capacity((n + 1) * 8);
    output.extend(a);
    for block in r {
        output.extend(block);
    }
    Ok(output)
}

fn aes_key_unwrap(kek: &[u8; 32], wrapped: &[u8]) -> Result<Vec<u8>, WsErr> {
    if wrapped.len() < 24 || wrapped.len() % 8 != 0 {
        return Err(err("Sync key could not be unwrapped"));
    }
    let n = wrapped.len() / 8 - 1;
    let mut a = <[u8; 8]>::try_from(&wrapped[..8]).unwrap_or([0; 8]);
    let mut r: Vec<[u8; 8]> = wrapped[8..]
        .chunks_exact(8)
        .map(|chunk| <[u8; 8]>::try_from(chunk).unwrap_or([0; 8]))
        .collect();
    for j in (0..=5u64).rev() {
        for i in (0..n).rev() {
            let t = (n as u64) * j + (i as u64) + 1;
            xor_block_t(&mut a, t);
            let mut input = [0u8; 16];
            input[..8].copy_from_slice(&a);
            input[8..].copy_from_slice(&r[i]);
            let decrypted = aes256_decrypt_block(kek, &input);
            a.copy_from_slice(&decrypted[..8]);
            r[i].copy_from_slice(&decrypted[8..]);
        }
    }
    if a != [0xa6u8; 8] {
        return Err(err("Sync key could not be unwrapped"));
    }
    let mut output = Vec::with_capacity(n * 8);
    for block in r {
        output.extend(block);
    }
    Ok(output)
}

pub(crate) fn wrap_sync_key(sync_key: &[u8; 32], recipient_public_jwk: &Value) -> Result<Value, WsErr> {
    let recipient_public = import_public(recipient_public_jwk)?;
    let ephemeral = generate_pairing_keypair();
    let kek = derive_pairing_kek(
        &ephemeral.private_key,
        &recipient_public,
        &ephemeral.public_jwk,
        recipient_public_jwk,
    )?;
    let wrapped = aes_key_wrap(&kek, sync_key)?;
    Ok(json!({
        "ephemeralPublicJwk": ephemeral.public_jwk,
        "recipientPublicJwk": recipient_public_jwk,
        "wrapped": b64u(&wrapped),
    }))
}

pub(crate) fn unwrap_sync_key(wrapped: &Value, recipient_private_key: &SecretKey) -> Result<[u8; 32], WsErr> {
    let ephemeral_public_jwk = wrapped.get("ephemeralPublicJwk").cloned().unwrap_or(Value::Null);
    let recipient_public_jwk = wrapped.get("recipientPublicJwk").cloned().unwrap_or(Value::Null);
    let ephemeral_public = import_public(&ephemeral_public_jwk)?;
    let kek = derive_pairing_kek(
        recipient_private_key,
        &ephemeral_public,
        &ephemeral_public_jwk,
        &recipient_public_jwk,
    )?;
    let bytes = aes_key_unwrap(
        &kek,
        &b64u_decode(wrapped.get("wrapped").and_then(Value::as_str).unwrap_or_default())?,
    )?;
    if bytes.len() != 32 {
        return Err(err("Sync key could not be unwrapped"));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

pub(crate) fn pairing_verification_code(sender_public_jwk: &Value, recipient_public_jwk: &Value) -> Result<String, WsErr> {
    let sender = canonical_pairing_jwk(sender_public_jwk, "senderPublicJwk")?;
    let recipient = canonical_pairing_jwk(recipient_public_jwk, "recipientPublicJwk")?;
    let mut ordered = [sender, recipient];
    ordered.sort();
    let input = json!([PAIRING_CODE_DOMAIN, ordered[0], ordered[1]]).to_string();
    let digest = <sha2::Sha256 as sha2::Digest>::digest(input.as_bytes());
    let mut remaining = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) >> 2;
    let mut chars = Vec::new();
    for _ in 0..6 {
        chars.push(CODE_ALPHABET[(remaining % 32) as usize]);
        remaining /= 32;
    }
    chars.reverse();
    String::from_utf8(chars).map_err(|_| err("Pairing verification failed"))
}

fn write_private_json(path: &PathBuf, value: &Value) -> Result<(), WsErr> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, format!("{}\n", serde_json::to_string_pretty(value).map_err(|e| err(e.to_string()))?))
        .map_err(|e| err(format!("Failed to save the Host Secrets Sync Key: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path).map_err(|e| err(format!("Failed to save the Host Secrets Sync Key: {e}")))?;
    Ok(())
}


fn read_stored_file() -> Result<Option<Value>, WsErr> {
    let raw = match std::fs::read_to_string(sync_key_path()) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(err(format!("Failed to read the Host Secrets Sync Key: {error}"))),
    };
    let stored: Value = serde_json::from_str(&raw).map_err(|_| err("Stored Host Secrets Sync Key is invalid"))?;
    Ok(Some(stored))
}

fn stored_belongs(stored: &Value, account_url: &str, user_id: &str) -> bool {
    stored.get("version").and_then(Value::as_i64) == Some(1)
        && stored.get("accountUrl").and_then(Value::as_str) == Some(account_url)
        && stored.get("userId").and_then(Value::as_str) == Some(user_id)
}

pub(crate) fn read_pending_rotation(account_url: &str, user_id: &str) -> Result<Option<Value>, WsErr> {
    let Some(stored) = read_stored_file()? else { return Ok(None) };
    if !stored_belongs(&stored, account_url, user_id) {
        return Ok(None);
    }
    Ok(stored.get("pendingRotation").cloned().filter(|pending| !pending.is_null()))
}

pub(crate) fn write_pending_rotation(
    account_url: &str,
    user_id: &str,
    current_key: &[u8; 32],
    pending_rotation: &Value,
) -> Result<(), WsErr> {
    write_private_json(&sync_key_path(), &json!({
        "version": 1,
        "accountUrl": account_url,
        "userId": user_id,
        "key": b64u(current_key),
        "pendingRotation": pending_rotation,
    }))
}

pub(crate) fn read_sync_key(account_url: &str, user_id: &str) -> Result<Option<[u8; 32]>, WsErr> {
    let raw = match std::fs::read_to_string(sync_key_path()) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(err(format!("Failed to read the Host Secrets Sync Key: {error}"))),
    };
    let stored: Value = serde_json::from_str(&raw).map_err(|_| err("Stored Host Secrets Sync Key is invalid"))?;
    if stored.get("version").and_then(Value::as_i64) != Some(1)
        || stored.get("accountUrl").and_then(Value::as_str) != Some(account_url)
        || stored.get("userId").and_then(Value::as_str) != Some(user_id)
    {
        return Ok(None);
    }
    let bytes = b64u_decode(stored.get("key").and_then(Value::as_str).unwrap_or_default())
        .map_err(|_| err("Stored Host Secrets Sync Key is invalid"))?;
    if bytes.len() != 32 {
        return Err(err("Stored Host Secrets Sync Key is invalid"));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(Some(key))
}

pub(crate) fn write_sync_key(account_url: &str, user_id: &str, key: &[u8; 32]) -> Result<(), WsErr> {
    write_private_json(&sync_key_path(), &json!({
        "version": 1,
        "accountUrl": account_url,
        "userId": user_id,
        "key": b64u(key),
    }))
}

struct PendingRecipient {
    keys: PairingKeyPair,
    received: Option<(Value, String, u32)>,
}

fn pending_recipient() -> &'static Mutex<Option<PendingRecipient>> {
    static PENDING: OnceLock<Mutex<Option<PendingRecipient>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(None))
}

pub(crate) fn begin_pairing(device_id: &str, account_url: &str, user_id: &str) -> Result<Value, WsErr> {
    if read_sync_key(account_url, user_id)?.is_some() {
        return Err(err("This device already has the Host Secrets Sync Key"));
    }
    let keys = generate_pairing_keypair();
    let public_jwk = keys.public_jwk.clone();
    if let Ok(mut pending) = pending_recipient().lock() {
        *pending = Some(PendingRecipient { keys, received: None });
    }
    Ok(json!({"recipientDeviceId": device_id, "recipientPublicJwk": public_jwk}))
}

fn pairing_state<T>(operation: impl FnOnce(&mut Option<PendingRecipient>) -> T) -> T {
    let mut pending = pending_recipient().lock().expect("pairing state poisoned");
    operation(&mut pending)
}

pub(crate) fn pairing_verification_for_wrap(wrap: &Value) -> Result<String, WsErr> {
    pairing_verification_code(
        wrap.get("ephemeralPublicJwk").unwrap_or(&Value::Null),
        wrap.get("recipientPublicJwk").unwrap_or(&Value::Null),
    )
}

pub(crate) fn remember_received_wrap(wrap: Value, code: String) {
    pairing_state(|pending| {
        if let Some(pending) = pending.as_mut() {
            pending.received = Some((wrap, code, MAX_CONFIRMATION_ATTEMPTS));
        }
    });
}

pub(crate) fn confirm_received_wrap(verification_code: &str, account_url: &str, user_id: &str) -> Result<(), WsErr> {
    enum Step {
        Missing,
        Mismatch(u32),
        Cancelled,
        Unwrap(Value, SecretKey),
    }
    let step = pairing_state(|pending| {
        let Some(recipient) = pending.as_mut() else { return Step::Missing };
        let Some((wrap, expected, remaining)) = recipient.received.clone() else { return Step::Missing };
        if verification_code != expected {
            let remaining = remaining.saturating_sub(1);
            if remaining == 0 {
                *pending = None;
                return Step::Cancelled;
            }
            recipient.received = Some((wrap, expected, remaining));
            return Step::Mismatch(remaining);
        }
        let private_key = recipient.keys.private_key.clone();
        Step::Unwrap(wrap, private_key)
    });
    match step {
        Step::Missing => Err(err("Receive a Sync-Key wrap before confirming pairing")),
        Step::Cancelled => Err(err("Too many incorrect codes. Pairing was cancelled — start again on both devices. If the codes kept differing, the request may not have come from the device you expect.")),
        Step::Mismatch(remaining) => Err(err(format!(
            "That code does not match. {remaining} {} left before pairing is cancelled.",
            if remaining == 1 { "attempt" } else { "attempts" }
        ))),
        Step::Unwrap(wrap, private_key) => {
            let key = unwrap_sync_key(&wrap, &private_key)?;
            write_sync_key(account_url, user_id, &key)?;
            pairing_state(|pending| *pending = None);
            Ok(())
        }
    }
}

pub(crate) fn rotate_host_secret_entries(
    old_key: &[u8; 32],
    new_key: &[u8; 32],
    entries: &[Value],
) -> Result<Vec<Value>, WsErr> {
    let mut rotated = Vec::new();
    for entry in entries {
        let host_id = entry.get("hostId").and_then(Value::as_str).unwrap_or_default();
        let owner = entry.get("ownerUserId").and_then(Value::as_str).unwrap_or_default();
        let envelope = entry.get("envelope").cloned().unwrap_or(Value::Null);
        let version = envelope.get("version").and_then(Value::as_i64).unwrap_or_default();
        let secret = open_host_secret(old_key, host_id, owner, version, &envelope)?;
        rotated.push(json!({
            "hostId": host_id,
            "ownerUserId": owner,
            "envelope": seal_host_secret(new_key, host_id, owner, version + 1, &secret)?,
        }));
    }
    Ok(rotated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_secret_round_trip() {
        let key = generate_sync_key();
        let secret = json!({"ssh":{"host":"example"}});
        let envelope = seal_host_secret(&key, "host-1", "user-1", 1, &secret).unwrap();
        assert_eq!(open_host_secret(&key, "host-1", "user-1", 1, &envelope).unwrap(), secret);
        assert!(open_host_secret(&key, "host-2", "user-1", 1, &envelope).is_err());
    }

    #[test]
    fn aes_kw_round_trip() {
        let key = [7u8; 32];
        let plaintext = [9u8; 32];
        let wrapped = aes_key_wrap(&key, &plaintext).unwrap();
        assert_eq!(wrapped.len(), 40);
        assert_eq!(aes_key_unwrap(&key, &wrapped).unwrap(), plaintext);
    }

    #[test]
    fn pairing_wrap_round_trip() {
        let sync_key = generate_sync_key();
        let recipient = generate_pairing_keypair();
        let wrapped = wrap_sync_key(&sync_key, &recipient.public_jwk).unwrap();
        assert_eq!(unwrap_sync_key(&wrapped, &recipient.private_key).unwrap(), sync_key);
        assert_eq!(pairing_verification_for_wrap(&wrapped).unwrap().len(), 6);
    }
}
