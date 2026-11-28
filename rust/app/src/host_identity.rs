//! Host identity, port of upstream hostIdentity/store.ts + jwt.ts.
//! Ed25519 keypair persisted as PKCS8/SPKI PEM in host-identity.json, plus
//! EdDSA JWT minting for host-link and host-proof tokens. Wire shapes and
//! lifetimes match packages/contracts/hostAuth.ts.

use std::path::{Path, PathBuf};

use base64::Engine;
use ed25519_dalek::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde_json::{Value, json};

use crate::orchestration_engine::uuid_v4;

const HOST_IDENTITY_FILE_NAME: &str = "host-identity.json";
pub const HOST_LINK_JWT_TYP: &str = "synara-host-link+jwt";
pub const HOST_PROOF_JWT_TYP: &str = "synara-host-proof+jwt";
pub const HOST_LINK_MAX_AGE_SECONDS: u64 = 5 * 60;
pub const HOST_PROOF_MAX_AGE_SECONDS: u64 = 60;

fn b64url(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

pub struct HostIdentity {
    pub private_key_pem: String,
    pub public_key_pem: String,
    pub public_key_jwk: Value,
    signing: SigningKey,
}

pub fn host_identity_path(secrets_dir: &str) -> PathBuf {
    Path::new(secrets_dir).join(HOST_IDENTITY_FILE_NAME)
}

fn jwk_for(verifying: &VerifyingKey) -> Value {
    json!({
        "kty": "OKP",
        "crv": "Ed25519",
        "x": b64url(verifying.as_bytes()),
    })
}

fn identity_from_keys(signing: SigningKey) -> Result<HostIdentity, String> {
    let verifying = signing.verifying_key();
    let private_key_pem = signing
        .to_pkcs8_pem(ed25519_dalek::pkcs8::spki::der::pem::LineEnding::LF)
        .map_err(|e| format!("pkcs8 encode failed: {e}"))?
        .to_string();
    let public_key_pem = verifying
        .to_public_key_pem(ed25519_dalek::pkcs8::spki::der::pem::LineEnding::LF)
        .map_err(|e| format!("spki encode failed: {e}"))?;
    Ok(HostIdentity {
        private_key_pem,
        public_key_pem,
        public_key_jwk: jwk_for(&verifying),
        signing,
    })
}

#[cfg(test)]
pub fn identity_from_keys_for_test(signing: SigningKey) -> HostIdentity {
    identity_from_keys(signing).expect("generated keys are valid")
}

fn decode_host_identity(raw: &str) -> Result<HostIdentity, String> {
    let parsed: Value =
        serde_json::from_str(raw).map_err(|e| format!("document is not json: {e}"))?;
    if parsed.get("version").and_then(Value::as_u64) != Some(1) {
        return Err("document shape is invalid".to_string());
    }
    let private_pem = parsed
        .get("privateKeyPem")
        .and_then(Value::as_str)
        .ok_or_else(|| "document shape is invalid".to_string())?;
    let public_pem = parsed
        .get("publicKeyPem")
        .and_then(Value::as_str)
        .ok_or_else(|| "document shape is invalid".to_string())?;
    let signing = SigningKey::from_pkcs8_pem(private_pem)
        .map_err(|e| format!("keypair is not a matching Ed25519 pair: {e}"))?;
    let persisted_public = VerifyingKey::from_public_key_pem(public_pem)
        .map_err(|e| format!("keypair is not a matching Ed25519 pair: {e}"))?;
    if signing.verifying_key() != persisted_public {
        return Err("keypair is not a matching Ed25519 pair".to_string());
    }
    Ok(HostIdentity {
        private_key_pem: private_pem.to_string(),
        public_key_pem: public_pem.to_string(),
        public_key_jwk: jwk_for(&persisted_public),
        signing,
    })
}

pub fn read_host_identity(path: &Path) -> Result<Option<HostIdentity>, String> {
    match std::fs::read_to_string(path) {
        Ok(raw) => decode_host_identity(&raw).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("host identity read failed: {e}")),
    }
}

