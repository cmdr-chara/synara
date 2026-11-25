//! OpenCode provider runtime: drives the official `opencode` CLI as a local
//! HTTP/SSE server (`opencode serve`), mirroring upstream's dedicated runtime
//! instead of ACP. The server owns provider credentials itself; Synara never
//! reads or replays them.
//!
//! v1 vertical slice: per-turn server process, session create/resume, async
//! prompt, SSE event stream translated into projection writes, terminal on
//! session.idle/error, and best-effort gateway MCP injection on V2 servers.
//! Later slices: pooled warm servers, permission/question reconciliation,
//! model/agent/command discovery, and native V2 session/prompt contracts.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use rusqlite::params;
use serde_json::{json, Value};

use crate::acp_runner::{append_activity, append_assistant_text, err, session_set_event, set_session, set_turn_state};
use crate::orchestration_engine::{open_state_rw, uuid_v4};
use crate::todo::{iso_now_ms, WsErr};

struct RunningTurn {
    child: Arc<Mutex<Child>>,
    cancelled: Arc<AtomicBool>,
}

static RUNNING: LazyLock<Mutex<HashMap<String, RunningTurn>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Warm OpenCode server parked after a clean turn. The server is expensive
/// to boot and owns the provider session/MCP registry for the thread; it is
/// reused only for the same workspace launch identity.
struct ParkedOpenCode {
    child: Arc<Mutex<Child>>,
    port: u16,
    password: String,
    session_id: String,
    launch_key: String,
}

