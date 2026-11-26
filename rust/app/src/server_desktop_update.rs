//! `server.desktopUpdateStatus` / `server.desktopUpdate` — the desktop updater
//! lives in the desktop shell upstream. The Rust shell owns the updater and
//! exposes a loopback, token-authenticated private bridge; this module forwards
//! status/actions there. Without a live shell bridge, updates are unsupported;
//! `busy` still reflects real work so callers see why a restart would be unsafe.
//!
//! Busy is read from a byte copy of `state.sqlite` (upstream holds an
//! exclusive lock): running turns or unsettled pending interactions count.
//! A failed read fails closed — busy=true.

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::todo::WsErr;

fn approved_remote_sessions() -> &'static std::sync::Mutex<std::collections::HashMap<String, String>> {
    static SESSIONS: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, String>>> = std::sync::OnceLock::new();
    SESSIONS.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

/// Mirrors upstream `authorizeRemoteDesktopUpdateSession`: only the verified
/// remote gateway receives updater authority; ordinary client sessions do not.
pub(crate) fn authorize_remote_session(session_id: &str) -> String {
    let authority = format!("remote:{}", crate::orchestration_engine::uuid_v4());
    if let Ok(mut sessions) = approved_remote_sessions().lock() {
        sessions.insert(session_id.to_string(), authority.clone());
    }
    authority
}

pub(crate) fn revoke_remote_session(session_id: &str) {
    if let Ok(mut sessions) = approved_remote_sessions().lock() {
        sessions.remove(session_id);
    }
}

fn valid_authority(authority: Option<&str>) -> Result<String, WsErr> {
    let Some(authority) = authority.filter(|value| !value.trim().is_empty()) else {
        return Err(WsErr(
            "Desktop updates require an authenticated owner or approved remote device.".to_string(),
            None,
        ));
    };
    let approved_remote = approved_remote_sessions()
        .lock()
        .ok()
        .is_some_and(|sessions| sessions.values().any(|value| value == authority));
    if authority.starts_with("owner:") || approved_remote {
        Ok(authority.to_string())
    } else {
        Err(WsErr(
            "Desktop updates require an authenticated owner or approved remote device.".to_string(),
            None,
        ))
    }
}

fn busy() -> bool {
    let db = std::path::PathBuf::from(crate::todo::state_dir()).join("state.sqlite");
    let tmp = std::env::temp_dir().join(format!(
        "synara-desktop-update-{}.sqlite",
        std::process::id()
    ));
    let tmp_wal = std::path::PathBuf::from(format!("{}-wal", tmp.display()));
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp_wal);
    let result = (|| -> Option<bool> {
        std::fs::copy(&db, &tmp).ok()?;
        let wal = db.with_file_name("state.sqlite-wal");
        if wal.exists() {
            let _ = std::fs::copy(&wal, &tmp_wal);
        }
        let conn = Connection::open(&tmp).ok()?;
        let running_turns: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM projection_turns WHERE state = 'running'",
                [],
                |r| r.get(0),
            )
            .ok()?;
        if running_turns > 0 {
            return Some(true);
        }
        let pending: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM projection_pending_interactions \
                 WHERE status NOT IN ('resolved', 'failed', 'cancelled', 'dismissed', 'completed')",
                [],
                |r| r.get(0),
            )
            .ok()?;
        Some(pending > 0)
    })();
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp_wal);
    // Upstream fails closed: a busy read that errors is treated as busy.
    result.unwrap_or(true)
}

fn unsupported_status() -> Value {
    json!({
        "supported": false,
        "state": Value::Null,
        "busy": busy(),
        "message": "Desktop updates require the packaged Synara Desktop app with its updater enabled."
    })
}

fn bridge_file() -> std::path::PathBuf {
    std::path::PathBuf::from(crate::todo::state_dir()).join("desktop-update-bridge.json")
}

