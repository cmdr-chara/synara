//! Controller-side trusted remote hosts, port of upstream
//! persistence/Layers/RemoteHostTrust.ts. Trust rows are scoped by the
//! controller installation plus the exact account binding, so changing either
//! installation or account automatically isolates paired computers.

use rusqlite::{Connection, params};
use serde_json::Value;

use crate::orchestration_engine::open_state_rw;

#[derive(Clone)]
pub struct Binding {
    pub controller_environment_id: String,
    pub account_authority: String,
    pub user_id: String,
    pub organization_id: String,
}

#[derive(Clone)]
pub struct TrustedHost {
    pub environment_id: String,
    pub root_certificate: String,
    pub root_fingerprint: String,
    pub channel: String,
    pub host_id: String,
    pub label: String,
    pub paired_at: Option<String>,
}

impl TrustedHost {
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "environmentId": self.environment_id,
            "rootCertificate": self.root_certificate,
            "rootFingerprint": self.root_fingerprint,
            "channel": self.channel,
            "hostId": self.host_id,
            "label": self.label,
            "pairedAt": self.paired_at,
        })
    }
}

pub fn open() -> Result<Connection, String> {
    open_state_rw().map_err(|e| e.0)
}

fn row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TrustedHost> {
    Ok(TrustedHost {
        environment_id: row.get(0)?,
        root_certificate: row.get(1)?,
        root_fingerprint: row.get(2)?,
        channel: row.get(3)?,
        host_id: row.get(4)?,
        label: row.get(5)?,
        paired_at: row.get(6)?,
    })
}

fn select(
    conn: &Connection,
    binding: &Binding,
    extra_where: &str,
    extra_params: &[&dyn rusqlite::ToSql],
) -> Result<Vec<TrustedHost>, String> {
    let sql = format!(
        "SELECT environment_id, root_certificate, root_fingerprint, channel, host_id, label, paired_at FROM remote_host_trust WHERE controller_environment_id = ? AND account_authority = ? AND user_id = ? AND organization_id = ? AND revoked_at IS NULL {extra_where}"
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let mut values: Vec<&dyn rusqlite::ToSql> = vec![
        &binding.controller_environment_id,
        &binding.account_authority,
        &binding.user_id,
        &binding.organization_id,
    ];
    values.extend_from_slice(extra_params);
    let rows = stmt
        .query_map(values.as_slice(), row)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

pub fn list_paired(conn: &Connection, binding: &Binding) -> Result<Vec<TrustedHost>, String> {
    select(conn, binding, "AND paired_at IS NOT NULL", &[])
}

pub fn list_desired(conn: &Connection, binding: &Binding) -> Result<Vec<TrustedHost>, String> {
    select(
        conn,
        binding,
        "AND paired_at IS NOT NULL AND desired = 1",
        &[],
    )
}

pub fn get(
    conn: &Connection,
    binding: &Binding,
    environment_id: &str,
) -> Result<Option<TrustedHost>, String> {
    Ok(
        select(conn, binding, "AND environment_id = ?", &[&environment_id])?
            .into_iter()
            .next(),
    )
}

pub fn import_invitation(
    conn: &Connection,
    binding: &Binding,
    bundle: &Value,
) -> Result<(), String> {
    let environment_id = bundle
        .get("environmentId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let root_certificate = bundle
        .get("rootCertificate")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let root_fingerprint = bundle
        .get("rootFingerprint")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let channel = bundle
        .get("channel")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let host_id = bundle
        .get("hostId")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let label = bundle
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let expires_at = bundle
        .get("expiresAt")
        .and_then(Value::as_str)
        .unwrap_or_default();
    crate::remote_tls::validate_anchor(environment_id, root_certificate, root_fingerprint)?;
    if binding.account_authority
        != bundle
            .get("accountAuthority")
            .and_then(Value::as_str)
            .unwrap_or_default()
        || binding.user_id
            != bundle
                .get("userId")
                .and_then(Value::as_str)
                .unwrap_or_default()
        || binding.organization_id
            != bundle
                .get("organizationId")
                .and_then(Value::as_str)
                .unwrap_or_default()
    {
        return Err("Invitation belongs to another account".to_string());
    }
    if environment_id == binding.controller_environment_id {
        return Err("Cannot pair this installation with itself".to_string());
    }
    let expires = chrono::DateTime::parse_from_rfc3339(expires_at)
        .map_err(|_| "Invitation expired or invalid".to_string())?
        .timestamp_millis();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    if expires <= now || expires > now + 10 * 60_000 + 60_000 {
        return Err("Invitation expired or invalid".to_string());
    }
    if channel.is_empty() || host_id.is_empty() || label.len() > 200 {
        return Err("Invitation expired or invalid".to_string());
    }
    if let Some(existing) = get(conn, binding, environment_id)? {
        if existing.root_fingerprint != root_fingerprint || existing.channel != channel {
            return Err("Host identity changed. Forget the old trust explicitly and pair again on the host.".to_string());
        }
        return Ok(());
    }
    conn.execute(
        "INSERT INTO remote_host_trust (controller_environment_id, account_authority, user_id, organization_id, environment_id, channel, root_certificate, root_fingerprint, host_id, label) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![binding.controller_environment_id, binding.account_authority, binding.user_id, binding.organization_id, environment_id, channel, root_certificate, root_fingerprint, host_id, label],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn confirm(
    conn: &Connection,
    binding: &Binding,
    environment_id: &str,
    root_fingerprint: &str,
    now: &str,
) -> Result<bool, String> {
    let changed = conn.execute(
        "UPDATE remote_host_trust SET paired_at = ?1 WHERE controller_environment_id = ?2 AND account_authority = ?3 AND user_id = ?4 AND organization_id = ?5 AND environment_id = ?6 AND root_fingerprint = ?7 AND revoked_at IS NULL AND paired_at IS NULL",
        params![now, binding.controller_environment_id, binding.account_authority, binding.user_id, binding.organization_id, environment_id, root_fingerprint],
    ).map_err(|e| e.to_string())?;
    if changed == 1 {
        return Ok(true);
    }
    Ok(get(conn, binding, environment_id)?
        .is_some_and(|host| host.root_fingerprint == root_fingerprint && host.paired_at.is_some()))
}

pub fn forget(conn: &Connection, binding: &Binding, environment_id: &str) -> Result<(), String> {
    conn.execute(
        "DELETE FROM remote_host_trust WHERE controller_environment_id = ?1 AND account_authority = ?2 AND user_id = ?3 AND organization_id = ?4 AND environment_id = ?5",
        params![binding.controller_environment_id, binding.account_authority, binding.user_id, binding.organization_id, environment_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_desired(
    conn: &Connection,
    binding: &Binding,
    host_id: &str,
    desired: bool,
) -> Result<(), String> {
    conn.execute(
        "UPDATE remote_host_trust SET desired = ?1 WHERE controller_environment_id = ?2 AND account_authority = ?3 AND user_id = ?4 AND organization_id = ?5 AND host_id = ?6 AND paired_at IS NOT NULL AND revoked_at IS NULL",
        params![if desired { 1 } else { 0 }, binding.controller_environment_id, binding.account_authority, binding.user_id, binding.organization_id, host_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}
