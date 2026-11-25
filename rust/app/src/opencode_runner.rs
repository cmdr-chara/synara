//! OpenCode provider runtime: drives the official `opencode` CLI as a local
//! HTTP/SSE server (`opencode serve`), mirroring upstream's dedicated runtime
//! instead of ACP. The server owns provider credentials itself; Synara never
//! reads or replays them.
//!
//! v1 vertical slice: per-turn server process, session create/resume, async
//! prompt, SSE event stream translated into projection writes, terminal on
//! session.idle/error. Later slices: pooled warm servers, permission/question
//! reconciliation, model/agent/command discovery, MCP injection config.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use rusqlite::params;
use serde_json::{json, Value};

use crate::acp_runner::{append_assistant_text, err, session_set_event, set_session, set_turn_state};
use crate::orchestration_engine::{open_state_rw, uuid_v4};
use crate::todo::{iso_now_ms, WsErr};

struct RunningTurn {
    child: Arc<Mutex<Child>>,
    cancelled: Arc<AtomicBool>,
}

static RUNNING: LazyLock<Mutex<HashMap<String, RunningTurn>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn has_running_turn(thread_id: &str) -> bool {
    RUNNING
        .lock()
        .ok()
        .map(|m| m.contains_key(thread_id))
        .unwrap_or(false)
}

pub fn cancel_turn(thread_id: &str) {
    let (child, flag) = match RUNNING.lock() {
        Ok(m) => match m.get(thread_id) {
            Some(rt) => (rt.child.clone(), rt.cancelled.clone()),
            None => return,
        },
        Err(_) => return,
    };
    flag.store(true, Ordering::SeqCst);
    if let Ok(mut c) = child.lock() {
        let _ = c.kill();
    }
}

fn opencode_binary() -> String {
    for key in ["SYNARA_OPENCODE_CLI", "OPENCODE_BINARY_PATH"] {
        if let Ok(p) = std::env::var(key) {
            if !p.trim().is_empty() {
                return p;
            }
        }
    }
    "opencode".to_string()
}

pub fn supports_provider(provider: &str) -> bool {
    provider == "opencode"
}

fn random_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}{:x}", nanos, std::process::id())
}

fn basic_auth_header(password: &str) -> String {
    use sha1::Digest as _;
    // base64 of "opencode:<password>" — minimal encoder, no new dependency.
    let raw = format!("opencode:{password}");
    base64_encode(raw.as_bytes())
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { ALPHABET[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { ALPHABET[n as usize & 63] as char } else { '=' });
    }
    out
}

fn http_request(
    port: u16,
    method: &str,
    path: &str,
    auth: &str,
    body: Option<&Value>,
) -> Result<(u16, String), WsErr> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .map_err(|e| err(format!("opencode connect: {e}")))?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .ok();
    let payload = body.map(|b| b.to_string()).unwrap_or_default();
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Basic {auth}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| err(format!("opencode write: {e}")))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|e| err(format!("opencode read: {e}")))?;
    let text = String::from_utf8_lossy(&response).to_string();
    let mut parts = text.splitn(2, "\r\n\r\n");
    let head = parts.next().unwrap_or("");
    let body_text = parts.next().unwrap_or("").to_string();
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    Ok((status, body_text))
}

fn open_sse_stream(port: u16, auth: &str) -> Result<TcpStream, WsErr> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .map_err(|e| err(format!("opencode sse connect: {e}")))?;
    let request = format!(
        "GET /event HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Basic {auth}\r\nAccept: text/event-stream\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| err(format!("opencode sse write: {e}")))?;
    Ok(stream)
}

pub fn start_turn(thread_id: String, message_id: String, text: String, cwd: String) {
    std::thread::spawn(move || {
        if let Err(e) = run_turn(&thread_id, &message_id, &text, &cwd) {
            eprintln!("opencode_runner: turn failed for {thread_id}: {}", e.0);
        }
    });
}