fn bridge_request(action: &str, payload: &Value, authority: &str) -> Option<Value> {
    use std::io::{BufRead as _, BufReader, Write as _};
    let descriptor: Value = serde_json::from_str(&std::fs::read_to_string(bridge_file()).ok()?).ok()?;
    let port = descriptor.get("port").and_then(Value::as_u64)?;
    let token = descriptor.get("token").and_then(Value::as_str)?.to_string();
    if descriptor.get("version").and_then(Value::as_u64) != Some(1) || port == 0 || port > u16::MAX as u64 {
        return None;
    }
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port as u16)).ok()?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(12)))
        .ok()?;
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(12)))
        .ok()?;
    let mut request = json!({
        "token": token,
        "authority": authority,
        "action": action,
    });
    if let Some(expected) = payload.get("expectedVersion").and_then(Value::as_str) {
        request["expectedVersion"] = json!(expected);
    }
    if let Some(restart_token) = payload.get("restartToken").and_then(Value::as_str) {
        request["restartToken"] = json!(restart_token);
    }
    stream
        .write_all(format!("{}\n", serde_json::to_string(&request).ok()?).as_bytes())
        .ok()?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).ok()?;
    if line.is_empty() || line.len() > 64 * 1024 {
        return None;
    }
    serde_json::from_str(&line).ok()
}

fn merged_status(status: Value) -> Value {
    let mut out = status;
    let local_busy = busy();
    let bridge_busy = out.get("busy").and_then(Value::as_bool).unwrap_or(false);
    out["busy"] = json!(local_busy || bridge_busy);
    if local_busy && out.get("message").is_none_or(Value::is_null) {
        out["message"] = json!("Finish running chats and tasks before restarting Synara.");
    }
    out
}

fn status(authority: &str) -> Value {
    match bridge_request("status", &json!({}), authority)
        .and_then(|response| response.get("status").cloned())
    {
        Some(status) => merged_status(status),
        None => unsupported_status(),
    }
}

pub fn desktop_update_status(authority: Option<&str>) -> Result<Value, WsErr> {
    let authority = valid_authority(authority)?;
    Ok(status(&authority))
}

pub fn desktop_update(payload: &Value, authority: Option<&str>) -> Result<Value, WsErr> {
    let authority = valid_authority(authority)?;
    let action = payload
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if action == "install"
        && (payload.get("confirmRestart") != Some(&Value::Bool(true))
            || payload
                .get("expectedVersion")
                .and_then(Value::as_str)
                .is_none_or(|s| s.trim().is_empty()))
    {
        return Err(WsErr(
            "Confirm the downloaded update version and the connection interruption before restarting."
                .to_string(),
            Some("VALIDATION"),
        ));
    }
    let current = status(&authority);
    if !current.get("supported").and_then(Value::as_bool).unwrap_or(false)
        || (action == "install" && current.get("busy").and_then(Value::as_bool).unwrap_or(false))
    {
        return Ok(json!({ "accepted": false, "status": current }));
    }
    // The status round trip can overlap a turn start; upstream re-reads busy
    // immediately before forwarding an install commit.
    if action == "install" && busy() {
        return Ok(json!({
            "accepted": false,
            "status": json!({
                "supported": current.get("supported").cloned().unwrap_or(Value::Bool(false)),
                "state": current.get("state").cloned().unwrap_or(Value::Null),
                "busy": true,
                "message": "Finish running chats and tasks before restarting Synara."
            })
        }));
    }
    if let Some(result) = bridge_request(action, payload, &authority) {
        if let Some(bridge_status) = result.get("status").cloned() {
            let mut out = result.clone();
            out["status"] = merged_status(bridge_status);
            return Ok(out);
        }
    }
    Ok(json!({
        "accepted": false,
        "status": json!({
            "supported": false,
            "state": Value::Null,
            "busy": busy(),
            "message": "Desktop updater is unavailable. Reconnect and refresh its status."
        })
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updater_authority_admits_only_owner_or_active_remote() {
        assert!(valid_authority(None).is_err());
        assert!(valid_authority(Some("client:session")).is_err());
        assert!(valid_authority(Some("owner:session-a")).is_ok());

        let session_id = format!("remote-test-{}", crate::orchestration_engine::uuid_v4());
        let authority = authorize_remote_session(&session_id);
        assert_eq!(valid_authority(Some(&authority)).ok(), Some(authority.clone()));
        revoke_remote_session(&session_id);
        assert!(valid_authority(Some(&authority)).is_err());
    }
}
