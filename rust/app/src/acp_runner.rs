//! Provider runtime: drives a real agent session over ACP (Agent Client
//! Protocol, JSON-RPC on stdio). Currently implements the Devin provider via
//! `devin acp`; the credentials stored by `devin auth login` are reused as-is.
//!
//! A turn lives on a dedicated std::thread: spawn `devin acp`, initialize,
//! session/new (or session/load when the thread already has a provider
//! session), session/prompt, then translate `session/update` notifications
//! into orchestration events and projection writes until the prompt resolves.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::orchestration_engine::{open_state_rw, uuid_v4};
use crate::todo::{iso_now_ms, WsErr};

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), None)
}

struct RunningTurn {
    stdin: Arc<Mutex<ChildStdin>>,
    session_id: String,
    cancelled: Arc<AtomicBool>,
}

static RUNNING: LazyLock<Mutex<HashMap<String, RunningTurn>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Whether a Devin turn is currently live for the thread.
pub fn has_running_turn(thread_id: &str) -> bool {
    RUNNING
        .lock()
        .ok()
        .map(|m| m.contains_key(thread_id))
        .unwrap_or(false)
}

/// Append one event row (actor `provider`, no command receipt) so the
/// subscribeThread poller picks it up and clients refetch the snapshot.
fn append_event(
    conn: &Connection,
    thread_id: &str,
    event_type: &str,
    occurred_at: &str,
    payload: Value,
) -> Result<(), WsErr> {
    let stream_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(stream_version), 0) + 1 \
             FROM orchestration_events WHERE aggregate_kind = 'thread' AND stream_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e.to_string()))?;
    conn.execute(
        "INSERT INTO orchestration_events \
         (event_id, aggregate_kind, stream_id, stream_version, event_type, occurred_at, \
          command_id, causation_event_id, correlation_id, actor_kind, payload_json, metadata_json) \
         VALUES (?1, 'thread', ?2, ?3, ?4, ?5, NULL, NULL, NULL, 'provider', ?6, '{}')",
        params![
            uuid_v4(),
            thread_id,
            stream_version,
            event_type,
            occurred_at,
            payload.to_string(),
        ],
    )
    .map_err(|e| err(format!("append {event_type}: {e}")))?;
    Ok(())
}

fn session_set_event(conn: &Connection, thread_id: &str, session: &Value) -> Result<(), WsErr> {
    append_event(
        conn,
        thread_id,
        "thread.session-set",
        &iso_now_ms(),
        json!({"threadId": thread_id, "session": session}),
    )
}

fn set_session(
    conn: &Connection,
    thread_id: &str,
    status: &str,
    active_turn: Option<&str>,
    last_error: Option<&str>,
    provider_session_id: Option<&str>,
) -> Result<(), WsErr> {
    let now = iso_now_ms();
    conn.execute(
        "INSERT OR IGNORE INTO projection_thread_sessions \
         (thread_id, status, updated_at, runtime_mode) VALUES (?1, ?2, ?3, 'full-access')",
        params![thread_id, status, now],
    )
    .map_err(|e| err(format!("session projection insert: {e}")))?;
    let mut v: Vec<rusqlite::types::Value> = vec![
        rusqlite::types::Value::Text(status.into()),
        rusqlite::types::Value::Text(now.clone()),
        rusqlite::types::Value::Text(thread_id.into()),
    ];
    let mut sets = String::from("status = ?1, updated_at = ?2, last_activity_at = ?2");
    if let Some(t) = active_turn {
        sets.push_str(&format!(", active_turn_id = ?{}", v.len() + 1));
        v.push(if t.is_empty() {
            rusqlite::types::Value::Null
        } else {
            rusqlite::types::Value::Text(t.into())
        });
    }
    if let Some(e) = last_error {
        sets.push_str(&format!(", last_error = ?{}", v.len() + 1));
        v.push(rusqlite::types::Value::Text(e.into()));
    }
    if let Some(s) = provider_session_id {
        sets.push_str(&format!(
            ", provider_session_id = ?{}, provider_thread_id = ?{}",
            v.len() + 1,
            v.len() + 2
        ));
        v.push(rusqlite::types::Value::Text(s.into()));
        v.push(rusqlite::types::Value::Text(s.into()));
    }
    let refs: Vec<&dyn rusqlite::ToSql> = v.iter().map(|x| x as _).collect();
    conn.execute(
        &format!("UPDATE projection_thread_sessions SET {sets} WHERE thread_id = ?3"),
        refs.as_slice(),
    )
    .map_err(|e| err(format!("session projection: {e}")))?;
    Ok(())
}

