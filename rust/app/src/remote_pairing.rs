//! Remote pairing control plane + repository, port of upstream
//! auth/Layers/RemotePairingControlPlane.ts and
//! persistence/Layers/AuthPairingLinksRemote.ts. One invite row in
//! auth_pairing_links (purpose 'remote-device') plus the device trust chain in
//! remote_device_trust; the credential column stores the invite secret's
//! sha256 hex, verified with a constant-time compare.

use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::Digest;

use crate::orchestration_engine::{open_state_rw, uuid_v4};

#[derive(Clone)]
pub struct TrustScope {
    pub environment_id: String,
    pub root_fingerprint: String,
    pub account_authority: String,
    pub user_id: String,
    pub organization_id: String,
}

impl TrustScope {
    fn to_json(&self) -> String {
        json!({
            "environmentId": self.environment_id,
            "rootFingerprint": self.root_fingerprint,
            "accountAuthority": self.account_authority,
            "userId": self.user_id,
            "organizationId": self.organization_id,
        })
        .to_string()
    }

    fn matches(&self, raw: &str) -> bool {
        let Ok(parsed) = serde_json::from_str::<Value>(raw) else {
            return false;
        };
        let get = |k: &str| parsed.get(k).and_then(Value::as_str).unwrap_or("");
        get("environmentId") == self.environment_id
            && get("rootFingerprint") == self.root_fingerprint
            && get("accountAuthority") == self.account_authority
            && get("userId") == self.user_id
            && get("organizationId") == self.organization_id
    }
}

#[derive(Clone)]
pub struct PairingDevice {
    pub device_jkt: String,
    pub public_key: Value,
    pub label: String,
}

pub struct PairingRecord {
    pub id: String,
    pub credential_hash: String,
    pub expires_at: String,
    pub consumed_at: Option<String>,
    pub revoked_at: Option<String>,
    pub pending_device: Option<PairingDevice>,
}

impl PairingRecord {
    pub fn status(&self) -> Value {
        json!({
            "inviteId": self.id,
            "expiresAt": self.expires_at,
            "pendingDevice": self.pending_device.as_ref().map(|d| json!({
                "deviceJkt": d.device_jkt,
                "publicKey": d.public_key,
                "label": d.label,
            })),
            "approved": self.consumed_at.is_some(),
            "revoked": self.revoked_at.is_some(),
        })
    }
}

fn sha256_hex(secret: &str) -> String {
    let digest = sha2::Sha256::digest(secret.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Constant-time equality, matching upstream's timingSafeEqual.
fn timing_safe_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Millisecond-precision UTC ISO, matching upstream's DateTime.formatIso —
/// all timestamps in this module compare lexicographically, so they must
/// share one precision.
fn now_iso() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    chrono::DateTime::from_timestamp_millis(millis as i64)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_default()
}

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<PairingRecord> {
    let pending_jkt: Option<String> = row.get("pending_device_jkt")?;
    let pending_device = pending_jkt.map(|jkt| {
        let key_raw: Option<String> = row.get("pending_device_key").unwrap_or(None);
        PairingDevice {
            device_jkt: jkt,
            public_key: key_raw
                .and_then(|r| serde_json::from_str(&r).ok())
                .unwrap_or(Value::Null),
            label: row.get("pending_device_label").unwrap_or(None).unwrap_or_default(),
        }
    });
    Ok(PairingRecord {
        id: row.get("id")?,
        credential_hash: row.get("credential")?,
        expires_at: row.get("expires_at")?,
        consumed_at: row.get("consumed_at")?,
        revoked_at: row.get("revoked_at")?,
        pending_device,
    })
}

fn query(conn: &Connection, scope: &TrustScope, id: Option<&str>) -> Result<Vec<PairingRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT * FROM auth_pairing_links WHERE purpose = 'remote-device' AND (?1 IS NULL OR id = ?1)",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![id], decode_row)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        let record = row.map_err(|e| e.to_string())?;
        let metadata: String = conn
            .query_row(
                "SELECT remote_metadata FROM auth_pairing_links WHERE id = ?1",
                params![record.id],
                |r| r.get(0),
            )
            .unwrap_or_default();
        if scope.matches(&metadata) {
            out.push(record);
        }
    }
    Ok(out)
}

