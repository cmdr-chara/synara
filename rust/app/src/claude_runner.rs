//! Claude provider runtime: drives the official `claude` CLI in headless
//! stream-json mode. Synara never reads Claude OAuth credentials or cookies:
//! the official binary alone owns authentication, so account tokens are never
//! extracted or replayed — the pattern that gets ADE users banned.
//!
//! v1 vertical slice: one headless process per turn with `--resume` for
//! session continuity, assistant text streamed into the projection, terminal
//! result handling. Missing vs upstream: SDK control protocol (permissions,
//! hooks, slash commands), MCP injection, effort/model switching, subagent
//! tracking, edge-block retry.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
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

/// Interrupt a live turn by killing the headless CLI process. The CLI is
/// stateless across turns (`--resume` rebinds the session), so killing it
/// is a clean cancellation unlike stateful app-servers.
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

fn claude_binary() -> String {
    for key in ["SYNARA_CLAUDE_CLI", "CLAUDE_BINARY_PATH"] {
        if let Ok(p) = std::env::var(key) {
            if !p.trim().is_empty() {
                return p;
            }
        }
    }
    "claude".to_string()
}

pub fn supports_provider(provider: &str) -> bool {
    provider == "claudeAgent"
}

pub fn start_turn(thread_id: String, message_id: String, text: String, cwd: String) {
    std::thread::spawn(move || {
        if let Err(e) = run_turn(&thread_id, &message_id, &text, &cwd) {
            eprintln!("claude_runner: turn failed for {thread_id}: {}", e.0);
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
         VALUES (?1, 'running', 'claudeAgent', NULL, ?2, ?3, NULL, ?4, \
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
            "providerName": "claudeAgent",
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

    let mut args: Vec<String> = vec![
        "-p".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
    ];
    if turn_runtime_mode == "full-access" {
        args.push("--dangerously-skip-permissions".into());
    }
    if let Some(model) = selected_model.as_deref() {
        args.push("--model".into());
        args.push(model.to_string());
    }
    if let Some(sid) = stored_session.as_deref() {
        args.push("--resume".into());
        args.push(sid.to_string());
    }
    // Expose the synara agent gateway (browser_*, computer_*, synara_*) to
    // the CLI over HTTP MCP; auth rides in the bearer header, never in files.
    let mcp_config = json!({
        "mcpServers": {
            "synara": {
                "type": "http",
                "url": crate::agent_gateway_mcp::agent_gateway_url(),
                "headers": {
                    "Authorization": format!(
                        "Bearer {}",
                        crate::agent_gateway_mcp::agent_gateway_token()
                    ),
                },
            },
        },
    });
    args.push("--mcp-config".into());
    args.push(mcp_config.to_string());

    let mut command = Command::new(claude_binary());
    command
        .args(&args)
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child: Child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = set_turn_state(&conn, thread_id, &turn_id, "error", true);
            let msg = format!("could not launch `claude` CLI: {e}");
            let _ = set_session(&conn, thread_id, "error", Some(""), Some(&msg), None);
            return Err(err(msg));
        }
    };
    let prompt_text = if stored_session.is_none() {
        format!("{}{}", crate::harness_policy::first_prompt_prefix(), text)
    } else {
        text.to_string()
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt_text.as_bytes());
        // Closing stdin starts the headless run.
        drop(stdin);
    }
    let stdout = child.stdout.take().ok_or_else(|| err("claude stdout"))?;
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
    set_session(&conn, thread_id, "running", Some(&turn_id), None, None)?;

    let result = (|| -> Result<(String, Option<String>), WsErr> {
        let mut reader = BufReader::new(stdout);
        let mut session_id: Option<String> = stored_session.clone();
        let mut terminal = "completed";
        let mut fatal: Option<String> = None;
        loop {
            let mut line = String::new();
            let n = reader
                .read_line(&mut line)
                .map_err(|e| err(format!("claude read: {e}")))?;
            if n == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            match v.get("type").and_then(Value::as_str).unwrap_or("") {
                "system" => {
                    if v.get("subtype").and_then(Value::as_str) == Some("init") {
                        if let Some(sid) = v.get("session_id").and_then(Value::as_str) {
                            session_id = Some(sid.to_string());
                            let _ = set_session(
                                &conn,
                                thread_id,
                                "running",
                                Some(&turn_id),
                                None,
                                Some(sid),
                            );
                        }
                    }
                }
                "assistant" => {
                    let text_out: String = v
                        .pointer("/message/content")
                        .and_then(Value::as_array)
                        .map(|parts| {
                            parts
                                .iter()
                                .filter(|p| p.get("type").and_then(Value::as_str) == Some("text"))
                                .filter_map(|p| p.get("text").and_then(Value::as_str))
                                .collect::<Vec<_>>()
                                .join("")
                        })
                        .unwrap_or_default();
                    if !text_out.is_empty() {
                        if let Ok(c) = open_state_rw() {
                            let _ = append_assistant_text(
                                &c,
                                thread_id,
                                &assistant_msg_id,
                                &turn_id,
                                &text_out,
                                true,
                            );
                        }
                    }
                }
                "result" => {
                    let subtype = v.get("subtype").and_then(Value::as_str).unwrap_or("");
                    if subtype == "success" {
                        terminal = "completed";
                    } else {
                        let msg = v
                            .get("result")
                            .and_then(Value::as_str)
                            .or_else(|| v.pointer("/error/message").and_then(Value::as_str))
                            .unwrap_or("claude turn error");
                        fatal = Some(msg.to_string());
                    }
                    break;
                }
                _ => {}
            }
        }
        if let Ok(c) = open_state_rw() {
            let _ = append_assistant_text(&c, thread_id, &assistant_msg_id, &turn_id, "", false);
        }
        if let Some(msg) = fatal {
            return Err(err(msg));
        }
        Ok((terminal.to_string(), session_id))
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
        Ok((reason, session_id)) => {
            let interrupted = cancelled_flag || reason == "cancelled";
            let state = if interrupted { "interrupted" } else { "completed" };
            let status = if interrupted { "interrupted" } else { "ready" };
            set_turn_state(&conn, thread_id, &turn_id, state, true)?;
            set_session(&conn, thread_id, status, Some(""), None, session_id.as_deref())?;
            session_set_event(
                &conn,
                thread_id,
                &json!({
                    "threadId": thread_id,
                    "status": status,
                    "providerName": "claudeAgent",
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
                    "providerName": "claudeAgent",
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
