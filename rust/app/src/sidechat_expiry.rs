//! Port of upstream `SidechatExpiryReactor`: side chats expire after a period
//! of inactivity (setting `sidechatExpiry`: 1h default, 24h, never). A poll
//! loop stands in for the upstream timer state machine; activity records and
//! running turns both hold off expiry. Viewers register through
//! `thread.sidechat.activity.record` heartbeats, same as upstream.

use serde_json::{json, Map, Value};

const POLL_INTERVAL_SECS: u64 = 60;

fn expiry_window_ms() -> Option<i64> {
    let settings = crate::server_settings::get_settings(&Value::Null).ok()?;
    match settings.get("sidechatExpiry").and_then(Value::as_str).unwrap_or("1h") {
        "1h" => Some(3_600_000),
        "24h" => Some(86_400_000),
        _ => None, // "never"
    }
}

fn iso_to_ms(s: &str) -> Option<i64> {
    crate::orchestration_engine::iso_ms(s)
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn spawn_expiry_watcher() {
    std::thread::spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_secs(POLL_INTERVAL_SECS));
        let _ = std::panic::catch_unwind(|| {
            if let Err(e) = poll_once() {
                eprintln!("sidechat_expiry: poll failed: {e}");
            }
        });
    });
}

fn poll_once() -> Result<(), String> {
    let Some(window_ms) = expiry_window_ms() else {
        return Ok(());
    };
    let conn = crate::orchestration_engine::open_state_rw().map_err(|e| e.0)?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            "SELECT thread_id, sidechat_last_activity_at, created_at, \
             COALESCE((SELECT status FROM projection_thread_sessions \
               WHERE thread_id = projection_threads.thread_id), ''), \
             (SELECT COUNT(*) FROM projection_turns \
               WHERE thread_id = projection_threads.thread_id AND state IN ('running','starting')) \
             FROM projection_threads \
             WHERE (sidechat_source_thread_id IS NOT NULL OR sidechat_context_json IS NOT NULL) \
               AND sidechat_expired_at IS NULL \
               AND archived_at IS NULL AND deleted_at IS NULL",
        )
        .map_err(|e| e.to_string())?;
    let rows: Vec<(String, Option<String>, String, String, i64)> = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    let now = now_ms();
    for (thread_id, last_activity, created_at, session_status, running_turns) in rows {
        // Running turns or live sessions hold off expiry, same as upstream.
        if running_turns > 0 || session_status == "running" || session_status == "starting" {
            continue;
        }
        let anchor = last_activity
            .as_deref()
            .and_then(iso_to_ms)
            .or_else(|| iso_to_ms(&created_at));
        let Some(anchor) = anchor else { continue };
        if now - anchor < window_ms {
            continue;
        }
        let mut cmd = Map::new();
        cmd.insert("type".into(), json!("thread.sidechat.expire"));
        cmd.insert(
            "commandId".into(),
            json!(format!("server:sidechat-expiry:{}", crate::orchestration_engine::uuid_v4())),
        );
        cmd.insert("threadId".into(), json!(thread_id.clone()));
        cmd.insert("createdAt".into(), json!(crate::todo::iso_now_ms()));
        let _ = crate::orchestration_engine::handle(
            "orchestration.dispatchCommand",
            &Value::Object(cmd),
        );
    }
    Ok(())
}
