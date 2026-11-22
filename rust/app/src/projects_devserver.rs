//! `projects.listDevServers` / `runDevServer` / `stopDevServer` /
//! `subscribeDevServerEvents` — background dev-server processes keyed by
//! project id. The server owns their lifecycle and broadcasts upsert/remove
//! events so every connected client stays in sync.
//!
//! `projects.subscribeFileChange` — per-request file watch stream polling
//! mtime; emits `changed`/`deleted` events for one workspace file.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

use crate::todo::WsErr;

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("FILE_IO"))
}

struct DevServer {
    command: String,
    cwd: String,
    child: Mutex<Option<Child>>,
    pid: u32,
    started_at: String,
    status: Mutex<&'static str>,
    started_instant: Instant,
}

static SERVERS: LazyLock<Mutex<HashMap<String, DevServer>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static EVENTS: LazyLock<broadcast::Sender<Value>> = LazyLock::new(|| broadcast::channel(64).0);

pub fn subscribe() -> broadcast::Receiver<Value> {
    EVENTS.subscribe()
}

fn broadcast(event: Value) {
    let _ = EVENTS.send(event);
}

fn server_json(project_id: &str, s: &DevServer) -> Value {
    json!({
        "projectId": project_id,
        "command": s.command,
        "cwd": s.cwd,
        "pid": s.pid,
        "startedAt": s.started_at,
        "status": *s.status.lock().unwrap(),
    })
}

fn snapshot_event() -> Value {
    let servers = SERVERS.lock().unwrap();
    json!({
        "type": "snapshot",
        "servers": servers.iter().map(|(id, s)| server_json(id, s)).collect::<Vec<_>>(),
    })
}

pub fn current_snapshot() -> Value {
    snapshot_event()
}

fn refresh_status(s: &DevServer) {
    let mut status = s.status.lock().unwrap();
    if *status == "starting" && s.started_instant.elapsed() >= Duration::from_secs(2) {
        *status = "running";
    }
}

fn upsert_broadcast(project_id: &str, s: &DevServer) {
    refresh_status(s);
    broadcast(json!({"type": "upserted", "server": server_json(project_id, s)}));
}

