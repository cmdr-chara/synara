//! Anonymous 24h usage snapshot relay input for Beta/Remote diagnostics.
//! Mirrors apps/server/src/betaUsageSnapshot.ts: fixed counts only, never
//! thread titles, paths, prompts, model names, tokens, or provider payloads.

use rusqlite::Connection;
use serde_json::json;
use std::io::Write as _;
use std::path::{Path, PathBuf};

const SNAPSHOT_INTERVAL: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);
const SNAPSHOT_WINDOW_HOURS: i64 = 24;

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn write_atomic(path: &Path, body: &str) -> Result<(), String> {
    let parent = path.parent().ok_or("path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = parent.join(format!(".{}.{}.tmp", path.file_name().and_then(|n| n.to_str()).unwrap_or("usage"), std::process::id()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new().create_new(true).write(true).open(&tmp).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
        }
        file.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&tmp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() { let _ = std::fs::remove_file(&tmp); }
    result
}

pub fn write_beta_usage_snapshot(beta_home_dir: &Path, db_path: &Path) -> Result<PathBuf, String> {
    let conn = Connection::open(db_path).map_err(|e| format!("open state database: {e}"))?;
    let since = (chrono::Utc::now() - chrono::Duration::hours(SNAPSHOT_WINDOW_HOURS))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let projects: i64 = conn
        .query_row("SELECT COUNT(*) FROM projection_projects WHERE deleted_at IS NULL", [], |row| row.get(0))
        .map_err(|e| format!("count projects: {e}"))?;
    let active_threads: i64 = conn
        .query_row("SELECT COUNT(DISTINCT thread_id) FROM projection_turns WHERE requested_at >= ?1", [&since], |row| row.get(0))
        .map_err(|e| format!("count active threads: {e}"))?;
    let path = beta_home_dir.join("diagnostics").join("usage-snapshot.json");
    let body = format!("{}\n", json!({
        "v": 1,
        "generatedAt": now_iso(),
        "projects": projects,
        "activeThreads": active_threads,
    }));
    write_atomic(&path, &body)?;
    Ok(path)
}

fn beta_remote_flavor() -> bool {
    matches!(std::env::var("SYNARA_DESKTOP_FLAVOR").ok().as_deref(), Some("beta") | Some("remote"))
}

/// Starts the upstream six-hour maintenance loop. It runs only inside Beta and
/// Remote backends; Stable must not even produce diagnostics input files.
pub fn start_beta_usage_snapshot_job() {
    if !beta_remote_flavor() { return; }
    std::thread::spawn(|| loop {
        let state_dir = std::path::PathBuf::from(crate::todo::state_dir());
        let home = state_dir.parent().map(Path::to_path_buf).unwrap_or_else(|| state_dir.clone());
        let db = state_dir.join("state.sqlite");
        if let Err(error) = write_beta_usage_snapshot(&home, &db) {
            eprintln!("beta usage snapshot write failed: {error}");
        }
        std::thread::sleep(SNAPSHOT_INTERVAL);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_contains_only_fixed_counts() {
        let root = std::env::temp_dir().join(format!("beta-usage-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let db = root.join("state.sqlite");
        let conn = Connection::open(&db).unwrap();
        conn.execute_batch("CREATE TABLE projection_projects (id TEXT, deleted_at TEXT); CREATE TABLE projection_turns (thread_id TEXT, requested_at TEXT); INSERT INTO projection_projects VALUES ('p1', NULL), ('p2', 'x'); INSERT INTO projection_turns VALUES ('t1', '2999-01-01T00:00:00.000Z'), ('t1', '2999-01-01T00:00:01.000Z'), ('t2', '2000-01-01T00:00:00.000Z');").unwrap();
        drop(conn);
        let path = write_beta_usage_snapshot(&root, &db).unwrap();
        let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(value["v"], 1);
        assert_eq!(value["projects"], 1);
        assert_eq!(value["activeThreads"], 1);
        assert!(value.get("threads").is_none() && value.get("tokens").is_none() && value.get("models").is_none());
        let _ = std::fs::remove_dir_all(root);
    }
}
