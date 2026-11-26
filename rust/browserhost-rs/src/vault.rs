//! Owner-side browser credential vault foundation.
//!
//! This ports the durable security core of upstream `browserVault.ts` and
//! `vaultKeyProtection.ts`: records are encrypted at rest, metadata-only
//! snapshots never contain secrets, master-password wrapping uses scrypt +
//! AES-256-GCM, and an OS keychain-backed copy of the data key is used when
//! available. Browser capture, generated-password form filling, cookie import
//! and the GPUI management surface are layered above this crate boundary.

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key as AesKey, Nonce};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use zeroize::Zeroizing;

const KEY_BYTES: usize = 32;
const IV_BYTES: usize = 12;
const TAG_BYTES: usize = 16;
const KEY_AAD: &[u8] = b"synara-browser-vault-key-v1";
const VAULT_AAD: &[u8] = b"betterwright-local-credential-vault:v1";
const KEYRING_SERVICE: &str = "synara-browser-vault";
const KEYRING_USER: &str = "data-key";
const GENERATED_PASSWORD_LEN: usize = 24;
const PENDING_TTL_MS: i64 = 60_000;

fn now_ms() -> i64 {
    chrono_lite_now()
}

fn chrono_lite_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn now_iso() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::getrandom(&mut bytes).expect("OS randomness is required for the browser vault");
    bytes
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn unb64url(text: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| "invalid vault encoding".to_string())
}

fn unb64(text: &str, len: usize) -> Result<Vec<u8>, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(text)
        .map_err(|_| "invalid vault encoding".to_string())?;
    if bytes.len() != len {
        return Err("invalid vault encoding".into());
    }
    Ok(bytes)
}

fn encrypt(
    key: &[u8; KEY_BYTES],
    plaintext: &[u8],
    aad: &[u8],
) -> Result<(String, String, String), String> {
    let iv = random_bytes::<IV_BYTES>();
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(key));
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| "vault encryption failed".to_string())?;
    if encrypted.len() < TAG_BYTES {
        return Err("vault encryption failed".into());
    }
    let (data, tag) = encrypted.split_at(encrypted.len() - TAG_BYTES);
    Ok((b64(&iv), b64(tag), b64(data)))
}

fn unb64_any(text: &str) -> Result<Vec<u8>, String> {
    base64::engine::general_purpose::STANDARD
        .decode(text)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(text))
        .map_err(|_| "invalid vault encoding".to_string())
}

fn unb64_len_any(text: &str, len: usize) -> Result<Vec<u8>, String> {
    let bytes = unb64_any(text)?;
    if bytes.len() != len {
        return Err("invalid vault encoding".into());
    }
    Ok(bytes)
}

fn decrypt(
    key: &[u8; KEY_BYTES],
    iv_text: &str,
    tag_text: &str,
    data_text: &str,
    aad: &[u8],
) -> Result<Zeroizing<Vec<u8>>, String> {
    let iv = unb64_len_any(iv_text, IV_BYTES)?;
    let mut encrypted = unb64_any(data_text)?;
    let tag = unb64_len_any(tag_text, TAG_BYTES)?;
    encrypted.extend_from_slice(&tag);
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(key));
    cipher
        .decrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: &encrypted,
                aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| "vault authentication failed".to_string())
}

fn atomic_write(path: &Path, text: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("invalid vault path")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("create vault directory: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
    }
    let temporary = parent.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    std::fs::write(&temporary, text).map_err(|e| format!("write vault file: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(windows)]
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| format!("replace vault file: {e}"))?;
    }
    std::fs::rename(&temporary, path).map_err(|e| format!("replace vault file: {e}"))?;
    Ok(())
}

fn canonical_origin(raw: &str) -> Result<String, String> {
    let mut parsed = url::Url::parse(raw).map_err(|_| "credential origin must be a valid URL")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("credential origin must use http or https".into());
    }
    parsed
        .set_username("")
        .map_err(|_| "invalid credential origin")?;
    let _ = parsed.set_password(None);
    parsed.set_path("/");
    parsed.set_query(None);
    parsed.set_fragment(None);
    Ok(parsed.origin().ascii_serialization())
}

fn keyring_account(directory: &Path) -> String {
    // The account is a stable non-secret locator, not key material. FNV-1a is
    // sufficient to separate per-app/per-channel vault directories in one OS account.
    let path = directory
        .canonicalize()
        .unwrap_or_else(|_| directory.to_path_buf());
    let text = path.to_string_lossy();
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{KEYRING_USER}-{hash:016x}")
}

fn keyring_entry(directory: &Path) -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, &keyring_account(directory))
        .map_err(|e| format!("OS keychain unavailable: {e}"))
}

fn keyring_read(directory: &Path) -> Option<[u8; KEY_BYTES]> {
    let encoded = keyring_entry(directory).ok()?.get_password().ok()?;
    let bytes = unb64(&encoded, KEY_BYTES).ok()?;
    let mut key = [0u8; KEY_BYTES];
    key.copy_from_slice(&bytes);
    Some(key)
}

fn keyring_write(directory: &Path, key: &[u8; KEY_BYTES]) -> Result<(), String> {
    keyring_entry(directory)?
        .set_password(&b64(key))
        .map_err(|e| format!("OS keychain write failed: {e}"))
}