fn set_turn_state(
    conn: &Connection,
    thread_id: &str,
    turn_id: &str,
    state: &str,
    completed: bool,
) -> Result<(), WsErr> {
    let now = iso_now_ms();
    conn.execute(
        "UPDATE projection_turns SET state = ?3, completed_at = ?4 \
         WHERE thread_id = ?1 AND turn_id = ?2",
        params![
            thread_id,
            turn_id,
            state,
            if completed { now.as_str() } else { "" }
        ],
    )
    .map_err(|e| err(format!("turn projection: {e}")))?;
    conn.execute(
        "UPDATE projection_thread_messages SET is_streaming = 0, updated_at = ?3 \
         WHERE thread_id = ?1 AND turn_id = ?2",
        params![thread_id, turn_id, now],
    )
    .map_err(|e| err(format!("message settle: {e}")))?;
    Ok(())
}

/// Append one assistant delta (or the final empty-delta settle) to the
/// projection exactly like `project_message_sent`.
fn append_assistant_text(
    conn: &Connection,
    thread_id: &str,
    message_id: &str,
    turn_id: &str,
    delta: &str,
    streaming: bool,
) -> Result<(), WsErr> {
    let now = iso_now_ms();
    let existing: Option<String> = conn
        .query_row(
            "SELECT text FROM projection_thread_messages \
             WHERE thread_id = ?1 AND message_id = ?2",
            params![thread_id, message_id],
            |r| r.get(0),
        )
        .ok();
    let text = match (&existing, streaming) {
        (Some(t), true) => format!("{t}{delta}"),
        (Some(t), false) => t.clone(),
        (None, _) => delta.to_string(),
    };
    conn.execute(
        "INSERT INTO projection_thread_messages \
         (message_id, thread_id, turn_id, role, text, is_streaming, created_at, updated_at, \
          attachments_json, source, starts_new_turn) \
         VALUES (?1, ?2, ?3, 'assistant', ?4, ?5, ?6, ?6, '[]', 'provider', 0) \
         ON CONFLICT(thread_id, message_id) DO UPDATE SET \
          text=excluded.text, is_streaming=excluded.is_streaming, updated_at=excluded.updated_at",
        params![message_id, thread_id, turn_id, text, streaming as i64, now],
    )
    .map_err(|e| err(format!("assistant message: {e}")))?;
    append_event(
        conn,
        thread_id,
        "thread.message-sent",
        &now,
        json!({
            "threadId": thread_id,
            "messageId": message_id,
            "role": "assistant",
            "text": delta,
            "turnId": turn_id,
            "streaming": streaming,
            "source": "provider",
            "createdAt": now,
            "updatedAt": now,
        }),
    )?;
    Ok(())
}

fn append_activity(
    conn: &Connection,
    thread_id: &str,
    turn_id: &str,
    tone: &str,
    kind: &str,
    summary: &str,
    payload: &Value,
) -> Result<(), WsErr> {
    let now = iso_now_ms();
    let next_seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM projection_thread_activities \
             WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or(1);
    let activity_id = uuid_v4();
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_activities \
         (activity_id, thread_id, turn_id, tone, kind, summary, payload_json, created_at, sequence) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            activity_id,
            thread_id,
            turn_id,
            tone,
            kind,
            summary,
            payload.to_string(),
            now,
            next_seq
        ],
    )
    .map_err(|e| err(format!("activity projection: {e}")))?;
    append_event(
        conn,
        thread_id,
        "thread.activity-appended",
        &now,
        json!({
            "threadId": thread_id,
            "activity": {
                "id": activity_id,
                "tone": tone,
                "kind": kind,
                "summary": summary,
                "payload": payload,
                "turnId": turn_id,
                "sequence": next_seq,
                "createdAt": now,
            }
        }),
    )?;
    Ok(())
}

