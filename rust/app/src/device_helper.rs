//! JSON-RPC stdio client for the native device helper. The helper is a Swift
//! binary compiled on demand from apps/server/native/device-helper — private
//! frameworks mean it cannot ship prebuilt, so the same source the TypeScript
//! server compiles is reused here unchanged. Protocol: newline-delimited
//! JSON-RPC 2.0 with integer ids on the control channel, a `ready`
//! notification at startup, coordinates normalized 0..1 against the geometry
//! `attach` returns.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};

struct HelperProcess {
    child: Child,
    stdin: ChildStdin,
    pending: std::sync::Arc<Mutex<HashMap<u64, mpsc::Sender<Value>>>>,
    next_id: AtomicU64,
}

static HELPER: OnceLock<Mutex<Option<HelperProcess>>> = OnceLock::new();

fn cell() -> &'static Mutex<Option<HelperProcess>> {
    HELPER.get_or_init(|| Mutex::new(None))
}

fn helper_binary() -> Option<String> {
    // Prebuilt output of build.sh; the Rust backend resolves the same repo-
    // relative path the TypeScript server compiles into.
    if let Ok(p) = std::env::var("SYNARA_DEVICE_HELPER") {
        if std::path::Path::new(&p).exists() {
            return Some(p);
        }
    }
    for cand in [
        "/tmp/devhelper-build/synara-device-helper",
        "apps/server/native/device-helper/build/synara-device-helper",
    ] {
        if std::path::Path::new(cand).exists() {
            return Some(cand.to_string());
        }
    }
    // Repo-relative fallback: walk up from the executable looking for the
    // helper build output.
    let mut dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
    for _ in 0..6 {
        let cand = dir.join("apps/server/native/device-helper/build/synara-device-helper");
        if cand.exists() {
            return Some(cand.to_string_lossy().to_string());
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

fn spawn_helper() -> Result<HelperProcess, String> {
    let binary = helper_binary().ok_or_else(|| {
        "device helper binary not found; build apps/server/native/device-helper".to_string()
    })?;
    let mut child = Command::new(&binary)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn helper: {e}"))?;
    let stdin = child.stdin.take().ok_or("helper stdin unavailable")?;
    let stdout = child.stdout.take().ok_or("helper stdout unavailable")?;
    let pending = std::sync::Arc::new(Mutex::new(HashMap::<u64, mpsc::Sender<Value>>::new()));
    let pending2 = pending.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let Ok(line) = line else { break };
            let Ok(msg) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                if let Ok(mut map) = pending2.lock() {
                    if let Some(tx) = map.remove(&id) {
                        let _ = tx.send(msg);
                    }
                }
            }
        }
        // Process exited: fail every pending request so callers do not hang.
        if let Ok(mut map) = pending2.lock() {
            for (_, tx) in map.drain() {
                let _ = tx.send(json!({"error": {"message": "helper exited"}}));
            }
        }
    });
    Ok(HelperProcess {
        child,
        stdin,
        pending,
        next_id: AtomicU64::new(1),
    })
}

/// One JSON-RPC round trip against the shared helper process, spawning it on
/// first use. A dead helper is respawned once.
pub fn request(method: &str, params: Value) -> Result<Value, String> {
    for _attempt in 0..2 {
        let mut guard = cell().lock().map_err(|_| "helper lock poisoned")?;
        if guard.is_none() {
            match spawn_helper() {
                Ok(p) => *guard = Some(p),
                Err(e) => return Err(e),
            }
        }
        let proc = guard.as_mut().unwrap();
        let id = proc.next_id.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel();
        proc.pending
            .lock()
            .map_err(|_| "pending lock poisoned")?
            .insert(id, tx);
        let line = json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});
        let write_result = proc
            .stdin
            .write_all(line.to_string().as_bytes())
            .and_then(|_| proc.stdin.write_all(b"\n"))
            .and_then(|_| proc.stdin.flush());
        if write_result.is_err() {
            *guard = None; // respawn next attempt
            continue;
        }
        drop(guard);
        match rx.recv_timeout(std::time::Duration::from_secs(15)) {
            Ok(msg) => {
                if let Some(err) = msg.get("error") {
                    let text = err
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("helper error");
                    return Err(text.to_string());
                }
                return Ok(msg.get("result").cloned().unwrap_or(Value::Null));
            }
            Err(_) => return Err(format!("helper request {method} timed out")),
        }
    }
    Err("helper unavailable".to_string())
}

/// Attach the helper to one simulator; returns its geometry for coordinate
/// normalization. One attachment at a time, per the helper's protocol.
pub fn attach(udid: &str) -> Result<Value, String> {
    request("attach", json!({ "udid": udid }))
}

pub fn ping() -> Result<Value, String> {
    request("ping", json!({}))
}

#[cfg(test)]
mod tests {
    #[test]
    fn binary_resolution_does_not_panic() {
        let _ = super::helper_binary();
    }
}