/// Generates a fresh identity and atomically persists it (0600, matching
/// upstream's PRIVATE_FILE_MODE through the secrets-dir permissions).
pub fn generate_and_persist_host_identity(path: &Path) -> Result<HostIdentity, String> {
    let mut rng = rand_core::OsRng;
    let signing = SigningKey::generate(&mut rng);
    let identity = identity_from_keys(signing)?;
    let persisted = json!({
        "version": 1,
        "privateKeyPem": identity.private_key_pem,
        "publicKeyPem": identity.public_key_pem,
    });
    let tmp = path.with_extension("json.tmp");
    let body = format!(
        "{}\n",
        serde_json::to_string_pretty(&persisted).map_err(|e| e.to_string())?
    );
    std::fs::write(&tmp, body).map_err(|e| format!("host identity write failed: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("host identity persist failed: {e}"))?;
    Ok(identity)
}

pub fn delete_host_identity(path: &Path) {
    let _ = std::fs::remove_file(path);
}

fn host_issuer(environment_id: &str) -> String {
    format!("synara-host:{environment_id}")
}

fn now_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl HostIdentity {
    /// Test-only accessors for the verifier's fixtures.
    pub fn signing_for_test(&self) -> SigningKey {
        self.signing.clone()
    }

    /// Hand-rolled EdDSA JWT: upstream uses jose SignJWT, the wire format is
    /// identical — base64url(header).base64url(payload).base64url(signature).
    pub(crate) fn sign_jwt(&self, header: Value, payload: Value) -> Result<String, String> {
        let header_b64 = b64url(
            serde_json::to_string(&header)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        );
        let payload_b64 = b64url(
            serde_json::to_string(&payload)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        );
        let signing_input = format!("{header_b64}.{payload_b64}");
        let signature: Signature = self.signing.sign(signing_input.as_bytes());
        Ok(format!(
            "{signing_input}.{}",
            b64url(signature.to_bytes().as_slice())
        ))
    }
}

#[allow(clippy::too_many_arguments)]
pub fn mint_host_link_proof(
    identity: &HostIdentity,
    api_issuer: &str,
    environment_id: &str,
    challenge_id: &str,
    nonce: &str,
    name: &str,
    platform: &str,
    app_version: Option<&str>,
) -> Result<String, String> {
    let now = now_seconds();
    let mut payload = json!({
        "challengeId": challenge_id,
        "nonce": nonce,
        "publicKeyJwk": identity.public_key_jwk,
        "name": name,
        "platform": platform,
        "iss": host_issuer(environment_id),
        "sub": environment_id,
        "aud": api_issuer,
        "iat": now,
        "exp": now + HOST_LINK_MAX_AGE_SECONDS,
        "jti": uuid_v4(),
    });
    if let Some(version) = app_version {
        payload["appVersion"] = json!(version);
    }
    let header = json!({ "alg": "EdDSA", "typ": HOST_LINK_JWT_TYP });
    identity.sign_jwt(header, payload)
}

pub fn mint_host_proof(
    identity: &HostIdentity,
    api_issuer: &str,
    environment_id: &str,
    host_id: &str,
    key_generation: u64,
) -> Result<String, String> {
    let now = now_seconds();
    let payload = json!({
        "keyGeneration": key_generation,
        "iss": host_issuer(environment_id),
        "sub": host_id,
        "aud": api_issuer,
        "iat": now,
        "exp": now + HOST_PROOF_MAX_AGE_SECONDS,
        "jti": uuid_v4(),
    });
    let header = json!({ "alg": "EdDSA", "typ": HOST_PROOF_JWT_TYP });
    identity.sign_jwt(header, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_roundtrip() {
        let dir = std::env::temp_dir().join(format!("synara-hid-test-{}", uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = host_identity_path(dir.to_str().unwrap());
        let identity = generate_and_persist_host_identity(&path).unwrap();
        assert!(identity.private_key_pem.contains("PRIVATE KEY"));
        assert!(identity.public_key_pem.contains("PUBLIC KEY"));
        assert_eq!(identity.public_key_jwk["kty"], json!("OKP"));
        assert_eq!(identity.public_key_jwk["crv"], json!("Ed25519"));

        let loaded = read_host_identity(&path)
            .unwrap()
            .expect("identity persisted");
        assert_eq!(loaded.private_key_pem, identity.private_key_pem);
        assert_eq!(loaded.public_key_jwk, identity.public_key_jwk);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn proof_is_signed_eddsa_jwt() {
        let mut rng = rand_core::OsRng;
        let signing = SigningKey::generate(&mut rng);
        let identity = identity_from_keys(signing).unwrap();
        let jwt = mint_host_proof(&identity, "https://api.example", "env-1", "host-1", 3).unwrap();
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        let header: Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[0])
                .unwrap(),
        )
        .unwrap();
        assert_eq!(header["alg"], json!("EdDSA"));
        assert_eq!(header["typ"], json!(HOST_PROOF_JWT_TYP));
        let payload: Value = serde_json::from_slice(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(parts[1])
                .unwrap(),
        )
        .unwrap();
        assert_eq!(payload["iss"], json!("synara-host:env-1"));
        assert_eq!(payload["sub"], json!("host-1"));
        assert_eq!(payload["keyGeneration"], json!(3));

        use ed25519_dalek::Verifier;
        let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[2])
            .unwrap();
        let signature = Signature::from_slice(&sig_bytes).unwrap();
        identity
            .signing
            .verifying_key()
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .expect("signature verifies");
    }

    #[test]
    fn mismatched_keypair_is_refused() {
        let dir = std::env::temp_dir().join(format!("synara-hid-test-{}", uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = host_identity_path(dir.to_str().unwrap());
        let identity = generate_and_persist_host_identity(&path).unwrap();
        let other = identity_from_keys(SigningKey::generate(&mut rand_core::OsRng)).unwrap();
        let tampered = json!({
            "version": 1,
            "privateKeyPem": identity.private_key_pem,
            "publicKeyPem": other.public_key_pem,
        });
        std::fs::write(&path, tampered.to_string()).unwrap();
        assert!(read_host_identity(&path).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
