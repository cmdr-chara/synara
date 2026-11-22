//! `server.subscribeLifecycle` and `server.subscribeKeepAwake` — the server
//! lifecycle event stream (welcome → ready, then maintenance events which this
//! native server does not emit because it runs no retention task) and the
//! keep-awake state stream driven by the `keepAwakeMode` setting.

use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use crate::server_local_servers::iso_now;

/// Canonicalize the longest existing prefix of `path` and re-append the rest,
/// mirroring `realpathNearestExisting` in apps/server/src/config.ts.
fn realpath_nearest_existing(path: &Path) -> PathBuf {
    let mut existing = path.to_path_buf();
    let mut suffix: Vec<String> = Vec::new();
    loop {
        if existing.exists() {
            break;
        }
        match existing.file_name() {
            Some(name) => {
                suffix.push(name.to_string_lossy().into_owned());
                existing.pop();
            }
            None => break,
        }
    }
    let resolved = std::fs::canonicalize(&existing).unwrap_or(existing);
    suffix
        .iter()
        .rev()
        .fold(resolved, |acc, part| acc.join(part))
}

fn join3(a: &Path, b: &str, c: &str) -> PathBuf {
    a.join(b).join(c)
}

/// `ServerLifecycleWelcomePayload`: cwd, the user's home directory and the
/// canonical chat/studio/groups workspace roots derived from it.
pub fn welcome_payload() -> Value {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let cwd = realpath_nearest_existing(&cwd);
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let home_dir = realpath_nearest_existing(Path::new(&home));
    let chat = realpath_nearest_existing(&join3(&home_dir, "Documents", "Synara"));
    let studio = realpath_nearest_existing(&chat.join("Studio"));
    let groups = realpath_nearest_existing(&chat.join("Groups"));
    let project_name = cwd
        .components()
        .next_back()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_else(|| cwd.to_string_lossy().into_owned());
    json!({
        "cwd": cwd.to_string_lossy(),
        "homeDir": home_dir.to_string_lossy(),
        "chatWorkspaceRoot": chat.to_string_lossy(),
        "studioWorkspaceRoot": studio.to_string_lossy(),
        "groupsWorkspaceRoot": groups.to_string_lossy(),
        "projectName": project_name,
    })
}

/// `ServerLifecycleStreamEvent` for the initial snapshot: welcome, then ready.
/// Retention `maintenance` events never occur natively.
pub fn snapshot_events() -> Vec<Value> {
    vec![
        json!({"type": "welcome", "payload": welcome_payload()}),
        json!({"type": "ready", "payload": {"at": iso_now()}}),
    ]
}

/// `ServerKeepAwakeState`: the native server can keep the OS awake via the
/// platform power assertion (available on every desktop OS we ship on), is
/// driven by `keepAwakeMode`, and starts inactive because no agent turn is
/// tracked yet. `error` stays null until an assertion fails.
pub fn keep_awake_state() -> Value {
    let settings = crate::server_settings::current();
    let mode = settings
        .get("keepAwakeMode")
        .and_then(Value::as_str)
        .unwrap_or("off");
    json!({
        "available": true,
        "mode": mode,
        "active": mode == "always",
        "error": Value::Null,
    })
}