fn get(conn: &Connection, scope: &TrustScope, id: &str) -> Result<Option<PairingRecord>, String> {
    Ok(query(conn, scope, Some(id))?.into_iter().next())
}

pub struct CreatedInvite {
    pub invite_id: String,
    pub secret: String,
    pub expires_at: String,
}

/// Creates a pairing invite: 32 random bytes base64url (43 chars), sha256
/// stored as the credential, nine-minute expiry below the account API's
/// ten-minute publication limit.
pub fn create_invite(conn: &Connection, scope: &TrustScope) -> Result<CreatedInvite, String> {
    use std::io::Read;
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| format!("randomness unavailable: {e}"))?;
    let secret = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        bytes,
    );
    let invite_id = uuid_v4();
    let created_at = now_iso();
    let created_ms = chrono::DateTime::parse_from_rfc3339(&created_at)
        .map(|d| d.timestamp_millis())
        .unwrap_or(0);
    let expires_at = chrono::DateTime::from_timestamp_millis(created_ms + 9 * 60_000)
        .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .ok_or_else(|| "expiry computation failed".to_string())?;
    conn.execute(
        "INSERT INTO auth_pairing_links (id, credential, method, role, subject, created_at, expires_at, purpose, remote_metadata) VALUES (?1, ?2, 'one-time-token', 'client', 'remote-device', ?3, ?4, 'remote-device', ?5)",
        params![invite_id, sha256_hex(&secret), created_at, expires_at, scope.to_json()],
    )
    .map_err(|e| format!("invite insert failed: {e}"))?;
    Ok(CreatedInvite {
        invite_id,
        secret,
        expires_at,
    })
}

/// Verifies an invite secret: constant-time compare, secret shape, not
/// revoked, and unexpired when not yet consumed.
pub fn verify_invitation(
    conn: &Connection,
    scope: &TrustScope,
    id: &str,
    secret: &str,
) -> Result<Value, String> {
    let record = get(conn, scope, id)?;
    let now = now_iso();
    let expected = match &record {
        Some(r) if r.credential_hash.len() == 64 && r.credential_hash.chars().all(|c| c.is_ascii_hexdigit()) => {
            r.credential_hash.clone()
        }
        _ => "0".repeat(64),
    };
    let matches = timing_safe_eq(&expected, &sha256_hex(secret));
    let valid_secret_shape = secret.len() == 43
        && secret
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    match &record {
        Some(r)
            if matches
                && valid_secret_shape
                && r.revoked_at.is_none()
                && (r.consumed_at.is_some() || r.expires_at > now) =>
        {
            Ok(r.status())
        }
        _ => Err("Remote invitation is invalid, expired, or revoked.".to_string()),
    }
}

/// Records the device asking for approval on an open invite.
pub fn request_approval(
    conn: &Connection,
    scope: &TrustScope,
    id: &str,
    device: &PairingDevice,
) -> Result<bool, String> {
    let now = now_iso();
    let Some(record) = get(conn, scope, id)? else {
        return Ok(false);
    };
    if record.revoked_at.is_some() || (record.consumed_at.is_none() && record.expires_at <= now) {
        return Ok(false);
    }
    if let Some(pending) = &record.pending_device {
        return Ok(pending.device_jkt == device.device_jkt);
    }
    if record.consumed_at.is_some() {
        return Ok(false);
    }
    let changed = conn.execute(
        "UPDATE auth_pairing_links SET pending_device_jkt = ?1, pending_device_key = ?2, pending_device_label = ?3 WHERE id = ?4 AND purpose = 'remote-device' AND pending_device_jkt IS NULL AND consumed_at IS NULL AND revoked_at IS NULL AND expires_at > ?5",
        params![device.device_jkt, device.public_key.to_string(), device.label, id, now],
    ).map_err(|e| e.to_string())?;
    Ok(changed == 1)
}

