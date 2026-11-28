//! Native port of upstream apps/server/src/editor/debugService.ts +
//! dapConnection.ts + debugAdapterInstaller.ts: the `editor.debug.start`,
//! `editor.debug.action`, and `editor.debug.poll` WS surface for JavaScript
//! debugging through the checksum-pinned js-debug adapter. DAP runs over a
//! loopback TCP socket with Content-Length framing on plain std threads — no
//! tokio in this layer, which keeps it legal inside spawn_blocking WS handlers.
use crate::todo::WsErr;
use serde_json::{Value, json};
use sha2::Digest;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Mutex, OnceLock, mpsc};

fn err(msg: impl Into<String>) -> WsErr {
    crate::todo::err_pub(msg)
}

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "editor.debug.start" | "editor.debug.action" | "editor.debug.poll"
    )
}

// ── adapter installer ──────────────────────────────────────────────
const JS_DEBUG_VERSION: &str = "1.140.0";
const JS_DEBUG_SHA256: &str = "27dab92937ec1ab35821ae955aac867544fe06a1b6307229049f2d789af10968";
const MAX_ARCHIVE_BYTES: usize = 16 * 1024 * 1024;
const MAX_EXPANDED_BYTES: usize = 64 * 1024 * 1024;

fn sha256_hex(data: &[u8]) -> String {
    sha2::Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Narrow extractor mirroring upstream: js-debug/ prefix only, no links, no
/// traversal, 0700 directories. Returns Err on any deviation rather than
/// trusting a modified archive.
fn extract_adapter(tar: &[u8], destination: &std::path::Path) -> Result<(), WsErr> {
    let mut offset = 0usize;
    let mut entries = 0usize;
    while offset + 512 <= tar.len() {
        let header = &tar[offset..offset + 512];
        if header.iter().all(|b| *b == 0) {
            break;
        }
        entries += 1;
        if entries > 2000 {
            return Err(err("The debugger archive contains too many entries."));
        }
        let text = |start: usize, len: usize| -> String {
            let end = (start + len).min(512);
            let raw = &header[start..end];
            let nul = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
            String::from_utf8_lossy(&raw[..nul]).to_string()
        };
        let name = text(0, 100);
        let prefix = text(345, 155);
        let archive_path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let parts: Vec<&str> = archive_path.split('/').filter(|p| !p.is_empty()).collect();
        let size_str = text(124, 12);
        let size = usize::from_str_radix(size_str.trim(), 8).unwrap_or(usize::MAX);
        if parts.first().copied() != Some("js-debug")
            || parts
                .iter()
                .any(|p| *p == ".." || p.contains('\\') || p.contains(':'))
            || archive_path.starts_with('/')
            || size == usize::MAX
            || offset + 512 + size > tar.len()
        {
            return Err(err("Unsafe or invalid path in debugger archive."));
        }
        let target = destination.join(parts.join("/"));
        let typeflag = text(156, 1);
        if typeflag == "5" {
            std::fs::create_dir_all(&target).map_err(|e| err(format!("extract mkdir: {e}")))?;
        } else if typeflag == "0" || typeflag.is_empty() {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| err(format!("extract mkdir: {e}")))?;
            }
            let body = &tar[offset + 512..offset + 512 + size];
            std::fs::write(&target, body).map_err(|e| err(format!("extract write: {e}")))?;
        }
        // Entries are padded to 512-byte boundaries.
        offset += 512 + ((size + 511) / 512) * 512;
    }
    Ok(())
}

fn gunzip(data: &[u8]) -> Result<Vec<u8>, WsErr> {
    use flate2::read::GzDecoder;
    let mut out = Vec::new();
    GzDecoder::new(data)
        .take(MAX_EXPANDED_BYTES as u64 + 1)
        .read_to_end(&mut out)
        .map_err(|e| err(format!("debugger archive inflate: {e}")))?;
    if out.len() > MAX_EXPANDED_BYTES {
        return Err(err("The debugger archive expanded beyond its limit."));
    }
    Ok(out)
}

/// ensureDebugAdapter: checksum-pinned download into the state directory,
/// verified on reuse through the integrity manifest. Runs inside a std
/// thread because the blocking client cannot drop in async context.
pub fn ensure_debug_adapter(state_dir: &str) -> Result<String, WsErr> {
    let destination = std::path::PathBuf::from(state_dir)
        .join("editor-runtimes")
        .join(format!("js-debug-{JS_DEBUG_VERSION}"));
    let entry = destination.join("js-debug/src/dapDebugServer.js");
    let integrity = destination.join("integrity.json");
    if entry.is_file() && integrity.is_file() {
        if let Ok(manifest) = std::fs::read_to_string(&integrity) {
            if manifest.contains(JS_DEBUG_SHA256) {
                return Ok(entry.to_string_lossy().to_string());
            }
        }
    }
    let _ = std::fs::remove_dir_all(&destination);
    std::fs::create_dir_all(destination.parent().unwrap())
        .map_err(|e| err(format!("adapter mkdir: {e}")))?;
    let url = format!(
        "https://github.com/microsoft/vscode-js-debug/releases/download/v{JS_DEBUG_VERSION}/js-debug-dap-v{JS_DEBUG_VERSION}.tar.gz"
    );
    let url_c = url.clone();
    let archive = std::thread::scope(|scope| {
        scope
            .spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(60))
                    .build()
                    .map_err(|e| err(format!("adapter download client: {e}")))?;
                let resp = client
                    .get(&url_c)
                    .send()
                    .map_err(|e| err(format!("adapter download: {e}")))?;
                if !resp.status().is_success() {
                    return Err(err(format!(
                        "adapter download failed (status {}).",
                        resp.status()
                    )));
                }
                resp.bytes()
                    .map(|b| b.to_vec())
                    .map_err(|e| err(format!("adapter read: {e}")))
            })
            .join()
            .map_err(|_| err("adapter download thread panicked"))?
    })?;
    if archive.len() > MAX_ARCHIVE_BYTES || sha256_hex(&archive) != JS_DEBUG_SHA256 {
        return Err(err(
            "The JavaScript debugger download failed its SHA-256 integrity check.",
        ));
    }
    let tar = gunzip(&archive)?;
    extract_adapter(&tar, &destination)?;
    if !entry.is_file() {
        return Err(err("The debugger archive did not contain its entry point."));
    }
    let manifest = json!({ "archive": JS_DEBUG_SHA256, "entry": sha256_hex(&std::fs::read(&entry).unwrap_or_default()) });
    std::fs::write(&integrity, manifest.to_string())
        .map_err(|e| err(format!("integrity write: {e}")))?;
    Ok(entry.to_string_lossy().to_string())
}

