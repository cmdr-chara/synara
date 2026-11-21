//! The Rust port of the bridgeable exports in `apps/desktop/src/cuaDriverHost.ts`
//! (`sweepOrphanedCuaDrivers`, `sweepOrphanedCuaDriversInBackground`), which together
//! reimplement `reapOrphanedCuaDrivers` plus the parts of
//! `apps/desktop/src/cuaRuntimeOwnership.ts` the sweep reaches
//! (`cuaHostProcessIsAlive`, `sweepOwnedCuaRuntimeDirectories`).

use std::io::Read;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::process::Command;

const STALE_AFTER_MS: f64 = 30_000.0;
const MARKER_NAME: &str = ".synara-cua-runtime.json";

/// `process.kill(pid, 0)`: alive on success, dead only on ESRCH.
fn host_process_is_alive(pid: f64) -> bool {
    if !pid.is_finite() || pid <= 0.0 || pid > i32::MAX as f64 {
        return false;
    }
    // SAFETY: `kill` with signal 0 only probes; any pid value is valid to pass.
    if unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
}

fn current_uid() -> u32 {
    // SAFETY: `getuid` has no preconditions.
    unsafe { libc::getuid() }
}

fn kill_process(pid: f64) {
    // SAFETY: best-effort SIGKILL of a stale daemon; a dead pid is an errno we
    // ignore, matching the `catch {}` around `process.kill`.
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGKILL);
    }
}

fn js_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// `/cua-driver\s+serve\s+--embedded/` — three tokens joined by whitespace only.
fn line_is_embedded_driver(line: &str) -> bool {
    let mut it = line.split_whitespace();
    while let Some(tok) = it.next() {
        if tok == "cua-driver" && it.next() == Some("serve") && it.next() == Some("--embedded") {
            return true;
        }
    }
    false
}

/// `line.match(/--socket\s+(\S+)\//)?.[1]` — the run of non-space chars after
/// `--socket`, trimmed back to (not including) its last `/`.
fn socket_dir_of(line: &str) -> Option<String> {
    let mut idx = 0;
    while let Some(found) = line[idx..].find("--socket") {
        let start = idx + found + "--socket".len();
        let rest = &line[start..];
        let trimmed = rest.trim_start();
        if !trimmed.is_empty() {
            let run_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
            let run = &trimmed[..run_end];
            if let Some(slash) = run.rfind('/')
                && slash > 0
            {
                return Some(run[..slash].to_string());
            }
        }
        idx = start;
    }
    None
}

