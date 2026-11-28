//! `editor.intelligence` — TypeScript language service bridge. Upstream runs
//! the compiler service in a worker thread; the Rust port runs the same
//! service as a Node subprocess (one per workspace root) speaking JSON lines
//! on stdio. The worker script is the JavaScript port of upstream
//! `typescriptService.ts`, embedded and materialized under the state dir.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Mutex, OnceLock};

use serde_json::Value;

use crate::todo::WsErr;

const WORKER_JS: &str = include_str!("../editor/typescript_worker.js");
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

pub fn handles(tag: &str) -> bool {
    tag == "editor.intelligence"
}

struct Worker {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<Value>,
    next_id: u64,
}

impl Worker {
    fn spawn(workspace: &str) -> Result<Self, WsErr> {
        let script = materialize_worker()?;
        let node = std::env::var("SYNARA_NODE")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "node".to_string());
        let mut child = Command::new(node)
            .arg(&script)
            .arg(workspace)
            .env("SYNARA_TS_WORKSPACE", workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| err(format!("editor worker spawn failed: {e}")))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| err("editor worker stdout missing"))?;
        let (tx, rx) = channel::<Value>();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if let Ok(value) = serde_json::from_str::<Value>(&line) {
                    if tx.send(value).is_err() {
                        break;
                    }
                }
            }
        });
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| err("editor worker stdin missing"))?;
        let worker = Worker {
            child,
            stdin,
            rx,
            next_id: 0,
        };
        // The init frame carries early failures (missing typescript, bad root).
        match worker.recv_frame()? {
            value if value.get("ok").and_then(Value::as_bool) == Some(true) => Ok(worker),
            value => Err(err(format!(
                "editor worker failed to start: {}",
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown startup failure")
            ))),
        }
    }

    fn recv_frame(&self) -> Result<Value, WsErr> {
        self.rx
            .recv_timeout(REQUEST_TIMEOUT)
            .map_err(|_| err("editor worker timed out"))
    }

    fn request(&mut self, input: &Value) -> Result<Value, WsErr> {
        self.next_id += 1;
        let id = format!("req-{}", self.next_id);
        let frame = serde_json::json!({ "id": id, "input": input });
        let mut line = serde_json::to_string(&frame).map_err(|e| err(format!("encode: {e}")))?;
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .and_then(|_| self.stdin.flush())
            .map_err(|e| err(format!("editor worker write failed: {e}")))?;
        loop {
            let value = self.recv_frame()?;
            if value.get("id").and_then(Value::as_str) == Some(id.as_str()) {
                if let Some(message) = value.get("error").and_then(Value::as_str) {
                    return Err(err(message.to_string()));
                }
                return Ok(value.get("result").cloned().unwrap_or(Value::Null));
            }
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn workers() -> &'static Mutex<HashMap<String, Worker>> {
    static WORKERS: OnceLock<Mutex<HashMap<String, Worker>>> = OnceLock::new();
    WORKERS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn materialize_worker() -> Result<String, WsErr> {
    let dir = format!("{}/editor", crate::todo::state_dir());
    std::fs::create_dir_all(&dir).map_err(|e| err(format!("editor dir: {e}")))?;
    let path = format!("{dir}/typescript_worker.js");
    // Rewrite only when the embedded script changed so the file stays stable
    // for debugging and node does not recompile it on every request.
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    if current != WORKER_JS {
        std::fs::write(&path, WORKER_JS).map_err(|e| err(format!("editor worker write: {e}")))?;
    }
    Ok(path)
}

fn with_worker<T>(
    workspace: &str,
    f: impl FnOnce(&mut Worker) -> Result<T, WsErr>,
) -> Result<T, WsErr> {
    let mut map = workers()
        .lock()
        .map_err(|_| err("editor workers lock poisoned"))?;
    // Respawn a dead worker instead of failing every later request.
    let alive = map
        .get(workspace)
        .map(|w| w.child.id() > 0)
        .unwrap_or(false);
    if !alive {
        map.remove(workspace);
    }
    if !map.contains_key(workspace) {
        let worker = Worker::spawn(workspace)?;
        map.insert(workspace.to_string(), worker);
    }
    let worker = map
        .get_mut(workspace)
        .ok_or_else(|| err("editor worker missing"))?;
    f(worker)
}

pub fn handle(_tag: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    let cwd = payload
        .get("cwd")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err("invalid `cwd`"))?;
    with_worker(cwd, |worker| worker.request(payload)).map(Some)
}

/// Shutdown hook: terminate every worker so no compiler process outlives the
/// server, mirroring upstream service.dispose().
pub fn shutdown_workers() {
    if let Some(map) = workers().lock().ok().as_mut() {
        map.clear();
    }
}

/// Test seam: is the typescript package resolvable for a workspace?
#[allow(dead_code)]
pub fn typescript_probe(workspace: &str) -> bool {
    with_worker(workspace, |_| Ok(true)).is_ok()
}
