//! Pi provider runtime: drives the official `pi` CLI through its documented
//! JSONL RPC mode (`pi --mode rpc`). Credentials stay inside the official
//! binary; Synara never reads or replays them.
//!
//! This is the subprocess-native slice: warm process reuse, prompts, native
//! steering, abort, streamed assistant text, and terminal settlement. Pi's
//! RPC surface does not expose Synara-style interactive tool approval, so
//! supervised modes launch with read-only tools instead of silently permitting
//! mutations. SDK-only extension/hook parity remains upstream-specific.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use rusqlite::params;
use serde_json::{json, Value};

use crate::acp_runner::{append_assistant_text, err, session_set_event, set_session, set_turn_state};
use crate::orchestration_engine::{open_state_rw, uuid_v4};
use crate::todo::{iso_now_ms, WsErr};

struct RunningTurn {
    stdin: Arc<Mutex<ChildStdin>>,
    cancelled: Arc<AtomicBool>,
}

static RUNNING: LazyLock<Mutex<HashMap<String, RunningTurn>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

struct ParkedPi {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    reader: BufReader<ChildStdout>,
    launch_key: String,
}

static PARKED: LazyLock<Mutex<HashMap<String, ParkedPi>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn drop_parked(mut parked: ParkedPi) {
    let _ = parked.child.kill();
    let _ = parked.child.wait();
}

fn take_parked(thread_id: &str, launch_key: &str) -> Option<ParkedPi> {
    let parked = PARKED.lock().ok()?.remove(thread_id)?;
    if parked.launch_key != launch_key {
        drop_parked(parked);
        return None;
    }
    let mut parked = parked;
    match parked.child.try_wait() {
        Ok(None) => Some(parked),
        _ => {
            drop_parked(parked);
            None
        }
    }
}

pub fn drop_thread_resources(thread_id: &str) {
    if let Ok(mut parked_map) = PARKED.lock() {
        if let Some(parked) = parked_map.remove(thread_id) {
            drop_parked(parked);
        }
    }
}

/// Stop every parked provider process when the host shuts down. Parked
/// processes are in-memory only; leaving them behind would keep provider
/// sessions and local servers alive after Synara can no longer control them.
pub fn drop_all_resources() {
    let parked = if let Ok(mut parked_map) = PARKED.lock() {
        parked_map.drain().map(|(_, parked)| parked).collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    for parked in parked {
        drop_parked(parked);
    }
}

pub fn has_running_turn(thread_id: &str) -> bool {
    RUNNING
        .lock()
        .ok()
        .map(|m| m.contains_key(thread_id))
        .unwrap_or(false)
}

pub fn supports_provider(provider: &str) -> bool {
    provider == "pi"
}

fn pi_binary() -> String {
    for key in ["SYNARA_PI_CLI", "PI_BINARY_PATH"] {
        if let Ok(path) = std::env::var(key) {
            if !path.trim().is_empty() {
                return path;
            }
        }
    }
    "pi".to_string()
}

fn write_line(stdin: &Arc<Mutex<ChildStdin>>, value: &Value) -> Result<(), WsErr> {
    let mut stdin = stdin.lock().map_err(|_| err("pi stdin lock"))?;
    stdin
        .write_all(value.to_string().as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|e| err(format!("pi write: {e}")))
}

pub fn steer_turn(thread_id: &str, text: &str) -> bool {
    let stdin = match RUNNING.lock() {
        Ok(map) => match map.get(thread_id) {
            Some(rt) => rt.stdin.clone(),
            None => return false,
        },
        Err(_) => return false,
    };
    write_line(&stdin, &json!({"type":"steer","message":text})).is_ok()
}

pub fn cancel_turn(thread_id: &str) {
    let (stdin, flag) = match RUNNING.lock() {
        Ok(map) => match map.get(thread_id) {
            Some(rt) => (rt.stdin.clone(), rt.cancelled.clone()),
            None => return,
        },
        Err(_) => return,
    };
    flag.store(true, Ordering::SeqCst);
    let _ = write_line(&stdin, &json!({"type":"abort"}));
}

fn read_json_line(reader: &mut BufReader<ChildStdout>) -> Result<Option<Value>, WsErr> {
    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| err(format!("pi read: {e}")))?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\n', '\r']);
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            return Ok(Some(value));
        }
    }
}