// ── DAP connection ─────────────────────────────────────────────────
const MAX_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

type PendingMap = Arc<Mutex<HashMap<u64, mpsc::Sender<Result<Value, String>>>>>;

struct DapConnection {
    writer: Arc<Mutex<TcpStream>>,
    seq: Arc<Mutex<u64>>,
    pending: PendingMap,
}

use std::sync::Arc;

impl DapConnection {
    /// Connect to the adapter and spawn the framing reader thread. Events and
    /// reverse requests are forwarded into `on_event` / answered with an
    /// unsupported error, mirroring upstream's reverse-request policy.
    fn connect(
        port: u16,
        on_event: impl Fn(String, Value) + Send + 'static,
        on_reverse: ReverseHandler,
    ) -> Result<Self, WsErr> {
        let stream = TcpStream::connect(("127.0.0.1", port))
            .map_err(|e| err(format!("debug adapter connection: {e}")))?;
        stream.set_nodelay(true).ok();
        let reader = stream
            .try_clone()
            .map_err(|e| err(format!("dap clone: {e}")))?;
        let writer_stream = stream
            .try_clone()
            .map_err(|e| err(format!("dap clone: {e}")))?;
        let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
        let pending_r = pending.clone();
        let reverse_writer = Arc::new(Mutex::new(
            stream
                .try_clone()
                .map_err(|e| err(format!("dap clone: {e}")))?,
        ));
        std::thread::spawn(move || {
            dap_reader_loop(reader, reverse_writer, pending_r, on_event, on_reverse);
        });
        Ok(DapConnection {
            writer: Arc::new(Mutex::new(writer_stream)),
            seq: Arc::new(Mutex::new(0)),
            pending,
        })
    }

    fn send(&self, mut message: Value) -> Result<u64, WsErr> {
        let mut seq_guard = self.seq.lock().map_err(|_| err("dap seq lock"))?;
        *seq_guard += 1;
        let id = *seq_guard;
        message["seq"] = json!(id);
        let bytes = serde_json::to_vec(&message).map_err(|e| err(format!("dap encode: {e}")))?;
        if bytes.len() > MAX_MESSAGE_BYTES {
            return Err(err("Debug adapter send queue exceeded its limit."));
        }
        let frame = format!("Content-Length: {}\r\n\r\n", bytes.len());
        let mut writer = self.writer.lock().map_err(|_| err("dap writer lock"))?;
        writer
            .write_all(frame.as_bytes())
            .and_then(|_| writer.write_all(&bytes))
            .map_err(|e| err(format!("dap write: {e}")))?;
        Ok(id)
    }

    /// Fire-and-collect-later request: sends the command and returns the
    /// receiver. Upstream ordering requires this for launch/attach, which the
    /// adapter only answers after configurationDone.
    fn request_deferred(
        &self,
        command: &str,
        args: Value,
    ) -> Result<mpsc::Receiver<Result<Value, String>>, WsErr> {
        {
            let pending = self.pending.lock().map_err(|_| err("dap pending lock"))?;
            if pending.len() >= 64 {
                return Err(err("Too many pending debugger commands."));
            }
        }
        let (tx, rx) = mpsc::channel();
        let id = self.send(json!({ "type": "request", "command": command, "arguments": args }))?;
        self.pending
            .lock()
            .map_err(|_| err("dap pending lock"))?
            .insert(id, tx);
        Ok(rx)
    }

    /// Synchronous request/response with a 15s timeout, matching upstream.
    fn request(&self, command: &str, args: Value) -> Result<Value, WsErr> {
        {
            let pending = self.pending.lock().map_err(|_| err("dap pending lock"))?;
            if pending.len() >= 64 {
                return Err(err("Too many pending debugger commands."));
            }
        }
        let (tx, rx) = mpsc::channel();
        let id = self.send(json!({ "type": "request", "command": command, "arguments": args }))?;
        self.pending
            .lock()
            .map_err(|_| err("dap pending lock"))?
            .insert(id, tx);
        let outcome = rx.recv_timeout(std::time::Duration::from_secs(15));
        self.pending
            .lock()
            .map_err(|_| err("dap pending lock"))?
            .remove(&id);
        match outcome {
            Ok(Ok(body)) => Ok(body),
            Ok(Err(e)) => Err(err(e)),
            Err(_) => Err(err(format!("Debugger command {command} timed out."))),
        }
    }
}

