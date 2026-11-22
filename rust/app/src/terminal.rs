//! `terminal.*` — native PTY sessions owned by this server.
//!
//! Sessions live keyed by `threadId:terminalId`. Each spawns the user's
//! login shell in a pty (portable-pty), drains output into a bounded history
//! and broadcasts the `TerminalEvent` union on a global channel that
//! `terminal.subscribeEvents` replays to subscribers.

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;

use crate::todo::WsErr;

const DEFAULT_TERMINAL_ID: &str = "default";
const MIN_COLS: u16 = 20;
const MAX_COLS: u16 = 2000;
const MIN_ROWS: u16 = 5;
const MAX_ROWS: u16 = 1000;
const WRITE_MAX: usize = 65_536;
const HISTORY_MAX: usize = 1_048_576;
const EVENT_BUFFER: usize = 256;
const ACK_MAX: u64 = 8_388_608;

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Compact ISO-8601 UTC; sub-second precision is not part of the contract.
    let days = secs / 86_400;
    let secs = secs % 86_400;
    let (h, m, s) = (secs / 3600, secs / 60 % 60, secs % 60);
    // civil-from-days (Howard Hinnant).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = if mo <= 2 { y + 1 } else { y };
    format!("{yr:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

pub struct Session {
    thread_id: String,
    terminal_id: String,
    cwd: Mutex<String>,
    env: Mutex<Vec<(String, String)>>,
    cols: Mutex<u16>,
    rows: Mutex<u16>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send>>,
    pid: u32,
    history: Mutex<String>,
    status: Mutex<&'static str>,
    exit: Mutex<(Option<i64>, Option<i64>)>,
    stream_output: AtomicBool,
    retired: AtomicBool,
    updated_at: Mutex<String>,
}

impl Session {
    fn key(&self) -> String {
        format!("{}:{}", self.thread_id, self.terminal_id)
    }

    fn snapshot(&self) -> Value {
        let status = *self.status.lock().unwrap();
        let (code, signal) = *self.exit.lock().unwrap();
        json!({
            "threadId": self.thread_id,
            "terminalId": self.terminal_id,
            "cwd": self.cwd.lock().unwrap().clone(),
            "status": status,
            "pid": if status == "exited" { Value::Null } else { json!(self.pid) },
            "history": self.history.lock().unwrap().clone(),
            "exitCode": code.map(Value::from).unwrap_or(Value::Null),
            "exitSignal": signal.map(Value::from).unwrap_or(Value::Null),
            "updatedAt": self.updated_at.lock().unwrap().clone(),
        })
    }

    fn emit(&self, mut event: Value) {
        if self.retired.load(Ordering::Relaxed) {
            return;
        }
        *self.updated_at.lock().unwrap() = now_iso();
        let obj = event.as_object_mut().unwrap();
        obj.insert("threadId".into(), json!(self.thread_id));
        obj.insert("terminalId".into(), json!(self.terminal_id));
        obj.insert("createdAt".into(), json!(now_iso()));
        emit(event);
    }
}

pub struct Store {
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    events: broadcast::Sender<Value>,
}

static STORE: LazyLock<Store> = LazyLock::new(|| {
    let (events, _) = broadcast::channel(EVENT_BUFFER);
    Store {
        sessions: Mutex::new(HashMap::new()),
        events,
    }
});

fn emit(event: Value) {
    let _ = STORE.events.send(event);
}

pub fn subscribe() -> broadcast::Receiver<Value> {
    STORE.events.subscribe()
}

fn string_field<'a>(payload: &'a Value, key: &str) -> Result<&'a str, WsErr> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| WsErr(format!("{key} is required"), Some("VALIDATION")))
}

fn session_key(payload: &Value) -> Result<(String, String), WsErr> {
    let thread_id = string_field(payload, "threadId")?.to_string();
    let terminal_id = payload
        .get("terminalId")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty() && s.len() <= 128)
        .unwrap_or(DEFAULT_TERMINAL_ID)
        .to_string();
    Ok((thread_id, terminal_id))
}

fn find_session(payload: &Value) -> Result<Arc<Session>, WsErr> {
    let (thread_id, terminal_id) = session_key(payload)?;
    let key = format!("{thread_id}:{terminal_id}");
    STORE
        .sessions
        .lock()
        .unwrap()
        .get(&key)
        .cloned()
        .ok_or_else(|| WsErr(format!("Unknown terminal: {key}"), Some("NOT_FOUND")))
}