/// Monitor a spawned child: on exit, drop the server and broadcast removal.
fn watch_child(project_id: String) {
    tokio::spawn(async move {
        loop {
            let exited = {
                let servers = SERVERS.lock().unwrap();
                match servers.get(&project_id) {
                    None => return,
                    Some(s) => {
                        let mut slot = s.child.lock().unwrap();
                        match slot.as_mut() {
                            Some(child) => match child.try_wait() {
                                Ok(Some(_)) => true,
                                Ok(None) => false,
                                Err(_) => true,
                            },
                            None => true,
                        }
                    }
                }
            };
            if exited {
                let removed = {
                    let mut servers = SERVERS.lock().unwrap();
                    servers.remove(&project_id)
                };
                if removed.is_some() {
                    broadcast(json!({
                        "type": "removed",
                        "projectId": project_id,
                        "reason": "exited",
                    }));
                }
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
}

pub fn run_dev_server(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload
        .get("projectId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| err("projectId is required"))?
        .to_string();
    let command = payload
        .get("command")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| err("command is required"))?
        .to_string();
    let cwd = payload
        .get("cwd")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| err("cwd is required"))?;
    if !cwd.is_dir() {
        return Err(err(format!("cwd is not a directory: {}", cwd.display())));
    }

    {
        let servers = SERVERS.lock().unwrap();
        if let Some(existing) = servers.get(&project_id) {
            return Ok(json!({ "server": server_json(&project_id, existing) }));
        }
    }

    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&command).current_dir(&cwd);
    if let Some(env) = payload.get("env").and_then(|v| v.as_object()) {
        for (k, v) in env {
            if let Some(val) = v.as_str() {
                cmd.env(k, val);
            }
        }
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let child = cmd
        .spawn()
        .map_err(|e| err(format!("cannot spawn dev server: {e}")))?;
    let pid = child.id();
    let started_at = iso_now();
    let server = DevServer {
        command,
        cwd: cwd.to_string_lossy().into_owned(),
        child: Mutex::new(Some(child)),
        pid,
        started_at,
        status: Mutex::new("starting"),
        started_instant: Instant::now(),
    };
    let out = {
        let mut servers = SERVERS.lock().unwrap();
        servers.insert(project_id.clone(), server);
        server_json(&project_id, servers.get(&project_id).unwrap())
    };
    broadcast(json!({"type": "upserted", "server": out}));
    watch_child(project_id);
    Ok(json!({ "server": out }))
}

pub fn stop_dev_server(payload: &Value) -> Result<Value, WsErr> {
    let project_id = payload
        .get("projectId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| err("projectId is required"))?;
    let removed = {
        let mut servers = SERVERS.lock().unwrap();
        servers.remove(project_id)
    };
    match removed {
        Some(s) => {
            if let Some(mut child) = s.child.lock().unwrap().take() {
                let _ = child.kill();
            }
            broadcast(json!({
                "type": "removed",
                "projectId": project_id,
                "reason": "stopped",
            }));
            Ok(json!({ "stopped": true }))
        }
        None => Ok(json!({ "stopped": false })),
    }
}

pub fn list_dev_servers() -> Result<Value, WsErr> {
    let servers = SERVERS.lock().unwrap();
    let list: Vec<Value> = servers
        .iter()
        .map(|(id, s)| {
            refresh_status(s);
            server_json(id, s)
        })
        .collect();
    Ok(json!({ "servers": list }))
}

/// Resolve the watch target and return (absolute path, display rel path).
/// Same containment rule as readFile.
fn resolve_watch(cwd: &Path, rel: &str) -> Result<(PathBuf, String), WsErr> {
    let cwd_canon = canon(cwd);
    let raw = PathBuf::from(rel);
    let (target, rel_out) = if raw.is_absolute() {
        let t = canon(&raw);
        if !t.starts_with(&cwd_canon) {
            return Err(err("path escapes the workspace"));
        }
        (t, rel.to_string())
    } else {
        let t = canon(&cwd.join(&raw));
        if !t.starts_with(&cwd_canon) {
            return Err(err("path escapes the workspace"));
        }
        (
            t.clone(),
            t.strip_prefix(&cwd_canon)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|_| rel.to_string()),
        )
    };
    Ok((target, rel_out))
}

fn canon(path: &Path) -> PathBuf {
    if let Ok(p) = path.canonicalize() {
        return p;
    }
    let mut dir = path.parent().map(Path::to_path_buf);
    while let Some(d) = dir {
        if let Ok(c) = d.canonicalize() {
            let rest = path.strip_prefix(&d).unwrap_or(path);
            return c.join(rest);
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    path.to_path_buf()
}

fn mtime_ms(path: &Path) -> Option<u128> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
}

/// Blocking poller driving one watch stream — the relay spawns it per request
/// and aborts on interrupt/disconnect.
pub async fn poll_watch(
    cwd: PathBuf,
    rel: String,
    mut emit: impl FnMut(Value) -> bool,
) -> Result<(), WsErr> {
    let (target, rel_out) = resolve_watch(&cwd, &rel)?;
    let mut last = mtime_ms(&target);
    loop {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let now = mtime_ms(&target);
        match (last, now) {
            (Some(prev), Some(cur)) if cur != prev => {
                last = now;
                if !emit(json!({
                    "type": "changed",
                    "relativePath": rel_out,
                    "mtimeMs": cur as f64,
                })) {
                    return Ok(());
                }
            }
            (Some(_), None) => {
                last = None;
                if !emit(json!({"type": "deleted", "relativePath": rel_out})) {
                    return Ok(());
                }
            }
            (None, Some(cur)) => {
                last = Some(cur);
                if !emit(json!({
                    "type": "changed",
                    "relativePath": rel_out,
                    "mtimeMs": cur as f64,
                })) {
                    return Ok(());
                }
            }
            _ => {}
        }
    }
}

fn iso_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    crate::projects_fileops::iso8601(secs)
}