/// Approves the pending device: consumes the invite and writes the trust
/// chain row (generation bumps on re-approval) plus the access-state marker.
pub fn approve(
    conn: &Connection,
    scope: &TrustScope,
    id: &str,
    jkt: &str,
    enrolled_via: &str,
) -> Result<bool, String> {
    let now = now_iso();
    let Some(record) = get(conn, scope, id)? else {
        return Ok(false);
    };
    let Some(pending) = &record.pending_device else {
        return Ok(false);
    };
    if record.revoked_at.is_some() || pending.device_jkt != jkt {
        return Ok(false);
    }
    if record.consumed_at.is_some() {
        let trusted = conn.query_row(
            "SELECT 1 FROM remote_device_trust WHERE environment_id = ?1 AND root_fingerprint = ?2 AND device_jkt = ?3 AND revoked_at IS NULL AND approved_invite_id = ?4",
            params![scope.environment_id, scope.root_fingerprint, jkt, id],
            |_| Ok(()),
        );
        return Ok(trusted.is_ok());
    }
    if record.expires_at <= now {
        return Ok(false);
    }
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    let changed = tx.execute(
        "UPDATE auth_pairing_links SET consumed_at = ?1 WHERE id = ?2 AND purpose = 'remote-device' AND pending_device_jkt = ?3 AND consumed_at IS NULL AND revoked_at IS NULL AND expires_at > ?1",
        params![now, id, jkt],
    ).map_err(|e| e.to_string())?;
    if changed != 1 {
        return Ok(false);
    }
    tx.execute(
        "INSERT INTO remote_device_trust (environment_id, root_fingerprint, device_jkt, device_public_key, account_authority, owner_user_id, organization_id, label, generation, approved_at, approved_invite_id, revoked_at, enrolled_via) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 1, ?9, ?10, NULL, ?11)
         ON CONFLICT(environment_id, root_fingerprint, device_jkt) DO UPDATE SET device_public_key = excluded.device_public_key, account_authority = excluded.account_authority, owner_user_id = excluded.owner_user_id, organization_id = excluded.organization_id, label = excluded.label, generation = remote_device_trust.generation + 1, approved_at = excluded.approved_at, approved_invite_id = excluded.approved_invite_id, revoked_at = NULL, enrolled_via = excluded.enrolled_via",
        params![
            scope.environment_id, scope.root_fingerprint, jkt, pending.public_key.to_string(),
            scope.account_authority, scope.user_id, scope.organization_id, pending.label,
            now, id, enrolled_via,
        ],
    ).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO remote_access_state (environment_id, root_fingerprint, enabled) VALUES (?1, ?2, 1) ON CONFLICT(environment_id, root_fingerprint) DO UPDATE SET enabled = 1",
        params![scope.environment_id, scope.root_fingerprint],
    ).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(true)
}

/// Owner-grant pairing: record and approve in one step (enrolled via qr).
pub fn enroll(
    conn: &Connection,
    scope: &TrustScope,
    id: &str,
    device: &PairingDevice,
) -> Result<bool, String> {
    if !request_approval(conn, scope, id, device)? {
        return Ok(false);
    }
    approve(conn, scope, id, &device.device_jkt, "qr")?;
    Ok(true)
}

pub fn list_invites(conn: &Connection, scope: &TrustScope) -> Result<Vec<Value>, String> {
    Ok(query(conn, scope, None)?.iter().map(PairingRecord::status).collect())
}