fn spawn_pi(cwd: &str, runtime_mode: &str, model: Option<&str>, thread_id: &str) -> Result<(Child, ChildStdin, ChildStdout), WsErr> {
    let session_id: String = thread_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        .collect();
    let session_dir = std::path::PathBuf::from(crate::todo::state_dir()).join("pi-sessions");
    let _ = std::fs::create_dir_all(&session_dir);
    let mut args = vec![
        "--mode".to_string(),
        "rpc".to_string(),
        "--session-dir".to_string(),
        session_dir.display().to_string(),
        "--session-id".to_string(),
        session_id,
    ];
    if runtime_mode != "full-access" {
        // Pi RPC has no interactive permission callback. Keep supervised modes
        // read-only rather than letting bash/edit/write execute unchecked.
        args.push("--tools".to_string());
        args.push("read,grep,find,ls".to_string());
    }
    if let Some(model) = model.map(str::trim).filter(|model| !model.is_empty()) {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    let mut child = Command::new(pi_binary())
        .args(&args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| err(format!("could not launch `pi --mode rpc`: {e}")))?;
    let stdin = child.stdin.take().ok_or_else(|| err("pi stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| err("pi stdout"))?;
    Ok((child, stdin, stdout))
}



/// Run one discovery command against a temporary Pi RPC process. The process
/// uses `--no-session`, so discovery cannot mutate the user's session store.
fn rpc_query(cwd: &str, command: &str) -> Result<Value, WsErr> {
    let mut cmd = Command::new(pi_binary());
    cmd.arg("--mode")
        .arg("rpc")
        .arg("--no-session")
        .arg("--no-context-files")
        .current_dir(if cwd.is_empty() { "/" } else { cwd })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().map_err(|e| err(format!("pi spawn: {e}")))?;
    let mut stdin = child.stdin.take().ok_or_else(|| err("pi stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| err("pi stdout"))?;
    let command_value = json!({"type":command});
    stdin
        .write_all(command_value.to_string().as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|e| err(format!("pi write: {e}")))?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        while reader.read_line(&mut line).map(|n| n > 0).unwrap_or(false) {
            if let Ok(value) = serde_json::from_str::<Value>(line.trim_end_matches(['\n','\r'])) {
                let _ = tx.send(value);
            }
            line.clear();
        }
    });
    let deadline = std::time::Duration::from_secs(20);
    let started = std::time::Instant::now();
    let mut result = None;
    while started.elapsed() < deadline {
        match rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(value)
                if value.get("type").and_then(Value::as_str) == Some("response")
                    && value.get("command").and_then(Value::as_str) == Some(command) =>
            {
                result = Some(value);
                break;
            }
            Ok(_) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    let response = result.ok_or_else(|| err(format!("pi {command} timed out")))?;
    if response.get("success").and_then(Value::as_bool) != Some(true) {
        return Err(err(format!(
            "pi {command} failed: {}",
            response.get("error").and_then(Value::as_str).unwrap_or("unknown error")
        )));
    }
    Ok(response.get("data").cloned().unwrap_or(json!({})))
}

pub fn discover_models(cwd: &str) -> Result<Vec<Value>, WsErr> {
    let data = rpc_query(cwd, "get_available_models")?;
    let models = data
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(models
        .iter()
        .filter_map(|model| {
            let provider = model.get("provider").and_then(Value::as_str)?;
            let id = model.get("id").and_then(Value::as_str)?;
            let name = model.get("name").and_then(Value::as_str).unwrap_or(id);
            let efforts = if model.get("reasoning").and_then(Value::as_bool) == Some(true) {
                vec!["low", "medium", "high"]
            } else {
                Vec::new()
            };
            Some(json!({
                "slug": format!("{provider}/{id}"),
                "providerID": provider,
                "modelID": id,
                "name": name,
                "variants": [],
                "supportedReasoningEfforts": efforts,
            }))
        })
        .collect())
}

pub fn discover_commands(cwd: &str) -> Result<Vec<Value>, WsErr> {
    let data = rpc_query(cwd, "get_commands")?;
    let commands = data
        .get("commands")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(commands
        .iter()
        .filter_map(|command| {
            let name = command.get("name").and_then(Value::as_str)?;
            let mut out = json!({"name": name});
            if let Some(description) = command.get("description").and_then(Value::as_str) {
                if !description.is_empty() {
                    out["description"] = json!(description);
                }
            }
            Some(out)
        })
        .collect())
}

/// Compact a warm Pi session using the documented RPC command. Pi performs the
/// summarization inside the official CLI and returns the compact response on
/// stdout, so no transcript or credential material crosses Synara's process.
pub fn compact_thread(thread_id: &str) -> Result<bool, WsErr> {
    if has_running_turn(thread_id) {
        return Err(err("cannot compact while a turn is running"));
    }
    let mut parked = {
        let mut parked_map = PARKED.lock().map_err(|_| err("parked map"))?;
        match parked_map.remove(thread_id) {
            Some(parked) => parked,
            None => return Ok(false),
        }
    };
    let result = (|| -> Result<bool, WsErr> {
        write_line(&parked.stdin, &json!({"type":"compact"}))?;
        for _ in 0..1200 {
            let Some(value) = read_json_line(&mut parked.reader)? else {
                return Ok(false);
            };
            if value.get("type").and_then(Value::as_str) == Some("response")
                && value.get("command").and_then(Value::as_str) == Some("compact")
            {
                if value.get("success").and_then(Value::as_bool) == Some(true) {
                    return Ok(true);
                }
                let message = value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("pi compact failed");
                return Err(err(message));
            }
        }
        Err(err("pi compact timed out"))
    })();
    let alive = matches!(parked.child.try_wait(), Ok(None));
    if alive {
        if let Ok(mut parked_map) = PARKED.lock() {
            parked_map.insert(thread_id.to_string(), parked);
        }
    } else {
        drop_parked(parked);
    }
    result
}


/// Start Pi ahead of the first user prompt. Pi owns the session file from
/// process startup, so parking this process gives the next turn a warm RPC
/// runtime without sending any model traffic.
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
    let (cwd, runtime_mode, model_selection_json): (String, String, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(NULLIF(working_directory, ''), NULLIF(worktree_path, ''), NULLIF(associated_worktree_path, ''), (SELECT workspace_root FROM projection_projects WHERE project_id=projection_threads.project_id), ''), \
                    COALESCE((SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id=?1), 'full-access'), \
                    model_selection_json \
             FROM projection_threads WHERE thread_id=?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| err(format!("pi prewarm thread lookup: {e}")))?;
    let model_selection = model_selection_json
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let selected_model = model_selection
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_string);
    let selected_variant = model_selection
        .pointer("/options/variant")
        .or_else(|| model_selection.pointer("/options/reasoningEffort"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let selected_model = selected_model.map(|model| {
        selected_variant
            .as_deref()
            .filter(|variant| !variant.is_empty() && !model.contains(':'))
            .map(|variant| format!("{model}:{variant}"))
            .unwrap_or(model)
    });
    let cwd = if !cwd.is_empty() && std::path::Path::new(&cwd).is_dir() {
        cwd
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    };
    let launch_key = format!(
        "{}|{}|{}",
        cwd,
        runtime_mode,
        selected_model.as_deref().unwrap_or("")
    );
    let (child, stdin, stdout) = spawn_pi(&cwd, &runtime_mode, selected_model.as_deref(), thread_id)?;
    if let Ok(mut parked_map) = PARKED.lock() {
        parked_map.insert(
            thread_id.to_string(),
            ParkedPi {
                child,
                stdin: Arc::new(Mutex::new(stdin)),
                reader: BufReader::new(stdout),
                launch_key,
            },
        );
    }
    Ok(true)
}

pub fn start_turn(thread_id: String, message_id: String, text: String, cwd: String) {
    std::thread::spawn(move || {
        if let Err(e) = run_turn(&thread_id, &message_id, &text, &cwd) {
            eprintln!("pi_runner: turn failed for {thread_id}: {}", e.0);
        }
    });
}

fn assistant_text(message: &Value) -> String {
    match message.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Array(parts)) => parts
            .iter()
            .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
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
         (thread_id, turn_id, pending_message_id, assistant_message_id, state, requested_at, started_at, checkpoint_files_json) \
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
         (thread_id, status, provider_name, provider_session_id, provider_thread_id, active_turn_id, last_error, updated_at, runtime_mode) \
         VALUES (?1, 'running', 'pi', NULL, NULL, ?2, NULL, ?3, \
          COALESCE((SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1), 'full-access'))",
        params![thread_id, turn_id, now],
    )
    .map_err(|e| err(format!("session start: {e}")))?;
    session_set_event(
        &conn,
        thread_id,
        &json!({"threadId": thread_id, "status": "running", "providerName": "pi", "activeTurnId": turn_id, "runtimeMode": turn_runtime_mode, "lastError": null, "updatedAt": now}),
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
    let selected_variant = model_selection
        .pointer("/options/variant")
        .or_else(|| model_selection.pointer("/options/reasoningEffort"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let selected_model = selected_model.map(|model| {
        selected_variant
            .as_deref()
            .filter(|variant| !variant.is_empty() && !model.contains(':'))
            .map(|variant| format!("{model}:{variant}"))
            .unwrap_or(model)
    });
    let launch_key = format!("{}|{}|{}", cwd, turn_runtime_mode, selected_model.as_deref().unwrap_or(""));
    let cancelled = Arc::new(AtomicBool::new(false));

    let (mut child, stdin, mut reader, reused) = if let Some(parked) = take_parked(thread_id, &launch_key) {
        (parked.child, parked.stdin, parked.reader, true)
    } else {
        let (child, stdin, stdout) = spawn_pi(&cwd, &turn_runtime_mode, selected_model.as_deref(), thread_id)?;
        (child, Arc::new(Mutex::new(stdin)), BufReader::new(stdout), false)
    };
    {
        let mut map = RUNNING.lock().map_err(|_| err("running map"))?;
        map.insert(thread_id.to_string(), RunningTurn { stdin: stdin.clone(), cancelled: cancelled.clone() });
    }

    let prompt_text = if reused || stored_session.is_some() {
        text.to_string()
    } else {
        format!("{}{}", crate::harness_policy::first_prompt_prefix(), text)
    };
    let attachments = crate::provider_attachments::message_attachments(thread_id, message_id)?;
    let prompt_text = crate::provider_attachments::append_file_attachments_prompt_block(
        &prompt_text,
        &attachments,
        false,
    );
    let prompt_images = crate::provider_attachments::image_blocks(&attachments)?;
    let result = (|| -> Result<String, WsErr> {
        let mut prompt_command = json!({"id": format!("prompt-{turn_id}"), "type":"prompt", "message": prompt_text});
        if !prompt_images.is_empty() {
            prompt_command["images"] = json!(prompt_images);
        }
        write_line(&stdin, &prompt_command)?;
        let mut saw_delta = false;
        loop {
            let Some(value) = read_json_line(&mut reader)? else {
                return Err(err("pi exited before agent_settled"));
            };
            match value.get("type").and_then(Value::as_str).unwrap_or("") {
                "response" => {
                    if value.get("command").and_then(Value::as_str) == Some("prompt")
                        && value.get("success").and_then(Value::as_bool) != Some(true)
                    {
                        let message = value.get("error").and_then(Value::as_str).unwrap_or("pi prompt rejected");
                        return Err(err(message));
                    }
                }
                "message_update" => {
                    let event = value.get("assistantMessageEvent").cloned().unwrap_or(Value::Null);
                    if event.get("type").and_then(Value::as_str) == Some("text_delta") {
                        if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                            if !delta.is_empty() {
                                saw_delta = true;
                                if let Ok(c) = open_state_rw() {
                                    let _ = append_assistant_text(&c, thread_id, &assistant_msg_id, &turn_id, delta, true);
                                }
                            }
                        }
                    }
                }
                "message_end" => {
                    let message = value.get("message").cloned().unwrap_or(Value::Null);
                    if !saw_delta && message.get("role").and_then(Value::as_str) == Some("assistant") {
                        let text_out = assistant_text(&message);
                        if !text_out.is_empty() {
                            if let Ok(c) = open_state_rw() {
                                let _ = append_assistant_text(&c, thread_id, &assistant_msg_id, &turn_id, &text_out, true);
                            }
                        }
                    }
                }
                "agent_settled" => {
                    if value.get("aborted").and_then(Value::as_bool) == Some(true) {
                        break Ok("cancelled".to_string());
                    }
                    break Ok("completed".to_string());
                }
                _ => {}
            }
        }
    })();

    if let Ok(mut map) = RUNNING.lock() {
        map.remove(thread_id);
    }
    if let Ok(c) = open_state_rw() {
        let _ = append_assistant_text(&c, thread_id, &assistant_msg_id, &turn_id, "", false);
    }

    let cancelled_flag = cancelled.load(Ordering::SeqCst);
    let clean = matches!(result, Ok(ref reason) if reason == "completed") && !cancelled_flag;
    if clean && child.try_wait().ok().flatten().is_none() {
        if let Ok(mut parked_map) = PARKED.lock() {
            parked_map.insert(thread_id.to_string(), ParkedPi { child, stdin, reader, launch_key });
        }
    } else {
        let _ = child.kill();
        let _ = child.wait();
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
            set_session(&conn, thread_id, status, Some(""), None, Some(thread_id))?;
            session_set_event(&conn, thread_id, &json!({"threadId": thread_id, "status": status, "providerName": "pi", "activeTurnId": null, "runtimeMode": turn_runtime_mode, "lastError": null, "updatedAt": iso_now_ms()}))?;
            crate::orchestration_engine::continue_queued_turn(thread_id);
            Ok(())
        }
        Err(e) => {
            let (state, status) = if cancelled_flag { ("interrupted", "interrupted") } else { ("error", "error") };
            set_turn_state(&conn, thread_id, &turn_id, state, true)?;
            set_session(&conn, thread_id, status, Some(""), Some(&e.0), Some(thread_id))?;
            session_set_event(&conn, thread_id, &json!({"threadId": thread_id, "status": status, "providerName": "pi", "activeTurnId": null, "runtimeMode": turn_runtime_mode, "lastError": e.0, "updatedAt": iso_now_ms()}))?;
            if cancelled_flag {
                crate::orchestration_engine::continue_queued_turn(thread_id);
            }
            Err(e)
        }
    }
}
