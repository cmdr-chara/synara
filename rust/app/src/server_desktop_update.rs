//! `server.desktopUpdateStatus` / `server.desktopUpdate` — the desktop updater
//! lives in the packaged Electron app upstream and is reached through an IPC
//! bridge. The Rust app has no updater bridge, so updates are reported
//! unsupported; `busy` still reflects real work so callers see why a restart
//! would be unsafe once an updater exists.
//!
//! Busy is read from a byte copy of `state.sqlite` (upstream holds an
//! exclusive lock): running turns or unsettled pending interactions count.
//! A failed read fails closed — busy=true.

use rusqlite::Connection;
use serde_json::{Value, json};

use crate::todo::WsErr;

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

fn status() -> Value {
    json!({
        "supported": false,
        "state": Value::Null,
        "busy": busy(),
        "message": "Desktop updates require the packaged Synara Desktop app with its updater enabled."
    })
}

pub fn desktop_update_status() -> Result<Value, WsErr> {
    Ok(status())
}

pub fn desktop_update(payload: &Value) -> Result<Value, WsErr> {
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
    let current = status();
    // No updater bridge in this app — nothing is ever accepted.
    Ok(json!({ "accepted": false, "status": current }))
}