fn derive_password_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; KEY_BYTES]>, String> {
    if password.len() > 1024 {
        return Err("invalid master password".into());
    }
    #[cfg(not(test))]
    let log_n = 17;
    #[cfg(test)]
    let log_n = 10;
    let params = scrypt::Params::new(log_n, 8, 1, KEY_BYTES)
        .map_err(|_| "invalid master password parameters".to_string())?;
    let mut key = Zeroizing::new([0u8; KEY_BYTES]);
    scrypt::scrypt(password.as_bytes(), salt, &params, &mut key[..])
        .map_err(|_| "master password verification failed".to_string())?;
    Ok(key)
}

fn login_category() -> String {
    "login".into()
}

fn exact_origin_match() -> String {
    "exact-origin".into()
}

fn random_base64url_32() -> String {
    b64url(&random_bytes::<32>())
}

fn iso_after_ms(ms: i64) -> String {
    (time::OffsetDateTime::now_utc() + time::Duration::milliseconds(ms))
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn iso_to_ms(text: &str) -> i64 {
    time::OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339)
        .map(|time| time.unix_timestamp() * 1000)
        .unwrap_or_default()
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CredentialSource {
    User,
    Agent,
    Unknown,
}

impl Default for CredentialSource {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct VaultSettings {
    #[serde(rename = "agentUse")]
    pub agent_use: bool,
    #[serde(rename = "offerSave")]
    pub offer_save: bool,
    pub autosave: bool,
}

impl Default for VaultSettings {
    fn default() -> Self {
        Self {
            agent_use: true,
            offer_save: false,
            autosave: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct LoginRecord {
    id: String,
    #[serde(default = "login_category")]
    category: String,
    #[serde(default = "exact_origin_match", rename = "matchMode")]
    match_mode: String,
    origin: String,
    username: String,
    password: String,
    #[serde(default)]
    fields: Value,
    #[serde(default)]
    notes: Option<String>,
    label: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PendingLogin {
    #[serde(rename = "pendingId")]
    pending_id: String,
    #[serde(rename = "recordId")]
    record_id: Option<String>,
    #[serde(rename = "recordVersion")]
    record_version: Option<String>,
    #[serde(rename = "requestFingerprint", skip_serializing_if = "Option::is_none")]
    request_fingerprint: Option<String>,
    origin: String,
    #[serde(rename = "creationOrigin")]
    creation_origin: String,
    #[serde(default = "exact_origin_match", rename = "matchMode")]
    match_mode: String,
    #[serde(rename = "usernameOverride")]
    username_override: bool,
    username: String,
    #[serde(rename = "labelOverride")]
    label_override: bool,
    label: Option<String>,
    #[serde(rename = "secret")]
    password: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "expiresAt")]
    expires_at: String,
}

#[derive(Serialize, Deserialize)]
struct VaultPayload {
    version: u32,
    revision: u64,
    #[serde(default)]
    records: Vec<LoginRecord>,
    #[serde(default)]
    pending: Vec<PendingLogin>,
}

impl Default for VaultPayload {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            records: Vec::new(),
            pending: Vec::new(),
        }
    }
}

mod source_entries {
    use super::CredentialSource;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    struct SourceEntry {
        id: String,
        source: CredentialSource,
    }

    pub fn serialize<S>(
        sources: &HashMap<String, CredentialSource>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut entries: Vec<_> = sources
            .iter()
            .map(|(id, source)| SourceEntry {
                id: id.clone(),
                source: *source,
            })
            .collect();
        entries.sort_by(|left, right| left.id.cmp(&right.id));
        entries.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, CredentialSource>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let entries = Vec::<SourceEntry>::deserialize(deserializer)?;
        Ok(entries
            .into_iter()
            .map(|entry| (entry.id, entry.source))
            .collect())
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Preferences {
    #[serde(default)]
    settings: VaultSettings,
    #[serde(default, with = "source_entries")]
    sources: HashMap<String, CredentialSource>,
    #[serde(default, rename = "neverOrigins")]
    never_origins: HashSet<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VaultProtection {
    pub configured: bool,
    pub locked: bool,
    pub os_protected: bool,
}

struct SavePrompt {
    id: String,
    origin: String,
    username: String,
    password: Zeroizing<String>,
    mode: &'static str,
    expires_at_ms: i64,
}

pub struct BrowserVault {
    directory: PathBuf,
    state: Mutex<VaultState>,
}

struct VaultState {
    key: Option<Zeroizing<[u8; KEY_BYTES]>>,
    master_configured: bool,
    os_protected: bool,
    payload: VaultPayload,
    preferences: Preferences,
    redaction_secrets: HashSet<String>,
    capture_error: Option<String>,
    save_prompts: Vec<SavePrompt>,
    master_retry_at: i64,
}

impl BrowserVault {
    pub fn open(directory: impl AsRef<Path>) -> Result<Self, String> {
        Self::open_with_keyring(directory, true)
    }

    pub fn open_with_keyring(
        directory: impl AsRef<Path>,
        use_keyring: bool,
    ) -> Result<Self, String> {
        let directory = directory.as_ref().to_path_buf();
        let mut state = VaultState {
            key: None,
            master_configured: false,
            os_protected: false,
            payload: VaultPayload::default(),
            preferences: Preferences::default(),
            redaction_secrets: HashSet::new(),
            capture_error: None,
            save_prompts: Vec::new(),
            master_retry_at: 0,
        };
        if let Ok(text) = std::fs::read_to_string(directory.join("preferences.json")) {
            state.preferences = serde_json::from_str(&text).unwrap_or_default();
        }

        let key_file = directory.join("key-protection.json");
        let legacy_key_file = directory.join("vault.key");
        let vault_file = directory.join("vault.enc");
        if key_file.exists() {
            let envelope: Value = serde_json::from_str(
                &std::fs::read_to_string(&key_file)
                    .map_err(|e| format!("read vault key protection: {e}"))?,
            )
            .map_err(|_| "invalid vault key protection".to_string())?;
            state.master_configured = envelope.get("master").is_some_and(|value| !value.is_null());
            state.os_protected = envelope.get("os").is_some_and(|value| !value.is_null());
            if state.os_protected && use_keyring {
                state.key = keyring_read(&directory).map(Zeroizing::new);
            }
        } else if legacy_key_file.exists() {
            let encoded = std::fs::read_to_string(&legacy_key_file)
                .map_err(|e| format!("read vault key: {e}"))?;
            let bytes = unb64(encoded.trim(), KEY_BYTES)?;
            let mut key = [0u8; KEY_BYTES];
            key.copy_from_slice(&bytes);
            state.key = Some(Zeroizing::new(key));
        } else if vault_file.exists() {
            return Err("vault key is missing; refusing to replace existing credentials".into());
        } else {
            let key = random_bytes::<KEY_BYTES>();
            if use_keyring && keyring_write(&directory, &key).is_ok() {
                state.os_protected = true;
                write_key_envelope(&directory, false, true, None)?;
            } else {
                std::fs::create_dir_all(&directory)
                    .map_err(|e| format!("create vault directory: {e}"))?;
                std::fs::write(&legacy_key_file, b64(&key))
                    .map_err(|e| format!("write vault key: {e}"))?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = std::fs::set_permissions(
                        &legacy_key_file,
                        std::fs::Permissions::from_mode(0o600),
                    );
                }
            }
            state.key = Some(Zeroizing::new(key));
        }

        if let (Some(key), Ok(text)) = (&state.key, std::fs::read_to_string(&vault_file)) {
            let envelope: Value =
                serde_json::from_str(&text).map_err(|_| "invalid encrypted vault".to_string())?;
            let plaintext = decrypt(
                key,
                envelope.get("iv").and_then(Value::as_str).unwrap_or(""),
                envelope.get("tag").and_then(Value::as_str).unwrap_or(""),
                envelope.get("data").and_then(Value::as_str).unwrap_or(""),
                VAULT_AAD,
            )?;
            state.payload = serde_json::from_slice(&plaintext)
                .map_err(|_| "invalid encrypted vault payload".to_string())?;
            for record in &state.payload.records {
                state.redaction_secrets.insert(record.password.clone());
            }
            for pending in &state.payload.pending {
                state.redaction_secrets.insert(pending.password.clone());
            }
        }
        Ok(Self {
            directory,
            state: Mutex::new(state),
        })
    }

    pub fn protection(&self) -> VaultProtection {
        let state = self.state.lock().expect("vault lock");
        VaultProtection {
            configured: state.master_configured,
            locked: state.key.is_none(),
            os_protected: state.os_protected,
        }
    }

    pub fn setup_master(&self, password: &str) -> Result<(), String> {
        if password.len() < 12 || password.len() > 1024 {
            return Err("use at least 12 characters for the master password".into());
        }
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        if state.master_configured {
            return Err("master password setup is unavailable".into());
        }
        let key = state.key.as_ref().ok_or("unlock the browser vault first")?;
        let salt = random_bytes::<16>();
        let derived = derive_password_key(password, &salt)?;
        let (iv, tag, data) = encrypt(&derived, &key[..], KEY_AAD)?;
        write_key_envelope(
            &self.directory,
            true,
            state.os_protected,
            Some(json!({"salt": b64(&salt), "iv": iv, "tag": tag, "data": data})),
        )?;
        state.master_configured = true;
        let _ = std::fs::remove_file(self.directory.join("vault.key"));
        Ok(())
    }

    pub fn unlock(&self, password: &str) -> Result<(), String> {
        let envelope_text = std::fs::read_to_string(self.directory.join("key-protection.json"))
            .map_err(|_| "master password verification is unavailable".to_string())?;
        let envelope: Value =
            serde_json::from_str(&envelope_text).map_err(|_| "invalid vault key protection")?;
        let master = envelope
            .get("master")
            .ok_or("master password is not configured")?;
        {
            let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
            if now_ms() < state.master_retry_at {
                return Err("master password verification failed. Try again shortly.".into());
            }
        }
        let salt = unb64(master.get("salt").and_then(Value::as_str).unwrap_or(""), 16)?;
        let derived = derive_password_key(password, &salt)?;
        let plaintext = match decrypt(
            &derived,
            master.get("iv").and_then(Value::as_str).unwrap_or(""),
            master.get("tag").and_then(Value::as_str).unwrap_or(""),
            master.get("data").and_then(Value::as_str).unwrap_or(""),
            KEY_AAD,
        ) {
            Ok(plaintext) => plaintext,
            Err(error) => {
                if let Ok(mut state) = self.state.lock() {
                    state.master_retry_at = now_ms() + 1000;
                }
                return Err(error);
            }
        };
        if plaintext.len() != KEY_BYTES {
            return Err("vault key verification failed".into());
        }
        let mut key = [0u8; KEY_BYTES];
        key.copy_from_slice(&plaintext);
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        state.key = Some(Zeroizing::new(key));
        self.load_payload_locked(&mut state)
    }

    pub fn lock(&self) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        if !state.master_configured {
            return Err("set a master password before locking".into());
        }
        state.key = None;
        state.payload = VaultPayload::default();
        state.save_prompts.clear();
        Ok(())
    }

    pub fn configure(&self, settings: VaultSettings) -> Result<Value, String> {
        if settings.autosave && !settings.offer_save {
            return Err("enable password saving before autosave".into());
        }
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        state.preferences.settings = settings;
        if !settings.offer_save {
            state.save_prompts.clear();
        }
        self.persist_preferences_locked(&state)?;
        drop(state);
        self.snapshot()
    }

    pub fn snapshot(&self) -> Result<Value, String> {
        let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        let locked = state.key.is_none();
        let now = now_ms();
        let mut logins = Vec::new();
        if !locked {
            for record in &state.payload.records {
                logins.push(json!({
                    "id": record.id,
                    "origin": record.origin,
                    "username": record.username,
                    "label": record.label,
                    "updatedAt": record.updated_at,
                    "status": "saved",
                    "source": state.preferences.sources.get(&record.id).copied().unwrap_or_default(),
                }));
            }
            for pending in &state.payload.pending {
                logins.push(json!({
                    "id": pending.pending_id,
                    "origin": pending.origin,
                    "username": pending.username,
                    "label": pending.label,
                    "updatedAt": pending.created_at,
                    "status": if iso_to_ms(&pending.expires_at) < now { "expired" } else { "pending" },
                    "source": state.preferences.sources.get(&pending.pending_id).copied().unwrap_or_default(),
                }));
            }
        }
        Ok(json!({
            "protection": {
                "configured": state.master_configured,
                "locked": locked,
                "osProtected": state.os_protected,
            },
            "settings": state.preferences.settings,
            "logins": logins,
            "pending": state.save_prompts.iter().filter(|prompt| prompt.expires_at_ms > now).map(|prompt| json!({
                "id": prompt.id,
                "origin": prompt.origin,
                "username": prompt.username,
                "mode": prompt.mode,
                "expiresAt": prompt.expires_at_ms,
            })).collect::<Vec<_>>(),
            "error": state.capture_error.clone().map(Value::String).unwrap_or(Value::Null),
        }))
    }

    /// Queue a metadata-visible save/update decision for the owner. The
    /// password remains only in process memory until the prompt is answered
    /// or expires; snapshots never include it.
    pub fn offer_save(
        &self,
        origin: &str,
        username: &str,
        password: &str,
    ) -> Result<Value, String> {
        let origin = canonical_origin(origin)?;
        if username.trim().is_empty() || password.is_empty() {
            return Err("username and password are required".into());
        }
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        self.require_unlocked(&state)?;
        if !state.preferences.settings.offer_save || state.preferences.never_origins.contains(&origin) {
            return Err("password saving is unavailable for this site".into());
        }
        let now = now_ms();
        state.save_prompts.retain(|prompt| prompt.expires_at_ms > now);
        state.save_prompts.retain(|prompt| {
            !(prompt.origin == origin && prompt.username.eq_ignore_ascii_case(username))
        });
        if state.save_prompts.len() >= 8 {
            return Err("too many password save prompts are waiting".into());
        }
        let mode = if state.payload.records.iter().any(|record| {
            record.origin == origin && record.username.eq_ignore_ascii_case(username)
        }) {
            "update"
        } else {
            "save"
        };
        let prompt = SavePrompt {
            id: uuid::Uuid::new_v4().to_string(),
            origin,
            username: username.to_string(),
            password: Zeroizing::new(password.to_string()),
            mode,
            expires_at_ms: now + 120_000,
        };
        let metadata = json!({
            "id": prompt.id,
            "origin": prompt.origin,
            "username": prompt.username,
            "mode": prompt.mode,
            "expiresAt": prompt.expires_at_ms,
        });
        state.save_prompts.push(prompt);
        Ok(metadata)
    }

    pub fn respond(&self, id: &str, save: bool) -> Result<Value, String> {
        let prompt = {
            let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
            self.require_unlocked(&state)?;
            let now = now_ms();
            state.save_prompts.retain(|prompt| prompt.expires_at_ms > now);
            let position = state
                .save_prompts
                .iter()
                .position(|prompt| prompt.id == id)
                .ok_or("save prompt is no longer available")?;
            state.save_prompts.remove(position)
        };
        if save {
            self.save_captured(
                &prompt.origin,
                &prompt.username,
                prompt.password.as_str(),
                None,
                CredentialSource::User,
                false,
            )?;
        }
        self.snapshot()
    }

    pub fn save_login(
        &self,
        origin: &str,
        username: &str,
        password: &str,
        label: Option<&str>,
        source: CredentialSource,
    ) -> Result<String, String> {
        let origin = canonical_origin(origin)?;
        if username.trim().is_empty() || password.is_empty() {
            return Err("username and password are required".into());
        }
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        self.require_unlocked(&state)?;
        let now = now_iso();
        let id = if let Some(record) = state.payload.records.iter_mut().find(|record| {
            record.origin == origin && record.username.eq_ignore_ascii_case(username)
        }) {
            record.password = password.to_string();
            record.label = label.map(str::to_string).or_else(|| record.label.clone());
            record.updated_at = now;
            record.id.clone()
        } else {
            let id = format!("cred_{}", uuid::Uuid::new_v4());
            state.payload.records.push(LoginRecord {
                id: id.clone(),
                category: login_category(),
                match_mode: exact_origin_match(),
                origin,
                username: username.to_string(),
                password: password.to_string(),
                fields: Value::Null,
                notes: None,
                label: label.map(str::to_string),
                created_at: now.clone(),
                updated_at: now,
            });
            id
        };
        state.preferences.sources.insert(id.clone(), source);
        state.redaction_secrets.insert(password.to_string());
        self.persist_all_locked(&state)?;
        Ok(id)
    }

    pub fn report_capture_failure(&self) {
        if let Ok(mut state) = self.state.lock() {
            if state.capture_error.is_none() {
                state.capture_error =
                    Some("Password saving is unavailable. Reopen the browser and try again.".into());
            }
        }
    }

    pub fn report_capture_ready(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.capture_error = None;
        }
    }

    pub fn track_secret(&self, secret: &str) {
        if secret.is_empty() {
            return;
        }
        if let Ok(mut state) = self.state.lock() {
            state.redaction_secrets.insert(secret.to_string());
        }
    }

    pub fn capture_enabled(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.preferences.settings.offer_save && state.key.is_some())
            .unwrap_or(false)
    }

    pub fn capture_policy(&self) -> (bool, bool) {
        self.state
            .lock()
            .map(|state| {
                (
                    state.preferences.settings.offer_save && state.key.is_some(),
                    state.preferences.settings.autosave,
                )
            })
            .unwrap_or((false, false))
    }

    pub fn is_never_origin(&self, origin: &str) -> bool {
        let Ok(origin) = canonical_origin(origin) else {
            return false;
        };
        self.state
            .lock()
            .map(|state| state.preferences.never_origins.contains(&origin))
            .unwrap_or(false)
    }

    pub fn never_origin(&self, origin: &str) -> Result<(), String> {
        let origin = canonical_origin(origin)?;
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        state.preferences.never_origins.insert(origin);
        while state.preferences.never_origins.len() > 500 {
            if let Some(first) = state.preferences.never_origins.iter().next().cloned() {
                state.preferences.never_origins.remove(&first);
            }
        }
        self.persist_preferences_locked(&state)
    }

    pub fn has_login(&self, origin: &str, username: &str) -> Result<bool, String> {
        let origin = canonical_origin(origin)?;
        let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        self.require_unlocked(&state)?;
        Ok(state.payload.records.iter().any(|record| {
            record.origin == origin && record.username.eq_ignore_ascii_case(username)
        }))
    }

    pub fn should_offer_save(
        &self,
        origin: &str,
        username: &str,
        password: &str,
    ) -> Result<bool, String> {
        let origin = canonical_origin(origin)?;
        let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        if !state.preferences.settings.offer_save || state.key.is_none() {
            return Ok(false);
        }
        if state.preferences.never_origins.contains(&origin) {
            return Ok(false);
        }
        Ok(!state.payload.records.iter().any(|record| {
            record.origin == origin
                && record.username.eq_ignore_ascii_case(username)
                && record.password == password
        }))
    }

    /// Save an accepted browser submission. `defer_to_pending` suppresses the
    /// duplicate produced when a generated password was typed into the page by
    /// the agent; only the vault may compare that secret.
    pub fn save_captured(
        &self,
        origin: &str,
        username: &str,
        password: &str,
        label: Option<&str>,
        source: CredentialSource,
        defer_to_pending: bool,
    ) -> Result<bool, String> {
        let origin = canonical_origin(origin)?;
        if defer_to_pending {
            let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
            self.require_unlocked(&state)?;
            let now = now_ms();
            let deferred = state.payload.pending.iter().any(|pending| {
                pending.origin == origin
                    && iso_to_ms(&pending.expires_at) >= now
                    && pending.password == password
            });
            if deferred {
                return Ok(false);
            }
        }
        self.save_login(&origin, username, password, label, source)?;
        Ok(true)
    }

    pub fn remove(&self, id: &str) -> Result<Value, String> {
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        self.require_unlocked(&state)?;
        state.payload.records.retain(|record| record.id != id);
        state
            .payload
            .pending
            .retain(|pending| pending.pending_id != id);
        state.preferences.sources.remove(id);
        self.persist_all_locked(&state)?;
        drop(state);
        self.snapshot()
    }

    pub fn reveal(&self, id: &str, password: &str) -> Result<Value, String> {
        self.unlock(password)?;
        let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        let record = state
            .payload
            .records
            .iter()
            .find(|record| record.id == id)
            .ok_or("no saved password exists for this login")?;
        let password = record.password.clone();
        drop(state);
        self.track_secret(&password);
        Ok(json!({"password": password, "expiresAt": now_ms() + 20_000}))
    }

    pub fn list_metadata(&self, origin: &str) -> Result<Vec<Value>, String> {
        let origin = canonical_origin(origin)?;
        let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        self.require_unlocked(&state)?;
        Ok(state
            .payload
            .records
            .iter()
            .filter(|record| record.origin == origin)
            .map(|record| {
                json!({
                    "id": record.id,
                    "origin": record.origin,
                    "username": record.username,
                    "label": record.label,
                    "updatedAt": record.updated_at,
                })
            })
            .collect())
    }

    pub fn list_pending_metadata(&self, origin: &str) -> Result<Vec<Value>, String> {
        let origin = canonical_origin(origin)?;
        let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        self.require_unlocked(&state)?;
        let now = now_ms();
        Ok(state
            .payload
            .pending
            .iter()
            .filter(|pending| pending.origin == origin)
            .map(|pending| {
                json!({
                    "pendingId": pending.pending_id,
                    "origin": pending.origin,
                    "username": pending.username,
                    "label": pending.label,
                    "createdAt": pending.created_at,
                    "expired": iso_to_ms(&pending.expires_at) < now,
                })
            })
            .collect())
    }

    pub fn agent_list_metadata(&self, origin: &str) -> Result<Vec<Value>, String> {
        let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        if !state.preferences.settings.agent_use {
            return Err("saved login use is unavailable".into());
        }
        drop(state);
        self.list_metadata(origin)
    }

    pub fn agent_list_pending_metadata(&self, origin: &str) -> Result<Vec<Value>, String> {
        let state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        if !state.preferences.settings.agent_use {
            return Err("saved login use is unavailable".into());
        }
        drop(state);
        self.list_pending_metadata(origin)
    }

    pub fn generate_password(
        &self,
        origin: &str,
        username: &str,
        label: Option<&str>,
        record_id: Option<&str>,
        source: CredentialSource,
    ) -> Result<Value, String> {
        const ALPHABET: &[u8] =
            b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*";
        let mut password = String::with_capacity(GENERATED_PASSWORD_LEN);
        while password.len() < GENERATED_PASSWORD_LEN {
            let random = random_bytes::<GENERATED_PASSWORD_LEN>();
            let accepted = (256usize / ALPHABET.len()) * ALPHABET.len();
            for byte in random {
                if (byte as usize) < accepted && password.len() < GENERATED_PASSWORD_LEN {
                    password.push(ALPHABET[byte as usize % ALPHABET.len()] as char);
                }
            }
        }
        let origin = canonical_origin(origin)?;
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        self.require_unlocked(&state)?;
        state
            .payload
            .pending
            .retain(|pending| iso_to_ms(&pending.expires_at) >= now_ms());
        if state.payload.pending.len() >= 64 {
            return Err("too many pending generated credentials".into());
        }
        let pending_id = uuid::Uuid::new_v4().to_string();
        let created_at = now_iso();
        state.payload.pending.push(PendingLogin {
            pending_id: pending_id.clone(),
            record_id: record_id.map(str::to_string),
            record_version: record_id.map(|_| random_base64url_32()),
            request_fingerprint: Some(random_base64url_32()),
            origin: origin.clone(),
            creation_origin: origin.clone(),
            match_mode: exact_origin_match(),
            username_override: true,
            username: username.to_string(),
            label_override: label.is_some(),
            label: label.map(str::to_string),
            password: password.clone(),
            created_at: created_at.clone(),
            updated_at: created_at,
            expires_at: iso_after_ms(PENDING_TTL_MS),
        });
        state.preferences.sources.insert(pending_id.clone(), source);
        state.redaction_secrets.insert(password);
        self.persist_all_locked(&state)?;
        Ok(json!({"pendingId": pending_id, "origin": origin, "username": username}))
    }

    pub fn commit_generated(&self, origin: &str, pending_id: &str) -> Result<String, String> {
        let origin = canonical_origin(origin)?;
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        self.require_unlocked(&state)?;
        let index = state
            .payload
            .pending
            .iter()
            .position(|pending| pending.pending_id == pending_id && pending.origin == origin)
            .ok_or("pending generated credential not found")?;
        let pending = state.payload.pending.remove(index);
        let now = now_iso();
        let id = if let Some(record_id) = pending.record_id {
            let record = state
                .payload
                .records
                .iter_mut()
                .find(|record| record.id == record_id)
                .ok_or("generated credential target not found")?;
            record.password = pending.password;
            record.username = pending.username;
            record.label = pending.label.or_else(|| record.label.clone());
            record.updated_at = now;
            record.id.clone()
        } else if let Some(record) = state.payload.records.iter_mut().find(|record| {
            record.origin == pending.origin
                && record.username.eq_ignore_ascii_case(&pending.username)
        }) {
            record.password = pending.password;
            record.label = pending.label.or_else(|| record.label.clone());
            record.updated_at = now;
            record.id.clone()
        } else {
            let id = format!("cred_{}", uuid::Uuid::new_v4());
            state.payload.records.push(LoginRecord {
                id: id.clone(),
                category: login_category(),
                match_mode: exact_origin_match(),
                origin: pending.origin,
                username: pending.username,
                password: pending.password,
                fields: Value::Null,
                notes: None,
                label: pending.label,
                created_at: pending.created_at,
                updated_at: now,
            });
            id
        };
        if let Some(source) = state.preferences.sources.remove(pending_id) {
            state.preferences.sources.insert(id.clone(), source);
        }
        self.persist_all_locked(&state)?;
        Ok(id)
    }

    pub fn discard_generated(&self, origin: &str, pending_id: &str) -> Result<(), String> {
        let origin = canonical_origin(origin)?;
        let mut state = self.state.lock().map_err(|_| "vault lock".to_string())?;
        self.require_unlocked(&state)?;
        let before = state.payload.pending.len();
        state
            .payload
            .pending
            .retain(|pending| !(pending.pending_id == pending_id && pending.origin == origin));
        if state.payload.pending.len() == before {
            return Err("pending generated credential not found".into());
        }
        state.preferences.sources.remove(pending_id);
        self.persist_all_locked(&state)
    }

    pub fn redact(&self, value: &Value) -> Value {
        let state = self.state.lock().expect("vault lock");
        redact_value(value, &state.redaction_secrets)
    }

    fn require_unlocked(&self, state: &VaultState) -> Result<(), String> {
        if state.key.is_some() {
            Ok(())
        } else {
            Err("unlock the browser vault first".into())
        }
    }

    fn persist_preferences_locked(&self, state: &VaultState) -> Result<(), String> {
        atomic_write(
            &self.directory.join("preferences.json"),
            &serde_json::to_string(&state.preferences).map_err(|e| e.to_string())?,
        )
    }

    fn persist_all_locked(&self, state: &VaultState) -> Result<(), String> {
        self.persist_preferences_locked(state)?;
        let key = state.key.as_ref().ok_or("unlock the browser vault first")?;
        let plaintext = serde_json::to_vec(&state.payload).map_err(|e| e.to_string())?;
        let (iv, tag, data) = encrypt(key, &plaintext, VAULT_AAD)?;
        atomic_write(
            &self.directory.join("vault.enc"),
            &serde_json::to_string(&json!({"version": 1, "iv": iv, "tag": tag, "data": data}))
                .map_err(|e| e.to_string())?,
        )
    }

    fn load_payload_locked(&self, state: &mut VaultState) -> Result<(), String> {
        let path = self.directory.join("vault.enc");
        if !path.exists() {
            state.payload = VaultPayload::default();
            return Ok(());
        }
        let key = state.key.as_ref().ok_or("unlock the browser vault first")?;
        let text =
            std::fs::read_to_string(path).map_err(|e| format!("read encrypted vault: {e}"))?;
        let envelope: Value = serde_json::from_str(&text).map_err(|_| "invalid encrypted vault")?;
        let plaintext = decrypt(
            key,
            envelope.get("iv").and_then(Value::as_str).unwrap_or(""),
            envelope.get("tag").and_then(Value::as_str).unwrap_or(""),
            envelope.get("data").and_then(Value::as_str).unwrap_or(""),
            VAULT_AAD,
        )?;
        state.payload =
            serde_json::from_slice(&plaintext).map_err(|_| "invalid encrypted vault payload")?;
        state.redaction_secrets = state
            .payload
            .records
            .iter()
            .map(|record| record.password.clone())
            .chain(
                state
                    .payload
                    .pending
                    .iter()
                    .map(|pending| pending.password.clone()),
            )
            .collect();
        Ok(())
    }
}

fn write_key_envelope(
    directory: &Path,
    master_configured: bool,
    os_protected: bool,
    master: Option<Value>,
) -> Result<(), String> {
    atomic_write(
        &directory.join("key-protection.json"),
        &serde_json::to_string(&json!({
            "version": 1,
            "os": if os_protected { Value::String("keyring".into()) } else { Value::Null },
            "master": if master_configured { master.unwrap_or(Value::Null) } else { Value::Null },
        }))
        .map_err(|e| e.to_string())?,
    )
}

fn redact_value(value: &Value, secrets: &HashSet<String>) -> Value {
    match value {
        Value::String(text) => {
            let mut redacted = text.clone();
            for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
                redacted = redacted.replace(secret, "[redacted]");
            }
            Value::String(redacted)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_value(item, secrets))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), redact_value(item, secrets)))
                .collect(),
        ),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arr_len(value: Option<&Value>) -> usize {
        value.and_then(Value::as_array).map(Vec::len).unwrap_or(0)
    }

    fn home(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("synara-vault-test-{name}-{}", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_dir_all(&path);
        path
    }

    #[test]
    fn encrypted_save_snapshot_reveal_and_remove_round_trip() {
        let dir = home("roundtrip");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        let id = vault
            .save_login(
                "https://example.com/login?next=1",
                "Ada",
                "correct horse battery staple",
                Some("work"),
                CredentialSource::User,
            )
            .unwrap();
        let snapshot = vault.snapshot().unwrap();
        assert_eq!(
            snapshot
                .pointer("/logins/0/username")
                .and_then(Value::as_str),
            Some("Ada")
        );
        assert!(!snapshot.to_string().contains("correct horse"));
        let ciphertext = std::fs::read_to_string(dir.join("vault.enc")).unwrap();
        assert!(!ciphertext.contains("correct horse"));
        vault.setup_master("a long master password").unwrap();
        vault.lock().unwrap();
        assert!(vault
            .snapshot()
            .unwrap()
            .pointer("/protection/locked")
            .and_then(Value::as_bool)
            .unwrap());
        let revealed = vault.reveal(&id, "a long master password").unwrap();
        assert_eq!(
            revealed.get("password").and_then(Value::as_str),
            Some("correct horse battery staple")
        );
        assert!(vault
            .remove(&id)
            .unwrap()
            .get("logins")
            .and_then(Value::as_array)
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn locked_vault_reopens_and_unlocks_with_master_password() {
        let dir = home("reopen");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        let id = vault
            .save_login(
                "https://example.com/",
                "ada",
                "persisted-secret",
                None,
                CredentialSource::User,
            )
            .unwrap();
        vault.setup_master("another long password").unwrap();
        drop(vault);

        let locked = BrowserVault::open_with_keyring(&dir, false).unwrap();
        assert!(locked
            .snapshot()
            .unwrap()
            .pointer("/protection/locked")
            .and_then(Value::as_bool)
            .unwrap());
        assert!(locked.unlock("the wrong password").is_err());
        locked.state.lock().unwrap().master_retry_at = 0;
        locked.unlock("another long password").unwrap();
        let revealed = locked.reveal(&id, "another long password").unwrap();
        assert_eq!(
            revealed.get("password").and_then(Value::as_str),
            Some("persisted-secret")
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn preferences_sources_use_upstream_entry_array() {
        let dir = home("preferences-format");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        vault
            .save_login(
                "https://example.com/",
                "ada",
                "secret",
                None,
                CredentialSource::User,
            )
            .unwrap();
        let text = std::fs::read_to_string(dir.join("preferences.json")).unwrap();
        let preferences: Value = serde_json::from_str(&text).unwrap();
        assert!(preferences.get("sources").and_then(Value::as_array).is_some());
        assert_eq!(
            preferences.pointer("/sources/0/source").and_then(Value::as_str),
            Some("user")
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn exact_origin_list_never_exposes_password() {
        let dir = home("metadata");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        vault
            .save_login(
                "https://example.com/",
                "ada",
                "secret",
                None,
                CredentialSource::User,
            )
            .unwrap();
        let listed = vault.list_metadata("https://example.com/account").unwrap();
        assert_eq!(listed.len(), 1);
        assert!(!serde_json::to_string(&listed).unwrap().contains("secret"));
        assert!(vault
            .list_metadata("https://other.example/")
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn generated_credentials_have_two_phase_commit_and_discard() {
        let dir = home("generated");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        let pending = vault
            .generate_password(
                "https://example.com/",
                "ada",
                None,
                None,
                CredentialSource::Agent,
            )
            .unwrap();
        let pending_id = pending
            .get("pendingId")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        assert!(vault
            .list_metadata("https://example.com/")
            .unwrap()
            .is_empty());
        let id = vault
            .commit_generated("https://example.com/", &pending_id)
            .unwrap();
        assert_eq!(
            vault.list_metadata("https://example.com/").unwrap()[0]
                .get("id")
                .and_then(Value::as_str),
            Some(id.as_str())
        );
        let second = vault
            .generate_password(
                "https://example.com/",
                "ada",
                None,
                None,
                CredentialSource::Agent,
            )
            .unwrap();
        vault
            .discard_generated(
                "https://example.com/",
                second.get("pendingId").and_then(Value::as_str).unwrap(),
            )
            .unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn agent_use_setting_gates_agent_metadata_access() {
        let dir = home("agent-gate");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        vault
            .save_login(
                "https://example.com/",
                "ada",
                "secret",
                None,
                CredentialSource::User,
            )
            .unwrap();
        let mut settings = VaultSettings::default();
        settings.agent_use = false;
        vault.configure(settings).unwrap();
        assert!(vault.agent_list_metadata("https://example.com/").is_err());
        assert_eq!(
            vault.list_metadata("https://example.com/").unwrap().len(),
            1
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn capture_policy_detects_updates_duplicates_and_never_origins() {
        let dir = home("capture-policy");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        let mut settings = VaultSettings::default();
        settings.offer_save = true;
        vault.configure(settings).unwrap();
        vault
            .save_login(
                "https://example.com/",
                "ada",
                "same-secret",
                None,
                CredentialSource::User,
            )
            .unwrap();
        assert!(!vault
            .should_offer_save("https://example.com/login", "ada", "same-secret")
            .unwrap());
        assert!(vault
            .should_offer_save("https://example.com/login", "ada", "changed-secret")
            .unwrap());
        vault.never_origin("https://example.com/login").unwrap();
        assert!(!vault
            .should_offer_save("https://example.com/login", "ada", "changed-secret")
            .unwrap());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pending_generated_secret_defers_duplicate_capture() {
        let dir = home("capture-defer");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        let mut settings = VaultSettings::default();
        settings.offer_save = true;
        vault.configure(settings).unwrap();
        let pending = vault
            .generate_password(
                "https://example.com/",
                "ada",
                None,
                None,
                CredentialSource::Agent,
            )
            .unwrap();
        let pending_id = pending
            .get("pendingId")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        let password = vault
            .state
            .lock()
            .unwrap()
            .payload
            .pending[0]
            .password
            .clone();
        assert!(!vault
            .save_captured(
                "https://example.com/",
                "ada",
                &password,
                None,
                CredentialSource::Agent,
                true,
            )
            .unwrap());
        assert!(vault
            .list_metadata("https://example.com/")
            .unwrap()
            .is_empty());
        vault
            .commit_generated("https://example.com/", &pending_id)
            .unwrap();
        assert_eq!(vault.list_metadata("https://example.com/").unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn save_prompt_response_updates_records_without_exposing_secret() {
        let dir = home("save-prompt");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        let mut settings = VaultSettings::default();
        settings.offer_save = true;
        vault.configure(settings).unwrap();
        let prompt = vault
            .offer_save("https://example.com/login", "ada", "new-secret")
            .unwrap();
        assert!(prompt.get("password").is_none());
        let id = prompt.get("id").and_then(Value::as_str).unwrap().to_string();
        let snapshot = vault.snapshot().unwrap();
        assert_eq!(arr_len(snapshot.get("pending")), 1);
        let after = vault.respond(&id, true).unwrap();
        assert_eq!(arr_len(after.get("pending")), 0);
        assert_eq!(arr_len(after.get("logins")), 1);
        assert!(vault.respond(&id, false).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn dismissing_save_prompt_does_not_save_login() {
        let dir = home("save-prompt-dismiss");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        let mut settings = VaultSettings::default();
        settings.offer_save = true;
        vault.configure(settings).unwrap();
        let prompt = vault
            .offer_save("https://example.com/login", "ada", "new-secret")
            .unwrap();
        let after = vault
            .respond(prompt.get("id").and_then(Value::as_str).unwrap(), false)
            .unwrap();
        assert!(after
            .get("logins")
            .and_then(Value::as_array)
            .unwrap()
            .is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn redaction_replaces_saved_secrets_recursively() {
        let dir = home("redaction");
        let vault = BrowserVault::open_with_keyring(&dir, false).unwrap();
        vault
            .save_login(
                "https://example.com/",
                "ada",
                "super-secret",
                None,
                CredentialSource::User,
            )
            .unwrap();
        let redacted = vault.redact(&json!({"outer": ["token super-secret end"]}));
        assert_eq!(
            redacted.pointer("/outer/0").and_then(Value::as_str),
            Some("token [redacted] end")
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
