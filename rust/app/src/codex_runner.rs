//! Codex provider runtime: drives the official `codex app-server` over
//! newline-delimited JSON-RPC on stdio. Authentication never passes through
//! Synara: the official binary reads its own credentials, so OAuth tokens
//! are never extracted or replayed outside it (Anthropic/OpenAI-style
//! account bans hit ADEs that piggyback on CLI credentials).
//!
//! v1 vertical slice: initialize handshake, thread start/resume, turn start,
//! streaming agentMessage deltas into the projection, terminal
//! turn/completed|aborted|error handling. Server->client approval requests
//! are auto-accepted for now; the upstream approval-card integration is a
//! later slice, as are long-lived process parking, MCP injection and
//! collaboration modes.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use rusqlite::params;
use serde_json::{json, Value};

use crate::acp_runner::{
    append_assistant_text, err, session_set_event, set_session, set_turn_state,
};
use crate::orchestration_engine::{open_state_rw, uuid_v4};
use crate::todo::{iso_now_ms, WsErr};

struct RunningTurn {
    stdin: Arc<Mutex<std::process::ChildStdin>>,
    thread_ref: String,
    turn_ref: String,
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

/// Interrupt the live turn through the app-server's own `turn/interrupt`
/// request; the process itself stays owned by the runner thread.
pub fn cancel_turn(thread_id: &str) {
    let (stdin, thread_ref, turn_ref, flag) = match RUNNING.lock() {
        Ok(m) => match m.get(thread_id) {
            Some(rt) => (
                rt.stdin.clone(),
                rt.thread_ref.clone(),
                rt.turn_ref.clone(),
                rt.cancelled.clone(),
            ),
            None => return,
        },
        Err(_) => return,
    };
    flag.store(true, Ordering::SeqCst);
    if turn_ref.is_empty() {
        return;
    }
    let cancel = json!({
        "jsonrpc": "2.0",
        "id": -1,
        "method": "turn/interrupt",
        "params": {"threadId": thread_ref, "turnId": turn_ref},
    });
    if let Ok(mut s) = stdin.lock() {
        let _ = s.write_all(cancel.to_string().as_bytes());
        let _ = s.write_all(b"\n");
        let _ = s.flush();
    }
}

fn codex_binary() -> String {
    for key in ["SYNARA_CODEX_CLI", "CODEX_BINARY_PATH"] {
        if let Ok(p) = std::env::var(key) {
            if !p.trim().is_empty() {
                return p;
            }
        }
    }
    "codex".to_string()
}

/// Runtime mode -> turn/thread overrides, mirroring upstream's mapping for
/// the modes this slice supports.
fn runtime_overrides(runtime_mode: &str) -> Value {
    match runtime_mode {
        "approval-required" | "plan" | "auto" => json!({
            "approvalPolicy": "on-request",
            "approvalsReviewer": "user",
            "sandbox": "workspace-write",
        }),
        _ => json!({
            "approvalPolicy": "never",
            "approvalsReviewer": "user",
            "sandbox": "danger-full-access",
        }),
    }
}

struct Rpc {
    stdin: Arc<Mutex<std::process::ChildStdin>>,
    reader: BufReader<std::process::ChildStdout>,
    next_id: i64,
}

impl Rpc {
    fn write_raw(&self, msg: &Value) -> Result<(), WsErr> {
        let mut s = self.stdin.lock().map_err(|_| err("codex stdin lock"))?;
        s.write_all(msg.to_string().as_bytes())
            .and_then(|_| s.write_all(b"\n"))
            .and_then(|_| s.flush())
            .map_err(|e| err(format!("codex write: {e}")))
    }

