//! Durable ES256 device identity, byte-compatible with upstream
//! `deviceIdentity/store.ts` and `shared/deviceKey.ts`.

use base64::Engine as _;
use p256::ecdsa::{signature::Signer, Signature, SigningKey, VerifyingKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use serde_json::{json, Value};
use sha2::Digest as _;
use std::path::{Path, PathBuf};

use crate::orchestration_engine::uuid_v4;

const DEVICE_IDENTITY_FILE_NAME: &str = "device-identity.json";

fn b64u(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn b64u_decode(text: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| "Value is not base64url".to_string())
}

pub(crate) struct DeviceIdentity {
    pub(crate) signing_key: SigningKey,
    pub(crate) public_jwk: Value,
    pub(crate) jkt: String,
}

pub(crate) fn device_identity_path(secrets_dir: &str) -> PathBuf {
    Path::new(secrets_dir).join(DEVICE_IDENTITY_FILE_NAME)
}

fn public_jwk(verifying: &VerifyingKey) -> Result<Value, String> {
    let point = verifying.to_encoded_point(false);
    let x = point.x().ok_or_else(|| "public key has no x coordinate".to_string())?;
    let y = point.y().ok_or_else(|| "public key has no y coordinate".to_string())?;
    Ok(json!({"kty":"EC","crv":"P-256","x":b64u(x),"y":b64u(y)}))
}

fn thumbprint(public_jwk: &Value) -> Result<String, String> {
    let canonical = format!(
        "{{\"crv\":\"{}\",\"kty\":\"{}\",\"x\":\"{}\",\"y\":\"{}\"}}",
        public_jwk.get("crv").and_then(Value::as_str).unwrap_or_default(),
        public_jwk.get("kty").and_then(Value::as_str).unwrap_or_default(),
        public_jwk.get("x").and_then(Value::as_str).unwrap_or_default(),
        public_jwk.get("y").and_then(Value::as_str).unwrap_or_default(),
    );
    Ok(b64u(&sha2::Sha256::digest(canonical.as_bytes())))
}

fn identity_from_secret(signing_key: SigningKey) -> Result<DeviceIdentity, String> {
    let public_jwk = public_jwk(&signing_key.verifying_key())?;
    let jkt = thumbprint(&public_jwk)?;
    Ok(DeviceIdentity { signing_key, public_jwk, jkt })
}

fn write_private(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("device identity write failed: {e}"))?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, format!("{}\n", serde_json::to_string_pretty(value).map_err(|e| e.to_string())?))
        .map_err(|e| format!("device identity write failed: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("device identity persist failed: {e}"))?;
    Ok(())
}

fn read_identity(path: &Path) -> Result<Option<DeviceIdentity>, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("device identity read failed: {error}")),
    };
    let stored: Value = serde_json::from_str(&raw).map_err(|_| "Stored device identity is invalid".to_string())?;
    if stored.get("version").and_then(Value::as_i64) != Some(1) {
        return Err("Stored device identity is invalid".to_string());
    }
    let private = stored.get("privateKeyJwk").cloned().unwrap_or(Value::Null);
    let public = stored.get("publicKeyJwk").cloned().unwrap_or(Value::Null);
    if private.get("kty").and_then(Value::as_str) != Some("EC")
        || private.get("crv").and_then(Value::as_str) != Some("P-256")
        || private.get("x") != public.get("x")
        || private.get("y") != public.get("y")
    {
        return Err("Stored device identity is invalid".to_string());
    }
    let d = b64u_decode(private.get("d").and_then(Value::as_str).unwrap_or_default())?;
    let signing_key = SigningKey::from_slice(&d).map_err(|_| "Stored device identity is invalid".to_string())?;
    let identity = identity_from_secret(signing_key)?;
    if identity.public_jwk.get("x") != public.get("x") || identity.public_jwk.get("y") != public.get("y") {
        return Err("Stored device identity is invalid".to_string());
    }
    Ok(Some(identity))
}