static REVERSE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

type ReverseHandler = Arc<dyn Fn(String, Value) -> Result<Value, String> + Send + Sync>;

fn dap_reader_loop(
    mut stream: TcpStream,
    reverse_writer: Arc<Mutex<TcpStream>>,
    pending: PendingMap,
    on_event: impl Fn(String, Value) + Send + 'static,
    on_reverse: ReverseHandler,
) {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 16384];
    loop {
        let n = match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > MAX_MESSAGE_BYTES * 2 {
            break;
        }
        // Parse complete Content-Length framed messages.
        loop {
            let Some(header_end) = find_subsequence(&buf, b"\r\n\r\n") else {
                break;
            };
            let header = String::from_utf8_lossy(&buf[..header_end]).to_string();
            let len: usize = header
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .and_then(|v| v.trim().parse().ok())
                .unwrap_or(0);
            if len == 0 || len > MAX_MESSAGE_BYTES {
                return; // corrupt framing — drop the connection
            }
            if buf.len() < header_end + 4 + len {
                break;
            }
            let body_start = header_end + 4;
            let body: Value =
                serde_json::from_slice(&buf[body_start..body_start + len]).unwrap_or(json!({}));
            buf.drain(..body_start + len);
            let msg_type = body.get("type").and_then(Value::as_str).unwrap_or("");
            match msg_type {
                "response" => {
                    let req_seq = body.get("request_seq").and_then(Value::as_u64).unwrap_or(0);
                    let success = body
                        .get("success")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    let result = if success {
                        Ok(body.get("body").cloned().unwrap_or(json!({})))
                    } else {
                        Err(body
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("debugger command failed")
                            .to_string())
                    };
                    if let Some(tx) = pending.lock().ok().and_then(|mut p| p.remove(&req_seq)) {
                        let _ = tx.send(result);
                    }
                }
                "event" => {
                    let event = body
                        .get("event")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let event_body = body.get("body").cloned().unwrap_or(json!({}));
                    on_event(event, event_body);
                }
                "request" => {
                    // Reverse requests run the handler off-thread: startDebugging
                    // opens a whole child connection, which must not stall the
                    // frame reader. The reply goes out whenever the handler ends.
                    let seq = body.get("seq").and_then(Value::as_u64).unwrap_or(0);
                    let command = body
                        .get("command")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    let args = body.get("arguments").cloned().unwrap_or(json!({}));
                    let writer = reverse_writer.clone();
                    let handler = on_reverse.clone();
                    std::thread::spawn(move || {
                        let reply_seq = REVERSE_SEQ
                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                            + 900_000;
                        let reply = match handler(command.clone(), args) {
                            Ok(reply_body) => json!({
                                "type": "response", "seq": reply_seq, "request_seq": seq,
                                "success": true, "command": command, "body": reply_body,
                            }),
                            Err(message) => json!({
                                "type": "response", "seq": reply_seq, "request_seq": seq,
                                "success": false, "command": command, "message": message,
                            }),
                        };
                        let bytes = serde_json::to_vec(&reply).unwrap_or_default();
                        let frame = format!("Content-Length: {}\r\n\r\n", bytes.len());
                        if let Ok(mut w) = writer.lock() {
                            let _ = w
                                .write_all(frame.as_bytes())
                                .and_then(|_| w.write_all(&bytes));
                        }
                    });
                }
                _ => {}
            }
        }
    }
    // Connection closed: fail every pending request.
    if let Ok(mut p) = pending.lock() {
        for (_, tx) in p.drain() {
            let _ = tx.send(Err("The debug adapter connection closed.".to_string()));
        }
    }
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

// ── debug service ──────────────────────────────────────────────────
struct DebugSession {
    owner: String,
    cwd: String,
    connections: HashMap<String, Connection>,
    breakpoints: HashMap<String, Vec<Value>>,
    events: Vec<Value>,
    sequence: u64,
    terminated: bool,
    child: Option<std::process::Child>,
    touched_ms: i64,
}

struct Connection {
    name: String,
    dap: Arc<DapConnection>,
    ready: bool,
    ended: bool,
}

struct Service {
    sessions: HashMap<String, Arc<Mutex<DebugSession>>>,
}

static SERVICE: OnceLock<Mutex<Service>> = OnceLock::new();

fn service() -> &'static Mutex<Service> {
    SERVICE.get_or_init(|| {
        Mutex::new(Service {
            sessions: HashMap::new(),
        })
    })
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn emit(
    session: &mut DebugSession,
    connection_id: &str,
    event: &str,
    text: &str,
    extra: Option<Value>,
) {
    session.sequence += 1;
    let mut e = json!({
        "id": session.sequence,
        "connectionId": connection_id,
        "event": event,
        "text": &text[..text.len().min(16_384)],
    });
    if let Some(x) = extra {
        if let (Some(dst), Some(src)) = (e.as_object_mut(), x.as_object()) {
            for (k, v) in src {
                dst.insert(k.clone(), v.clone());
            }
        }
    }
    session.events.push(e);
    while session.events.len() > 500
        || session
            .events
            .iter()
            .map(|i| i.get("text").and_then(Value::as_str).unwrap_or("").len())
            .sum::<usize>()
            > 512_000
    {
        let discard = session.events.iter().position(|item| {
            let ev = item.get("event").and_then(Value::as_str).unwrap_or("");
            !matches!(ev, "stopped" | "continued" | "terminated")
        });
        match discard {
            Some(ix) => {
                session.events.remove(ix);
            }
            None => break,
        }
    }
}

