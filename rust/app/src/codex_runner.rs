//! Codex provider runtime: drives the official `codex app-server` over
//! newline-delimited JSON-RPC on stdio. Authentication never passes through
//! Synara: the official binary reads its own credentials, so OAuth tokens
//! are never extracted or replayed outside it (Anthropic/OpenAI-style
//! account bans hit ADEs that piggyback on CLI credentials).
//!
//! v1 vertical slice: initialize handshake, thread start/resume, turn start,
//! streaming agentMessage deltas into the projection, terminal
//! turn/completed|aborted|error handling. Server->client approval requests
//! are routed through pending approval interactions outside full-access,
//! while long-lived process parking and collaboration modes remain later slices.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use rusqlite::params;
use serde_json::{json, Value};

use crate::acp_runner::{
    append_activity, append_assistant_text, err, session_set_event, set_session, set_turn_state,
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

/// A live Codex app-server parked after a clean turn. Keeping the process
/// preserves authentication, thread state and MCP server startup cost; it is
/// reused only for the same provider thread and launch-fixed settings.
struct ParkedCodex {
    child: Child,
    rpc: Rpc,
    provider_thread_id: String,
    launch_key: String,
}

static PARKED: LazyLock<Mutex<HashMap<String, ParkedCodex>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn drop_parked(mut parked: ParkedCodex) {
    let _ = parked.child.kill();
    let _ = parked.child.wait();
}

fn take_parked(thread_id: &str, launch_key: &str) -> Option<ParkedCodex> {
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

pub fn has_running_turn(thread_id: &str) -> bool {
    RUNNING
        .lock()
        .ok()
        .map(|m| m.contains_key(thread_id))
        .unwrap_or(false)
}

/// Inject user text into the live Codex turn. The app-server consumes the
/// input at the model's next turn boundary; its response frame is read and
/// ignored by the runner's notification loop, so this is intentionally a
/// fire-and-forget write like upstream's steer delivery surface.
pub fn drop_thread_resources(thread_id: &str) {
    if let Ok(mut parked_map) = PARKED.lock() {
        if let Some(parked) = parked_map.remove(thread_id) {
            drop_parked(parked);
        }
    }
}

pub fn steer_turn(thread_id: &str, text: &str) -> bool {
    let (stdin, thread_ref, turn_ref) = match RUNNING.lock() {
        Ok(m) => match m.get(thread_id) {
            Some(rt) if !rt.turn_ref.is_empty() => (
                rt.stdin.clone(),
                rt.thread_ref.clone(),
                rt.turn_ref.clone(),
            ),
            _ => return false,
        },
        Err(_) => return false,
    };
    let steer = json!({
        "jsonrpc": "2.0",
        "id": -2,
        "method": "turn/steer",
        "params": {
            "threadId": thread_ref,
            "input": [{"type": "text", "text": text, "text_elements": []}],
            "expectedTurnId": turn_ref,
        },
    });
    if let Ok(mut s) = stdin.lock() {
        return s
            .write_all(steer.to_string().as_bytes())
            .and_then(|_| s.write_all(b"\n"))
            .and_then(|_| s.flush())
            .is_ok();
    }
    false
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


fn parse_codex_user_input_questions(params: &Value) -> Vec<Value> {
    params
        .get("questions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|question| {
            let id = question.get("id").and_then(Value::as_str)?.trim().to_string();
            let header = question.get("header").and_then(Value::as_str)?.trim().to_string();
            let prompt = question.get("question").and_then(Value::as_str)?.trim().to_string();
            if id.is_empty() || header.is_empty() || prompt.is_empty() {
                return None;
            }
            let options = question
                .get("options")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .iter()
                .filter_map(|option| {
                    let label = option.get("label").and_then(Value::as_str)?.trim().to_string();
                    if label.is_empty() {
                        return None;
                    }
                    let description = option
                        .get("description")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or(&label)
                        .to_string();
                    Some(json!({"label": label, "description": description}))
                })
                .collect::<Vec<_>>();
            Some(json!({
                "id": id,
                "header": header,
                "question": prompt,
                "options": options,
                "multiSelect": question.get("multiSelect").and_then(Value::as_bool).unwrap_or(false),
            }))
        })
        .collect()
}

fn codex_user_input_answer(value: &Value) -> Value {
    if let Some(text) = value.as_str() {
        return json!({"answers": [text]});
    }
    if let Some(items) = value.as_array() {
        return json!({"answers": items.iter().filter_map(Value::as_str).collect::<Vec<_>>()});
    }
    if let Some(items) = value.get("answers").and_then(Value::as_array) {
        return json!({"answers": items.iter().filter_map(Value::as_str).collect::<Vec<_>>()});
    }
    json!({"answers": []})
}

fn codex_user_input_answers(answers: &Value) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(object) = answers.as_object() {
        for (question_id, value) in object {
            out.insert(question_id.clone(), codex_user_input_answer(value));
        }
    }
    Value::Object(out)
}

struct Rpc {
    stdin: Arc<Mutex<std::process::ChildStdin>>,
    reader: BufReader<std::process::ChildStdout>,
    next_id: i64,
    approval_ctx: Option<CodexApprovalCtx>,
}

#[derive(Clone)]
struct CodexApprovalCtx {
    thread_id: String,
    turn_id: String,
    runtime_mode: String,
    cancelled: Arc<AtomicBool>,
}

fn codex_request_kind(method: &str) -> Option<&'static str> {
    match method {
        "item/commandExecution/requestApproval" => Some("command"),
        "item/fileRead/requestApproval" => Some("file-read"),
        "item/fileChange/requestApproval" => Some("file-change"),
        "item/permissions/requestApproval" => Some("permissions"),
        _ => None,
    }
}

