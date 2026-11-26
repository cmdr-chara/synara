//! `server.desktopUpdateStatus` / `server.desktopUpdate` — the desktop updater
//! lives in the desktop shell upstream. The Rust shell owns the updater and
//! exposes a loopback, token-authenticated private bridge; this module forwards
//! status/actions there. Without a live shell bridge, updates are unsupported;
//! `busy` still reflects real work so callers see why a restart would be unsafe.
//!
//! Busy is read from a byte copy of `state.sqlite` (upstream holds an
//! exclusive lock): active sessions/turns, unanswered interactions, cache
//! review, and durable provider runtime work count. A failed read fails closed.

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

fn busy_from_connection(conn: &Connection) -> Result<bool, rusqlite::Error> {
    // Mirrors upstream hasDesktopUpdateThreadWork: active provider/session
    // intent, running latest turns, unanswered human interactions, and open
    // Claude cache review all make a restart unsafe. Deleted threads do not.
    let thread_work: i64 = conn.query_row(
        "SELECT COUNT(*)          FROM projection_threads t          LEFT JOIN projection_thread_sessions s ON s.thread_id = t.thread_id          WHERE t.deleted_at IS NULL AND (            s.status = 'starting'            OR s.active_turn_id IS NOT NULL            OR COALESCE(t.pending_approval_count, 0) > 0            OR COALESCE(t.pending_user_input_count, 0) > 0            OR EXISTS (              SELECT 1 FROM projection_turns lt              WHERE lt.thread_id = t.thread_id                AND lt.turn_id = t.latest_turn_id                AND lt.state = 'running'            )            OR EXISTS (              SELECT 1 FROM projection_pending_interactions pi              WHERE pi.thread_id = t.thread_id AND pi.status <> 'confirmed'            )            OR (              t.claude_cache_review_json IS NOT NULL              AND json_extract(t.claude_cache_review_json, '$.status') <> 'failed'            )          )",
        [],
        |r| r.get(0),
    )?;
    if thread_work > 0 { return Ok(true); }

    // Provider background work can outlive a foreground turn. The durable
    // runtime projection is the Rust backend's available evidence for it.
    let provider_work: i64 = conn.query_row(
        "SELECT COUNT(*) FROM provider_session_runtime          WHERE status IN ('running', 'connecting')",
        [],
        |r| r.get(0),
    )?;
    if provider_work > 0 { return Ok(true); }

    // Upstream also blocks while recoverable automation work exists. Deferred
    // pending rows are future schedules, not work the restart would interrupt.
    let automation_work: i64 = conn.query_row(
        "SELECT COUNT(*) FROM automation_runs          WHERE status IN ('pending', 'claimed', 'running', 'waiting-for-approval')            AND NOT (status = 'pending' AND deferred_until IS NOT NULL)",
        [],
        |r| r.get(0),
    )?;
    if automation_work > 0 { return Ok(true); }

    // Durable queued-turn promotions are accepted user work waiting for the
    // active turn boundary; restarting before dispatch loses their in-flight ordering.
    let queued_work: i64 = conn.query_row(
        "SELECT COUNT(*) FROM queued_turn_promotions WHERE state IN ('queued', 'promoting')",
        [],
        |r| r.get(0),
    )?;
    Ok(queued_work > 0)
}

