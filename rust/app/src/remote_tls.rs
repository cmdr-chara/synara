//! Remote TLS identity, port of upstream remoteTransport/certificates.ts.
//! ECDSA P-256 root CA plus a leaf server certificate per environment,
//! persisted in remote-tls.json. The trust anchor fingerprint is the sha256
//! of the root public key's SPKI DER, matching upstream byte for byte.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use x509_parser::prelude::FromDer;
use sha2::Digest;

const DAY_MS: i64 = 86_400_000;
const ROOT_VALIDITY_DAYS: i64 = 3650;
const LEAF_VALIDITY_DAYS: i64 = 90;
const LEAF_RENEW_THRESHOLD_DAYS: i64 = 30;
const ROOT_REPAIR_THRESHOLD_DAYS: i64 = 180;

pub struct RemoteTlsIdentity {
    pub environment_id: String,
    pub root_certificate: String,
    pub root_private_key: String,
    pub leaf_certificate: String,
    pub leaf_private_key: String,
}

pub struct RemoteTlsAnchor {
    pub environment_id: String,
    pub root_certificate: String,
    pub root_fingerprint: String,
}

pub fn remote_tls_identity_path(secrets_dir: &str) -> PathBuf {
    Path::new(secrets_dir).join("remote-tls.json")
}

pub fn remote_tls_server_name(environment_id: &str) -> Result<String, String> {
    if environment_id.is_empty()
        || environment_id.len() > 63
        || !environment_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("Invalid remote environment identity".to_string());
    }
    Ok(format!("host-{}.synara.invalid", environment_id.to_lowercase()))
}

fn sha256_hex(data: &[u8]) -> String {
    sha2::Sha256::digest(data).iter().map(|b| format!("{b:02x}")).collect()
}

/// SPKI DER of a PEM-encoded P-256 private key's public half.
fn spki_der_from_private_pem(private_pem: &str) -> Result<Vec<u8>, String> {
    use p256::pkcs8::DecodePrivateKey;
    use p256::pkcs8::EncodePublicKey;
    let secret = p256::SecretKey::from_pkcs8_pem(private_pem)
        .map_err(|e| format!("private key parse failed: {e}"))?;
    let public = secret.public_key();
    public
        .to_public_key_der()
        .map(|d| d.as_bytes().to_vec())
        .map_err(|e| format!("spki export failed: {e}"))
}

/// Validity window (not_before, not_after) of a PEM certificate in unix ms.
fn cert_validity(cert_pem: &str) -> Result<(i64, i64), String> {
    let pem = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes())
        .map_err(|e| format!("pem parse failed: {e}"))?
        .1;
    let cert = x509_parser::certificate::X509Certificate::from_der(&pem.contents)
        .map_err(|e| format!("cert parse failed: {e}"))?
        .1;
    Ok((
        cert.validity().not_before.timestamp() * 1000,
        cert.validity().not_after.timestamp() * 1000,
    ))
}

fn cert_is_ca(cert_pem: &str) -> Result<bool, String> {
    let pem = x509_parser::pem::parse_x509_pem(cert_pem.as_bytes())
        .map_err(|e| format!("pem parse failed: {e}"))?
        .1;
    let cert = x509_parser::certificate::X509Certificate::from_der(&pem.contents)
        .map_err(|e| format!("cert parse failed: {e}"))?
        .1;
    Ok(cert.is_ca())
}

fn to_time(unix_ms: i64) -> time::OffsetDateTime {
    time::OffsetDateTime::from_unix_timestamp_nanos((unix_ms as i128) * 1_000_000)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
}

