//! Atomic lifecycle ownership for one `state.sqlite` data home.
//!
//! This mirrors the upstream ownership contract: ownership is published as a
//! complete directory, never as a bare lockfile; a live owner is authoritative;
//! a dead owner is quarantined before recovery. The lock intentionally survives
//! process termination — the next startup reaps it only after the owner pid is
//! demonstrably dead.

use serde_json::json;
use std::path::{Path, PathBuf};

fn valid_uuid(value: &str) -> bool {
    let parts: Vec<&str> = value.split('-').collect();
    parts.len() == 5
        && parts[0].len() == 8
        && parts[1].len() == 4
        && parts[2].len() == 4
        && parts[3].len() == 4
        && parts[4].len() == 12
        && parts
            .iter()
            .all(|part| part.bytes().all(|byte| byte.is_ascii_hexdigit()))
        && parts[2]
            .as_bytes()
            .first()
            .is_some_and(|byte| *byte == b'4')
        && parts[3]
            .as_bytes()
            .first()
            .is_some_and(|byte| matches!(byte.to_ascii_lowercase(), b'8' | b'9' | b'a' | b'b'))
}

fn new_token() -> String {
    crate::orchestration_engine::uuid_v4()
}

#[derive(Debug, Clone)]
struct Owner {
    pid: u32,
    token: String,
    created_at: String,
}

pub struct DatabaseLifecycleLock {
    lock_path: PathBuf,
    owner: Owner,
}

impl std::fmt::Debug for DatabaseLifecycleLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DatabaseLifecycleLock")
            .field("lock_path", &self.lock_path)
            .finish_non_exhaustive()
    }
}

fn private_dir(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn write_owner(path: &Path, owner: &Owner) -> Result<(), String> {
    private_dir(path)?;
    let owner_path = path.join("owner.json");
    let mut options = std::fs::OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options
        .open(&owner_path)
        .map_err(|e| format!("write lock owner: {e}"))?;
    use std::io::Write as _;
    let body = serde_json::to_string(&json!({
        "pid": owner.pid,
        "token": owner.token,
        "createdAt": owner.created_at,
    }))
    .map_err(|e| e.to_string())?;
    file.write_all(format!("{body}\n").as_bytes())
        .map_err(|e| format!("write lock owner: {e}"))?;
    file.sync_all().map_err(|e| format!("sync lock owner: {e}"))
}

fn read_owner(lock_path: &Path) -> Result<Owner, String> {
    let meta =
        std::fs::symlink_metadata(lock_path).map_err(|e| format!("lock path unreadable: {e}"))?;
    if !meta.file_type().is_dir() || meta.file_type().is_symlink() {
        return Err("lock path is not a real directory".into());
    }
    let owner_path = lock_path.join("owner.json");
    let meta = std::fs::symlink_metadata(&owner_path)
        .map_err(|e| format!("lock owner unreadable: {e}"))?;
    if !meta.file_type().is_file() || meta.file_type().is_symlink() {
        return Err("lock owner is not a real regular file".into());
    }
    let text =
        std::fs::read_to_string(&owner_path).map_err(|e| format!("lock owner unreadable: {e}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("lock owner metadata is invalid: {e}"))?;
    let pid = value
        .get("pid")
        .and_then(serde_json::Value::as_u64)
        .filter(|pid| *pid > 0 && *pid <= u32::MAX as u64)
        .ok_or("lock owner pid is invalid")? as u32;
    let token = value
        .get("token")
        .and_then(serde_json::Value::as_str)
        .filter(|token| valid_uuid(token))
        .ok_or("lock owner token is invalid")?
        .to_string();
    let created_at = value
        .get("createdAt")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or("lock owner creation time is invalid")?
        .to_string();
    Ok(Owner {
        pid,
        token,
        created_at,
    })
}

fn owner_process_state(pid: u32) -> &'static str {
    #[cfg(unix)]
    {
        // SAFETY: signal 0 only checks process existence and never delivers a signal.
        let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if result == 0 {
            return "live";
        }
        let error = std::io::Error::last_os_error().raw_os_error();
        if error == Some(libc::ESRCH) {
            return "dead";
        }
        return "unknown";
    }
    #[cfg(windows)]
    {
        let output = std::process::Command::new("tasklist.exe")
            .args(["/FO", "CSV", "/NH", "/FI", &format!("PID eq {pid}")])
            .output();
        return match output {
            Ok(output) if String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()) => {
                "live"
            }
            Ok(_) => "dead",
            Err(_) => "unknown",
        };
    }
    #[allow(unreachable_code)]
    "unknown"
}

fn try_publish(lock_path: &Path, owner: &Owner) -> Result<bool, String> {
    let staging = lock_path.with_file_name(format!(
        "{}.acquiring.{}.{}",
        lock_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("lifecycle-lock"),
        owner.pid,
        owner.token,
    ));
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir(&staging).map_err(|e| format!("create lock staging directory: {e}"))?;
    if let Err(error) = write_owner(&staging, owner) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(error);
    }
    match std::fs::rename(&staging, lock_path) {
        Ok(()) => {
            // Unix rename may replace an empty foreign directory. Ownership is
            // authoritative only if the exact owner metadata we wrote is there.
            match read_owner(lock_path) {
                Ok(published) if published.pid == owner.pid && published.token == owner.token => {
                    Ok(true)
                }
                _ => {
                    let _ = std::fs::remove_dir_all(lock_path);
                    Ok(false)
                }
            }
        }
        Err(error) => {
            let _ = std::fs::remove_dir_all(&staging);
            if lock_path.exists() {
                Ok(false)
            } else {
                Err(format!("publish lifecycle lock: {error}"))
            }
        }
    }
}