    fn read_msg(&mut self) -> Result<Value, WsErr> {
        loop {
            let mut line = String::new();
            let n = self
                .reader
                .read_line(&mut line)
                .map_err(|e| err(format!("codex read: {e}")))?;
            if n == 0 {
                return Err(err("codex app-server closed stdout"));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<Value>(trimmed) {
                Ok(v) => return Ok(v),
                Err(_) => continue,
            }
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, WsErr> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_raw(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        loop {
            let v = self.read_msg()?;
            if v.get("id").and_then(Value::as_i64) == Some(id)
                && (v.get("result").is_some() || v.get("error").is_some())
            {
                if let Some(e) = v.get("error") {
                    return Err(err(format!(
                        "codex {method} error: {}",
                        e.get("message").and_then(Value::as_str).unwrap_or("?")
                    )));
                }
                return Ok(v.get("result").cloned().unwrap_or(Value::Null));
            }
            self.answer_server_request(&v);
        }
    }

    /// Server->client requests (approvals, user input) are auto-accepted in
    /// this slice; approval cards are a later integration.
    fn answer_server_request(&mut self, v: &Value) {
        if v.get("method").is_none() || v.get("id").is_none() {
            return;
        }
        if v.get("id").and_then(Value::as_i64) == Some(-1) {
            return;
        }
        if let Some(rid) = v.get("id").cloned() {
            let _ = self.write_raw(&json!({
                "jsonrpc": "2.0",
                "id": rid,
                "result": {"decision": "accept"},
            }));
        }
    }
}

pub fn supports_provider(provider: &str) -> bool {
    provider == "codex"
}

pub fn start_turn(thread_id: String, message_id: String, text: String, cwd: String) {
    std::thread::spawn(move || {
        if let Err(e) = run_turn(&thread_id, &message_id, &text, &cwd) {
            eprintln!("codex_runner: turn failed for {thread_id}: {}", e.0);
        }
    });
}

fn run_turn(thread_id: &str, message_id: &str, text: &str, cwd: &str) -> Result<(), WsErr> {
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    let stored_thread: Option<String> = conn
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
         VALUES (?1, 'running', 'codex', NULL, ?2, ?3, NULL, ?4, \
          COALESCE((SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1), 'full-access'))",
        params![thread_id, stored_thread, turn_id, now],
    )
    .map_err(|e| err(format!("session start: {e}")))?;
    session_set_event(
        &conn,
        thread_id,
        &json!({
            "threadId": thread_id,
            "status": "running",
            "providerName": "codex",
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
    let selected_model: Option<String> = conn
        .query_row(
            "SELECT model_selection_json FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.get("model").and_then(Value::as_str).map(str::to_string));

    let mut command = Command::new(codex_binary());
    command
        .arg("app-server")
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child: Child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = set_turn_state(&conn, thread_id, &turn_id, "error", true);
            let msg = format!("could not launch `codex app-server`: {e}");
            let _ = set_session(&conn, thread_id, "error", Some(""), Some(&msg), None);
            return Err(err(msg));
        }
    };
    let stdin = child.stdin.take().ok_or_else(|| err("codex stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| err("codex stdout"))?;
    let stdin = Arc::new(Mutex::new(stdin));
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut rpc = Rpc {
        stdin: stdin.clone(),
        reader: BufReader::new(stdout),
        next_id: 1,
    };

    let result = (|| -> Result<String, WsErr> {
        rpc.call(
            "initialize",
            json!({
                "clientInfo": {"name": "synara_desktop", "title": "Synara Desktop", "version": "0.1.0"},
                "capabilities": {"experimentalApi": true},
            }),
        )?;
        rpc.write_raw(&json!({"jsonrpc": "2.0", "method": "initialized"}))?;

        let overrides = runtime_overrides(&turn_runtime_mode);
        let mcp_config = json!({
            "mcpServers": {
                "synara": {
                    "url": crate::agent_gateway_mcp::agent_gateway_url(),
                    "httpHeaders": {
                        "Authorization": format!(
                            "Bearer {}",
                            crate::agent_gateway_mcp::agent_gateway_token()
                        ),
                    },
                },
            },
        });
        let mut open_params = json!({
            "cwd": cwd,
            "approvalPolicy": overrides["approvalPolicy"],
            "approvalsReviewer": overrides["approvalsReviewer"],
            "sandbox": overrides["sandbox"],
            "config": mcp_config,
        });
        if let Some(model) = selected_model.as_deref() {
            open_params["model"] = json!(model);
        }
        let (open_method, open_params) = if let Some(sid) = stored_thread.as_deref() {
            open_params["threadId"] = json!(sid);
            open_params["excludeTurns"] = json!(true);
            ("thread/resume", open_params)
        } else {
            open_params["experimentalRawEvents"] = json!(false);
            ("thread/start", open_params)
        };
        let open_res = rpc.call(open_method, open_params)?;
        let provider_thread_id = open_res
            .pointer("/thread/id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                open_res
                    .get("threadId")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .ok_or_else(|| err(format!("{open_method} returned no thread id")))?;

        // Fresh sessions get the host policy + browser routing rules
        // prepended to the first prompt; resumed threads already carry it.
        let prompt_text = if stored_thread.is_none() {
            format!("{}{}", crate::harness_policy::first_prompt_prefix(), text)
        } else {
            text.to_string()
        };
        let mut turn_params = json!({
            "threadId": provider_thread_id,
            "input": [{"type": "text", "text": prompt_text}],
            "summary": "auto",
        });
        if let Some(model) = selected_model.as_deref() {
            turn_params["model"] = json!(model);
        }
        let turn_res = rpc.call("turn/start", turn_params)?;
        let provider_turn_id = turn_res
            .pointer("/turn/id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        {
            let mut map = RUNNING.lock().map_err(|_| err("running map"))?;
            map.insert(
                thread_id.to_string(),
                RunningTurn {
                    stdin: stdin.clone(),
                    thread_ref: provider_thread_id.clone(),
                    turn_ref: provider_turn_id.clone(),
                    cancelled: cancelled.clone(),
                },
            );
        }
        set_session(
            &conn,
            thread_id,
            "running",
            Some(&turn_id),
            None,
            Some(&provider_thread_id),
        )?;

        let mut text_buf = String::new();
        let mut last_flush = std::time::Instant::now();
        let terminal = loop {
            let v = rpc.read_msg()?;
            let method = v.get("method").and_then(Value::as_str).unwrap_or("");
            match method {
                "item/agentMessage/delta" => {
                    if let Some(d) = v.pointer("/params/delta").and_then(Value::as_str) {
                        text_buf.push_str(d);
                    }
                }
                "turn/completed" => break "completed",
                "turn/aborted" => break "cancelled",
                "error" => {
                    let will_retry = v
                        .pointer("/params/willRetry")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    if !will_retry {
                        let msg = v
                            .pointer("/params/error/message")
                            .and_then(Value::as_str)
                            .unwrap_or("codex turn error")
                            .to_string();
                        return Err(err(msg));
                    }
                }
                "" => {
                    // Bare response frames (e.g. turn/interrupt reply).
                }
                _ => {
                    if v.get("id").is_some() {
                        rpc.answer_server_request(&v);
                    }
                }
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
    let _ = child.kill();
    let _ = child.wait();

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
                    "providerName": "codex",
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
                    "providerName": "codex",
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
    fn runtime_overrides_map_full_access() {
        let v = super::runtime_overrides("full-access");
        assert_eq!(v["approvalPolicy"], "never");
        assert_eq!(v["sandbox"], "danger-full-access");
        let g = super::runtime_overrides("approval-required");
        assert_eq!(g["approvalPolicy"], "on-request");
    }
}