/// Stop a running turn for the thread: send `session/cancel`, so the session
/// reports a cancelled prompt instead of being killed mid-write.
pub fn cancel_turn(thread_id: &str) {
    let (stdin, session_id, flag) = match RUNNING.lock() {
        Ok(m) => match m.get(thread_id) {
            Some(rt) => (rt.stdin.clone(), rt.session_id.clone(), rt.cancelled.clone()),
            None => return,
        },
        Err(_) => return,
    };
    flag.store(true, Ordering::SeqCst);
    let cancel = json!({
        "jsonrpc": "2.0",
        "method": "session/cancel",
        "params": {"sessionId": session_id},
    });
    if let Ok(mut s) = stdin.lock() {
        let _ = s.write_all(cancel.to_string().as_bytes());
        let _ = s.write_all(b"\n");
        let _ = s.flush();
    }
}

/// Called after a `thread.turn-start-requested` event is committed for a
/// Devin thread. Spawns the runner thread that drives the ACP session.
pub fn start_turn(thread_id: String, message_id: String, text: String, cwd: String) {
    std::thread::spawn(move || {
        if let Err(e) = run_turn(&thread_id, &message_id, &text, &cwd) {
            tracing_error(&thread_id, &e);
        }
    });
}

fn tracing_error(thread_id: &str, e: &WsErr) {
    eprintln!("[acp] turn on {thread_id} failed: {}", e.0);
    if let Ok(conn) = open_state_rw() {
        let _ = set_session(&conn, thread_id, "error", Some(""), Some(&e.0), None);
        let _ = session_set_event(
            &conn,
            thread_id,
            &json!({
                "threadId": thread_id,
                "status": "error",
                "lastError": e.0,
                "updatedAt": iso_now_ms(),
            }),
        );
    }
}

struct Rpc {
    stdin: Arc<Mutex<ChildStdin>>,
    reader: BufReader<std::process::ChildStdout>,
    next_id: i64,
    pending: Vec<Value>,
}

impl Rpc {
    fn write_raw(&self, msg: &Value) -> Result<(), WsErr> {
        let mut s = self.stdin.lock().map_err(|_| err("acp stdin lock"))?;
        s.write_all(msg.to_string().as_bytes())
            .and_then(|_| s.write_all(b"\n"))
            .and_then(|_| s.flush())
            .map_err(|e| err(format!("acp write: {e}")))
    }