fn generate_and_persist(path: &Path) -> Result<DeviceIdentity, String> {
    let signing_key = SigningKey::random(&mut rand_core::OsRng);
    let identity = identity_from_secret(signing_key.clone())?;
    let scalar = signing_key.to_bytes();
    let private_jwk = json!({
        "kty":"EC",
        "crv":"P-256",
        "x":identity.public_jwk["x"],
        "y":identity.public_jwk["y"],
        "d":b64u(&scalar),
    });
    write_private(path, &json!({
        "version":1,
        "privateKeyJwk":private_jwk,
        "publicKeyJwk":identity.public_jwk,
    }))?;
    Ok(identity)
}

pub(crate) fn load_or_create(path: &Path) -> Result<DeviceIdentity, String> {
    match read_identity(path) {
        Ok(Some(identity)) => Ok(identity),
        Ok(None) => generate_and_persist(path),
        // A pre-release Ed25519 experiment occupied this path on some dev
        // machines. The shipping wire shape is ES256; replace that local-only
        // key rather than permanently disabling remote dialing.
        Err(_) => generate_and_persist(path),
    }
}

impl DeviceIdentity {
    pub(crate) fn sign_jwt(&self, mut header: Value, payload: Value) -> Result<String, String> {
        header["alg"] = json!("ES256");
        let header_b64 = b64u(serde_json::to_string(&header).map_err(|e| e.to_string())?.as_bytes());
        let payload_b64 = b64u(serde_json::to_string(&payload).map_err(|e| e.to_string())?.as_bytes());
        let signing_input = format!("{header_b64}.{payload_b64}");
        let signature: Signature = self.signing_key.sign(signing_input.as_bytes());
        Ok(format!("{signing_input}.{}", b64u(&signature.to_bytes())))
    }

    pub(crate) fn sign_registration(&self, user_id: &str, display_name: &str, platform: &str, audience: &str) -> Result<String, String> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        self.sign_jwt(
            json!({"typ":"synara-device-register+jwt"}),
            json!({
                "publicKeyJwk": self.public_jwk,
                "displayName": display_name,
                "platform": platform,
                "iss":"synara-device",
                "sub":user_id,
                "aud":audience,
                "iat":now,
                "exp":now+60,
                "jti":uuid_v4(),
            }),
        )
    }

    pub(crate) fn sign_mint_request(&self, user_id: &str, grant: &str, environment_id: &str) -> Result<String, String> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        self.sign_jwt(
            json!({"typ":"synara-mint-request+jwt"}),
            json!({
                "publicKeyJwk": self.public_jwk,
                "grant": grant,
                "iss":"synara-device",
                "sub":user_id,
                "aud":format!("synara-host:{environment_id}"),
                "iat":now,
                "exp":now+120,
                "jti":uuid_v4(),
            }),
        )
    }

    pub(crate) fn sign_dpop(&self, method: &str, url: &str, access_token: Option<&str>) -> Result<String, String> {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
        let host = parsed.host_str().ok_or_else(|| "URL has no host".to_string())?;
        let host = if let Some(port) = parsed.port() { format!("{host}:{port}") } else { host.to_string() };
        let htu = format!("{}://{}{}", parsed.scheme(), host, parsed.path());
        let mut claims = json!({
            "htu": htu,
            "htm": method.to_ascii_uppercase(),
            "jti": uuid_v4(),
            "iat": now,
        });
        if let Some(token) = access_token {
            claims["ath"] = json!(b64u(&sha2::Sha256::digest(token.as_bytes())));
        }
        self.sign_jwt(json!({"typ":"dpop+jwt","jwk":self.public_jwk}), claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_identity_round_trip() {
        let dir = std::env::temp_dir().join(format!("synara-device-test-{}", uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = device_identity_path(dir.to_str().unwrap());
        let first = load_or_create(&path).unwrap();
        let second = load_or_create(&path).unwrap();
        assert_eq!(first.jkt, second.jkt);
        assert_eq!(first.public_jwk, second.public_jwk);
        let jwt = first.sign_registration("user", "host", "darwin", "issuer").unwrap();
        assert_eq!(jwt.split('.').count(), 3);
        std::fs::remove_dir_all(&dir).ok();
    }
}