fn generate_key() -> Result<rcgen::KeyPair, String> {
    rcgen::KeyPair::generate_for(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| format!("key generation failed: {e}"))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn issue_leaf(
    root_cert_pem: &str,
    root_key: &rcgen::KeyPair,
    environment_id: &str,
    now: i64,
) -> Result<(String, String), String> {
    let server_name = remote_tls_server_name(environment_id)?;
    let leaf_key = generate_key()?;
    let root_pem = x509_parser::pem::parse_x509_pem(root_cert_pem.as_bytes())
        .map_err(|e| format!("root pem parse failed: {e}"))?
        .1;
    // signed_by needs the issuer as a Certificate object, but only reads its
    // params (distinguished name, key usages, identifier method) — the actual
    // signing uses the key. Rebuild those params from the persisted root and
    // use a throwaway self-signed cert purely as their carrier.
    let carrier_params = rcgen::CertificateParams::from_ca_cert_der(&root_pem.contents.as_slice().into())
        .map_err(|e| format!("root cert import failed: {e}"))?;
    let (_, root_not_after) = cert_validity(root_cert_pem)?;

    let mut params = rcgen::CertificateParams::new(vec![server_name.clone()])
        .map_err(|e| format!("leaf params failed: {e}"))?;
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(rcgen::DnType::CommonName, server_name);
    params.distinguished_name = dn;
    params.is_ca = rcgen::IsCa::NoCa;
    params.key_usages = vec![rcgen::KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ServerAuth];
    params.not_before = to_time(now - 60_000);
    params.not_after = to_time((now + LEAF_VALIDITY_DAYS * DAY_MS).min(root_not_after));

    let issuer_carrier = carrier_params
        .self_signed(root_key)
        .map_err(|e| format!("issuer carrier failed: {e}"))?;
    let leaf = params
        .signed_by(&leaf_key, &issuer_carrier, root_key)
        .map_err(|e| format!("leaf signing failed: {e}"))?;
    Ok((leaf.pem(), leaf_key.serialize_pem()))
}

fn self_signed_root(environment_id: &str, now: i64) -> Result<(String, rcgen::KeyPair), String> {
    let root_key = generate_key()?;
    let mut params = rcgen::CertificateParams::default();
    let mut dn = rcgen::DistinguishedName::new();
    dn.push(
        rcgen::DnType::CommonName,
        format!("Synara {environment_id} remote root"),
    );
    params.distinguished_name = dn;
    params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Constrained(0));
    params.key_usages = vec![
        rcgen::KeyUsagePurpose::KeyCertSign,
        rcgen::KeyUsagePurpose::CrlSign,
    ];
    params.not_before = to_time(now - 60_000);
    params.not_after = to_time(now + ROOT_VALIDITY_DAYS * DAY_MS);
    let cert = params
        .self_signed(&root_key)
        .map_err(|e| format!("root signing failed: {e}"))?;
    Ok((cert.pem(), root_key))
}

pub fn anchor_for(environment_id: &str, root_certificate: &str) -> Result<RemoteTlsAnchor, String> {
    // The fingerprint upstream takes from the root cert's SPKI; the persisted
    // private key yields the same public key, so derive from it when we hold
    // the identity — for anchor-only reads parse the certificate directly.
    let pem = x509_parser::pem::parse_x509_pem(root_certificate.as_bytes())
        .map_err(|e| format!("root pem parse failed: {e}"))?
        .1;
    let cert = x509_parser::certificate::X509Certificate::from_der(&pem.contents)
        .map_err(|e| format!("root cert parse failed: {e}"))?
        .1;
    let spki_raw = cert.public_key().raw;
    Ok(RemoteTlsAnchor {
        environment_id: environment_id.to_string(),
        root_certificate: root_certificate.to_string(),
        root_fingerprint: sha256_hex(spki_raw),
    })
}

pub fn root_validity(identity: &RemoteTlsIdentity) -> Result<(i64, i64), String> {
    cert_validity(&identity.root_certificate)
}

pub fn reset_remote_tls_identity(path: &Path, environment_id: &str) -> Result<RemoteTlsIdentity, String> {
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| format!("identity reset failed: {e}"))?;
    }
    initialize_remote_tls_identity(path, environment_id)
}

pub fn root_needs_repair(identity: &RemoteTlsIdentity, now: i64) -> bool {
    match cert_validity(&identity.root_certificate) {
        Ok((_, not_after)) => not_after - now <= ROOT_REPAIR_THRESHOLD_DAYS * DAY_MS,
        Err(_) => true,
    }
}

