//! Browser-host discovery: how the server finds the shell-hosted socket and
//! its capability token. Upstream passes both through the desktop-spawned
//! server environment; the native shell and backend start independently, so
//! the shell publishes a 0600 discovery file inside SYNARA_HOME.

use serde_json::{json, Value};
use std::path::PathBuf;

pub const DISCOVERY_FILE_NAME: &str = "browser-host.json";

pub fn synara_home() -> PathBuf {
    if let Ok(home) = std::env::var("SYNARA_HOME") {
        if !home.trim().is_empty() {
            return PathBuf::from(home);
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".synara")
}

pub fn discovery_file() -> PathBuf {
    synara_home().join(DISCOVERY_FILE_NAME)
}

/// Default unix socket path mirroring upstream's /tmp layout (bounded for the
/// 104-byte sockaddr_un limit on macOS).
pub fn default_pipe_path() -> PathBuf {
    let uid = unsafe { libc_getuid() };
    let dir = format!("synara-browser-host-{uid}");
    let suffix = format!("{}-{}", std::process::id(), uuid::Uuid::new_v4());
    PathBuf::from("/tmp").join(dir).join(format!("{suffix}.sock"))
}

fn libc_getuid() -> u32 {
    // getuid() is stable libc on unix; avoid the libc crate dependency for one call.
    #[cfg(unix)]
    {
        extern "C" {
            fn getuid() -> u32;
        }
        unsafe { getuid() }
    }
    #[cfg(not(unix))]
    {
        0
    }
}

/// Publish discovery for clients: pipe path + capability, file mode 0600.
pub fn publish(pipe_path: &str, capability: &str) -> Result<(), String> {
    let file = discovery_file();
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = json!({
        "type": "synara-browser-host-discovery",
        "pipePath": pipe_path,
        "capability": capability,
        "pid": std::process::id(),
    });
    let tmp = file.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec_pretty(&body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &file).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn unpublish() {
    let _ = std::fs::remove_file(discovery_file());
}

pub struct Discovery {
    pub pipe_path: String,
    pub capability: String,
}

/// Resolve connection parameters: explicit env first, then the discovery file.
pub fn resolve() -> Option<Discovery> {
    if let Ok(pipe) = std::env::var("SYNARA_BROWSER_HOST_PIPE_PATH") {
        let pipe = pipe.trim().to_string();
        if !pipe.is_empty() {
            if let Ok(cap) = std::env::var("SYNARA_BROWSER_HOST_CAPABILITY") {
                let cap = cap.trim().to_string();
                if cap.len() >= 32 {
                    return Some(Discovery { pipe_path: pipe, capability: cap });
                }
            }
        }
    }
    let file = discovery_file();
    let raw = std::fs::read_to_string(file).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    let pipe = value.get("pipePath")?.as_str()?.to_string();
    let cap = value.get("capability")?.as_str()?.to_string();
    // Stale discovery from a dead shell is worse than none: verify the pid.
    if let Some(pid) = value.get("pid").and_then(Value::as_u64) {
        if !pid_alive(pid as u32) {
            return None;
        }
    }
    Some(Discovery { pipe_path: pipe, capability: cap })
}

fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }
        // signal 0 probes existence without delivering anything.
        unsafe { kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}