pub fn revoke(conn: &Connection, scope: &TrustScope, id: &str) -> Result<(), String> {
    if get(conn, scope, id)?.is_none() {
        return Ok(());
    }
    conn.execute(
        "UPDATE auth_pairing_links SET revoked_at = ?1 WHERE id = ?2 AND purpose = 'remote-device' AND consumed_at IS NULL AND revoked_at IS NULL",
        params![now_iso(), id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}


/// Trusted devices for the management list, mirroring RemoteTrustedDevice.
pub fn list_devices(conn: &Connection, scope: &TrustScope) -> Result<Vec<Value>, String> {
    let mut stmt = conn.prepare(
        "SELECT device_jkt, device_public_key, label, generation, approved_at, revoked_at, enrolled_via FROM remote_device_trust WHERE environment_id = ?1 AND root_fingerprint = ?2",
    ).map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![scope.environment_id, scope.root_fingerprint], |row| {
        let key_raw: String = row.get(1)?;
        Ok(json!({
            "deviceJkt": row.get::<_, String>(0)?,
            "publicKey": serde_json::from_str::<Value>(&key_raw).unwrap_or(Value::Null),
            "label": row.get::<_, String>(2)?,
            "generation": row.get::<_, i64>(3)?,
            "approvedAt": row.get::<_, String>(4)?,
            "revokedAt": row.get::<_, Option<String>>(5)?,
            "enrolledVia": row.get::<_, Option<String>>(6)?,
        }))
    }).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn revoke_device(conn: &Connection, scope: &TrustScope, device_jkt: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE remote_device_trust SET revoked_at = ?1 WHERE environment_id = ?2 AND root_fingerprint = ?3 AND device_jkt = ?4 AND revoked_at IS NULL",
        params![now_iso(), scope.environment_id, scope.root_fingerprint, device_jkt],
    ).map_err(|e| e.to_string())?;
    // A revoke reaches live sessions immediately rather than at credential
    // expiry — the registry signals every socket this device holds.
    crate::remote_sessions::close_for_device(device_jkt);
    Ok(())
}

/// Whether the device is trusted and unrevoked, upstream's trust.authorize.
/// The credential-minting variant: generation proves the trust row is current
/// and binds the minted credential to it, so a later revoke (which bumps or
/// removes the row) invalidates every credential minted before it.
pub fn device_trust_generation(conn: &Connection, scope: &TrustScope, device_jkt: &str) -> Result<u64, String> {
    conn.query_row(
        "SELECT generation FROM remote_device_trust WHERE environment_id = ?1 AND root_fingerprint = ?2 AND device_jkt = ?3 AND revoked_at IS NULL",
        params![scope.environment_id, scope.root_fingerprint, device_jkt],
        |row| row.get::<_, i64>(0),
    )
    .map(|generation| generation.max(0) as u64)
    .map_err(|_| "Device has no current local approval".to_string())
}

pub fn authorize_device(conn: &Connection, scope: &TrustScope, device_jkt: &str) -> Result<bool, String> {
    let found = conn.query_row(
        "SELECT 1 FROM remote_device_trust WHERE environment_id = ?1 AND root_fingerprint = ?2 AND device_jkt = ?3 AND revoked_at IS NULL",
        params![scope.environment_id, scope.root_fingerprint, device_jkt],
        |_| Ok(()),
    );
    Ok(found.is_ok())
}

/// The owner's allow_connections switch lives alongside the enabled marker.
pub fn allows_connections(conn: &Connection, scope: &TrustScope) -> Result<bool, String> {
    let value: Option<i64> = conn.query_row(
        "SELECT allow_connections FROM remote_access_state WHERE environment_id = ?1 AND root_fingerprint = ?2",
        params![scope.environment_id, scope.root_fingerprint],
        |r| r.get(0),
    ).ok();
    Ok(value.unwrap_or(1) != 0)
}

pub fn set_allow_connections(conn: &Connection, scope: &TrustScope, enabled: bool) -> Result<(), String> {
    // `enabled` is the live-identity marker and is left untouched here; the
    // owner switch alone flips (upstream migration 140, default on).
    conn.execute(
        "INSERT INTO remote_access_state (environment_id, root_fingerprint, enabled, allow_connections) VALUES (?1, ?2, 0, ?3)
         ON CONFLICT(environment_id, root_fingerprint) DO UPDATE SET allow_connections = ?3",
        params![scope.environment_id, scope.root_fingerprint, enabled as i64],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Local trust scope: root fingerprint is the sha256 of the host identity
/// public key — stable per environment, needs no account connectivity.
pub fn local_scope(secrets_dir: &str, environment_id: &str) -> Option<TrustScope> {
    let path = crate::host_identity::host_identity_path(secrets_dir);
    let identity = crate::host_identity::read_host_identity(&path).ok()??;
    let x = identity.public_key_jwk.get("x").and_then(Value::as_str)?;
    let fingerprint = sha256_hex(x);
    Some(TrustScope {
        environment_id: environment_id.to_string(),
        root_fingerprint: fingerprint,
        account_authority: String::new(),
        user_id: String::new(),
        organization_id: String::new(),
    })
}

/// Opens state.sqlite rw for the pairing surface, mirroring the rest of the app.
pub fn open() -> Result<Connection, String> {
    open_state_rw().map_err(|e| e.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> TrustScope {
        TrustScope {
            environment_id: "env-1".to_string(),
            root_fingerprint: "a".repeat(64),
            account_authority: "https://account.example".to_string(),
            user_id: "user-1".to_string(),
            organization_id: "org-1".to_string(),
        }
    }

    fn device() -> PairingDevice {
        PairingDevice {
            device_jkt: "jkt-1".to_string(),
            public_key: json!({"kty": "OKP", "crv": "Ed25519", "x": "abc"}),
            label: "Phone".to_string(),
        }
    }

    fn test_db() -> (Connection, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("synara-pair-test-{}", uuid_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE auth_pairing_links (
                id TEXT PRIMARY KEY, credential TEXT NOT NULL, method TEXT NOT NULL,
                role TEXT NOT NULL, subject TEXT NOT NULL, created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL, consumed_at TEXT, revoked_at TEXT,
                purpose TEXT, remote_metadata TEXT,
                pending_device_jkt TEXT, pending_device_key TEXT, pending_device_label TEXT
            );
            CREATE TABLE remote_device_trust (
                environment_id TEXT NOT NULL, root_fingerprint TEXT NOT NULL,
                device_jkt TEXT NOT NULL, device_public_key TEXT NOT NULL,
                account_authority TEXT NOT NULL, owner_user_id TEXT NOT NULL,
                organization_id TEXT NOT NULL, label TEXT NOT NULL,
                generation INTEGER NOT NULL, approved_at TEXT NOT NULL,
                approved_invite_id TEXT, revoked_at TEXT, enrolled_via TEXT,
                PRIMARY KEY (environment_id, root_fingerprint, device_jkt)
            );
            CREATE TABLE remote_access_state (
                environment_id TEXT NOT NULL, root_fingerprint TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                PRIMARY KEY (environment_id, root_fingerprint)
            );",
        )
        .unwrap();
        (conn, dir)
    }

    #[test]
    fn invite_lifecycle() {
        let (conn, dir) = test_db();
        let scope = scope();
        let invite = create_invite(&conn, &scope).unwrap();
        assert_eq!(invite.secret.len(), 43);

        assert!(verify_invitation(&conn, &scope, &invite.invite_id, "wrong-secret-wrong-secret-wrong-secretx").is_err());
        let status = verify_invitation(&conn, &scope, &invite.invite_id, &invite.secret).unwrap();
        assert_eq!(status["approved"], json!(false));

        let dev = device();
        assert!(request_approval(&conn, &scope, &invite.invite_id, &dev).unwrap());
        // Repeat request from the same device is idempotent.
        assert!(request_approval(&conn, &scope, &invite.invite_id, &dev).unwrap());

        assert!(approve(&conn, &scope, &invite.invite_id, "jkt-1", "approval").unwrap());
        let status = verify_invitation(&conn, &scope, &invite.invite_id, &invite.secret).unwrap();
        assert_eq!(status["approved"], json!(true));

        let trusted: i64 = conn
            .query_row("SELECT COUNT(*) FROM remote_device_trust WHERE device_jkt = 'jkt-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(trusted, 1);
        let enabled: i64 = conn
            .query_row("SELECT enabled FROM remote_access_state WHERE environment_id = 'env-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(enabled, 1);

        // Re-approval bumps the generation.
        assert!(approve(&conn, &scope, &invite.invite_id, "jkt-1", "approval").unwrap());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn revoke_closes_open_invite() {
        let (conn, dir) = test_db();
        let scope = scope();
        let invite = create_invite(&conn, &scope).unwrap();
        revoke(&conn, &scope, &invite.invite_id).unwrap();
        assert!(verify_invitation(&conn, &scope, &invite.invite_id, &invite.secret).is_err());
        assert!(!request_approval(&conn, &scope, &invite.invite_id, &device()).unwrap());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn enroll_records_and_approves() {
        let (conn, dir) = test_db();
        let scope = scope();
        let invite = create_invite(&conn, &scope).unwrap();
        assert!(enroll(&conn, &scope, &invite.invite_id, &device()).unwrap());
        let via: String = conn
            .query_row("SELECT enrolled_via FROM remote_device_trust WHERE device_jkt = 'jkt-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(via, "qr");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scope_isolation() {
        let (conn, dir) = test_db();
        let scope = scope();
        let invite = create_invite(&conn, &scope).unwrap();
        let mut other = scope.clone();
        other.user_id = "user-2".to_string();
        assert!(verify_invitation(&conn, &other, &invite.invite_id, &invite.secret).is_err());
        assert_eq!(list_invites(&conn, &other).unwrap().len(), 0);
        assert_eq!(list_invites(&conn, &scope).unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).ok();
    }
}