fn persist(path: &Path, identity: &RemoteTlsIdentity) -> Result<(), String> {
    let body = json!({
        "version": 2,
        "environmentId": identity.environment_id,
        "rootCertificate": identity.root_certificate,
        "rootPrivateKey": identity.root_private_key,
        "leafCertificate": identity.leaf_certificate,
        "leafPrivateKey": identity.leaf_private_key,
    });
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, body.to_string()).map_err(|e| format!("identity write failed: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("identity persist failed: {e}"))?;
    Ok(())
}

fn identity_from_json(raw: &str, environment_id: &str) -> Result<RemoteTlsIdentity, String> {
    let parsed: Value = serde_json::from_str(raw).map_err(|e| format!("identity is not json: {e}"))?;
    let get = |k: &str| {
        parsed
            .get(k)
            .and_then(Value::as_str)
            .map(String::from)
            .ok_or_else(|| "Invalid remote TLS identity".to_string())
    };
    if parsed.get("version").and_then(Value::as_u64) != Some(2)
        || get("environmentId")? != environment_id
    {
        return Err("Remote TLS identity belongs to another installation".to_string());
    }
    Ok(RemoteTlsIdentity {
        environment_id: environment_id.to_string(),
        root_certificate: get("rootCertificate")?,
        root_private_key: get("rootPrivateKey")?,
        leaf_certificate: get("leafCertificate")?,
        leaf_private_key: get("leafPrivateKey")?,
    })
}

/// Validates a persisted identity, renewing the leaf inside its last 30 days.
/// Fail-closed like upstream: corrupt or expired roots demand a re-pair.
pub fn load_remote_tls_identity(path: &Path, environment_id: &str) -> Result<RemoteTlsIdentity, String> {
    let now = now_ms();
    let raw = std::fs::read_to_string(path).map_err(|e| format!("identity read failed: {e}"))?;
    let identity = identity_from_json(&raw, environment_id)?;

    if !cert_is_ca(&identity.root_certificate)? {
        return Err("Remote TLS identity is invalid or its root expired; local re-pair is required".to_string());
    }
    if cert_is_ca(&identity.leaf_certificate)? {
        return Err("Remote TLS identity is invalid or its root expired; local re-pair is required".to_string());
    }
    // The private keys must match their certificates' public keys.
    let root_spki = spki_der_from_private_pem(&identity.root_private_key)?;
    let anchor = anchor_for(environment_id, &identity.root_certificate)?;
    if sha256_hex(&root_spki) != anchor.root_fingerprint {
        return Err("Remote TLS identity is invalid or its root expired; local re-pair is required".to_string());
    }
    let (root_from, root_to) = cert_validity(&identity.root_certificate)?;
    let (leaf_from, leaf_to) = cert_validity(&identity.leaf_certificate)?;
    if root_from > now || root_to <= now || leaf_from > now {
        return Err("Remote TLS identity is invalid or its root expired; local re-pair is required".to_string());
    }
    if leaf_to - now > LEAF_RENEW_THRESHOLD_DAYS * DAY_MS {
        return Ok(identity);
    }
    let root_key = rcgen::KeyPair::from_pem(&identity.root_private_key)
        .map_err(|e| format!("root key parse failed: {e}"))?;
    let (leaf_certificate, leaf_private_key) =
        issue_leaf(&identity.root_certificate, &root_key, environment_id, now)?;
    let renewed = RemoteTlsIdentity { leaf_certificate, leaf_private_key, ..identity };
    persist(path, &renewed)?;
    Ok(renewed)
}

/// Creates a fresh root and leaf. Upstream never recreates a lost root on
/// load — creation is an explicit pairing action here too.
pub fn initialize_remote_tls_identity(path: &Path, environment_id: &str) -> Result<RemoteTlsIdentity, String> {
    remote_tls_server_name(environment_id)?;
    if path.exists() {
        return load_remote_tls_identity(path, environment_id);
    }
    let now = now_ms();
    let (root_certificate, root_key) = self_signed_root(environment_id, now)?;
    let root_private_key = root_key.serialize_pem();
    let (leaf_certificate, leaf_private_key) =
        issue_leaf(&root_certificate, &root_key, environment_id, now)?;
    let identity = RemoteTlsIdentity {
        environment_id: environment_id.to_string(),
        root_certificate,
        root_private_key,
        leaf_certificate,
        leaf_private_key,
    };
    persist(path, &identity)?;
    Ok(identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration_engine::uuid_v4;

    fn test_path() -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("synara-tls-test-{}", uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        (dir.join("remote-tls.json"), dir)
    }

    #[test]
    fn initialize_and_reload() {
        let (path, dir) = test_path();
        let identity = initialize_remote_tls_identity(&path, "env-abc").unwrap();
        assert!(identity.root_certificate.contains("BEGIN CERTIFICATE"));
        assert!(identity.leaf_certificate.contains("BEGIN CERTIFICATE"));

        let loaded = load_remote_tls_identity(&path, "env-abc").unwrap();
        assert_eq!(loaded.root_certificate, identity.root_certificate);
        // A fresh leaf is far from expiry, so no renewal rewrote it.
        assert_eq!(loaded.leaf_certificate, identity.leaf_certificate);

        let anchor = anchor_for("env-abc", &identity.root_certificate).unwrap();
        assert_eq!(anchor.root_fingerprint.len(), 64);
        assert!(!root_needs_repair(&identity, now_ms()));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wrong_environment_is_refused() {
        let (path, dir) = test_path();
        initialize_remote_tls_identity(&path, "env-abc").unwrap();
        assert!(load_remote_tls_identity(&path, "env-other").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tampered_root_key_is_refused() {
        let (path, dir) = test_path();
        let identity = initialize_remote_tls_identity(&path, "env-abc").unwrap();
        let other_key = generate_key().unwrap().serialize_pem();
        let tampered = json!({
            "version": 2,
            "environmentId": "env-abc",
            "rootCertificate": identity.root_certificate,
            "rootPrivateKey": other_key,
            "leafCertificate": identity.leaf_certificate,
            "leafPrivateKey": identity.leaf_private_key,
        });
        std::fs::write(&path, tampered.to_string()).unwrap();
        assert!(load_remote_tls_identity(&path, "env-abc").is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn server_name_validation() {
        assert!(remote_tls_server_name("env-abc-123").is_ok());
        assert!(remote_tls_server_name("bad env").is_err());
        assert!(remote_tls_server_name(&"x".repeat(64)).is_err());
    }
}
