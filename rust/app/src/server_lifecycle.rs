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

/// `server.getConfig` — static paths, merged keybindings config (+ issues),
/// editors resolvable on this host, and provider statuses ([] until the
/// provider matrix is ported).
pub fn get_config() -> Value {
    let w = welcome_payload();
    let state = std::path::PathBuf::from(crate::todo::state_dir());
    let base = state.parent().unwrap_or(state.as_path()).to_path_buf();
    let kb = crate::server_keybindings::config_state();
    json!({
        "cwd": w.get("cwd").cloned().unwrap_or(Value::Null),
        "homeDir": w.get("homeDir").cloned().unwrap_or(Value::Null),
        "chatWorkspaceRoot": w.get("chatWorkspaceRoot").cloned().unwrap_or(Value::Null),
        "studioWorkspaceRoot": w.get("studioWorkspaceRoot").cloned().unwrap_or(Value::Null),
        "groupsWorkspaceRoot": w.get("groupsWorkspaceRoot").cloned().unwrap_or(Value::Null),
        "worktreesDir": realpath_nearest_existing(&base.join("worktrees")).to_string_lossy(),
        "keybindingsConfigPath": realpath_nearest_existing(&state.join("keybindings.json"))
            .to_string_lossy(),
        "keybindings": kb.get("keybindings").cloned().unwrap_or(Value::Array(vec![])),
        "defaultKeybindings": crate::server_keybindings::default_resolved(),
        "issues": kb.get("issues").cloned().unwrap_or(Value::Array(vec![])),
        "providers": crate::provider_cache::provider_statuses(),
        "availableEditors": crate::shell_editor::available_editors(),
    })
}

struct KeepAwakeProcess {
    child: Option<std::process::Child>,
    restarts: u32,
    active: bool,
    error: Option<String>,
    last_desired: bool,
}

impl Default for KeepAwakeProcess {
    fn default() -> Self {
        Self {
            child: None,
            restarts: 0,
            active: false,
            error: None,
            last_desired: false,
        }
    }
}

fn keep_awake_process() -> &'static std::sync::Mutex<KeepAwakeProcess> {
    static PROCESS: std::sync::OnceLock<std::sync::Mutex<KeepAwakeProcess>> =
        std::sync::OnceLock::new();
    PROCESS.get_or_init(|| std::sync::Mutex::new(KeepAwakeProcess::default()))
}

fn caffeinate_available() -> bool {
    std::env::consts::OS == "macos"
        && (Path::new("/usr/bin/caffeinate").exists()
            || std::env::var_os("PATH").is_some_and(|paths| {
                std::env::split_paths(&paths).any(|dir| dir.join("caffeinate").exists())
            }))
}

fn active_turn_count() -> usize {
    let db = std::path::PathBuf::from(crate::todo::state_dir()).join("state.sqlite");
    let Ok(conn) = rusqlite::Connection::open_with_flags(
        db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    ) else {
        return 0;
    };
    conn.query_row(
        "SELECT COUNT(*) FROM projection_thread_sessions WHERE status = 'running' AND active_turn_id IS NOT NULL",
        [],
        |row| row.get::<_, i64>(0),
    ).ok().filter(|count| *count > 0).unwrap_or(0) as usize
}

fn desired_keep_awake(mode: &str) -> bool {
    mode == "always" || (mode == "agent" && active_turn_count() > 0)
}

fn stop_caffeinate(child: &mut std::process::Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(800);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(20))
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return;
            }
            Err(_) => return,
        }
    }
}

fn reconcile_keep_awake() {
    let mode = crate::server_settings::current()
        .get("keepAwakeMode")
        .and_then(Value::as_str)
        .unwrap_or("off")
        .to_string();
    let desired = caffeinate_available() && desired_keep_awake(&mode);
    let mut process = match keep_awake_process().lock() {
        Ok(process) => process,
        Err(_) => return,
    };

    if desired && !process.last_desired {
        process.restarts = 0;
        process.error = None;
    }
    process.last_desired = desired;

    let mut exited_unexpectedly = false;
    if let Some(child) = process.child.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => exited_unexpectedly = true,
            Ok(None) => {}
            Err(error) => process.error = Some(format!("caffeinate wait failed: {error}")),
        }
    }
    if exited_unexpectedly {
        process.child = None;
        process.active = false;
        if desired {
            process.restarts += 1;
            if process.restarts > 5 {
                process.error = Some(
                    "caffeinate exited unexpectedly 6 times; giving up until the mode changes"
                        .into(),
                );
            }
        }
    }

    if !desired {
        if let Some(mut child) = process.child.take() {
            stop_caffeinate(&mut child);
        }
        process.active = false;
        if mode == "off" {
            process.error = None;
        }
        return;
    }

    if process.error.is_some() || process.child.is_some() {
        return;
    }
    match std::process::Command::new("caffeinate")
        .args(["-dims", "-w", &std::process::id().to_string()])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(child) => {
            process.child = Some(child);
            process.active = true;
        }
        Err(error) => {
            process.active = false;
            process.error = Some(format!("caffeinate spawn failed: {error}"));
        }
    }
}

/// Start the small power-assertion supervisor. `caffeinate -w <server pid>`
/// is deliberately tied to this process so a crashed backend cannot leave the
/// computer awake forever.
pub fn start_keep_awake() {
    tokio::spawn(async move {
        reconcile_keep_awake();
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            reconcile_keep_awake();
        }
    });
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
    let process = keep_awake_process().lock().ok();
    json!({
        "available": caffeinate_available(),
        "mode": mode,
        "active": process.as_ref().is_some_and(|process| process.active),
        "error": process.as_ref().and_then(|process| process.error.clone()).map(Value::String).unwrap_or(Value::Null),
    })
}