fn reap_dead_owner(lock_path: &Path, owner: &Owner) -> Result<(), String> {
    let observed = read_owner(lock_path)?;
    if observed.pid != owner.pid || observed.token != owner.token {
        return Err("lock ownership changed while stale recovery was starting".into());
    }
    if owner_process_state(observed.pid) != "dead" {
        return Err(format!("owner pid {} is not dead", observed.pid));
    }
    let stale = lock_path.with_file_name(format!(
        "{}.stale.{}.{}",
        lock_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("lifecycle-lock"),
        observed.token,
        new_token(),
    ));
    std::fs::rename(lock_path, &stale)
        .map_err(|e| format!("quarantine stale lifecycle lock: {e}"))?;
    std::fs::remove_dir_all(&stale).map_err(|e| format!("remove stale lifecycle lock: {e}"))
}

fn canonical_database_path(db_path: &Path) -> Result<PathBuf, String> {
    let parent = db_path.parent().ok_or("database path has no parent")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("create database parent: {e}"))?;
    let meta = std::fs::symlink_metadata(parent)
        .map_err(|e| format!("database parent unreadable: {e}"))?;
    if !meta.file_type().is_dir() || meta.file_type().is_symlink() {
        return Err(format!(
            "database parent path is not a real directory: {}",
            parent.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        if meta.permissions().mode() & 0o022 != 0 {
            // Existing Rust dev homes predate the lifecycle lock and were often
            // created with the umask default. Tighten an owned directory in
            // place rather than bricking startup; ownership failure stays fatal.
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).map_err(
                |_| {
                    format!(
                        "database parent directory is group/other writable: {}",
                        parent.display()
                    )
                },
            )?;
        }
    }
    Ok(parent
        .canonicalize()
        .map_err(|e| format!("database parent canonicalization: {e}"))?
        .join(
            db_path
                .file_name()
                .ok_or("database path has no file name")?,
        ))
}

pub fn acquire_database_lifecycle_lock(db_path: &Path) -> Result<DatabaseLifecycleLock, String> {
    let canonical_db = canonical_database_path(db_path)?;
    let lock_path = PathBuf::from(format!("{}.lifecycle-lock", canonical_db.display()));
    for attempt in 0..2 {
        let owner = Owner {
            pid: std::process::id(),
            token: new_token().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        if try_publish(&lock_path, &owner)? {
            return Ok(DatabaseLifecycleLock { lock_path, owner });
        }
        let existing = read_owner(&lock_path).map_err(|error| {
            format!(
                "Database lifecycle is locked for {}: owner is live or unknown: {error} ({})",
                canonical_db.display(),
                lock_path.display()
            )
        })?;
        if owner_process_state(existing.pid) != "dead" {
            return Err(format!(
                "Database lifecycle is locked for {}: owner pid {} is {} ({})",
                canonical_db.display(),
                existing.pid,
                owner_process_state(existing.pid),
                lock_path.display(),
            ));
        }
        if attempt > 0 {
            return Err(format!(
                "Database lifecycle is locked for {}: dead owner could not be recovered safely ({})",
                canonical_db.display(),
                lock_path.display(),
            ));
        }
        reap_dead_owner(&lock_path, &existing)?;
    }
    Err(format!(
        "Database lifecycle is locked for {}: acquisition failed",
        canonical_db.display()
    ))
}

impl DatabaseLifecycleLock {
    pub fn release(mut self) -> Result<(), String> {
        self.release_inner()
    }

    fn release_inner(&mut self) -> Result<(), String> {
        let owner = match read_owner(&self.lock_path) {
            Ok(owner) => owner,
            Err(error) if error.contains("unreadable") && !self.lock_path.exists() => return Ok(()),
            Err(error) => return Err(error),
        };
        if owner.pid != self.owner.pid || owner.token != self.owner.token {
            return Err("refusing to release a lock owned by another process or token".into());
        }
        let released = self.lock_path.with_file_name(format!(
            "{}.released.{}.{}",
            self.lock_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("lifecycle-lock"),
            self.owner.token,
            new_token(),
        ));
        std::fs::rename(&self.lock_path, &released)
            .map_err(|e| format!("release lifecycle lock: {e}"))?;
        std::fs::remove_dir_all(&released)
            .map_err(|e| format!("remove released lifecycle lock: {e}"))
    }
}

impl Drop for DatabaseLifecycleLock {
    fn drop(&mut self) {
        let _ = self.release_inner();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_owner_blocks_second_acquisition_and_release_recovers() {
        let root = std::env::temp_dir().join(format!("lifecycle-test-{}", new_token()));
        std::fs::create_dir_all(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let db = root.join("state.sqlite");
        let first = acquire_database_lifecycle_lock(&db)
            .map_err(|e| e.to_string())
            .unwrap();
        assert!(acquire_database_lifecycle_lock(&db).is_err());
        first.release().unwrap();
        let second = acquire_database_lifecycle_lock(&db)
            .map_err(|e| e.to_string())
            .unwrap();
        second.release().unwrap();
        let _ = std::fs::remove_dir_all(root);
    }
}
