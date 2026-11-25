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
    port: u16,
    password: String,
    provider_session_id: Arc<Mutex<Option<String>>>,
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
    let (child, flag, port, password, provider_session_id) = match RUNNING.lock() {
        Ok(m) => match m.get(thread_id) {
            Some(rt) => (
                rt.child.clone(),
                rt.cancelled.clone(),
                rt.port,
                rt.password.clone(),
                rt.provider_session_id.clone(),
            ),
            None => return,
        },
        Err(_) => return,
    };
    flag.store(true, Ordering::SeqCst);
    let session_id = provider_session_id
        .lock()
        .ok()
        .and_then(|id| id.clone());
    let interrupted = session_id
        .map(|session_id| {
            let auth = basic_auth_header(&password);
            let path = format!("/api/session/{session_id}/interrupt");
            http_request(port, "POST", &path, &auth, None)
                .map(|(status, _)| (200..300).contains(&status) || status == 204)
                .unwrap_or(false)
        })
        .unwrap_or(false);
    if !interrupted {
        if let Ok(mut c) = child.lock() {
            let _ = c.kill();
        }
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



fn opencode_permission_rules(runtime_mode: &str, interaction_mode: &str) -> Value {
    if interaction_mode == "plan" {
        let mut rules = vec![json!({"action":"*","resource":"*","effect":"deny"})];
        for action in [
            "read", "glob", "grep", "list", "lsp", "webfetch", "websearch", "codesearch",
            "todoread", "todowrite", "question",
        ] {
            rules.push(json!({"action":action,"resource":"*","effect":"allow"}));
        }
        return json!(rules);
    }
    if runtime_mode == "full-access" {
        return json!([{"action":"*","resource":"*","effect":"allow"}]);
    }
    let mut rules = vec![json!({"action":"*","resource":"*","effect":"ask"})];
    for action in [
        "shell", "edit", "webfetch", "websearch", "codesearch", "external_directory", "doom_loop",
    ] {
        rules.push(json!({"action":action,"resource":"*","effect":"ask"}));
    }
    rules.push(json!({"action":"question","resource":"*","effect":"allow"}));
    json!(rules)
}

fn question_id(index: usize, header: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in header.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        format!("question-{index}")
    } else {
        format!("question-{index}-{slug}")
    }
}