fn busy() -> bool {
    if crate::projects_devserver::has_active_dev_servers() {
        return true;
    }
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
        busy_from_connection(&conn).ok()
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
    fn busy_fixture() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE projection_threads (               thread_id TEXT PRIMARY KEY, latest_turn_id TEXT, deleted_at TEXT,                pending_approval_count INTEGER DEFAULT 0, pending_user_input_count INTEGER DEFAULT 0,                claude_cache_review_json TEXT             );             CREATE TABLE projection_thread_sessions (thread_id TEXT PRIMARY KEY, status TEXT, active_turn_id TEXT);             CREATE TABLE projection_turns (thread_id TEXT, turn_id TEXT, state TEXT);             CREATE TABLE projection_pending_interactions (thread_id TEXT, status TEXT);             CREATE TABLE provider_session_runtime (thread_id TEXT PRIMARY KEY, status TEXT);             CREATE TABLE automation_runs (run_id TEXT PRIMARY KEY, status TEXT, deferred_until TEXT);             CREATE TABLE queued_turn_promotions (queued_event_sequence INTEGER PRIMARY KEY, state TEXT);",
        ).unwrap();
        conn.execute(
            "INSERT INTO projection_threads (thread_id, latest_turn_id) VALUES ('thread-a', 'turn-a')",
            [],
        ).unwrap();
        conn
    }

    #[test]
    fn updater_busy_projection_matches_active_work_shapes() {
        let conn = busy_fixture();
        assert!(!busy_from_connection(&conn).unwrap());

        conn.execute("INSERT INTO projection_thread_sessions (thread_id, status) VALUES ('thread-a', 'starting')", []).unwrap();
        assert!(busy_from_connection(&conn).unwrap());
        conn.execute("UPDATE projection_thread_sessions SET status = 'stopped'", []).unwrap();
        assert!(!busy_from_connection(&conn).unwrap());

        conn.execute("INSERT INTO projection_turns (thread_id, turn_id, state) VALUES ('thread-a', 'turn-a', 'running')", []).unwrap();
        assert!(busy_from_connection(&conn).unwrap());
        conn.execute("UPDATE projection_turns SET state = 'completed'", []).unwrap();
        assert!(!busy_from_connection(&conn).unwrap());

        conn.execute("INSERT INTO projection_pending_interactions (thread_id, status) VALUES ('thread-a', 'pending')", []).unwrap();
        assert!(busy_from_connection(&conn).unwrap());
        conn.execute("UPDATE projection_pending_interactions SET status = 'confirmed'", []).unwrap();
        assert!(!busy_from_connection(&conn).unwrap());

        conn.execute("UPDATE projection_threads SET pending_user_input_count = 1", []).unwrap();
        assert!(busy_from_connection(&conn).unwrap());
        conn.execute("UPDATE projection_threads SET pending_user_input_count = 0, claude_cache_review_json = '{\"status\":\"pending\"}'", []).unwrap();
        assert!(busy_from_connection(&conn).unwrap());
        conn.execute("UPDATE projection_threads SET claude_cache_review_json = '{\"status\":\"failed\"}'", []).unwrap();
        assert!(!busy_from_connection(&conn).unwrap());

        conn.execute("INSERT INTO provider_session_runtime (thread_id, status) VALUES ('thread-a', 'running')", []).unwrap();
        assert!(busy_from_connection(&conn).unwrap());

        conn.execute("UPDATE provider_session_runtime SET status = 'stopped'", []).unwrap();
        conn.execute("INSERT INTO automation_runs (run_id, status) VALUES ('run-a', 'running')", []).unwrap();
        assert!(busy_from_connection(&conn).unwrap());
        conn.execute("UPDATE automation_runs SET status = 'pending', deferred_until = 'later'", []).unwrap();
        assert!(!busy_from_connection(&conn).unwrap());
        conn.execute("UPDATE automation_runs SET deferred_until = NULL", []).unwrap();
        assert!(busy_from_connection(&conn).unwrap());
        conn.execute("UPDATE automation_runs SET status = 'succeeded'", []).unwrap();

        conn.execute("INSERT INTO queued_turn_promotions (queued_event_sequence, state) VALUES (1, 'queued')", []).unwrap();
        assert!(busy_from_connection(&conn).unwrap());
        conn.execute("UPDATE queued_turn_promotions SET state = 'promoted'", []).unwrap();
        assert!(!busy_from_connection(&conn).unwrap());

        conn.execute("UPDATE projection_threads SET deleted_at = 'now'", []).unwrap();
        conn.execute("UPDATE provider_session_runtime SET status = 'stopped'", []).unwrap();
        assert!(!busy_from_connection(&conn).unwrap());
    }

}