fn run_turn(thread_id: &str, message_id: &str, text: &str, cwd: &str) -> Result<(), WsErr> {
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    let stored_session: Option<String> = conn
        .query_row(
            "SELECT provider_thread_id FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();

    let turn_id = uuid_v4();
    let assistant_msg_id = uuid_v4();
    let now = iso_now_ms();
    conn.execute(
        "INSERT INTO projection_turns \
         (thread_id, turn_id, pending_message_id, assistant_message_id, state, \
          requested_at, started_at, checkpoint_files_json) \
         VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?5, '[]')",
        params![thread_id, turn_id, message_id, assistant_msg_id, now],
    )
    .map_err(|e| err(format!("insert turn: {e}")))?;
    let turn_runtime_mode: String = conn
        .query_row(
            "SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "full-access".to_string());
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_sessions \
         (thread_id, status, provider_name, provider_session_id, provider_thread_id, \
          active_turn_id, last_error, updated_at, runtime_mode) \
         VALUES (?1, 'running', 'opencode', NULL, ?2, ?3, NULL, ?4, \
          COALESCE((SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1), 'full-access'))",
        params![thread_id, stored_session, turn_id, now],
    )
    .map_err(|e| err(format!("session start: {e}")))?;
    session_set_event(
        &conn,
        thread_id,
        &json!({
            "threadId": thread_id,
            "status": "running",
            "providerName": "opencode",
            "activeTurnId": turn_id,
            "runtimeMode": turn_runtime_mode,
            "lastError": null,
            "updatedAt": now,
        }),
    )?;

    let cwd = if !cwd.is_empty() && std::path::Path::new(cwd).is_dir() {
        cwd.to_string()
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    };

    // Reserve a free loopback port, then spawn the server onto it.
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .unwrap_or(4096);
    let password = random_token();
    let auth = basic_auth_header(&password);
    let mut command = Command::new(opencode_binary());
    command
        .args(["serve", "--hostname", "127.0.0.1", "--port", &port.to_string()])
        .current_dir(&cwd)
        .env("OPENCODE_SERVER_USERNAME", "opencode")
        .env("OPENCODE_SERVER_PASSWORD", &password)
        .env("OPENCODE_PASSWORD", &password)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child: Child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = set_turn_state(&conn, thread_id, &turn_id, "error", true);
            let msg = format!("could not launch `opencode serve`: {e}");
            let _ = set_session(&conn, thread_id, "error", Some(""), Some(&msg), None);
            return Err(err(msg));
        }
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    let child_shared = Arc::new(Mutex::new(child));
    {
        let mut map = RUNNING.lock().map_err(|_| err("running map"))?;
        map.insert(
            thread_id.to_string(),
            RunningTurn {
                child: child_shared.clone(),
                cancelled: cancelled.clone(),
            },
        );
    }

    let result = (|| -> Result<String, WsErr> {
        // Wait for the server to accept connections.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            if cancelled.load(Ordering::SeqCst) {
                return Ok("cancelled".to_string());
            }
            if let Ok((status, _)) = http_request(port, "GET", "/provider", &auth, None) {
                if status > 0 {
                    break;
                }
            }
            if std::time::Instant::now() >= deadline {
                return Err(err("opencode server did not become ready in 20s"));
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        // Reuse the stored session when present; otherwise create one.
        let session_id = if let Some(sid) = stored_session.clone() {
            sid
        } else {
            let (status, body) = http_request(
                port,
                "POST",
                "/session",
                &auth,
                Some(&json!({"title": format!("synara-{thread_id}")})),
            )?;
            if status >= 400 {
                return Err(err(format!("opencode session create HTTP {status}: {body}")));
            }
            let parsed: Value = serde_json::from_str(&body)
                .map_err(|e| err(format!("opencode session parse: {e}")))?;
            parsed
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| err("opencode session create returned no id"))?
                .to_string()
        };
        set_session(&conn, thread_id, "running", Some(&turn_id), None, Some(&session_id))?;

        // Subscribe to events before prompting so no delta is missed.
        let sse = open_sse_stream(port, &auth)?;
        let (status, body) = http_request(
            port,
            "POST",
            &format!("/session/{session_id}/prompt_async"),
            &auth,
            Some(&json!({
                "parts": [{"type": "text", "text": text}],
            })),
        )?;
        if status >= 400 {
            return Err(err(format!("opencode prompt HTTP {status}: {body}")));
        }

        let mut text_buf = String::new();
        let mut last_flush = std::time::Instant::now();
        let mut reader = BufReader::new(sse);
        // Skip HTTP headers.
        let mut header_line = String::new();
        let mut blank_seen = false;
        while !blank_seen {
            header_line.clear();
            if reader.read_line(&mut header_line).unwrap_or(0) == 0 {
                return Err(err("opencode sse closed during headers"));
            }
            if header_line.trim().is_empty() {
                blank_seen = true;
            }
        }
        let terminal = loop {
            if cancelled.load(Ordering::SeqCst) {
                break "cancelled";
            }
            let mut line = String::new();
            let n = reader
                .read_line(&mut line)
                .map_err(|e| err(format!("opencode sse read: {e}")))?;
            if n == 0 {
                break "ended";
            }
            let trimmed = line.trim();
            if !trimmed.starts_with("data:") {
                continue;
            }
            let payload = trimmed.trim_start_matches("data:").trim();
            let Ok(v) = serde_json::from_str::<Value>(payload) else {
                continue;
            };
            let event_type = v
                .get("type")
                .and_then(Value::as_str)
                .or_else(|| v.pointer("/properties/type").and_then(Value::as_str))
                .unwrap_or("");
            match event_type {
                "message.part.delta" => {
                    if let Some(d) = v
                        .pointer("/properties/delta")
                        .and_then(Value::as_str)
                        .or_else(|| v.pointer("/properties/part/text").and_then(Value::as_str))
                    {
                        text_buf.push_str(d);
                    }
                }
                "session.idle" => break "completed",
                "session.error" => {
                    let msg = v
                        .pointer("/properties/error/message")
                        .and_then(Value::as_str)
                        .unwrap_or("opencode session error")
                        .to_string();
                    return Err(err(msg));
                }
                _ => {}
            }
            if !text_buf.is_empty() && last_flush.elapsed().as_millis() >= 200 {
                if let Ok(c) = open_state_rw() {
                    let _ = append_assistant_text(
                        &c,
                        thread_id,
                        &assistant_msg_id,
                        &turn_id,
                        &text_buf,
                        true,
                    );
                }
                text_buf.clear();
                last_flush = std::time::Instant::now();
            }
        };
        if let Ok(c) = open_state_rw() {
            if !text_buf.is_empty() {
                let _ = append_assistant_text(
                    &c,
                    thread_id,
                    &assistant_msg_id,
                    &turn_id,
                    &text_buf,
                    true,
                );
            }
            let _ = append_assistant_text(&c, thread_id, &assistant_msg_id, &turn_id, "", false);
        }
        Ok(terminal.to_string())
    })();

    if let Ok(mut map) = RUNNING.lock() {
        map.remove(thread_id);
    }
    if let Ok(mut c) = child_shared.lock() {
        let _ = c.kill();
        let _ = c.wait();
    }

    let cancelled_flag = cancelled.load(Ordering::SeqCst);
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    match result {
        Ok(reason) => {
            let interrupted = cancelled_flag || reason == "cancelled";
            let state = if interrupted { "interrupted" } else { "completed" };
            let status = if interrupted { "interrupted" } else { "ready" };
            set_turn_state(&conn, thread_id, &turn_id, state, true)?;
            set_session(&conn, thread_id, status, Some(""), None, None)?;
            session_set_event(
                &conn,
                thread_id,
                &json!({
                    "threadId": thread_id,
                    "status": status,
                    "providerName": "opencode",
                    "activeTurnId": null,
                    "runtimeMode": turn_runtime_mode,
                    "lastError": null,
                    "updatedAt": iso_now_ms(),
                }),
            )?;
            Ok(())
        }
        Err(e) => {
            let (state, status) = if cancelled_flag {
                ("interrupted", "interrupted")
            } else {
                ("error", "error")
            };
            set_turn_state(&conn, thread_id, &turn_id, state, true)?;
            set_session(&conn, thread_id, status, Some(""), Some(&e.0), None)?;
            session_set_event(
                &conn,
                thread_id,
                &json!({
                    "threadId": thread_id,
                    "status": status,
                    "providerName": "opencode",
                    "activeTurnId": null,
                    "runtimeMode": turn_runtime_mode,
                    "lastError": e.0,
                    "updatedAt": iso_now_ms(),
                }),
            )?;
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_encoder_matches_rfc() {
        assert_eq!(super::base64_encode(b""), "");
        assert_eq!(super::base64_encode(b"f"), "Zg==");
        assert_eq!(super::base64_encode(b"fo"), "Zm8=");
        assert_eq!(super::base64_encode(b"foo"), "Zm9v");
        assert_eq!(super::base64_encode(b"opencode:abc"), "b3BlbmNvZGU6YWJj");
    }
}