fn form_unsupported(form: &Value) -> bool {
    form.get("fields")
        .and_then(Value::as_array)
        .map(|fields| {
            fields.iter().any(|field| {
                field.get("type").and_then(Value::as_str) == Some("external")
                    || field.get("hidden").and_then(Value::as_bool).unwrap_or(false)
                    || field
                        .get("when")
                        .and_then(Value::as_array)
                        .map(|w| !w.is_empty())
                        .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn normalized_questions(form: &Value) -> Vec<Value> {
    let title = form.get("title").and_then(Value::as_str).unwrap_or("OpenCode form");
    if form_unsupported(form) {
        return vec![json!({
            "id": question_id(0, title),
            "header": title,
            "question": "This form needs OpenCode's interface. Complete it there, or cancel it here.",
            "options": [{"label": "Cancel", "description": "Cancel this OpenCode form"}],
        })];
    }
    form.get("fields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let key = field.get("key").and_then(Value::as_str).unwrap_or("value");
            let header = field.get("title").and_then(Value::as_str).unwrap_or(title);
            let kind = field.get("type").and_then(Value::as_str).unwrap_or("string");
            let mut options = Vec::new();
            if kind == "boolean" {
                options.push(json!({"label":"true","description":"Yes"}));
                options.push(json!({"label":"false","description":"No"}));
            } else if let Some(items) = field.get("options").and_then(Value::as_array) {
                for option in items {
                    let value = option.get("value").and_then(Value::as_str).unwrap_or("");
                    let label = option.get("label").and_then(Value::as_str).unwrap_or(value);
                    options.push(json!({
                        "label": label,
                        "description": option.get("description").and_then(Value::as_str).unwrap_or(value),
                    }));
                }
            }
            let mut q = json!({
                "id": question_id(index, header),
                "header": header,
                "question": field.get("description").and_then(Value::as_str)
                    .or(field.get("title").and_then(Value::as_str))
                    .unwrap_or(key),
                "options": options,
            });
            if kind == "multiselect" {
                q["multiSelect"] = json!(true);
            }
            q
        })
        .collect()
}

fn scalar_answer_values(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| match item {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(n.to_string()),
                Value::Bool(b) => Some(b.to_string()),
                _ => None,
            })
            .collect(),
        Value::String(s) => vec![s.clone()],
        Value::Number(n) => vec![n.to_string()],
        Value::Bool(b) => vec![b.to_string()],
        _ => Vec::new(),
    }
}

/// Convert Synara's object-shaped user-input answers into OpenCode V2's
/// native form answer object, including option-label -> internal-value and
/// boolean/number coercion from upstream's openCodeV2Answer boundary.
fn opencode_form_answer(form: &Value, answers: &Value) -> Result<Value, WsErr> {
    if form_unsupported(form) {
        return Err(err("form must be completed in OpenCode or cancelled"));
    }
    let answers_obj = answers.as_object().cloned().unwrap_or_default();
    let fields = form
        .get("fields")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let title = form.get("title").and_then(Value::as_str).unwrap_or("OpenCode form");
    let mut out = serde_json::Map::new();
    for (index, field) in fields.iter().enumerate() {
        let header = field.get("title").and_then(Value::as_str).unwrap_or(title);
        let key = field.get("key").and_then(Value::as_str).unwrap_or("value");
        let id = question_id(index, header);
        let values = answers_obj
            .get(&id)
            .or_else(|| answers_obj.get(key))
            .map(scalar_answer_values)
            .unwrap_or_default();
        if values.is_empty() {
            if field.get("required").and_then(Value::as_bool).unwrap_or(false) {
                return Err(err(format!("OpenCode requires an answer for {key}")));
            }
            continue;
        }
        let kind = field.get("type").and_then(Value::as_str).unwrap_or("string");
        let mapped: Vec<String> = values
            .iter()
            .map(|value| {
                field
                    .get("options")
                    .and_then(Value::as_array)
                    .and_then(|options| {
                        options.iter().find(|option| {
                            let internal = option.get("value").and_then(Value::as_str).unwrap_or("");
                            let label = option.get("label").and_then(Value::as_str).unwrap_or(internal);
                            label == value
                        })
                    })
                    .and_then(|option| option.get("value").and_then(Value::as_str).map(str::to_string))
                    .unwrap_or_else(|| value.clone())
            })
            .collect();
        match kind {
            "multiselect" => {
                out.insert(key.to_string(), json!(mapped));
            }
            "boolean" => {
                match mapped.first().map(String::as_str) {
                    Some("true") => out.insert(key.to_string(), json!(true)),
                    Some("false") => out.insert(key.to_string(), json!(false)),
                    _ => return Err(err(format!("invalid boolean answer for {key}"))),
                };
            }
            "number" | "integer" => {
                let value = mapped
                    .first()
                    .and_then(|v| v.parse::<f64>().ok())
                    .ok_or_else(|| err(format!("invalid numeric answer for {key}")))?;
                if kind == "integer" && value.fract() != 0.0 {
                    return Err(err(format!("invalid integer answer for {key}")));
                }
                out.insert(key.to_string(), json!(value));
            }
            _ => {
                out.insert(key.to_string(), json!(mapped.join(", ")));
            }
        }
    }
    Ok(Value::Object(out))
}

/// Bridge a native V2 form into Synara's structured user-input surface. Empty
/// answers cancel the form, matching upstream's user-input cancellation
/// contract; non-empty answers are converted positionally by question id.
fn handle_form_request(
    port: u16,
    auth: &str,
    event_session_id: &str,
    data: &Value,
    thread_id: &str,
    turn_id: &str,
    cancelled: &Arc<AtomicBool>,
) -> Result<(), WsErr> {
    let form = data.get("form").unwrap_or(data);
    let request_id = form
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .ok_or_else(|| err("opencode form missing id"))?
        .to_string();
    let form_session_id = form
        .get("sessionID")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .unwrap_or(event_session_id)
        .to_string();
    let questions = normalized_questions(form);
    if let Ok(c) = open_state_rw() {
        let _ = c.execute_batch("PRAGMA busy_timeout = 5000;");
        let now = iso_now_ms();
        let _ = c.execute(
            "INSERT OR REPLACE INTO projection_pending_interactions \
             (interaction_kind, request_id, thread_id, turn_id, lifecycle_generation, status, decision, response_command_id, response_requested_at, created_at, resolved_at) \
             VALUES ('userInput', ?1, ?2, ?3, NULL, 'pending', NULL, NULL, NULL, ?4, NULL)",
            params![request_id, thread_id, turn_id, now],
        );
        let _ = append_activity(
            &c,
            thread_id,
            turn_id,
            "info",
            "user-input.requested",
            "User input requested",
            &json!({
                "requestId": request_id,
                "questions": questions,
                "providerMethod": "form.created",
                "providerPayload": form,
            }),
        );
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    let mut encoded: Option<String> = None;
    while encoded.is_none() {
        if cancelled.load(Ordering::SeqCst) || std::time::Instant::now() >= deadline {
            encoded = Some("{}".to_string());
            break;
        }
        if let Ok(c) = open_state_rw() {
            match c.query_row(
                "SELECT decision FROM projection_pending_interactions \
                 WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='userInput'",
                params![thread_id, request_id],
                |r| r.get::<_, Option<String>>(0),
            ) {
                Ok(Some(d)) => encoded = Some(d),
                Ok(None) => {}
                Err(_) => encoded = Some("{}".to_string()),
            }
        }
        if encoded.is_none() {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
    let encoded = encoded.unwrap_or_else(|| "{}".to_string());
    let answers = serde_json::from_str::<Value>(&encoded).unwrap_or(json!({}));
    let empty = answers.as_object().map(|o| {
        o.values().all(|v| v.is_null())
            || o.is_empty()
            || (o.len() == 1 && o.values().any(|v| scalar_answer_values(v).iter().any(|x| x == "Cancel")))
    }).unwrap_or(true);
    if let Ok(c) = open_state_rw() {
        let _ = c.execute(
            "DELETE FROM projection_pending_interactions WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='userInput'",
            params![thread_id, request_id],
        );
        let _ = append_activity(
            &c,
            thread_id,
            turn_id,
            "info",
            "user-input.resolved",
            "User input submitted",
            &json!({"requestId": request_id, "answers": answers}),
        );
    }

    let (method, path, body) = if empty {
        (
            "DELETE",
            format!("/api/session/{form_session_id}/form/{request_id}"),
            None,
        )
    } else {
        let answer = opencode_form_answer(form, &answers)?;
        (
            "POST",
            format!("/api/session/{form_session_id}/form/{request_id}/reply"),
            Some(json!({"answer": answer})),
        )
    };
    let (status, response_body) = http_request(port, method, &path, auth, body.as_ref())?;
    if status >= 400 {
        return Err(err(format!("opencode form reply HTTP {status}: {response_body}")));
    }
    Ok(())
}


/// Compact the warm OpenCode session through the native V2 endpoint. The
/// request is intentionally limited to idle warm sessions: compacting a busy
/// provider would race the active turn, while a cold parked process is cheaper
/// to resume on the next prompt than to boot only for maintenance.
pub fn compact_thread(thread_id: &str) -> Result<bool, WsErr> {
    if has_running_turn(thread_id) {
        return Err(err("cannot compact while a turn is running"));
    }
    let (port, password, session_id) = {
        let parked_map = PARKED.lock().map_err(|_| err("parked map"))?;
        let Some(parked) = parked_map.get(thread_id) else {
            return Ok(false);
        };
        (parked.port, parked.password.clone(), parked.session_id.clone())
    };
    let auth = basic_auth_header(&password);
    let path = format!("/api/session/{session_id}/compact");
    let (status, body) = http_request(
        port,
        "POST",
        &path,
        &auth,
        Some(&json!({"delivery":"steer"})),
    )?;
    if status == 404 || status == 405 {
        return Ok(false);
    }
    if status >= 400 {
        return Err(err(format!("opencode compact HTTP {status}: {body}")));
    }
    Ok(true)
}




fn opencode_display_name(name: &str) -> String {
    name.split(['-', '_', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// List user-selectable primary agents from a warm OpenCode server.
pub fn discover_agents(thread_id: &str, cwd: &str) -> Result<Option<Vec<Value>>, WsErr> {
    let (port, password) = {
        let parked_map = PARKED.lock().map_err(|_| err("parked map"))?;
        let Some(parked) = parked_map.get(thread_id) else {
            return Ok(None);
        };
        (parked.port, parked.password.clone())
    };
    let auth = basic_auth_header(&password);
    let location = if cwd.is_empty() {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    } else {
        cwd.to_string()
    };
    let path = format!("/api/agent?location={}", urlencode_component(&location));
    let (status, body) = http_request(port, "GET", &path, &auth, None)?;
    if status == 404 || status == 405 {
        return Ok(None);
    }
    if status >= 400 {
        return Err(err(format!("opencode agent list HTTP {status}: {body}")));
    }
    let parsed: Value = serde_json::from_str(&body)
        .map_err(|e| err(format!("opencode agent parse: {e}")))?;
    let entries = parsed
        .get("data")
        .cloned()
        .or(Some(parsed))
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let mut agents: Vec<Value> = entries
        .iter()
        .filter(|agent| {
            agent.get("hidden").and_then(Value::as_bool) != Some(true)
                && matches!(
                    agent.get("mode").and_then(Value::as_str),
                    Some("primary") | Some("all") | None
                )
        })
        .filter_map(|agent| {
            let name = agent.get("name").and_then(Value::as_str)?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let display_name = agent
                .get("displayName")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| opencode_display_name(&name));
            let mut out = json!({"name": name, "displayName": display_name});
            if let Some(description) = agent.get("description").and_then(Value::as_str) {
                if !description.trim().is_empty() {
                    out["description"] = json!(description.trim());
                }
            }
            if let Some(model) = agent.get("model") {
                let provider = model.get("providerID").and_then(Value::as_str);
                let model_id = model
                    .get("modelID")
                    .and_then(Value::as_str)
                    .or_else(|| model.get("id").and_then(Value::as_str));
                if let (Some(provider), Some(model_id)) = (provider, model_id) {
                    out["model"] = json!(format!("{provider}/{model_id}"));
                }
            }
            Some(out)
        })
        .collect();
    agents.sort_by(|a, b| {
        a.get("displayName")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("displayName").and_then(Value::as_str).unwrap_or(""))
    });
    Ok(Some(agents))
}

/// List native OpenCode slash commands from a warm thread-owned server. Cold
/// discovery is intentionally not hidden behind a model/session mutation: the
/// caller can prewarm explicitly when it wants a live server.
pub fn discover_commands(thread_id: &str, cwd: &str) -> Result<Option<Vec<Value>>, WsErr> {
    let (port, password) = {
        let parked_map = PARKED.lock().map_err(|_| err("parked map"))?;
        let Some(parked) = parked_map.get(thread_id) else {
            return Ok(None);
        };
        (parked.port, parked.password.clone())
    };
    let auth = basic_auth_header(&password);
    let location = if cwd.is_empty() {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    } else {
        cwd.to_string()
    };
    let path = format!("/api/command?location={}", urlencode_component(&location));
    let (status, body) = http_request(port, "GET", &path, &auth, None)?;
    if status == 404 || status == 405 {
        return Ok(None);
    }
    if status >= 400 {
        return Err(err(format!("opencode command list HTTP {status}: {body}")));
    }
    let parsed: Value = serde_json::from_str(&body)
        .map_err(|e| err(format!("opencode command parse: {e}")))?;
    let entries = parsed
        .get("data")
        .cloned()
        .or(Some(parsed))
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    let mut commands: Vec<Value> = entries
        .iter()
        .filter_map(|entry| {
            let name = entry.get("name").and_then(Value::as_str)?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let mut out = json!({"name": name});
            if let Some(description) = entry.get("description").and_then(Value::as_str) {
                let description = description.trim();
                if !description.is_empty() {
                    out["description"] = json!(description);
                }
            }
            Some(out)
        })
        .collect();
    commands.sort_by(|a, b| {
        a.get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("name").and_then(Value::as_str).unwrap_or(""))
    });
    commands.dedup_by(|a, b| {
        a.get("name").and_then(Value::as_str) == b.get("name").and_then(Value::as_str)
    });
    Ok(Some(commands))
}

/// Start and configure an idle OpenCode server/session before the first user
/// prompt. Unlike a turn, prewarm performs no model traffic: it only pays the
/// process startup, MCP registration, permission and model/agent switching
/// costs once, then parks the server for `start_turn` to reuse.
pub fn prewarm_thread(thread_id: &str) -> Result<bool, WsErr> {
    if has_running_turn(thread_id) {
        return Ok(false);
    }
    if let Ok(parked_map) = PARKED.lock() {
        if parked_map.contains_key(thread_id) {
            return Ok(false);
        }
    }

    let conn = open_state_rw()?;
    let stored_session: Option<String> = conn
        .query_row(
            "SELECT provider_thread_id FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let turn_runtime_mode: String = conn
        .query_row(
            "SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "full-access".to_string());
    let (interaction_mode, model_selection_json, working_directory): (String, Option<String>, Option<String>) = conn
        .query_row(
            "SELECT interaction_mode, model_selection_json, COALESCE(working_directory, worktree_path, associated_worktree_path) \
             FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| err(format!("opencode prewarm thread lookup: {e}")))?;
    let cwd = working_directory
        .filter(|path| !path.is_empty() && std::path::Path::new(path).is_dir())
        .unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "/".to_string()));
    let model_selection = model_selection_json
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let selected_model = model_selection
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_string);
    let selected_agent = model_selection
        .pointer("/options/agent")
        .and_then(Value::as_str)
        .map(str::to_string);
    let selected_variant = model_selection
        .pointer("/options/variant")
        .and_then(Value::as_str)
        .map(str::to_string);

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
    let mut child = command
        .spawn()
        .map_err(|e| err(format!("could not launch `opencode serve`: {e}")))?;

    let auth = basic_auth_header(&password);
    let prepared = (|| -> Result<String, WsErr> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut protocol = detect_protocol(port, &auth);
        loop {
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
        protocol = detect_protocol(port, &auth);
        install_gateway_mcp(port, &auth, &cwd);

        let session_id = if let Some(session_id) = stored_session.clone() {
            session_id
        } else if protocol == OpenCodeProtocol::V2 {
            let (status, body) = http_request(
                port,
                "POST",
                "/api/session",
                &auth,
                Some(&json!({
                    "location": {"directory": cwd},
                    "title": format!("synara-{thread_id}"),
                    "permissions": opencode_permission_rules(&turn_runtime_mode, &interaction_mode),
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

        if protocol == OpenCodeProtocol::V2 {
            let path = format!("/api/session/{session_id}");
            let (status, body) = http_request(
                port,
                "PATCH",
                &path,
                &auth,
                Some(&json!({
                    "permissions": opencode_permission_rules(&turn_runtime_mode, &interaction_mode),
                })),
            )?;
            if status >= 400 && status != 404 && status != 405 {
                return Err(err(format!("opencode permission rules HTTP {status}: {body}")));
            }
            if stored_session.is_none() {
                let path = format!(
                    "/api/experimental/session/{session_id}/instructions/entries/synara"
                );
                let (status, _) = http_request(
                    port,
                    "PUT",
                    &path,
                    &auth,
                    Some(&json!({"value": crate::harness_policy::first_prompt_prefix()})),
                )?;
                if status >= 400 && status != 404 && status != 405 {
                    eprintln!("opencode_runner: prewarm instruction install HTTP {status}");
                }
            }
            if let Some(agent) = selected_agent.as_deref() {
                let path = format!("/api/session/{session_id}/agent");
                let (status, body) = http_request(
                    port,
                    "POST",
                    &path,
                    &auth,
                    Some(&json!({"agent": agent})),
                )?;
                if status >= 400 && status != 404 && status != 405 {
                    return Err(err(format!("opencode agent switch HTTP {status}: {body}")));
                }
            }
            if let Some(model) = selected_model.as_deref() {
                if let Some((provider_id, model_id)) = model.split_once('/') {
                    let mut native_model = json!({"providerID": provider_id, "id": model_id});
                    if let Some(variant) = selected_variant.as_deref() {
                        native_model["variant"] = json!(variant);
                    }
                    let path = format!("/api/session/{session_id}/model");
                    let (status, body) = http_request(
                        port,
                        "POST",
                        &path,
                        &auth,
                        Some(&json!({"model": native_model})),
                    )?;
                    if status >= 400 && status != 404 && status != 405 {
                        return Err(err(format!("opencode model switch HTTP {status}: {body}")));
                    }
                }
            }
        }
        Ok(session_id)
    })();

    match prepared {
        Ok(session_id) => {
            if let Ok(mut parked_map) = PARKED.lock() {
                parked_map.insert(
                    thread_id.to_string(),
                    ParkedOpenCode {
                        child: Arc::new(Mutex::new(child)),
                        port,
                        password,
                        session_id,
                        launch_key: cwd,
                    },
                );
            }
            Ok(true)
        }
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(e)
        }
    }
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
    let interaction_mode: String = conn
        .query_row(
            "SELECT interaction_mode FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "default".to_string());
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

    let model_selection = conn
        .query_row(
            "SELECT model_selection_json FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let selected_model = model_selection
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_string);
    let selected_agent = model_selection
        .pointer("/options/agent")
        .and_then(Value::as_str)
        .map(str::to_string);
    let selected_variant = model_selection
        .pointer("/options/variant")
        .and_then(Value::as_str)
        .map(str::to_string);

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
    let running_provider_session_id = Arc::new(Mutex::new(reused_session_id.clone()));
    {
        let mut map = RUNNING.lock().map_err(|_| err("running map"))?;
        map.insert(
            thread_id.to_string(),
            RunningTurn {
                child: child_shared.clone(),
                cancelled: cancelled.clone(),
                port,
                password: password.clone(),
                provider_session_id: running_provider_session_id.clone(),
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
                    "permissions": opencode_permission_rules(&turn_runtime_mode, &interaction_mode),
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
        if let Ok(mut running_session) = running_provider_session_id.lock() {
            *running_session = Some(session_id.clone());
        }
        if protocol == OpenCodeProtocol::V2 {
            let path = format!("/api/session/{session_id}");
            let (status, body) = http_request(
                port,
                "PATCH",
                &path,
                &auth,
                Some(&json!({
                    "permissions": opencode_permission_rules(&turn_runtime_mode, &interaction_mode),
                })),
            )?;
            if status >= 400 && status != 404 && status != 405 {
                return Err(err(format!("opencode permission rules HTTP {status}: {body}")));
            }
        }
        set_session(&conn, thread_id, "running", Some(&turn_id), None, Some(&session_id))?;

        // Subscribe to events before prompting so no delta is missed.
        let event_path = if protocol == OpenCodeProtocol::V2 { "/api/event" } else { "/event" };
        let sse = open_sse_stream(port, &auth, event_path)?;

        // V2 keeps durable system instructions out of the user message and
        // switches agent/model through native session endpoints, mirroring the
        // upstream client boundary. Older fakes/servers without the experimental
        // instruction endpoint fall back to first-prompt prefix injection.
        let mut use_instruction_fallback = false;
        if protocol == OpenCodeProtocol::V2 {
            if stored_session.is_none() {
                let path = format!(
                    "/api/experimental/session/{session_id}/instructions/entries/synara"
                );
                let (status, _) = http_request(
                    port,
                    "PUT",
                    &path,
                    &auth,
                    Some(&json!({"value": crate::harness_policy::first_prompt_prefix()})),
                )?;
                use_instruction_fallback = status == 404 || status == 405;
                if status >= 400 && !use_instruction_fallback {
                    eprintln!("opencode_runner: instruction install HTTP {status}");
                }
            }
            if let Some(agent) = selected_agent.as_deref() {
                let path = format!("/api/session/{session_id}/agent");
                let (status, body) = http_request(
                    port,
                    "POST",
                    &path,
                    &auth,
                    Some(&json!({"agent": agent})),
                )?;
                if status >= 400 && status != 404 && status != 405 {
                    return Err(err(format!("opencode agent switch HTTP {status}: {body}")));
                }
            }
            if let Some(model) = selected_model.as_deref() {
                if let Some((provider_id, model_id)) = model.split_once('/') {
                    let mut native_model = json!({"providerID": provider_id, "id": model_id});
                    if let Some(variant) = selected_variant.as_deref() {
                        native_model["variant"] = json!(variant);
                    }
                    let path = format!("/api/session/{session_id}/model");
                    let (status, body) = http_request(
                        port,
                        "POST",
                        &path,
                        &auth,
                        Some(&json!({"model": native_model})),
                    )?;
                    if status >= 400 && status != 404 && status != 405 {
                        return Err(err(format!("opencode model switch HTTP {status}: {body}")));
                    }
                }
            }
        }
        let prompt_text = if stored_session.is_none()
            && (protocol != OpenCodeProtocol::V2 || use_instruction_fallback)
        {
            format!("{}{}", crate::harness_policy::first_prompt_prefix(), text)
        } else {
            text.to_string()
        };
        let attachments = crate::provider_attachments::message_attachments(thread_id, message_id)?;
        let prompt_text = crate::provider_attachments::append_file_attachments_prompt_block(
            &prompt_text,
            &attachments,
            false,
        );
        let file_parts = crate::provider_attachments::opencode_image_file_parts(&attachments);
        let files: Vec<Value> = file_parts
            .iter()
            .map(|part| {
                json!({
                    "uri": part.get("uri").cloned().unwrap_or(Value::Null),
                    "name": part.get("name").cloned().unwrap_or(Value::Null),
                })
            })
            .collect();
        let mut v2_command: Option<(String, String)> = None;
        if protocol == OpenCodeProtocol::V2 {
            let trimmed = prompt_text.trim();
            if let Some(rest) = trimmed.strip_prefix('/') {
                let mut parts = rest.splitn(2, char::is_whitespace);
                let name = parts.next().unwrap_or("").to_string();
                let argument = parts.next().unwrap_or("").trim().to_string();
                if !name.is_empty() {
                    let list_path = format!(
                        "/api/command?location={}",
                        urlencode_component(&cwd)
                    );
                    if let Ok((status, body)) = http_request(port, "GET", &list_path, &auth, None) {
                        if status < 400 {
                            let commands = serde_json::from_str::<Value>(&body)
                                .ok()
                                .and_then(|v| {
                                    v.get("data")
                                        .cloned()
                                        .or(Some(v))
                                        .and_then(|v| v.as_array().cloned())
                                })
                                .unwrap_or_default();
                            if commands.iter().any(|entry| {
                                entry.get("name").and_then(Value::as_str) == Some(name.as_str())
                            }) {
                                v2_command = Some((name, argument));
                            }
                        }
                    }
                }
            }
        }
        let (prompt_path, prompt_body) = if protocol == OpenCodeProtocol::V2 {
            if let Some((name, argument)) = v2_command {
                (
                    format!("/api/session/{session_id}/command"),
                    json!({
                        "name": name,
                        "text": argument,
                        "files": files,
                        "delivery": "steer",
                    }),
                )
            } else {
                (
                    format!("/api/session/{session_id}/prompt"),
                    json!({
                        "text": prompt_text,
                        "files": files,
                        "delivery": "steer",
                    }),
                )
            }
        } else {
            (
                format!("/session/{session_id}/prompt_async"),
                json!({
                    "parts": std::iter::once(json!({"type": "text", "text": prompt_text}))
                        .chain(file_parts.iter().map(|part| json!({
                            "type": "file",
                            "mime": part.get("mime").cloned().unwrap_or(Value::Null),
                            "filename": part.get("filename").cloned().unwrap_or(Value::Null),
                            "url": part.get("url").cloned().unwrap_or(Value::Null),
                        })))
                        .collect::<Vec<_>>(),
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
                "form.created" if protocol == OpenCodeProtocol::V2 => {
                    let data = v.get("data").cloned().unwrap_or(Value::Null);
                    handle_form_request(
                        port,
                        &auth,
                        &session_id,
                        &data,
                        thread_id,
                        &turn_id,
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