fn dim(payload: &Value, key: &str, min: u16, max: u16, default: u16) -> Result<u16, WsErr> {
    match payload.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(n)) => n
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .filter(|v| *v >= min && *v <= max)
            .ok_or_else(|| {
                WsErr(
                    format!("{key} must be an integer between {min} and {max}"),
                    Some("VALIDATION"),
                )
            }),
        Some(_) => Err(WsErr(
            format!("{key} must be an integer"),
            Some("VALIDATION"),
        )),
    }
}

fn env_pairs(payload: &Value) -> Vec<(String, String)> {
    payload
        .get("env")
        .and_then(|v| v.as_object())
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

fn spawn(
    thread_id: &str,
    terminal_id: &str,
    cwd: &str,
    cols: u16,
    rows: u16,
    env: &[(String, String)],
    stream_output: bool,
) -> Result<Arc<Session>, WsErr> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| WsErr(format!("openpty failed: {e}"), Some("TERMINAL_OPEN_FAILED")))?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.arg("-l");
    cmd.cwd(cwd);
    for (k, v) in env {
        cmd.env(k, v);
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("SYNARA_TERMINAL_ID", terminal_id);
    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| WsErr(format!("spawn failed: {e}"), Some("TERMINAL_OPEN_FAILED")))?;
    let pid = child.process_id().unwrap_or(0);
    let mut reader = pair.master.try_clone_reader().map_err(|e| {
        WsErr(
            format!("pty reader failed: {e}"),
            Some("TERMINAL_OPEN_FAILED"),
        )
    })?;
    let writer = pair.master.take_writer().map_err(|e| {
        WsErr(
            format!("pty writer failed: {e}"),
            Some("TERMINAL_OPEN_FAILED"),
        )
    })?;
    let session = Arc::new(Session {
        thread_id: thread_id.to_string(),
        terminal_id: terminal_id.to_string(),
        cwd: Mutex::new(cwd.to_string()),
        env: Mutex::new(env.to_vec()),
        cols: Mutex::new(cols),
        rows: Mutex::new(rows),
        master: Mutex::new(pair.master),
        writer: Mutex::new(writer),
        child: Mutex::new(child),
        pid,
        history: Mutex::new(String::new()),
        status: Mutex::new("running"),
        exit: Mutex::new((None, None)),
        stream_output: AtomicBool::new(stream_output),
        retired: AtomicBool::new(false),
        updated_at: Mutex::new(now_iso()),
    });
    // Drain the pty into history and live events on a dedicated thread.
    let drain = Arc::clone(&session);
    std::thread::spawn(move || {
        let mut buf = [0u8; 16_384];
        loop {
            match reader.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    {
                        let mut history = drain.history.lock().unwrap();
                        history.push_str(&data);
                        if history.len() > HISTORY_MAX {
                            let cut = history.len() - HISTORY_MAX;
                            let cut = history.ceil_char_boundary(cut);
                            history.drain(..cut);
                        }
                    }
                    if drain.stream_output.load(Ordering::Relaxed)
                        && !drain.retired.load(Ordering::Relaxed)
                    {
                        drain.emit(json!({"type": "output", "data": data, "byteLength": n}));
                    }
                }
            }
        }
    });
    // Watch the child; on exit mark status and broadcast `exited`. Poll
    // `try_wait` so the `child` lock is never held across the process's
    // lifetime — `close`/`restart` need it to kill.
    let watch = Arc::clone(&session);
    std::thread::spawn(move || {
        let code = loop {
            {
                let mut child = watch.child.lock().unwrap();
                match child.try_wait() {
                    Ok(Some(status)) => break Some(status.exit_code() as i64),
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        *watch.exit.lock().unwrap() = (code, None);
        *watch.status.lock().unwrap() = "exited";
        watch.emit(json!({"type": "exited", "exitCode": code.map(Value::from).unwrap_or(Value::Null), "exitSignal": Value::Null}));
    });
    // Retire any session already occupying this key before replacing it.
    // Drop the map guard before touching the old session's locks.
    let old = {
        STORE
            .sessions
            .lock()
            .unwrap()
            .insert(session.key(), Arc::clone(&session))
    };
    if let Some(old) = old {
        old.retired.store(true, Ordering::Relaxed);
        let _ = old.child.lock().unwrap().kill();
    }
    Ok(session)
}

fn terminal_error() -> WsErr {
    WsErr(
        "terminal operation failed".to_string(),
        Some("TERMINAL_FAILED"),
    )
}

pub fn handle(tag: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    match tag {
        "terminal.open" => {
            let (thread_id, terminal_id) = session_key(payload)?;
            let cwd = string_field(payload, "cwd")?.to_string();
            let cols = dim(payload, "cols", MIN_COLS, MAX_COLS, 80)?;
            let rows = dim(payload, "rows", MIN_ROWS, MAX_ROWS, 24)?;
            let env = env_pairs(payload);
            let stream = payload
                .get("streamOutput")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let session = spawn(&thread_id, &terminal_id, &cwd, cols, rows, &env, stream)?;
            session.emit(json!({"type": "started", "snapshot": session.snapshot()}));
            Ok(Some(session.snapshot()))
        }
        "terminal.restart" => {
            let session = find_session(payload)?;
            let cwd = string_field(payload, "cwd")?.to_string();
            let cols = dim(payload, "cols", MIN_COLS, MAX_COLS, 80)?;
            let rows = dim(payload, "rows", MIN_ROWS, MAX_ROWS, 24)?;
            let env = {
                let merged = env_pairs(payload);
                if merged.is_empty() {
                    session.env.lock().unwrap().clone()
                } else {
                    merged
                }
            };
            session.retired.store(true, Ordering::Relaxed);
            let _ = session.child.lock().unwrap().kill();
            let fresh = spawn(
                &session.thread_id,
                &session.terminal_id,
                &cwd,
                cols,
                rows,
                &env,
                session.stream_output.load(Ordering::Relaxed),
            )?;
            // Replace the stored session wholesale; the key is identical.
            STORE
                .sessions
                .lock()
                .unwrap()
                .insert(fresh.key(), Arc::clone(&fresh));
            fresh.emit(json!({"type": "restarted", "snapshot": fresh.snapshot()}));
            Ok(Some(fresh.snapshot()))
        }
        "terminal.write" => {
            let session = find_session(payload)?;
            let data = payload
                .get("data")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty() && s.len() <= WRITE_MAX)
                .ok_or_else(|| {
                    WsErr(
                        format!(
                            "data must be a non-empty string of at most {WRITE_MAX} characters"
                        ),
                        Some("VALIDATION"),
                    )
                })?;
            if payload
                .get("onlyIfIdle")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                && *session.status.lock().unwrap() == "running"
            {
                return Ok(None);
            }
            let mut w = session.writer.lock().unwrap();
            w.write_all(data.as_bytes())
                .and_then(|_| w.flush())
                .map_err(|_| terminal_error())?;
            Ok(None)
        }
        "terminal.ackOutput" => {
            let _session = find_session(payload)?;
            payload
                .get("bytes")
                .and_then(|v| v.as_u64())
                .filter(|b| *b >= 1 && *b <= ACK_MAX)
                .ok_or_else(|| {
                    WsErr(
                        format!("bytes must be an integer between 1 and {ACK_MAX}"),
                        Some("VALIDATION"),
                    )
                })?;
            Ok(None)
        }
        "terminal.resize" => {
            let session = find_session(payload)?;
            let cols = dim(payload, "cols", MIN_COLS, MAX_COLS, 80)?;
            let rows = dim(payload, "rows", MIN_ROWS, MAX_ROWS, 24)?;
            session
                .master
                .lock()
                .unwrap()
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|_| terminal_error())?;
            *session.cols.lock().unwrap() = cols;
            *session.rows.lock().unwrap() = rows;
            Ok(None)
        }
        "terminal.clear" => {
            let session = find_session(payload)?;
            session.history.lock().unwrap().clear();
            session.emit(json!({"type": "cleared"}));
            Ok(None)
        }
        "terminal.close" => {
            let (thread_id, terminal_id) = session_key(payload)?;
            let key = format!("{thread_id}:{terminal_id}");
            let session = STORE.sessions.lock().unwrap().remove(&key);
            let Some(session) = session else {
                return Err(WsErr(format!("Unknown terminal: {key}"), Some("NOT_FOUND")));
            };
            if payload
                .get("onlyIfIdle")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                && *session.status.lock().unwrap() == "running"
            {
                STORE
                    .sessions
                    .lock()
                    .unwrap()
                    .insert(key, Arc::clone(&session));
                return Err(WsErr("terminal is busy".to_string(), Some("TERMINAL_BUSY")));
            }
            let _ = session.child.lock().unwrap().kill();
            if payload
                .get("deleteHistory")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                session.history.lock().unwrap().clear();
            }
            Ok(None)
        }
        _ => Err(WsErr(format!("unknown terminal method: {tag}"), None)),
    }
}