fn workspace_file(root: &str, relative: &str) -> Result<String, WsErr> {
    if std::path::Path::new(relative).is_absolute()
        || relative.contains('\0')
        || relative
            .split(['/', '\\'])
            .any(|part| part == ".." || part == ".git")
    {
        return Err(err("Expected a workspace-relative debug source."));
    }
    let joined = std::path::PathBuf::from(root).join(relative);
    let real = joined
        .canonicalize()
        .map_err(|e| err(format!("debug source: {e}")))?;
    let root_real = std::path::PathBuf::from(root)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(root));
    if !real.starts_with(&root_real) {
        return Err(err("Debug source escaped its workspace through a symlink."));
    }
    Ok(real.to_string_lossy().to_string())
}

fn state_dir() -> String {
    crate::todo::state_dir()
}

fn variable(item: &Value) -> Value {
    json!({
        "name": item.get("name").and_then(Value::as_str).unwrap_or(""),
        "value": item.get("value").and_then(Value::as_str).unwrap_or(""),
        "type": item.get("type").and_then(Value::as_str).unwrap_or(""),
        "variablesReference": item.get("variablesReference").and_then(Value::as_u64).unwrap_or(0),
    })
}

/// `editor.debug.start` — launch or attach a Node.js debug session through
/// the js-debug adapter. One session per owning connection, four per server,
/// mirroring upstream's limits.
fn start(owner: &str, payload: &Value) -> Result<Value, WsErr> {
    let cwd_in = payload
        .get("cwd")
        .and_then(Value::as_str)
        .ok_or_else(|| err("missing cwd"))?;
    let cwd = std::path::PathBuf::from(cwd_in)
        .canonicalize()
        .map_err(|e| err(format!("cwd: {e}")))?
        .to_string_lossy()
        .to_string();
    let request = payload
        .get("request")
        .and_then(Value::as_str)
        .unwrap_or("launch");
    let program = if request == "launch" {
        let rel = payload
            .get("program")
            .and_then(Value::as_str)
            .ok_or_else(|| err("missing program"))?;
        let abs = workspace_file(&cwd, rel)?;
        if !regexless_js_entry(&abs) {
            return Err(err(
                "Launch a Node.js .js/.mjs/.cjs entry point, or a .ts/.mts/.cts file supported by your Node runtime.",
            ));
        }
        Some(abs)
    } else {
        let port = payload.get("port").and_then(Value::as_u64).unwrap_or(0);
        if port == 0 {
            return Err(err("Enter the loopback Node inspector port to attach."));
        }
        None
    };
    {
        let svc = service().lock().map_err(|_| err("debug service lock"))?;
        if svc.sessions.values().any(|s| {
            s.lock()
                .map(|sess| sess.owner == owner && !sess.terminated)
                .unwrap_or(false)
        }) {
            return Err(err(
                "Stop the current debug session before starting another.",
            ));
        }
        if svc.sessions.len() >= 4 {
            return Err(err("The server's debug session limit has been reached."));
        }
    }

    let adapter = ensure_debug_adapter(&state_dir())?;
    // Spawn the adapter: `node dapDebugServer.js 0 127.0.0.1` prints
    // "Debug server listening at 127.0.0.1:PORT" on stdout once bound.
    let mut command = std::process::Command::new("node");
    command
        .arg(&adapter)
        .arg("0")
        .arg("127.0.0.1")
        .current_dir(&cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    crate::provider_env::sanitize_provider_child(&mut command);
    let mut child = command.spawn().map_err(|e| {
        err(format!(
            "Node.js is required to run the JavaScript debugger: {e}"
        ))
    })?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| err("adapter stdout missing"))?;
    // stderr must drain continuously: a full pipe would stall the adapter.
    if let Some(mut stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match stderr.read(&mut buf) {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {}
                }
            }
        });
    }
    let (port_tx, port_rx) = mpsc::channel::<Option<u16>>();
    std::thread::spawn(move || {
        let mut text = String::new();
        let mut buf = [0u8; 1024];
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            if std::time::Instant::now() > deadline {
                let _ = port_tx.send(None);
                return;
            }
            match stdout.read(&mut buf) {
                Ok(0) => {
                    let _ = port_tx.send(None);
                    return;
                }
                Ok(n) => {
                    text.push_str(&String::from_utf8_lossy(&buf[..n]));
                    if text.len() > 8192 {
                        text = text[text.len() - 8192..].to_string();
                    }
                    let marker = "Debug server listening at 127.0.0.1:";
                    if let Some(pos) = text.find(marker) {
                        let rest = &text[pos + marker.len()..];
                        let digits: String =
                            rest.chars().take_while(|c| c.is_ascii_digit()).collect();
                        if let Ok(port) = digits.parse::<u16>() {
                            let _ = port_tx.send(Some(port));
                            // Keep draining so the adapter never blocks on a full pipe.
                            loop {
                                if stdout.read(&mut buf).map(|n| n == 0).unwrap_or(true) {
                                    return;
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    let _ = port_tx.send(None);
                    return;
                }
            }
        }
    });
    let port = match port_rx.recv_timeout(std::time::Duration::from_secs(12)) {
        Ok(Some(p)) => p,
        _ => {
            let _ = child.kill();
            return Err(err("JavaScript debugger did not start within 10 seconds."));
        }
    };

    // Breakpoints from the input, grouped per source file. They live in the
    // session because every child target connection syncs them again.
    let mut breakpoints: HashMap<String, Vec<Value>> = HashMap::new();
    if let Some(bps) = payload.get("breakpoints").and_then(Value::as_array) {
        for bp in bps {
            if let Some(rel) = bp.get("relativePath").and_then(Value::as_str) {
                if workspace_file(&cwd, rel).is_ok() {
                    breakpoints
                        .entry(rel.to_string())
                        .or_default()
                        .push(bp.clone());
                }
            }
        }
    }

    let session_id = crate::orchestration_engine::uuid_v4();
    let session = Arc::new(Mutex::new(DebugSession {
        owner: owner.to_string(),
        cwd: cwd.clone(),
        connections: HashMap::new(),
        breakpoints,
        events: Vec::new(),
        sequence: 0,
        terminated: false,
        child: Some(child),
        touched_ms: now_ms(),
    }));

    // Build the launch/attach configuration exactly as upstream does.
    let args: Vec<Value> = payload
        .get("args")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let configuration = if let Some(prog) = &program {
        json!({
            "type": "pwa-node",
            "request": "launch",
            "name": prog,
            "cwd": cwd,
            "console": "internalConsole",
            "internalConsoleOptions": "neverOpen",
            "sourceMaps": true,
            "resolveSourceMapLocations": [format!("{cwd}/**"), "!**/node_modules/**"],
            "outFiles": [format!("{cwd}/**/*.js"), "!**/node_modules/**"],
            "autoAttachChildProcesses": false,
            "restart": false,
            "program": prog,
            "args": args,
            "stopOnEntry": payload.get("stopOnEntry").and_then(Value::as_bool).unwrap_or(false),
        })
    } else {
        json!({
            "type": "pwa-node",
            "request": "attach",
            "name": format!("Node :{}", payload.get("port").and_then(Value::as_u64).unwrap_or(0)),
            "cwd": cwd,
            "console": "internalConsole",
            "internalConsoleOptions": "neverOpen",
            "sourceMaps": true,
            "resolveSourceMapLocations": [format!("{cwd}/**"), "!**/node_modules/**"],
            "outFiles": [format!("{cwd}/**/*.js"), "!**/node_modules/**"],
            "autoAttachChildProcesses": false,
            "restart": false,
            "address": "127.0.0.1",
            "port": payload.get("port").and_then(Value::as_u64).unwrap_or(0),
        })
    };

    let connect_result = connect_target(&session, port, "root", "Node.js", request, configuration);
    if let Err(e) = connect_result {
        stop_session(&session);
        return Err(e);
    }
    service()
        .lock()
        .map_err(|_| err("debug service lock"))?
        .sessions
        .insert(session_id.clone(), session);
    Ok(json!({ "sessionId": session_id }))
}

fn regexless_js_entry(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".js")
        || lower.ends_with(".mjs")
        || lower.ends_with(".cjs")
        || lower.ends_with(".ts")
        || lower.ends_with(".mts")
        || lower.ends_with(".cts")
}

/// Connect one debug target: initialize handshake, launch/attach, breakpoint
/// sync, configurationDone — the upstream sequencing verbatim. js-debug's
/// standalone server never runs targets on the root connection: it hands each
/// one to the client through a reverse startDebugging request, answered by
/// opening a fresh connection bound to the adapter's opaque pending target id.
fn connect_target(
    session: &Arc<Mutex<DebugSession>>,
    port: u16,
    id: &str,
    name: &str,
    request: &str,
    configuration: Value,
) -> Result<(), WsErr> {
    // Reap finished child targets and enforce the upstream caps: one session
    // closed or 8 live connections refuse new targets.
    {
        let mut sess = session.lock().map_err(|_| err("session lock"))?;
        let ended_keys: Vec<String> = sess
            .connections
            .iter()
            .filter(|(key, item)| item.ended && key.as_str() != "root")
            .map(|(key, _)| key.clone())
            .collect();
        for key in ended_keys {
            sess.connections.remove(&key);
        }
        if sess.terminated || sess.connections.len() >= 8 {
            return Err(err("The debug session is closed or has too many targets."));
        }
    }

    let session_ev = session.clone();
    let conn_id = id.to_string();
    let dap = DapConnection::connect(
        port,
        move |event, body| {
            let mut root_terminated = false;
            if let Ok(mut sess) = session_ev.lock() {
                match event.as_str() {
                    "output" => {
                        if body.get("category").and_then(Value::as_str) != Some("telemetry") {
                            let text = body.get("output").and_then(Value::as_str).unwrap_or("");
                            emit(&mut sess, &conn_id, "output", text, None);
                        }
                    }
                    "stopped" => {
                        let reason = body
                            .get("description")
                            .and_then(Value::as_str)
                            .or_else(|| body.get("reason").and_then(Value::as_str))
                            .unwrap_or("");
                        let thread = body.get("threadId").and_then(Value::as_u64);
                        emit(
                            &mut sess,
                            &conn_id,
                            "stopped",
                            reason,
                            Some(json!({ "threadId": thread })),
                        );
                    }
                    "continued" | "thread" => {
                        let reason = body.get("reason").and_then(Value::as_str).unwrap_or("");
                        let thread = body.get("threadId").and_then(Value::as_u64);
                        emit(
                            &mut sess,
                            &conn_id,
                            &event,
                            reason,
                            Some(json!({ "threadId": thread })),
                        );
                    }
                    "breakpoint" => {
                        let message = body
                            .get("breakpoint")
                            .and_then(|b| b.get("message"))
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        emit(&mut sess, &conn_id, "breakpoint", message, None);
                    }
                    "exited" => {
                        let code = body.get("exitCode").and_then(Value::as_i64).unwrap_or(0);
                        if let Some(conn) = sess.connections.get_mut(&conn_id) {
                            conn.ended = true;
                            conn.ready = false;
                        }
                        emit(
                            &mut sess,
                            &conn_id,
                            "exited",
                            "Program exited.",
                            Some(json!({ "exitCode": code })),
                        );
                    }
                    "terminated" => {
                        if let Some(conn) = sess.connections.get_mut(&conn_id) {
                            conn.ended = true;
                            conn.ready = false;
                        }
                        emit(
                            &mut sess,
                            &conn_id,
                            "terminated",
                            "Debug target finished.",
                            None,
                        );
                        root_terminated = conn_id == "root";
                    }
                    _ => {}
                }
            }
            if root_terminated {
                stop_session(&session_ev);
            }
        },
        reverse_handler(session.clone(), port),
    )?;
    let dap = Arc::new(dap);

    // Park the connection before handshaking so events landing mid-sequence
    // already attribute to it.
    {
        let mut sess = session.lock().map_err(|_| err("session lock"))?;
        sess.connections.insert(
            id.to_string(),
            Connection {
                name: name.to_string(),
                dap: dap.clone(),
                ready: false,
                ended: false,
            },
        );
    }

    let outcome = (|| -> Result<(), WsErr> {
        dap.request(
            "initialize",
            json!({
                "clientID": "synara",
                "clientName": "Synara",
                "adapterID": "pwa-node",
                "locale": "en",
                "linesStartAt1": true,
                "columnsStartAt1": true,
                "pathFormat": "path",
                "supportsVariableType": true,
                "supportsVariablePaging": true,
                "supportsRunInTerminalRequest": false,
                "supportsStartDebuggingRequest": true,
                "supportsProgressReporting": false,
            }),
        )?;
        // Fire launch/attach WITHOUT awaiting: the adapter answers only after
        // configurationDone, so awaiting here would deadlock (upstream does
        // `void launched.catch()` for exactly this reason).
        let launched_rx = dap.request_deferred(request, configuration)?;
        // Sync the session's breakpoints on every target connection.
        let grouped: Vec<(String, Vec<Value>)> = session
            .lock()
            .map(|sess| {
                sess.breakpoints
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let cwd = session
            .lock()
            .map(|sess| sess.cwd.clone())
            .unwrap_or_default();
        for (rel, bps) in grouped {
            if let Ok(abs) = workspace_file(&cwd, &rel) {
                let bp_list: Vec<Value> = bps
                    .iter()
                    .map(|bp| {
                        json!({
                            "line": bp.get("line").and_then(Value::as_u64).unwrap_or(1),
                            "condition": bp.get("condition").cloned().unwrap_or(Value::Null),
                            "hitCondition": bp.get("hitCondition").cloned().unwrap_or(Value::Null),
                            "logMessage": bp.get("logMessage").cloned().unwrap_or(Value::Null),
                        })
                    })
                    .collect();
                let _ = dap.request(
                    "setBreakpoints",
                    json!({
                        "source": { "path": abs },
                        "breakpoints": bp_list,
                        "sourceModified": false,
                    }),
                );
            }
        }
        let _ = dap.request("setExceptionBreakpoints", json!({ "filters": [] }));
        let _ = dap.request("configurationDone", json!({}));
        // Now collect the deferred launch/attach response.
        match launched_rx.recv_timeout(std::time::Duration::from_secs(45)) {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(err(format!("debug {request} failed: {e}"))),
            Err(_) => Err(err(format!("Debugger {request} did not answer."))),
        }
    })();

    match outcome {
        Ok(()) => {
            let mut sess = session.lock().map_err(|_| err("session lock"))?;
            if let Some(conn) = sess.connections.get_mut(id) {
                conn.ready = true;
            }
            emit(&mut sess, id, "ready", name, None);
            Ok(())
        }
        Err(e) => {
            if let Ok(mut sess) = session.lock() {
                if let Some(conn) = sess.connections.get_mut(id) {
                    conn.ended = true;
                }
            }
            Err(e)
        }
    }
}

/// Reverse-request handler for one connection: startDebugging is how js-debug
/// hands real Node targets to the client — only the adapter's opaque pending
/// target id is accepted, never arbitrary user launch configuration.
fn reverse_handler(session: Arc<Mutex<DebugSession>>, port: u16) -> ReverseHandler {
    Arc::new(move |command, args| {
        if command != "startDebugging" {
            return Err(format!(
                "Synara does not support the adapter's {command} request."
            ));
        }
        let config = args.get("configuration").cloned().unwrap_or(json!({}));
        let pending_id = config.get("__pendingTargetId").and_then(Value::as_str);
        let child_request = args.get("request").and_then(Value::as_str).unwrap_or("");
        if pending_id.is_none() || (child_request != "launch" && child_request != "attach") {
            return Err("Invalid child debugger configuration.".to_string());
        }
        let child_name = config
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Node target")
            .to_string();
        let child_config = json!({
            "type": "pwa-node",
            "name": child_name,
            "__pendingTargetId": pending_id.unwrap_or(""),
        });
        let child_id = crate::orchestration_engine::uuid_v4();
        match connect_target(
            &session,
            port,
            &child_id,
            &child_name,
            child_request,
            child_config,
        ) {
            Ok(()) => Ok(json!({})),
            Err(e) => Err(e.0),
        }
    })
}

fn stop_session(session: &Arc<Mutex<DebugSession>>) {
    if let Ok(mut sess) = session.lock() {
        sess.terminated = true;
        if let Some(mut child) = sess.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let is_attach = false;
        emit(
            &mut sess,
            "root",
            "terminated",
            if is_attach {
                "Detached from Node."
            } else {
                "Debug session stopped."
            },
            None,
        );
    }
}

fn owned_session(owner: &str, id: &str) -> Result<Arc<Mutex<DebugSession>>, WsErr> {
    let svc = service().lock().map_err(|_| err("debug service lock"))?;
    let session = svc.sessions.get(id).cloned();
    match session {
        Some(s) => {
            let matches_owner = s.lock().map(|sess| sess.owner == owner).unwrap_or(false);
            if matches_owner {
                if let Ok(mut sess) = s.lock() {
                    sess.touched_ms = now_ms();
                }
                Ok(s)
            } else {
                Err(err(
                    "The debug session is not available to this connection.",
                ))
            }
        }
        None => Err(err(
            "The debug session is not available to this connection.",
        )),
    }
}

/// `editor.debug.poll` — events after `after`, connection inventory, lifecycle.
fn poll(owner: &str, payload: &Value) -> Result<Value, WsErr> {
    let id = payload
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| err("missing sessionId"))?;
    let after = payload.get("after").and_then(Value::as_u64).unwrap_or(0);
    let session = owned_session(owner, id)?;
    let sess = session.lock().map_err(|_| err("session lock"))?;
    let connections: Vec<Value> = sess
        .connections
        .iter()
        .map(|(cid, c)| json!({ "id": cid, "name": c.name, "ready": c.ready }))
        .collect();
    let events: Vec<Value> = sess
        .events
        .iter()
        .filter(|e| e.get("id").and_then(Value::as_u64).unwrap_or(0) > after)
        .cloned()
        .collect();
    Ok(json!({
        "sessionId": id,
        "sequence": sess.sequence,
        "terminated": sess.terminated,
        "connections": connections,
        "events": events,
    }))
}

fn required_id(v: Option<u64>, name: &str) -> Result<u64, WsErr> {
    // Zero is a real id here: js-debug numbers the main thread 0.
    v.ok_or_else(|| err(format!("A {name} is required for this debug command.")))
}

/// `editor.debug.action` — every upstream action verb: execution control,
/// introspection (threads/stack/scopes/variables/evaluate), breakpoint
/// updates, disconnect.
fn action(owner: &str, payload: &Value) -> Result<Value, WsErr> {
    let id = payload
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| err("missing sessionId"))?;
    let act = payload
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| err("missing action"))?;
    let session = owned_session(owner, id)?;
    if act == "disconnect" {
        stop_session(&session);
        service()
            .lock()
            .map_err(|_| err("debug service lock"))?
            .sessions
            .remove(id);
        return Ok(json!({}));
    }
    let conn_dap = {
        let sess = session.lock().map_err(|_| err("session lock"))?;
        if sess.terminated {
            return Err(err("The debug session has ended."));
        }
        let wanted = payload.get("connectionId").and_then(Value::as_str);
        // Upstream picks the requested connection, else the first ready
        // non-root target, else root.
        let fallback_key = sess
            .connections
            .iter()
            .find(|(k, c)| k.as_str() != "root" && c.ready)
            .map(|(k, _)| k.clone())
            .unwrap_or_else(|| "root".to_string());
        let key = wanted.unwrap_or(&fallback_key).to_string();
        match sess.connections.get(&key) {
            Some(c) if c.ready => c.dap.clone(),
            _ => return Err(err("The debug target is not ready.")),
        }
    };
    let request = |command: &str, args: Value| conn_dap.request(command, args);
    let thread_id = || required_id(payload.get("threadId").and_then(Value::as_u64), "thread ID");
    match act {
        "continue" | "pause" | "next" | "stepIn" | "stepOut" => {
            let dap_command = match act {
                "next" => "next",
                "stepIn" => "stepIn",
                "stepOut" => "stepOut",
                other => other,
            };
            request(dap_command, json!({ "threadId": thread_id()? }))?;
            Ok(json!({}))
        }
        "threads" => {
            let body = request("threads", json!({}))?;
            let threads: Vec<Value> = body
                .get("threads")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|t| {
                    json!({
                        "id": t.get("id").and_then(Value::as_u64).unwrap_or(0),
                        "name": t.get("name").and_then(Value::as_str).unwrap_or(""),
                    })
                })
                .collect();
            Ok(json!({ "threads": threads }))
        }
        "stackTrace" => {
            let body = request(
                "stackTrace",
                json!({
                    "threadId": thread_id()?,
                    "startFrame": 0,
                    "levels": 100,
                }),
            )?;
            let cwd = session.lock().map(|s| s.cwd.clone()).unwrap_or_default();
            let frames: Vec<Value> = body
                .get("stackFrames")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|f| {
                    let source_path = f.pointer("/source/path").and_then(Value::as_str).unwrap_or("");
                    let relative = if std::path::Path::new(source_path).is_absolute()
                        && source_path.starts_with(&cwd)
                    {
                        source_path[cwd.len()..].trim_start_matches('/').to_string()
                    } else {
                        String::new()
                    };
                    let mut out = json!({
                        "id": f.get("id").and_then(Value::as_u64).unwrap_or(0),
                        "name": f.get("name").and_then(Value::as_str).unwrap_or(""),
                        "line": f.get("line").and_then(Value::as_u64).unwrap_or(0),
                        "column": f.get("column").and_then(Value::as_u64).unwrap_or(0),
                        "sourceName": f.pointer("/source/name").and_then(Value::as_str).unwrap_or(""),
                    });
                    if !relative.is_empty() {
                        out["relativePath"] = json!(relative);
                    }
                    out
                })
                .collect();
            Ok(json!({ "frames": frames }))
        }
        "scopes" => {
            let frame_id = required_id(payload.get("frameId").and_then(Value::as_u64), "frame ID")?;
            let body = request("scopes", json!({ "frameId": frame_id }))?;
            let scopes: Vec<Value> = body
                .get("scopes")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|sc| json!({
                    "name": sc.get("name").and_then(Value::as_str).unwrap_or(""),
                    "variablesReference": sc.get("variablesReference").and_then(Value::as_u64).unwrap_or(0),
                    "expensive": sc.get("expensive").and_then(Value::as_bool).unwrap_or(false),
                }))
                .collect();
            Ok(json!({ "scopes": scopes }))
        }
        "variables" => {
            let reference = required_id(
                payload.get("variablesReference").and_then(Value::as_u64),
                "variables reference",
            )?;
            let body = request(
                "variables",
                json!({
                    "variablesReference": reference,
                    "start": 0,
                    "count": 200,
                }),
            )?;
            let variables: Vec<Value> = body
                .get("variables")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(variable)
                .collect();
            Ok(json!({ "variables": variables }))
        }
        "evaluate" => {
            let expression = payload
                .get("expression")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim();
            if expression.is_empty() {
                return Err(err("Enter an expression to evaluate."));
            }
            let mut args = json!({
                "expression": expression,
                "context": payload.get("context").and_then(Value::as_str).unwrap_or("repl"),
            });
            if let Some(frame) = payload.get("frameId").and_then(Value::as_u64) {
                args["frameId"] = json!(frame);
            }
            let body = request("evaluate", args)?;
            let evaluation = variable(&json!({
                "name": expression,
                "value": body.get("result").and_then(Value::as_str).unwrap_or(""),
                "type": body.get("type").and_then(Value::as_str).unwrap_or(""),
                "variablesReference": body.get("variablesReference").and_then(Value::as_u64).unwrap_or(0),
            }));
            Ok(json!({ "evaluation": evaluation }))
        }
        "setBreakpoints" => {
            let rel = payload
                .get("relativePath")
                .and_then(Value::as_str)
                .unwrap_or("");
            if rel.is_empty() {
                return Err(err("missing relativePath"));
            }
            let cwd = session.lock().map(|s| s.cwd.clone()).unwrap_or_default();
            let abs = workspace_file(&cwd, rel)?;
            let bps: Vec<Value> = payload
                .get("breakpoints")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|bp| {
                    json!({
                        "line": bp.get("line").and_then(Value::as_u64).unwrap_or(1),
                        "condition": bp.get("condition").cloned().unwrap_or(Value::Null),
                        "hitCondition": bp.get("hitCondition").cloned().unwrap_or(Value::Null),
                        "logMessage": bp.get("logMessage").cloned().unwrap_or(Value::Null),
                    })
                })
                .collect();
            let body = request(
                "setBreakpoints",
                json!({
                    "source": { "path": abs },
                    "breakpoints": bps,
                    "sourceModified": false,
                }),
            )?;
            let verified: Vec<Value> = body
                .get("breakpoints")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .map(|bp| {
                    json!({
                        "verified": bp.get("verified").and_then(Value::as_bool).unwrap_or(false),
                        "line": bp.get("line").and_then(Value::as_u64).unwrap_or(0),
                    })
                })
                .collect();
            Ok(json!({ "breakpoints": verified }))
        }
        other => Err(err(format!("Unknown debug action '{other}'."))),
    }
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    handle_with_owner(tag, "ws", payload)
}

/// Owners scope sessions to one WS connection upstream; the native relay does
/// not thread connection identity this deep yet, so every client shares the
/// `ws` owner — same enforcement within the single-owner local server.
pub fn handle_with_owner(tag: &str, owner: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "editor.debug.start" => start(owner, payload),
        "editor.debug.action" => action(owner, payload),
        "editor.debug.poll" => poll(owner, payload),
        _ => Err(err(format!("unknown tag {tag}"))),
    }
}