static PARKED: LazyLock<Mutex<HashMap<String, ParkedOpenCode>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn drop_parked(parked: ParkedOpenCode) {
    if let Ok(mut child) = parked.child.lock() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn take_parked(thread_id: &str, launch_key: &str) -> Option<ParkedOpenCode> {
    let parked = PARKED.lock().ok()?.remove(thread_id)?;
    if parked.launch_key != launch_key {
        drop_parked(parked);
        return None;
    }
    let alive = parked
        .child
        .lock()
        .ok()
        .and_then(|mut child| child.try_wait().ok())
        .map(|status| status.is_none())
        .unwrap_or(false);
    if alive {
        Some(parked)
    } else {
        drop_parked(parked);
        None
    }
}

pub fn has_running_turn(thread_id: &str) -> bool {
    RUNNING
        .lock()
        .ok()
        .map(|m| m.contains_key(thread_id))
        .unwrap_or(false)
}

pub fn drop_thread_resources(thread_id: &str) {
    if let Ok(mut parked_map) = PARKED.lock() {
        if let Some(parked) = parked_map.remove(thread_id) {
            drop_parked(parked);
        }
    }
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


fn urlencode_component(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

/// Install the Synara gateway as a remote MCP server on OpenCode V2. The
/// registry is server/directory scoped, so this runtime intentionally uses a
/// thread-owned server process. Older V1 servers do not expose the endpoint;
/// injection is optional there and the turn continues without it.
fn install_gateway_mcp(port: u16, auth: &str, directory: &str) -> bool {
    let location = format!(
        "location%5Bdirectory%5D={}",
        urlencode_component(directory)
    );
    let config = json!({
        "config": {
            "type": "remote",
            "url": crate::agent_gateway_mcp::agent_gateway_url(),
            "headers": {
                "Authorization": format!(
                    "Bearer {}",
                    crate::agent_gateway_mcp::agent_gateway_token()
                ),
            },
            "oauth": false,
            "disabled": false,
        }
    });
    let path = format!("/api/experimental/mcp/synara?{location}");
    let Ok((status, _)) = http_request(port, "PUT", &path, auth, Some(&config)) else {
        return false;
    };
    if status == 404 || status == 405 {
        return false;
    }
    if status >= 400 {
        eprintln!("opencode_runner: gateway MCP registration HTTP {status}");
        return false;
    }
    for _ in 0..90 {
        let list_path = format!("/api/mcp?{location}");
        if let Ok((list_status, body)) = http_request(port, "GET", &list_path, auth, None) {
            if list_status < 400 {
                if let Ok(parsed) = serde_json::from_str::<Value>(&body) {
                    let entries = parsed
                        .get("data")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    if let Some(entry) = entries.iter().find(|entry| {
                        entry.get("name").and_then(Value::as_str) == Some("synara")
                    }) {
                        match entry.pointer("/status/status").and_then(Value::as_str) {
                            Some("connected") => return true,
                            Some("failed") => {
                                eprintln!("opencode_runner: gateway MCP connection failed");
                                return false;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}


#[derive(Clone, Copy, PartialEq, Eq)]
enum OpenCodeProtocol {
    V1,
    V2,
}

/// Detect the server contract before sending mutating provider requests,
/// mirroring upstream: `/api/info` is the V2-only server-info endpoint.
fn detect_protocol(port: u16, auth: &str) -> OpenCodeProtocol {
    if let Ok((status, body)) = http_request(port, "GET", "/api/info", auth, None) {
        if status >= 200 && status < 300 {
            if let Ok(info) = serde_json::from_str::<Value>(&body) {
                if info.get("version").and_then(Value::as_str).is_some()
                    && info.get("pid").and_then(Value::as_i64).is_some()
                    && info.get("urls").and_then(Value::as_array).is_some()
                {
                    return OpenCodeProtocol::V2;
                }
            }
        }
    }
    OpenCodeProtocol::V1
}

fn open_sse_stream(port: u16, auth: &str, path: &str) -> Result<TcpStream, WsErr> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .map_err(|e| err(format!("opencode sse connect: {e}")))?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Basic {auth}\r\nAccept: text/event-stream\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| err(format!("opencode sse write: {e}")))?;
    Ok(stream)
}


fn permission_request_kind(action: &str) -> &'static str {
    match action {
        "shell" | "bash" => "command",
        "read" => "file-read",
        "edit" | "write" | "patch" => "file-change",
        _ => "tool",
    }
}

/// Bridge one native V2 permission request into Synara's pending-interaction
/// approval surface, wait for `thread.approval.respond`, then answer the
/// server. Full Access auto-accepts because that mode is itself the user's
/// blanket grant; other modes require the explicit card decision.
fn handle_permission_request(
    port: u16,
    auth: &str,
    session_id: &str,
    data: &Value,
    thread_id: &str,
    turn_id: &str,
    runtime_mode: &str,
    cancelled: &Arc<AtomicBool>,
) -> Result<(), WsErr> {
    let request_id = data
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| err("opencode permission request missing id"))?
        .to_string();
    let action = data
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("tool");
    let resources = data
        .get("resources")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let detail = resources
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let title = if detail.is_empty() {
        format!("OpenCode {action}")
    } else {
        detail.clone()
    };
    let reply_for = |decision: &str| match decision {
        "accept" | "acceptForSession" => {
            if decision == "acceptForSession" { "always" } else { "once" }
        }
        _ => "reject",
    };

    if runtime_mode == "full-access" {
        let path = format!("/api/session/{session_id}/permission/{request_id}/reply");
        let (status, body) = http_request(
            port,
            "POST",
            &path,
            auth,
            Some(&json!({"decision": "once"})),
        )?;
        if status >= 400 {
            return Err(err(format!("opencode permission reply HTTP {status}: {body}")));
        }
        return Ok(());
    }

    if let Ok(c) = open_state_rw() {
        let _ = c.execute_batch("PRAGMA busy_timeout = 5000;");
        let now = iso_now_ms();
        if let Err(e) = c.execute(
            "INSERT OR REPLACE INTO projection_pending_interactions              (interaction_kind, request_id, thread_id, turn_id,               lifecycle_generation, status, decision,               response_command_id, response_requested_at,               created_at, resolved_at)              VALUES ('approval', ?1, ?2, ?3, NULL, 'pending', NULL,               NULL, NULL, ?4, NULL)",
            params![request_id, thread_id, turn_id, now],
        ) {
            eprintln!("opencode_runner: pending permission insert: {e}");
        }
        let _ = append_activity(
            &c,
            thread_id,
            turn_id,
            "warn",
            "approval.requested",
            "Approval requested",
            &json!({
                "requestId": request_id,
                "requestKind": permission_request_kind(action),
                "toolName": title,
                "detail": title,
                "providerMethod": "permission.asked",
                "providerPayload": data,
                "sessionApprovalAvailable": true,
            }),
        );
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    let mut decision: Option<String> = None;
    while decision.is_none() {
        if cancelled.load(Ordering::SeqCst) || std::time::Instant::now() >= deadline {
            decision = Some("cancel".to_string());
            break;
        }
        if let Ok(c) = open_state_rw() {
            match c.query_row(
                "SELECT decision FROM projection_pending_interactions                  WHERE thread_id=?1 AND request_id=?2                  AND interaction_kind='approval'",
                params![thread_id, request_id],
                |r| r.get::<_, Option<String>>(0),
            ) {
                Ok(d) => {
                    if let Some(d) = d {
                        decision = Some(d);
                    }
                }
                Err(_) => decision = Some("cancel".to_string()),
            }
        }
        if decision.is_none() {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
    let decision = decision.unwrap_or_else(|| "cancel".to_string());
    if let Ok(c) = open_state_rw() {
        let _ = c.execute(
            "DELETE FROM projection_pending_interactions              WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='approval'",
            params![thread_id, request_id],
        );
        let _ = append_activity(
            &c,
            thread_id,
            turn_id,
            "info",
            "approval.resolved",
            "Approval resolved",
            &json!({"requestId": request_id, "decision": decision}),
        );
    }
    let path = format!("/api/session/{session_id}/permission/{request_id}/reply");
    let (status, body) = http_request(
        port,
        "POST",
        &path,
        auth,
        Some(&json!({"decision": reply_for(&decision)})),
    )?;
    if status >= 400 {
        return Err(err(format!("opencode permission reply HTTP {status}: {body}")));
    }
    Ok(())
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

    let launch_key = cwd.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let (child_shared, port, password, reused_session_id) = if let Some(parked) =
        take_parked(thread_id, &launch_key)
    {
        (
            parked.child,
            parked.port,
            parked.password,
            Some(parked.session_id),
        )
    } else {
        // Reserve a free loopback port, then spawn the server onto it.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .and_then(|l| l.local_addr())
            .map(|a| a.port())
            .unwrap_or(4096);
        let password = random_token();
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
        let child: Child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = set_turn_state(&conn, thread_id, &turn_id, "error", true);
                let msg = format!("could not launch `opencode serve`: {e}");
                let _ = set_session(&conn, thread_id, "error", Some(""), Some(&msg), None);
                return Err(err(msg));
            }
        };
        (Arc::new(Mutex::new(child)), port, password, None)
    };
    let auth = basic_auth_header(&password);
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

    let mut active_session_id: Option<String> = None;
    let result = (|| -> Result<String, WsErr> {
        let mut protocol = detect_protocol(port, &auth);
        if reused_session_id.is_none() {
            // Wait for a newly spawned server to accept connections.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
            loop {
                if cancelled.load(Ordering::SeqCst) {
                    return Ok("cancelled".to_string());
                }
                protocol = detect_protocol(port, &auth);
                let ready = if protocol == OpenCodeProtocol::V2 {
                    true
                } else {
                    http_request(port, "GET", "/provider", &auth, None)
                        .map(|(status, _)| status > 0)
                        .unwrap_or(false)
                };
                if ready {
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    return Err(err("opencode server did not become ready in 20s"));
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
        protocol = detect_protocol(port, &auth);

        install_gateway_mcp(port, &auth, &cwd);

        // Reuse the warm server's live session first; a respawned server can
        // still resume OpenCode's stored session id when one exists.
        let session_id = if let Some(sid) = reused_session_id.clone().or(stored_session.clone()) {
            sid
        } else if protocol == OpenCodeProtocol::V2 {
            let (status, body) = http_request(
                port,
                "POST",
                "/api/session",
                &auth,
                Some(&json!({
                    "location": {"directory": cwd},
                    "title": format!("synara-{thread_id}"),
                })),
            )?;
            if status >= 400 {
                return Err(err(format!("opencode session create HTTP {status}: {body}")));
            }
            let parsed: Value = serde_json::from_str(&body)
                .map_err(|e| err(format!("opencode session parse: {e}")))?;
            parsed
                .pointer("/data/id")
                .and_then(Value::as_str)
                .or_else(|| parsed.get("id").and_then(Value::as_str))
                .ok_or_else(|| err("opencode session create returned no id"))?
                .to_string()
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
        active_session_id = Some(session_id.clone());
        set_session(&conn, thread_id, "running", Some(&turn_id), None, Some(&session_id))?;

        // Subscribe to events before prompting so no delta is missed.
        let event_path = if protocol == OpenCodeProtocol::V2 { "/api/event" } else { "/event" };
        let sse = open_sse_stream(port, &auth, event_path)?;
        let prompt_text = if stored_session.is_none() {
            format!("{}{}", crate::harness_policy::first_prompt_prefix(), text)
        } else {
            text.to_string()
        };
        let (prompt_path, prompt_body) = if protocol == OpenCodeProtocol::V2 {
            (
                format!("/api/session/{session_id}/prompt"),
                json!({
                    "text": prompt_text,
                    "delivery": "steer",
                }),
            )
        } else {
            (
                format!("/session/{session_id}/prompt_async"),
                json!({
                    "parts": [{"type": "text", "text": prompt_text}],
                }),
            )
        };
        let (status, body) = http_request(port, "POST", &prompt_path, &auth, Some(&prompt_body))?;
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
                "message.part.delta" | "session.text.delta" => {
                    if let Some(d) = v
                        .pointer("/properties/delta")
                        .and_then(Value::as_str)
                        .or_else(|| v.pointer("/properties/part/text").and_then(Value::as_str))
                        .or_else(|| v.pointer("/data/delta").and_then(Value::as_str))
                    {
                        text_buf.push_str(d);
                    }
                }
                "permission.asked" if protocol == OpenCodeProtocol::V2 => {
                    let data = v.get("data").cloned().unwrap_or(Value::Null);
                    handle_permission_request(
                        port,
                        &auth,
                        &session_id,
                        &data,
                        thread_id,
                        &turn_id,
                        &turn_runtime_mode,
                        &cancelled,
                    )?;
                }
                "session.idle" | "session.execution.succeeded" => break "completed",
                "session.execution.interrupted" => break "cancelled",
                "session.error" | "session.execution.failed" => {
                    let msg = v
                        .pointer("/properties/error/message")
                        .and_then(Value::as_str)
                        .or_else(|| v.pointer("/data/error/message").and_then(Value::as_str))
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
    let cancelled_flag = cancelled.load(Ordering::SeqCst);
    let server_alive = child_shared
        .lock()
        .ok()
        .and_then(|mut child| child.try_wait().ok())
        .map(|status| status.is_none())
        .unwrap_or(false);
    let parkable = matches!(result, Ok(ref reason) if reason == "completed")
        && !cancelled_flag
        && server_alive;
    if parkable {
        if let (Some(session_id), Ok(mut parked_map)) =
            (active_session_id.clone(), PARKED.lock())
        {
            parked_map.insert(
                thread_id.to_string(),
                ParkedOpenCode {
                    child: child_shared.clone(),
                    port,
                    password,
                    session_id,
                    launch_key,
                },
            );
        }
    } else if let Ok(mut c) = child_shared.lock() {
        let _ = c.kill();
        let _ = c.wait();
    }

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
            crate::orchestration_engine::continue_queued_turn(thread_id);
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
            if cancelled_flag {
                crate::orchestration_engine::continue_queued_turn(thread_id);
            }
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
