//! Native `server.getEnvironment` — the `ExecutionEnvironmentDescriptor` the
//! original builds in `environment/Layers/ServerEnvironment.ts`. Values
//! describe THIS server: the descriptor belongs to whoever answers the RPC.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MACHINE_ID_SALT: &str = "synara-machine-id-v1:";

fn hash_machine_id(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    let mut hasher = Sha256::new();
    hasher.update(MACHINE_ID_SALT.as_bytes());
    hasher.update(normalized.as_bytes());
    let hex = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    Some(hex[..32].to_string())
}

fn raw_machine_id() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("/usr/sbin/ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let marker = "\"IOPlatformUUID\" = \"";
        let start = text.find(marker)? + marker.len();
        let end = text[start..].find('"')? + start;
        return Some(text[start..end].to_string());
    }
    #[cfg(target_os = "linux")]
    {
        for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
            if let Ok(value) = std::fs::read_to_string(path) {
                let value = value.trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
        return None;
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        None
    }
}

fn platform_os() -> &'static str {
    if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    }
}

fn platform_arch() -> &'static str {
    if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86_64") {
        "x64"
    } else {
        "other"
    }
}

fn hostname() -> Option<String> {
    let out = std::process::Command::new("hostname").output().ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if name.is_empty() { None } else { Some(name) }
}

/// `apps/server/package.json` version — the server build string the
/// descriptor reports (`packageJson.version` upstream).
fn server_version() -> String {
    const PKG: &str = include_str!("../../../apps/server/package.json");
    serde_json::from_str::<Value>(PKG)
        .ok()
        .and_then(|v| v.get("version").and_then(|v| v.as_str()).map(String::from))
        .unwrap_or_else(|| "0.0.0".to_string())
}

/// Environment id persisted by whoever owns `<stateDir>` — the file format is
/// a bare UUID per line, created on first boot (`accountAuth.ts`).
fn environment_id(state_dir: &str) -> Option<String> {
    let path = std::path::Path::new(state_dir).join("environment-id");
    let raw = std::fs::read_to_string(path).ok()?.trim().to_string();
    if raw.is_empty() { None } else { Some(raw) }
}

/// The full `server.getEnvironment` result.
pub fn get_environment(state_dir: &str) -> Result<Value, String> {
    let environment_id = environment_id(state_dir)
        .ok_or_else(|| "environment-id file is missing or empty".to_string())?;
    let channel = "dev";
    let label = hostname().unwrap_or_else(|| "Synara".to_string());
    // Remote connections and profile sync report unavailable until they are
    // served natively; the reasons are operator-facing strings upstream.
    let remote_reason = "Remote connections are not served by this build yet.";
    let profile_sync_reason = "Account profile sync is not served by this build yet.";
    let _ = profile_sync_reason;
    let mut descriptor = json!({
        "environmentId": environment_id,
        "channel": channel,
        "label": label,
        "platform": { "os": platform_os(), "arch": platform_arch() },
        "serverVersion": server_version(),
        "capabilities": {
            "desktopUpdates": false,
            "repositoryIdentity": true,
            "accountProfileSync": false,
            "remoteConnections": false,
            "remoteResources": false,
            "remoteUnavailableReason": remote_reason,
            "composerPredictions": true,
        },
    });
    if let Some(id) = raw_machine_id().and_then(|raw| hash_machine_id(&raw)) {
        descriptor["machineId"] = json!(id);
    }
    Ok(descriptor)
}