    fn read_msg(&mut self) -> Result<Value, WsErr> {
        loop {
            let mut line = String::new();
            let n = self
                .reader
                .read_line(&mut line)
                .map_err(|e| err(format!("acp read: {e}")))?;
            if n == 0 {
                return Err(err("devin acp closed stdout"));
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
                        "acp {method} error: {}",
                        e.get("message").and_then(Value::as_str).unwrap_or("?")
                    )));
                }
                return Ok(v.get("result").cloned().unwrap_or(Value::Null));
            }
            self.pending.push(v);
        }
    }

    /// Read messages until the response to the call arrives, streaming
    /// notifications to `on_notify`. Returns the result value.
    fn call_streaming<F>(
        &mut self,
        method: &str,
        params: Value,
        cancelled: &Arc<AtomicBool>,
        mut on_notify: F,
    ) -> Result<Value, WsErr>
    where
        F: FnMut(&Value),
    {
        let id = self.next_id;
        self.next_id += 1;
        self.write_raw(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        // `pending` holds notifications received while an earlier call waited
        // for its response — e.g. the transcript replay `session/load` emits.
        // Answer any queued server→client requests but drop the notifications:
        // replayed chunks must not leak into the new turn's message.
        for v in std::mem::take(&mut self.pending) {
            if v.get("method").is_some() && v.get("id").is_some() {
                self.dispatch_incoming(&v, &mut on_notify);
            }
        }
        loop {
            let v = self.read_msg()?;
            if v.get("id").and_then(Value::as_i64) == Some(id)
                && (v.get("result").is_some() || v.get("error").is_some())
            {
                if let Some(e) = v.get("error") {
                    if cancelled.load(Ordering::SeqCst) {
                        return Ok(json!({"stopReason": "cancelled"}));
                    }
                    return Err(err(format!(
                        "acp {method} error: {}",
                        e.get("message").and_then(Value::as_str).unwrap_or("?")
                    )));
                }
                return Ok(v.get("result").cloned().unwrap_or(Value::Null));
            }
            self.dispatch_incoming(&v, &mut on_notify);
        }
    }

    /// Server→client requests get a reply; notifications go to the streamer.
    fn dispatch_incoming<F>(&mut self, v: &Value, on_notify: &mut F)
    where
        F: FnMut(&Value),
    {
        let method = v.get("method").and_then(Value::as_str).unwrap_or("");
        match method {
            "session/request_permission" => {
                // Auto-approve: pick the first "allow" option so unattended
                // runs proceed (upstream gates these behind approval cards).
                let options = v
                    .pointer("/params/options")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let pick = options
                    .iter()
                    .find(|o| {
                        o.get("kind")
                            .and_then(Value::as_str)
                            .map(|k| k.contains("allow_always"))
                            .unwrap_or(false)
                    })
                    .or_else(|| {
                        options.iter().find(|o| {
                            o.get("kind")
                                .and_then(Value::as_str)
                                .map(|k| k.contains("allow_once"))
                                .unwrap_or(false)
                        })
                    })
                    .or_else(|| options.first())
                    .cloned()
                    .unwrap_or(Value::Null);
                let option_id = pick.get("optionId").cloned().unwrap_or(Value::Null);
                if let Some(rid) = v.get("id").cloned() {
                    let _ = self.write_raw(&json!({
                        "jsonrpc": "2.0",
                        "id": rid,
                        "result": {"outcome": {"outcome": "selected", "optionId": option_id}},
                    }));
                }
            }
            "fs/read_text_file" | "fs/write_text_file" | "terminal/create"
            | "terminal/output" | "terminal/wait_for_exit" | "terminal/kill"
            | "terminal/release" => {
                if let Some(rid) = v.get("id").cloned() {
                    let _ = self.write_raw(&json!({
                        "jsonrpc": "2.0",
                        "id": rid,
                        "error": {"code": -32601, "message": "not supported by synara host"},
                    }));
                }
            }
            _ => on_notify(v),
        }
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
         (thread_id, turn_id, pending_message_id, assistant_message_id, state, \
          requested_at, started_at, checkpoint_files_json) \
         VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?5, '[]')",
        params![thread_id, turn_id, message_id, assistant_msg_id, now],
    )
    .map_err(|e| err(format!("insert turn: {e}")))?;
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_sessions \
         (thread_id, status, provider_name, provider_session_id, provider_thread_id, \
          active_turn_id, last_error, updated_at, runtime_mode) \
         VALUES (?1, 'running', 'devin', NULL, ?2, ?3, NULL, ?4, \
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
            "providerName": "devin",
            "activeTurnId": turn_id,
            "lastError": null,
            "updatedAt": now,
        }),
    )?;

    let spawn_result = Command::new("devin")
        .arg("acp")
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let mut child: Child = match spawn_result {
        Ok(c) => c,
        Err(e) => {
            let _ = set_turn_state(&conn, thread_id, &turn_id, "error", true);
            let msg = format!("could not launch `devin acp`: {e}");
            let _ = set_session(&conn, thread_id, "error", Some(""), Some(&msg), None);
            return Err(err(msg));
        }
    };
    let stdin = child.stdin.take().ok_or_else(|| err("acp stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| err("acp stdout"))?;
    let stdin = Arc::new(Mutex::new(stdin));
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut rpc = Rpc {
        stdin: stdin.clone(),
        reader: BufReader::new(stdout),
        next_id: 1,
        pending: Vec::new(),
    };

    let result = (|| -> Result<String, WsErr> {
        rpc.call(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientCapabilities": {
                    "fs": {"readTextFile": false, "writeTextFile": false},
                    "terminal": false,
                },
                "clientInfo": {"name": "synara", "version": "1.0.0"},
            }),
        )?;

        let session_id = if let Some(sid) = stored_session.as_deref() {
            match rpc.call(
                "session/load",
                json!({"sessionId": sid, "cwd": cwd, "mcpServers": []}),
            ) {
                Ok(_) => sid.to_string(),
                Err(_) => rpc
                    .call("session/new", json!({"cwd": cwd, "mcpServers": []}))?
                    .get("sessionId")
                    .and_then(Value::as_str)
                    .ok_or_else(|| err("session/new returned no sessionId"))?
                    .to_string(),
            }
        } else {
            rpc.call("session/new", json!({"cwd": cwd, "mcpServers": []}))?
                .get("sessionId")
                .and_then(Value::as_str)
                .ok_or_else(|| err("session/new returned no sessionId"))?
                .to_string()
        };

        {
            let mut map = RUNNING.lock().map_err(|_| err("running map"))?;
            map.insert(
                thread_id.to_string(),
                RunningTurn {
                    stdin: stdin.clone(),
                    session_id: session_id.clone(),
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
            Some(&session_id),
        )?;

        let mut text_buf = String::new();
        let mut last_flush = std::time::Instant::now();
        let tid = thread_id.to_string();
        let amsg = assistant_msg_id.clone();
        let t_id = turn_id.clone();
        let result = rpc.call_streaming(
            "session/prompt",
            json!({
                "sessionId": session_id,
                "prompt": [{"type": "text", "text": text}],
            }),
            &cancelled,
            |v| {
                if v.get("method").and_then(Value::as_str) != Some("session/update") {
                    return;
                }
                let update = v
                    .pointer("/params/update")
                    .cloned()
                    .unwrap_or(Value::Null);
                let kind = update
                    .get("sessionUpdate")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match kind {
                    "agent_message_chunk" => {
                        if let Some(t) =
                            update.pointer("/content/text").and_then(Value::as_str)
                        {
                            text_buf.push_str(t);
                        }
                    }
                    "tool_call" => {
                        let title = update
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or("Tool call");
                        let kind_name = update
                            .get("kind")
                            .and_then(Value::as_str)
                            .unwrap_or("tool");
                        if let Ok(c) = open_state_rw() {
                            if let Err(e) = append_activity(&c, &tid, &t_id, "tool", kind_name, title, &update) { eprintln!("acp_runner: activity failed: {}", e.0); }
                        }
                    }
                    "tool_call_update" => {
                        let status = update
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if status == "failed" {
                            let title = update
                                .get("title")
                                .and_then(Value::as_str)
                                .unwrap_or("Tool call failed");
                            if let Ok(c) = open_state_rw() {
                                if let Err(e) = append_activity(&c, &tid, &t_id, "error", "tool", title, &update) { eprintln!("acp_runner: error activity failed: {}", e.0); }
                            }
                        }
                    }
                    _ => {}
                }
                if !text_buf.is_empty() && last_flush.elapsed().as_millis() >= 200 {
                    if let Ok(c) = open_state_rw() {
                        if let Err(e) = append_assistant_text(&c, &tid, &amsg, &t_id, &text_buf, true) { eprintln!("acp_runner: assistant flush failed: {}", e.0); }
                    }
                    text_buf.clear();
                    last_flush = std::time::Instant::now();
                }
            },
        )?;
        if let Ok(c) = open_state_rw() {
            if !text_buf.is_empty() {
                if let Err(e) = append_assistant_text(&c, &tid, &amsg, &t_id, &text_buf, true) { eprintln!("acp_runner: assistant flush failed: {}", e.0); }
            }
            if let Err(e) = append_assistant_text(&c, &tid, &amsg, &t_id, "", false) { eprintln!("acp_runner: final settle failed: {}", e.0); }
        }
        Ok(result
            .get("stopReason")
            .and_then(Value::as_str)
            .unwrap_or("end_turn")
            .to_string())
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
                    "providerName": "devin",
                    "activeTurnId": null,
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
                    "providerName": "devin",
                    "activeTurnId": null,
                    "lastError": e.0,
                    "updatedAt": iso_now_ms(),
                }),
            )?;
            Err(e)
        }
    }
}