fn env_host_pid(pid: f64) -> Option<f64> {
    let out = Command::new("ps")
        .args(["eww", "-p", &format!("{pid}"), "-o", "command"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let pos = text.find("CUA_DRIVER_EMBEDDED_HOST_PID=")?;
    let digits: String = text[pos + "CUA_DRIVER_EMBEDDED_HOST_PID=".len()..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    Some(js_number(&digits))
}

/// `/^synara-cua-[a-zA-Z0-9]{6}$/`
fn is_runtime_directory(entry: &str) -> bool {
    entry.len() == "synara-cua-".len() + 6
        && entry.starts_with("synara-cua-")
        && entry["synara-cua-".len()..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric())
}

fn realpath(path: &std::path::Path) -> std::path::PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Port of `sweepOwnedCuaRuntimeDirectories` (`cuaRuntimeOwnership.ts:46`).
fn sweep_owned_runtime_directories(
    directory: &std::path::Path,
    live_socket_dirs: &std::collections::HashSet<String>,
    now: f64,
) -> Vec<String> {
    let uid = current_uid();
    let protected: std::collections::HashSet<std::path::PathBuf> = live_socket_dirs
        .iter()
        .map(|p| realpath(std::path::Path::new(p)))
        .collect();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut removed = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_runtime_directory(&name) {
            continue;
        }
        let dir = entry.path();
        let Ok(before) = std::fs::symlink_metadata(&dir) else {
            continue;
        };
        let mtime_ms = before.mtime() as f64 * 1000.0 + before.mtime_nsec() as f64 / 1e6;
        if !before.is_dir()
            || before.file_type().is_symlink()
            || before.uid() != uid
            || (before.mode() & 0o077) != 0
            || now - mtime_ms < STALE_AFTER_MS
            || protected.contains(&realpath(&dir))
        {
            continue;
        }
        let marker_path = dir.join(MARKER_NAME);
        let marker = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&marker_path);
        let Ok(mut file) = marker else {
            continue;
        };
        let Ok(stat) = file.metadata() else {
            continue;
        };
        if !stat.is_file()
            || stat.uid() != uid
            || stat.nlink() != 1
            || (stat.mode() & 0o077) != 0
            || stat.size() == 0
            || stat.size() > 1_024
        {
            continue;
        }
        let mut buf = [0u8; 1_025];
        let Ok(length) = file.read(&mut buf) else {
            continue;
        };
        if length == 0 || length > 1_024 {
            continue;
        }
        let Ok(marker) = serde_json::from_slice::<serde_json::Value>(&buf[..length]) else {
            continue;
        };
        let Some(data) = marker.as_object() else {
            continue;
        };
        let num = |k: &str| data.get(k).and_then(|v: &serde_json::Value| v.as_f64());
        let host_pid = num("hostPid").unwrap_or(f64::NAN);
        let created_at = num("createdAt").unwrap_or(f64::NAN);
        if data.get("schema").and_then(|v| v.as_str()) != Some("synara-cua-runtime")
            || data
                .get("version")
                .and_then(|v: &serde_json::Value| v.as_f64())
                != Some(1.0)
            || data.get("directory").and_then(|v| v.as_str()) != Some(name.as_str())
            || num("ownerUid") != Some(uid as f64)
            || !host_pid.is_finite()
            || host_pid.fract() != 0.0
            || host_pid <= 0.0
            || host_pid > 2_147_483_647.0
            || !created_at.is_finite()
            || created_at <= 0.0
            || now - created_at < STALE_AFTER_MS
            || host_process_is_alive(host_pid)
        {
            continue;
        }
        // Refuse replacement/symlink races between the ownership read and removal.
        let Ok(after) = std::fs::symlink_metadata(&dir) else {
            continue;
        };
        if !after.is_dir()
            || after.file_type().is_symlink()
            || after.uid() != uid
            || after.ino() != before.ino()
            || after.dev() != before.dev()
            || (after.mode() & 0o077) != 0
        {
            continue;
        }
        if std::fs::remove_dir_all(&dir).is_ok() || !dir.exists() {
            removed.push(name);
        }
    }
    removed
}

fn reap_orphaned_cua_drivers(listing: &str) {
    let mut live_socket_dirs = std::collections::HashSet::new();
    for line in listing.split('\n') {
        if !line_is_embedded_driver(line) {
            continue;
        }
        let pid = js_number(line.split_whitespace().next().unwrap_or(""));
        if !pid.is_finite() || pid == 0.0 || pid == f64::from(std::process::id()) {
            continue;
        }
        if let Some(dir) = socket_dir_of(line) {
            live_socket_dirs.insert(dir);
        }
        let Some(host_pid) = env_host_pid(pid) else {
            continue;
        };
        if !host_pid.is_finite() || host_pid == 0.0 {
            continue;
        }
        if host_process_is_alive(host_pid) {
            continue;
        }
        kill_process(pid);
    }
    let tmp = std::env::temp_dir();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    let _ = sweep_owned_runtime_directories(&tmp, &live_socket_dirs, now_ms);
}

/// Port of `sweepOrphanedCuaDrivers` (`apps/desktop/src/cuaDriverHost.ts:300`).
pub fn sweep_orphaned_cua_drivers() -> Result<(), String> {
    if cfg!(windows) {
        return Ok(());
    }
    let Ok(out) = Command::new("ps").args(["-axo", "pid,args"]).output() else {
        return Ok(());
    };
    if !out.status.success() {
        return Ok(());
    }
    reap_orphaned_cua_drivers(&String::from_utf8_lossy(&out.stdout));
    Ok(())
}

/// Port of `sweepOrphanedCuaDriversInBackground` (`apps/desktop/src/cuaDriverHost.ts:319`).
/// The napi async binding already runs off the main thread, so the sweep itself
/// is the same work.
pub async fn sweep_orphaned_cua_drivers_in_background() -> Result<(), String> {
    sweep_orphaned_cua_drivers()
}