fn is_mcp_tool_approval(method: &str, params: &Value) -> bool {
    method == "mcpServer/elicitation/request"
        && params
            .pointer("/_meta/codex_approval_kind")
            .and_then(Value::as_str)
            == Some("mcp_tool_call")
}

fn without_null_values(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), without_null_values(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(without_null_values).collect()),
        _ => value.clone(),
    }
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

    /// Server→client requests. Native approval methods become pending
    /// interactions and block until `thread.approval.respond` writes the
    /// user's decision, matching the upstream approval-card contract. In
    /// full-access mode they are auto-accepted because the runtime override
    /// already expresses the user's blanket grant. Unknown requests receive
    /// a protocol error instead of a silent approval.
    fn answer_server_request(&mut self, v: &Value) {
        let Some(method) = v.get("method").and_then(Value::as_str) else {
            return;
        };
        if v.get("id").is_none() || v.get("id").and_then(Value::as_i64) == Some(-1) {
            return;
        }
        let params = v.get("params").cloned().unwrap_or(Value::Null);
        if method == "item/tool/requestUserInput" {
            if let Some(ctx) = self.approval_ctx.clone() {
                self.handle_user_input_request(v, &params, &ctx);
            } else if let Some(rid) = v.get("id").cloned() {
                let _ = self.write_raw(&json!({"jsonrpc":"2.0","id":rid,"error":{"code":-32603,"message":"user input is unavailable before turn start"}}));
            }
            return;
        }
        let mcp_approval = is_mcp_tool_approval(method, &params);
        let request_kind = if mcp_approval {
            Some("tool")
        } else {
            codex_request_kind(method)
        };
        let Some(request_kind) = request_kind else {
            if let Some(rid) = v.get("id").cloned() {
                let (result, error) = if method == "mcpServer/elicitation/request" {
                    (Some(json!({"action":"cancel","content":null,"_meta":null})), None)
                } else {
                    (None, Some(json!({"code":-32601,"message":format!("unsupported server request: {method}")})))
                };
                let mut response = json!({"jsonrpc":"2.0","id":rid});
                if let Some(result) = result { response["result"] = result; }
                if let Some(error) = error { response["error"] = error; }
                let _ = self.write_raw(&response);
            }
            return;
        };
        let Some(ctx) = self.approval_ctx.clone() else {
            self.auto_accept(v, method, request_kind);
            return;
        };
        if ctx.runtime_mode == "full-access" {
            self.auto_accept(v, method, request_kind);
        } else {
            self.handle_interactive_approval(v, method, request_kind, &ctx);
        }
    }

    fn auto_accept(&mut self, v: &Value, method: &str, _request_kind: &str) {
        if let Some(rid) = v.get("id").cloned() {
            let params = v.get("params").cloned().unwrap_or(Value::Null);
            let result = if is_mcp_tool_approval(method, &params) {
                json!({"action":"accept","content":null,"_meta":null})
            } else if method == "item/permissions/requestApproval" {
                json!({"permissions": without_null_values(params.get("permissions").unwrap_or(&Value::Null)), "scope":"turn"})
            } else {
                json!({"decision":"accept"})
            };
            let _ = self.write_raw(&json!({"jsonrpc":"2.0","id":rid,"result":result}));
        }
    }

    fn handle_user_input_request(&mut self, v: &Value, params: &Value, ctx: &CodexApprovalCtx) {
        let questions = parse_codex_user_input_questions(params);
        let Some(rid) = v.get("id").cloned() else { return };
        if questions.is_empty() {
            let _ = self.write_raw(&json!({
                "jsonrpc":"2.0",
                "id":rid,
                "error":{"code":-32602,"message":"item/tool/requestUserInput did not include a renderable question."},
            }));
            return;
        }
        let request_id = uuid_v4();
        let now = iso_now_ms();
        if let Ok(c) = open_state_rw() {
            let _ = c.execute_batch("PRAGMA busy_timeout = 5000;");
            let _ = c.execute(
                "INSERT OR REPLACE INTO projection_pending_interactions \
                 (interaction_kind, request_id, thread_id, turn_id, lifecycle_generation, status, decision, response_command_id, response_requested_at, created_at, resolved_at) \
                 VALUES ('userInput', ?1, ?2, ?3, NULL, 'pending', NULL, NULL, NULL, ?4, NULL)",
                params![request_id, ctx.thread_id, ctx.turn_id, now],
            );
            let _ = append_activity(
                &c,
                &ctx.thread_id,
                &ctx.turn_id,
                "info",
                "user-input.requested",
                "User input requested",
                &json!({
                    "requestId": request_id,
                    "questions": questions,
                    "providerMethod": "item/tool/requestUserInput",
                    "providerPayload": params,
                }),
            );
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
        let mut answers_json: Option<String> = None;
        while answers_json.is_none() {
            if ctx.cancelled.load(Ordering::SeqCst) || std::time::Instant::now() >= deadline {
                answers_json = Some("{}".to_string());
                break;
            }
            if let Ok(c) = open_state_rw() {
                match c.query_row(
                    "SELECT decision FROM projection_pending_interactions \
                     WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='userInput'",
                    params![ctx.thread_id, request_id],
                    |r| r.get::<_, Option<String>>(0),
                ) {
                    Ok(Some(value)) => answers_json = Some(value),
                    Ok(None) => {}
                    Err(_) => answers_json = Some("{}".to_string()),
                }
            }
            if answers_json.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        }
        let answers = answers_json
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .unwrap_or(json!({}));
        let codex_answers = codex_user_input_answers(&answers);
        if let Ok(c) = open_state_rw() {
            let _ = c.execute(
                "DELETE FROM projection_pending_interactions WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='userInput'",
                params![ctx.thread_id, request_id],
            );
            let _ = append_activity(
                &c,
                &ctx.thread_id,
                &ctx.turn_id,
                "info",
                "user-input.resolved",
                "User input resolved",
                &json!({"requestId": request_id, "answers": codex_answers}),
            );
        }
        let _ = self.write_raw(&json!({
            "jsonrpc":"2.0",
            "id":rid,
            "result":{"answers": codex_answers},
        }));
    }

    fn handle_interactive_approval(
        &mut self,
        v: &Value,
        method: &str,
        request_kind: &str,
        ctx: &CodexApprovalCtx,
    ) {
        let params = v.get("params").cloned().unwrap_or(Value::Null);
        let request_id = uuid_v4();
        let title = if is_mcp_tool_approval(method, &params) {
            params
                .pointer("/_meta/tool_name")
                .and_then(Value::as_str)
                .or_else(|| {
                    params
                        .get("message")
                        .and_then(Value::as_str)
                        .and_then(|message| {
                            message
                                .strip_prefix("Allow the synara MCP server to run tool \"")?
                                .strip_suffix("\"?")
                        })
                })
                .map(|name| format!("mcp__synara__{name}"))
                .unwrap_or_else(|| "MCP tool call".to_string())
        } else {
            params
                .get("command")
                .or_else(|| params.get("path"))
                .or_else(|| params.get("reason"))
                .and_then(Value::as_str)
                .unwrap_or("Codex approval")
                .to_string()
        };
        let now = iso_now_ms();
        if let Ok(c) = open_state_rw() {
            let _ = c.execute_batch("PRAGMA busy_timeout = 5000;");
            if let Err(e) = c.execute(
                "INSERT OR REPLACE INTO projection_pending_interactions                  (interaction_kind, request_id, thread_id, turn_id,                   lifecycle_generation, status, decision,                   response_command_id, response_requested_at,                   created_at, resolved_at)                  VALUES ('approval', ?1, ?2, ?3, NULL, 'pending', NULL,                   NULL, NULL, ?4, NULL)",
                params![request_id, ctx.thread_id, ctx.turn_id, now],
            ) {
                eprintln!("codex_runner: pending approval insert: {e}");
            }
            let payload = json!({
                "requestId": request_id,
                "requestKind": request_kind,
                "toolName": title,
                "detail": title,
                "providerMethod": method,
                "providerPayload": params,
                "sessionApprovalAvailable": true,
            });
            let _ = append_activity(
                &c,
                &ctx.thread_id,
                &ctx.turn_id,
                "warn",
                "approval.requested",
                "Approval requested",
                &payload,
            );
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
        let mut decision: Option<String> = None;
        while decision.is_none() {
            if ctx.cancelled.load(Ordering::SeqCst) || std::time::Instant::now() >= deadline {
                decision = Some("cancel".to_string());
                break;
            }
            if let Ok(c) = open_state_rw() {
                match c.query_row(
                    "SELECT decision FROM projection_pending_interactions                      WHERE thread_id=?1 AND request_id=?2                      AND interaction_kind='approval'",
                    params![ctx.thread_id, request_id],
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
                "DELETE FROM projection_pending_interactions                  WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='approval'",
                params![ctx.thread_id, request_id],
            );
            let _ = append_activity(
                &c,
                &ctx.thread_id,
                &ctx.turn_id,
                "info",
                "approval.resolved",
                "Approval resolved",
                &json!({"requestId": request_id, "decision": decision}),
            );
        }

        if let Some(rid) = v.get("id").cloned() {
            let result = if is_mcp_tool_approval(method, &params) {
                let action = match decision.as_str() {
                    "accept" | "acceptForSession" => "accept",
                    "decline" => "decline",
                    _ => "cancel",
                };
                let persist_session = decision == "acceptForSession"
                    && match params.pointer("/_meta/persist") {
                        Some(Value::Array(items)) => items.iter().any(|x| x.as_str() == Some("session")),
                        Some(Value::String(s)) => s == "session",
                        _ => false,
                    };
                json!({
                    "action": action,
                    "content": null,
                    "_meta": if persist_session { json!({"persist":"session"}) } else { Value::Null },
                })
            } else if method == "item/permissions/requestApproval" {
                let granted = if matches!(decision.as_str(), "accept" | "acceptForSession") {
                    without_null_values(params.get("permissions").unwrap_or(&Value::Null))
                } else {
                    json!({})
                };
                json!({
                    "permissions": granted,
                    "scope": if decision == "acceptForSession" { "session" } else { "turn" },
                })
            } else {
                json!({"decision": decision})
            };
            let _ = self.write_raw(&json!({"jsonrpc":"2.0","id":rid,"result":result}));
        }
        if decision == "cancel" && matches!(request_kind, "permissions" | "tool") {
            cancel_turn(&ctx.thread_id);
        }
    }
}


fn write_json_line(stdin: &mut ChildStdin, value: &Value) -> Result<(), WsErr> {
    stdin
        .write_all(value.to_string().as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|e| err(format!("codex discovery write: {e}")))
}

fn reasoning_efforts(value: Option<&Value>) -> Vec<Value> {
    value
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            if let Some(value) = entry.as_str() {
                let value = value.trim();
                return (!value.is_empty()).then(|| json!({"value": value}));
            }
            let effort = entry
                .get("reasoningEffort")
                .or_else(|| entry.get("reasoning_effort"))
                .or_else(|| entry.get("value"))
                .and_then(Value::as_str)?
                .trim()
                .to_string();
            if effort.is_empty() {
                return None;
            }
            let mut object = json!({"value": effort});
            if let Some(description) = entry
                .get("description")
                .or_else(|| entry.get("label"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
            {
                object["description"] = json!(description);
            }
            Some(object)
        })
        .collect()
}

pub fn parse_codex_models_response(response: &Value) -> Vec<Value> {
    let raw_models = response
        .get("items")
        .or_else(|| response.get("data"))
        .or_else(|| response.get("models"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    let mut models = Vec::new();
    for model in raw_models {
        let slug = model
            .get("id")
            .or_else(|| model.get("slug"))
            .or_else(|| model.get("model"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("")
            .to_string();
        if slug.is_empty() || !seen.insert(slug.clone()) {
            continue;
        }
        let name = model
            .get("name")
            .or_else(|| model.get("displayName"))
            .or_else(|| model.get("display_name"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or(&slug)
            .to_string();
        let efforts = reasoning_efforts(
            model
                .get("supportedReasoningEfforts")
                .or_else(|| model.get("supported_reasoning_efforts")),
        );
        let mut out = json!({
            "slug": slug,
            "name": name,
            "supportedReasoningEfforts": efforts,
        });
        if let Some(default_effort) = model
            .get("defaultReasoningEffort")
            .or_else(|| model.get("default_reasoning_effort"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            out["defaultReasoningEffort"] = json!(default_effort);
        }
        if let Some(description) = model
            .get("description")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            out["description"] = json!(description);
        }
        models.push(out);
    }
    models
}

/// Discover the account-aware Codex catalog through the official app-server.
/// The process is temporary and is killed after `model/list`; no credential
/// file is opened or copied by Synara.
pub fn discover_models(cwd: &str) -> Result<Vec<Value>, WsErr> {
    let mut child = Command::new(codex_binary())
        .arg("app-server")
        .current_dir(if cwd.is_empty() { "/" } else { cwd })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| err(format!("could not launch `codex app-server`: {e}")))?;
    let mut stdin = child.stdin.take().ok_or_else(|| err("codex stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| err("codex stdout"))?;
    write_json_line(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"initialize",
            "params":{
                "clientInfo":{"name":"synara_desktop","title":"Synara Desktop","version":"0.1.0"},
                "capabilities":{"experimentalApi":true},
            },
        }),
    )?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            let mut line = String::new();
            let Ok(n) = reader.read_line(&mut line) else { break };
            if n == 0 { break }
            if let Ok(value) = serde_json::from_str::<Value>(line.trim_end_matches(['\n','\r'])) {
                let _ = tx.send(value);
            }
        }
    });
    let deadline = std::time::Duration::from_secs(20);
    let started = std::time::Instant::now();
    let mut initialized = false;
    let mut models_response = None;
    let mut sent_model_list = false;
    while started.elapsed() < deadline && models_response.is_none() {
        match rx.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(value) => {
                if value.get("id").and_then(Value::as_i64) == Some(1) {
                    if value.get("error").is_some() {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(err(format!("codex initialize failed: {value}")));
                    }
                    initialized = true;
                    write_json_line(&mut stdin, &json!({"jsonrpc":"2.0","method":"initialized"}))?;
                    write_json_line(
                        &mut stdin,
                        &json!({"jsonrpc":"2.0","id":2,"method":"model/list","params":{}}),
                    )?;
                    sent_model_list = true;
                } else if initialized
                    && sent_model_list
                    && value.get("id").and_then(Value::as_i64) == Some(2)
                {
                    models_response = Some(value);
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    let response = models_response.ok_or_else(|| err("codex model/list timed out"))?;
    if let Some(error) = response.get("error") {
        return Err(err(format!("codex model/list failed: {error}")));
    }
    Ok(parse_codex_models_response(
        response.get("result").unwrap_or(&Value::Null),
    ))
}



/// Start Codex ahead of the first user prompt. The parked app-server already
/// owns a provider thread, so the first real turn only sends `turn/start`.
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
    let (raw_cwd, turn_runtime_mode, model_selection_json): (String, String, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(NULLIF(working_directory, ''), NULLIF(worktree_path, ''), NULLIF(associated_worktree_path, ''), (SELECT workspace_root FROM projection_projects WHERE project_id=projection_threads.project_id), ''), \
                    COALESCE((SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id=?1), 'full-access'), \
                    model_selection_json \
             FROM projection_threads WHERE thread_id=?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| err(format!("codex prewarm thread lookup: {e}")))?;
    let stored_thread: Option<String> = conn
        .query_row(
            "SELECT provider_thread_id FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let cwd = if !raw_cwd.is_empty() && std::path::Path::new(&raw_cwd).is_dir() {
        raw_cwd
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    };
    let selected_model = model_selection_json
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.get("model").and_then(Value::as_str).map(str::to_string));
    let launch_key = format!(
        "{}|{}|{}",
        turn_runtime_mode,
        selected_model.as_deref().unwrap_or(""),
        cwd
    );

    let mut command = Command::new(codex_binary());
    command
        .arg("app-server")
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|e| err(format!("could not launch `codex app-server`: {e}")))?;
    let child_stdin = child.stdin.take().ok_or_else(|| err("codex stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| err("codex stdout"))?;
    let stdin = Arc::new(Mutex::new(child_stdin));
    let mut rpc = Rpc {
        stdin,
        reader: BufReader::new(stdout),
        next_id: 1,
        approval_ctx: None,
    };
    let prepared = (|| -> Result<String, WsErr> {
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
        open_res
            .pointer("/thread/id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                open_res
                    .get("threadId")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .ok_or_else(|| err(format!("{open_method} returned no thread id")))
    })();
    match prepared {
        Ok(provider_thread_id) => {
            if let Ok(mut parked_map) = PARKED.lock() {
                parked_map.insert(
                    thread_id.to_string(),
                    ParkedCodex {
                        child,
                        rpc,
                        provider_thread_id,
                        launch_key,
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

/// Compact a warm Codex thread through the app-server's native control
/// request. Unlike Pi/OpenCode there is no process-local transcript copy here:
/// the official binary summarizes its own thread state and Synara only awaits
/// the acknowledgement.
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
    let provider_thread_id = parked.provider_thread_id.clone();
    let result = parked
        .rpc
        .call(
            "thread/compact/start",
            json!({"threadId": provider_thread_id}),
        )
        .map(|_| true);
    let alive = matches!(parked.child.try_wait(), Ok(None));
    if alive {
        if let Ok(mut parked_map) = PARKED.lock() {
            parked.rpc.approval_ctx = None;
            parked_map.insert(thread_id.to_string(), parked);
        }
    } else {
        drop_parked(parked);
    }
    result
}

fn codex_collaboration_instructions(plan: bool) -> String {
    if plan {
        format!(
            "<collaboration_mode># Plan Mode (Conversational)\n\nYou are in **Plan Mode** until developer instructions explicitly end it. Plan execution; do not perform it. You may inspect, search, test and otherwise gather facts only when those actions do not mutate repo-tracked state. Do not edit files, apply patches, run rewriting formatters/codegen, or execute plan-implementing side effects. Explore discoverable facts before asking. Use the request_user_input tool for decisions that materially change the plan, meaningful tradeoffs, or information that cannot be discovered. Ask only questions that cannot be answered locally. Continue until the specification is decision-complete. When final, emit exactly one Markdown plan wrapped in <proposed_plan> on its own line and </proposed_plan> on its own line. The plan must include title, summary, API/interface changes, tests, assumptions, and defaults. Do not ask whether to proceed.\n</collaboration_mode>{}\n\n{}",
            crate::harness_policy::browser_routing_instructions(),
            crate::harness_policy::harness_policy(),
        )
    } else {
        format!(
            "<collaboration_mode># Collaboration Mode: Default\n\nYou are now in Default mode. Previous collaboration-mode instructions are inactive. The request_user_input tool is unavailable in Default mode. Prefer reasonable assumptions and execution over stopping to ask questions; ask directly in plain text only when local context cannot answer a high-risk question.\n</collaboration_mode>{}\n\n{}",
            crate::harness_policy::browser_routing_instructions(),
            crate::harness_policy::harness_policy(),
        )
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
    let (model_selection_raw, interaction_mode): (Option<String>, String) = conn
        .query_row(
            "SELECT model_selection_json, interaction_mode FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get::<_, Option<String>>(1)?.unwrap_or_default())),
        )
        .unwrap_or((None, "default".to_string()));
    let model_selection: Value = model_selection_raw
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let selected_model: Option<String> = model_selection
        .get("model")
        .and_then(Value::as_str)
        .map(str::to_string);
    let selected_effort: Option<String> = model_selection
        .pointer("/options/reasoningEffort")
        .and_then(Value::as_str)
        .or_else(|| model_selection.pointer("/options/effort").and_then(Value::as_str))
        .map(str::to_string);

    let launch_key = format!(
        "{}|{}|{}",
        turn_runtime_mode,
        selected_model.as_deref().unwrap_or(""),
        cwd
    );
    let cancelled = Arc::new(AtomicBool::new(false));
    let approval_ctx = CodexApprovalCtx {
        thread_id: thread_id.to_string(),
        turn_id: turn_id.clone(),
        runtime_mode: turn_runtime_mode.clone(),
        cancelled: cancelled.clone(),
    };
    let (mut child, mut rpc, reused_thread) = if let Some(mut parked) =
        take_parked(thread_id, &launch_key)
    {
        parked.rpc.approval_ctx = Some(approval_ctx.clone());
        let provider_thread_id = parked.provider_thread_id.clone();
        (parked.child, parked.rpc, Some(provider_thread_id))
    } else {
        let mut command = Command::new(codex_binary());
        command
            .arg("app-server")
            .current_dir(&cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let child: Child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                let _ = set_turn_state(&conn, thread_id, &turn_id, "error", true);
                let msg = format!("could not launch `codex app-server`: {e}");
                let _ = set_session(&conn, thread_id, "error", Some(""), Some(&msg), None);
                return Err(err(msg));
            }
        };
        let mut child = child;
        let child_stdin = child.stdin.take().ok_or_else(|| err("codex stdin"))?;
        let stdout = child.stdout.take().ok_or_else(|| err("codex stdout"))?;
        let stdin = Arc::new(Mutex::new(child_stdin));
        let rpc = Rpc {
            stdin,
            reader: BufReader::new(stdout),
            next_id: 1,
            approval_ctx: Some(approval_ctx.clone()),
        };
        (child, rpc, None)
    };
    let stdin = rpc.stdin.clone();

    let mut active_provider_thread_id: Option<String> = None;
    let result = (|| -> Result<String, WsErr> {
        let provider_thread_id = if let Some(reused) = reused_thread.clone() {
            reused
        } else {
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
            open_res
                .pointer("/thread/id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    open_res
                        .get("threadId")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .ok_or_else(|| err(format!("{open_method} returned no thread id")))?
        };
        active_provider_thread_id = Some(provider_thread_id.clone());

        // Fresh sessions get the host policy + browser routing rules
        // prepended to the first prompt; resumed threads already carry it.
        let prompt_text = if stored_thread.is_none() {
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
        let mut input_items = Vec::new();
        if !prompt_text.is_empty() {
            input_items.push(json!({
                "type": "text",
                "text": prompt_text,
                "text_elements": [],
            }));
        }
        for attachment in &attachments {
            if attachment.get("type").and_then(Value::as_str) != Some("image") {
                continue;
            }
            let Some(path) = crate::provider_attachments::resolve_attachment_path(attachment) else {
                return Err(err("invalid attachment id"));
            };
            input_items.push(json!({
                "type": "localImage",
                "path": path.to_string_lossy(),
            }));
        }
        if input_items.is_empty() {
            return Err(err("turn input must include text or attachments"));
        }
        let mut turn_params = json!({
            "threadId": provider_thread_id,
            "input": input_items,
            "summary": "auto",
        });
        if let Some(model) = selected_model.as_deref() {
            turn_params["model"] = json!(model);
            if model == "gpt-5.3-codex-spark" {
                turn_params["summary"] = json!("none");
            }
        }
        if let Some(effort) = selected_effort.as_deref() {
            turn_params["effort"] = json!(effort);
        }
        let plan_mode = interaction_mode == "plan";
        let collaboration_model = selected_model
            .clone()
            .unwrap_or_else(|| "gpt-5.3-codex".to_string());
        turn_params["collaborationMode"] = json!({
            "mode": if plan_mode { "plan" } else { "default" },
            "settings": {
                "model": collaboration_model,
                "reasoning_effort": selected_effort.clone().unwrap_or_else(|| "medium".to_string()),
                "developer_instructions": codex_collaboration_instructions(plan_mode),
            },
        });
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
    let cancelled_flag = cancelled.load(Ordering::SeqCst);
    let parkable = matches!(result, Ok(ref reason) if reason == "completed")
        && !cancelled_flag
        && child.try_wait().ok().flatten().is_none();
    if parkable {
        if let (Some(provider_thread_id), Ok(mut parked_map)) =
            (active_provider_thread_id.clone(), PARKED.lock())
        {
            rpc.approval_ctx = None;
            parked_map.insert(
                thread_id.to_string(),
                ParkedCodex {
                    child,
                    rpc,
                    provider_thread_id,
                    launch_key,
                },
            );
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
    fn runtime_overrides_map_full_access() {
        let v = super::runtime_overrides("full-access");
        assert_eq!(v["approvalPolicy"], "never");
        assert_eq!(v["sandbox"], "danger-full-access");
        let g = super::runtime_overrides("approval-required");
        assert_eq!(g["approvalPolicy"], "on-request");
    }
}
