//! Claude provider runtime: drives the official `claude` CLI through its
//! bidirectional stream-json control protocol. Authentication never passes
//! through Synara: only the official binary reads OAuth credentials, so
//! tokens/cookies are never extracted or replayed outside it.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
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
    stdin: Arc<Mutex<ChildStdin>>,
    cancelled: Arc<AtomicBool>,
}

static RUNNING: LazyLock<Mutex<HashMap<String, RunningTurn>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// A live Claude CLI parked after a clean result. The process owns the SDK
/// session loop, hooks, queued input and control protocol; reusing it avoids
/// the one-shot `--resume` restart cost and preserves native mid-turn input.
struct ParkedClaude {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    reader: BufReader<ChildStdout>,
    session_id: Option<String>,
    launch_key: String,
}

static PARKED: LazyLock<Mutex<HashMap<String, ParkedClaude>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn drop_parked(mut parked: ParkedClaude) {
    let _ = parked.child.kill();
    let _ = parked.child.wait();
}

fn take_parked(thread_id: &str, launch_key: &str) -> Option<ParkedClaude> {
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

fn write_line(stdin: &Arc<Mutex<ChildStdin>>, value: &Value) -> Result<(), WsErr> {
    let mut stdin = stdin.lock().map_err(|_| err("claude stdin lock"))?;
    stdin
        .write_all(value.to_string().as_bytes())
        .and_then(|_| stdin.write_all(b"\n"))
        .and_then(|_| stdin.flush())
        .map_err(|e| err(format!("claude write: {e}")))
}

/// Port of upstream isProviderEdgeBlockedText: Cloudflare can refuse a
/// healthy request with an HTML 403 page; the CLI surfaces only its text.
/// The login itself is fine, so the turn retries instead of failing.
fn is_edge_blocked_text(text: &str) -> bool {
    let lower = text.to_lowercase();
    text.contains("403")
        && (lower.contains("cloudflare")
            || lower.contains("attention required")
            || lower.contains("just a moment"))
}

/// Upstream CLAUDE_EDGE_BLOCK_RETRY_DELAYS_MS = [5s, 15s, 45s]; env override
/// keeps integration tests fast.
fn edge_block_retry_delays_ms() -> Vec<u64> {
    if let Ok(raw) = std::env::var("SYNARA_CLAUDE_EDGE_RETRY_DELAYS_MS") {
        let parsed: Vec<u64> = raw
            .split(',')
            .filter_map(|part| part.trim().parse().ok())
            .collect();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    vec![5_000, 15_000, 45_000]
}

fn user_message(content: Vec<Value>) -> Value {
    json!({
        "type": "user",
        "session_id": "",
        "message": {"role": "user", "content": content},
        "parent_tool_use_id": Value::Null,
    })
}

fn claude_content_blocks(text: &str, attachments: &[Value]) -> Result<Vec<Value>, WsErr> {
    let mut content = Vec::new();
    if !text.is_empty() {
        content.push(json!({"type": "text", "text": text}));
    }
    let supported = ["image/gif", "image/jpeg", "image/png", "image/webp"];
    let mut projected = Vec::new();
    for attachment in attachments {
        if attachment.get("type").and_then(Value::as_str) != Some("image") {
            projected.push(attachment.clone());
            continue;
        }
        let mime = attachment
            .get("mimeType")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !supported.contains(&mime.as_str()) {
            projected.push(attachment.clone());
            continue;
        }
        let Some(path) = crate::provider_attachments::resolve_attachment_path(attachment) else {
            return Err(err("invalid attachment id"));
        };
        let bytes = std::fs::read(&path)
            .map_err(|e| err(format!("failed to read attachment file: {e}")))?;
        content.push(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": mime,
                "data": crate::provider_attachments::base64_encode(&bytes),
            }
        }));
    }
    let file_block = crate::provider_attachments::append_file_attachments_prompt_block(
        "",
        &projected,
        true,
    );
    if !file_block.is_empty() {
        content.push(json!({"type": "text", "text": file_block}));
    }
    Ok(content)
}

/// Native steering: push the user message into the live streaming-input
/// session. Claude consumes it inside the same agent loop instead of needing
/// an interrupt plus a fresh process.
pub fn steer_turn(thread_id: &str, text: &str) -> bool {
    let stdin = match RUNNING.lock() {
        Ok(m) => match m.get(thread_id) {
            Some(rt) => rt.stdin.clone(),
            None => return false,
        },
        Err(_) => return false,
    };
    write_line(&stdin, &user_message(vec![json!({"type":"text","text":text})])).is_ok()
}

/// Interrupt through the CLI control protocol. The response is consumed by
/// the runner's stdout loop; cancellation is therefore best-effort and does
/// not require killing a stateful process.
pub fn cancel_turn(thread_id: &str) {
    let (stdin, flag) = match RUNNING.lock() {
        Ok(m) => match m.get(thread_id) {
            Some(rt) => (rt.stdin.clone(), rt.cancelled.clone()),
            None => return,
        },
        Err(_) => return,
    };
    flag.store(true, Ordering::SeqCst);
    let request = json!({
        "request_id": format!("interrupt-{}", uuid_v4()),
        "type": "control_request",
        "request": {"subtype": "interrupt"},
    });
    let _ = write_line(&stdin, &request);
}

/// Credential keepalive, ported from upstream `claudeCredentialKeepalive.ts`.
/// Long-lived parked sessions ride the macOS Keychain OAuth token, which
/// lapses after ~8h; the file-based auth check never observes it there. The
/// refresh is delegated to the official CLI itself: `claude auth status`
/// validates and rotates its own Keychain token through its own ACL, so
/// Synara never reads, copies, or replays any credential. Opt-in via
/// SYNARA_CLAUDE_KEEPALIVE=1; interval SYNARA_CLAUDE_KEEPALIVE_MINUTES
/// (default 30). Simplification vs upstream: the Rust port does not yet
/// derive multi-account provider instances, so the default config dir is
/// kept fresh; per-account env arrives with multi-instance support.
pub(crate) fn spawn_credential_keepalive() {
    let enabled = std::env::var("SYNARA_CLAUDE_KEEPALIVE")
        .map(|v| matches!(v.trim().to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(false);
    if !enabled {
        return;
    }
    let minutes: u64 = std::env::var("SYNARA_CLAUDE_KEEPALIVE_MINUTES")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|m: &u64| *m > 0)
        .unwrap_or(30);
    std::thread::spawn(move || loop {
        // Run first, then wait: a boot-time refresh costs nothing and covers
        // sessions parked across the 8h boundary right after startup.
        let binary = claude_binary();
        let mut command = Command::new(&binary);
        command
            .args(["auth", "status"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        crate::provider_env::sanitize_claude_child(&mut command);
        let child = command.spawn();
        if let Ok(mut child) = child {
            // Bound the wait at the upstream 20s command timeout; a hung CLI
            // must never wedge the keepalive thread.
            let (tx, rx) = std::sync::mpsc::channel();
            let pid = child.id();
            std::thread::spawn(move || {
                let _ = child.wait();
                let _ = tx.send(());
            });
            if rx.recv_timeout(std::time::Duration::from_secs(20)).is_err() {
                unsafe extern "C" {
                    fn kill(pid: i32, sig: i32) -> i32;
                }
                unsafe {
                    kill(pid as i32, 9);
                }
                eprintln!("claude keepalive: auth status timed out, killed");
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(minutes * 60));
    });
    eprintln!("claude credential keepalive: enabled every {minutes} minute(s)");
}

pub(crate) fn claude_binary() -> String {
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

fn permission_kind(tool_name: &str) -> &'static str {
    match tool_name {
        "Bash" => "command",
        "Read" | "Glob" | "Grep" => "file-read",
        "Edit" | "Write" | "NotebookEdit" => "file-change",
        _ => "tool",
    }
}


fn normalize_ask_questions(input: &Value) -> Vec<Value> {
    input
        .get("questions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
        .map(|(index, question)| {
            let header = question
                .get("header")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("Question {}", index + 1));
            let id = question
                .get("header")
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("q-{index}"));
            json!({
                "id": id,
                "header": header,
                "question": question.get("question").and_then(Value::as_str).unwrap_or(""),
                "options": question.get("options").cloned().unwrap_or(json!([])),
                "multiSelect": question.get("multiSelect").and_then(Value::as_bool).unwrap_or(false),
            })
        })
        .collect()
}

fn claude_answer_string(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_string();
    }
    value
        .as_array()
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "))
        .unwrap_or_default()
}

fn remap_claude_answers(questions: &[Value], answers: &Value) -> Value {
    let mut remapped = serde_json::Map::new();
    if let Some(object) = answers.as_object() {
        for (key, value) in object {
            remapped.insert(key.clone(), json!(claude_answer_string(value)));
        }
    }
    for question in questions {
        let question_text = question.get("question").and_then(Value::as_str).unwrap_or("");
        if question_text.is_empty() || remapped.contains_key(question_text) {
            continue;
        }
        for key in [
            question.get("id").and_then(Value::as_str).unwrap_or(""),
            question.get("header").and_then(Value::as_str).unwrap_or(""),
        ] {
            if !key.is_empty() && remapped.contains_key(key) {
                let value = remapped.remove(key).unwrap_or(json!(""));
                remapped.insert(question_text.to_string(), value);
                break;
            }
        }
    }
    Value::Object(remapped)
}

fn handle_can_use_tool(
    stdin: &Arc<Mutex<ChildStdin>>,
    request: &Value,
    request_id: &str,
    thread_id: &str,
    turn_id: &str,
    runtime_mode: &str,
    cancelled: &Arc<AtomicBool>,
) -> Result<(), WsErr> {
    let tool_name = request
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("tool");
    let input = request.get("input").cloned().unwrap_or(json!({}));
    let detail = serde_json::to_string(&input).unwrap_or_else(|_| tool_name.to_string());
    let respond = |result: Value| -> Result<(), WsErr> {
        write_line(
            stdin,
            &json!({
                "type": "control_response",
                "response": {
                    "subtype": "success",
                    "request_id": request_id,
                    "response": result,
                }
            }),
        )
    };
    if tool_name == "ExitPlanMode" {
        let plan_markdown = input
            .get("plan")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|plan| !plan.is_empty())
            .map(str::to_string);
        if let Some(plan_markdown) = plan_markdown {
            let now = iso_now_ms();
            let plan = json!({
                "id": format!("claude-plan-{request_id}"),
                "turnId": turn_id,
                "planMarkdown": plan_markdown,
                "implementedAt": Value::Null,
                "implementationThreadId": Value::Null,
                "createdAt": now,
                "updatedAt": now,
            });
            let _ = crate::orchestration_engine::upsert_proposed_plan_projection(thread_id, &plan);
            if let Ok(c) = open_state_rw() {
                let _ = append_activity(
                    &c,
                    thread_id,
                    turn_id,
                    "info",
                    "proposed-plan.completed",
                    "Proposed plan completed",
                    &json!({
                        "requestId": request_id,
                        "plan": plan,
                        "providerMethod": "can_use_tool/ExitPlanMode",
                        "providerPayload": request,
                    }),
                );
            }
        }
        return respond(json!({
            "behavior": "deny",
            "message": "The client captured your proposed plan. Stop here and wait for the user's feedback or implementation request in a later turn.",
        }));
    }

    if tool_name == "AskUserQuestion" {
        let questions = normalize_ask_questions(&input);
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
                    "providerMethod": "can_use_tool/AskUserQuestion",
                    "providerPayload": request,
                }),
            );
        }

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
        let mut answers_json: Option<String> = None;
        while answers_json.is_none() {
            if cancelled.load(Ordering::SeqCst) || std::time::Instant::now() >= deadline {
                answers_json = Some("{}".to_string());
                break;
            }
            if let Ok(c) = open_state_rw() {
                match c.query_row(
                    "SELECT decision FROM projection_pending_interactions \
                     WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='userInput'",
                    params![thread_id, request_id],
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
        let empty = answers.as_object().map(|object| object.is_empty()).unwrap_or(true);
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
                "User input resolved",
                &json!({"requestId": request_id, "answers": answers, "cancelled": empty}),
            );
        }
        if empty {
            return respond(json!({
                "behavior": "deny",
                "message": "User cancelled tool execution.",
            }));
        }
        return respond(json!({
            "behavior": "allow",
            "updatedInput": {
                "questions": input.get("questions").cloned().unwrap_or(json!([])),
                "answers": remap_claude_answers(&questions, &answers),
            },
        }));
    }

    if runtime_mode == "full-access" {
        return respond(json!({"behavior": "allow", "updatedInput": input}));
    }

    if let Ok(c) = open_state_rw() {
        let _ = c.execute_batch("PRAGMA busy_timeout = 5000;");
        let now = iso_now_ms();
        let _ = c.execute(
            "INSERT OR REPLACE INTO projection_pending_interactions \
             (interaction_kind, request_id, thread_id, turn_id, lifecycle_generation, status, decision, response_command_id, response_requested_at, created_at, resolved_at) \
             VALUES ('approval', ?1, ?2, ?3, NULL, 'pending', NULL, NULL, NULL, ?4, NULL)",
            params![request_id, thread_id, turn_id, now],
        );
        let _ = append_activity(
            &c,
            thread_id,
            turn_id,
            "warn",
            "approval.requested",
            "Approval requested",
            &json!({
                "requestId": request_id,
                "requestKind": permission_kind(tool_name),
                "toolName": tool_name,
                "detail": detail,
                "providerMethod": "can_use_tool",
                "providerPayload": request,
                "sessionApprovalAvailable": request.get("permission_suggestions").is_some(),
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
                "SELECT decision FROM projection_pending_interactions \
                 WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='approval'",
                params![thread_id, request_id],
                |r| r.get::<_, Option<String>>(0),
            ) {
                Ok(Some(d)) => decision = Some(d),
                Ok(None) => {}
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
            "DELETE FROM projection_pending_interactions WHERE thread_id=?1 AND request_id=?2 AND interaction_kind='approval'",
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
    if decision == "accept" || decision == "acceptForSession" {
        let mut result = json!({"behavior": "allow", "updatedInput": input});
        if decision == "acceptForSession" {
            if let Some(suggestions) = request.get("permission_suggestions").cloned() {
                result["updatedPermissions"] = suggestions;
            }
        }
        respond(result)
    } else {
        respond(json!({
            "behavior": "deny",
            "message": if decision == "cancel" { "User cancelled tool execution." } else { "User declined tool execution." },
        }))
    }
}

fn claude_window_tokens(value: &str) -> Option<i64> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized == "1m" {
        return Some(1_000_000);
    }
    if normalized == "200k" {
        return Some(200_000);
    }
    if let Some(number) = normalized.strip_suffix('k').and_then(|v| v.parse::<i64>().ok()) {
        return Some(number * 1_000);
    }
    if let Some(number) = normalized.strip_suffix('m').and_then(|v| v.parse::<i64>().ok()) {
        return Some(number * 1_000_000);
    }
    normalized.parse::<i64>().ok().filter(|value| *value > 0)
}

fn selected_claude_settings(model_selection: &Value) -> Value {
    let mut settings = serde_json::Map::new();
    if let Some(thinking) = model_selection.pointer("/options/thinking").and_then(Value::as_bool) {
        settings.insert("alwaysThinkingEnabled".to_string(), json!(thinking));
    }
    if model_selection.pointer("/options/fastMode").and_then(Value::as_bool) == Some(true) {
        settings.insert("fastMode".to_string(), json!(true));
    }
    let effort = model_selection
        .pointer("/options/effort")
        .and_then(Value::as_str)
        .or_else(|| model_selection.pointer("/options/reasoningEffort").and_then(Value::as_str))
        .unwrap_or("");
    if effort == "ultracode" {
        settings.insert("ultracode".to_string(), json!(true));
    }
    let window = model_selection
        .pointer("/options/autoCompactWindow")
        .and_then(Value::as_str)
        .or_else(|| model_selection.pointer("/options/contextWindow").and_then(Value::as_str));
    if let Some(tokens) = window.and_then(claude_window_tokens) {
        settings.insert("autoCompactWindow".to_string(), json!(tokens));
    }
    Value::Object(settings)
}

fn selected_claude_model(model_selection: &Value) -> Option<String> {
    let model = model_selection
        .get("model")
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    if model.is_empty() {
        return None;
    }
    let context_window = model_selection
        .pointer("/options/autoCompactWindow")
        .and_then(Value::as_str)
        .or_else(|| model_selection.pointer("/options/contextWindow").and_then(Value::as_str))
        .map(str::trim)
        .unwrap_or("");
    let has_suffix = model.ends_with(']') && model.rfind('[').is_some_and(|pos| pos < model.len() - 1);
    if context_window == "1m" && !has_suffix {
        return Some(format!("{model}[1m]"));
    }
    Some(model)
}

fn selected_claude_effort(model_selection: &Value) -> Option<String> {
    let effort = model_selection
        .pointer("/options/effort")
        .and_then(Value::as_str)
        .or_else(|| model_selection.pointer("/options/reasoningEffort").and_then(Value::as_str))?
        .trim();
    match effort {
        "low" | "medium" | "high" | "xhigh" | "max" => Some(effort.to_string()),
        "ultracode" => Some("xhigh".to_string()),
        _ => None,
    }
}

fn spawn_claude(
    cwd: &str,
    runtime_mode: &str,
    model: Option<&str>,
    effort: Option<&str>,
    settings: Option<&Value>,
    resume: Option<&str>,
    thread_id: Option<&str>,
    instance_id: Option<&str>,
) -> Result<(Child, ChildStdin, ChildStdout), WsErr> {
    let mut args = vec![
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--verbose".to_string(),
        "--input-format".to_string(),
        "stream-json".to_string(),
    ];
    if runtime_mode == "full-access" {
        args.push("--dangerously-skip-permissions".to_string());
    } else if runtime_mode == "auto" {
        args.push("--permission-mode".to_string());
        args.push("auto".to_string());
    }
    if let Some(model) = model {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    if let Some(effort) = effort {
        args.push("--effort".to_string());
        args.push(effort.to_string());
    }
    if let Some(settings) = settings.filter(|value| !value.as_object().is_none_or(|object| object.is_empty())) {
        args.push("--settings".to_string());
        args.push(settings.to_string());
    }
    if let Some(sid) = resume {
        args.push("--resume".to_string());
        args.push(sid.to_string());
    }
    let mcp_config = json!({
        "mcpServers": {"synara": {
            "type": "http",
            "url": crate::agent_gateway_mcp::agent_gateway_url(),
            "headers": {"Authorization": format!("Bearer {}", match thread_id {
                Some(tid) => crate::agent_gateway_mcp::issue_thread_token(tid),
                None => crate::agent_gateway_mcp::agent_gateway_token().to_string(),
            })},
        }}
    });
    args.push("--mcp-config".to_string());
    args.push(mcp_config.to_string());

    let mut command = Command::new(claude_binary());
    command
        .args(&args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    crate::provider_env::sanitize_claude_child(&mut command);
    // Provider instance environment, applied after sanitization so deliberate
    // per-account configuration always wins over ambient filtering. A
    // non-default instance without an explicit home is forced into its
    // isolated provider home, upstream `buildClaudeInstanceProcessEnv`.
    let settings = crate::server_settings::current();
    if let Some(instance) = crate::provider_instances::resolve_instance(&settings, instance_id, Some("claudeAgent")) {
        for (key, value) in crate::provider_instances::instance_start_environment(&instance) {
            command.env(key, value);
        }
        if !instance.is_default && crate::provider_instances::instance_home_path(&instance).is_none() {
            let userdata = crate::todo::state_dir();
            let home = crate::claude_environment::isolated_home_path(&userdata, Some(&instance.instance_id));
            let _ = std::fs::create_dir_all(&home);
            for (key, value) in crate::claude_environment::home_environment(&home) {
                command.env(key, value);
            }
        }
    }
    let mut child = command
        .spawn()
        .map_err(|e| err(format!("could not launch `claude` CLI: {e}")))?;
    let stdin = child.stdin.take().ok_or_else(|| err("claude stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| err("claude stdout"))?;
    Ok((child, stdin, stdout))
}

fn read_json_line(reader: &mut BufReader<ChildStdout>) -> Result<Option<Value>, WsErr> {
    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| err(format!("claude read: {e}")))?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            return Ok(Some(v));
        }
    }
}

fn initialize_cli_response(
    stdin: &Arc<Mutex<ChildStdin>>,
    reader: &mut BufReader<ChildStdout>,
) -> Result<Value, WsErr> {
    write_line(
        stdin,
        &json!({
            "request_id": "synara-initialize",
            "type": "control_request",
            "request": {"subtype": "initialize"},
        }),
    )?;
    loop {
        let Some(v) = read_json_line(reader)? else {
            return Err(err("claude exited during initialize"));
        };
        if v.get("type").and_then(Value::as_str) == Some("control_response")
            && v.pointer("/response/request_id").and_then(Value::as_str) == Some("synara-initialize")
        {
            if v.pointer("/response/subtype").and_then(Value::as_str) == Some("success") {
                return Ok(v.pointer("/response/response").cloned().unwrap_or(json!({})));
            }
            let msg = v
                .pointer("/response/error")
                .and_then(Value::as_str)
                .unwrap_or("claude initialize failed");
            return Err(err(msg));
        }
    }
}

fn initialize_cli(
    stdin: &Arc<Mutex<ChildStdin>>,
    reader: &mut BufReader<ChildStdout>,
) -> Result<(), WsErr> {
    initialize_cli_response(stdin, reader).map(|_| ())
}



fn claude_reasoning_efforts(model: &Value) -> Vec<Value> {
    model
        .get("supportedEffortLevels")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|value| json!({"value": value, "label": value}))
        .collect()
}

fn claude_catalog_from_initialize(init: &Value) -> Value {
    let commands = init
        .get("commands")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|command| {
            let name = command.get("name").and_then(Value::as_str)?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let mut object = json!({"name": name});
            if let Some(description) = command.get("description").and_then(Value::as_str) {
                if !description.trim().is_empty() {
                    object["description"] = json!(description);
                }
            }
            Some(object)
        })
        .collect::<Vec<_>>();
    let models = init
        .get("models")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|model| {
            let value = model.get("value").and_then(Value::as_str)?.trim().to_string();
            if value.is_empty() {
                return None;
            }
            let efforts = claude_reasoning_efforts(model);
            let mut object = json!({
                "slug": value,
                "name": model.get("displayName").and_then(Value::as_str).unwrap_or(""),
            });
            if let Some(resolved) = model.get("resolvedModel").and_then(Value::as_str) {
                object["resolvedModel"] = json!(resolved);
            }
            if !efforts.is_empty() || model.get("supportsEffort").and_then(Value::as_bool) == Some(false) {
                object["supportedReasoningEfforts"] = json!(efforts);
            }
            if let Some(fast) = model.get("supportsFastMode").and_then(Value::as_bool) {
                object["supportsFastMode"] = json!(fast);
            }
            if model.get("supportsAdaptiveThinking").and_then(Value::as_bool) == Some(true) {
                object["supportsThinkingToggle"] = json!(false);
            }
            if let Some(auto) = model.get("supportsAutoMode").and_then(Value::as_bool) {
                object["supportsAutoMode"] = json!(auto);
            }
            Some(object)
        })
        .collect::<Vec<_>>();
    let agents = init
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|agent| {
            let name = agent.get("name").and_then(Value::as_str)?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let mut object = json!({"name": name, "displayName": name});
            if let Some(description) = agent.get("description").and_then(Value::as_str) {
                if !description.trim().is_empty() {
                    object["description"] = json!(description);
                }
            }
            if let Some(model) = agent.get("model").and_then(Value::as_str) {
                object["model"] = json!(model);
            }
            Some(object)
        })
        .collect::<Vec<_>>();
    json!({"commands": commands, "models": models, "agents": agents})
}

/// Discover the official CLI's session catalog by initializing a temporary
/// stream-json process. The process exits before any user prompt and only the
/// official binary reads its configured credentials.
pub fn discover_catalog(cwd: &str) -> Result<Value, WsErr> {
    let cwd = if !cwd.trim().is_empty() && std::path::Path::new(cwd).is_dir() {
        cwd.to_string()
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    };
    let (mut child, stdin_raw, stdout) = spawn_claude(&cwd, "approval-required", None, None, None, None, None, None)?;
    let stdin = Arc::new(Mutex::new(stdin_raw));
    let mut reader = BufReader::new(stdout);
    let result = initialize_cli_response(&stdin, &mut reader).map(|init| claude_catalog_from_initialize(&init));
    let _ = child.kill();
    let _ = child.wait();
    result
}


/// Compact a warm Claude session with the CLI's native `/compact` command.
/// This is intentionally limited to idle parked sessions: compaction is a
/// session mutation, not something to multiplex with a live turn.
pub fn compact_thread(thread_id: &str) -> Result<bool, WsErr> {
    if has_running_turn(thread_id) {
        return Err(err("cannot compact while a turn is running"));
    }
    let mut parked = {
        let mut parked_map = PARKED.lock().map_err(|_| err("claude parked map"))?;
        match parked_map.remove(thread_id) {
            Some(parked) => parked,
            None => return Ok(false),
        }
    };
    let result = (|| -> Result<bool, WsErr> {
        write_line(
            &parked.stdin,
            &user_message(vec![json!({"type":"text","text":"/compact"})]),
        )?;
        loop {
            let Some(v) = read_json_line(&mut parked.reader)? else {
                return Err(err("claude exited during compaction"));
            };
            if v.get("type").and_then(Value::as_str) == Some("result") {
                let subtype = v.get("subtype").and_then(Value::as_str).unwrap_or("");
                if subtype == "success" {
                    return Ok(true);
                }
                return Err(err(format!("claude compaction failed: {subtype}")));
            }
        }
    })();
    match result {
        Ok(value) => {
            if let Ok(mut parked_map) = PARKED.lock() {
                parked_map.insert(thread_id.to_string(), parked);
            } else {
                drop_parked(parked);
            }
            Ok(value)
        }
        Err(error) => {
            drop_parked(parked);
            Err(error)
        }
    }
}

/// Start Claude's streaming-input CLI before the first prompt and park it
/// after the control initialize handshake. No user/model traffic is sent.
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
    let (raw_cwd, runtime_mode, model_selection_json): (String, String, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(NULLIF(working_directory, ''), NULLIF(worktree_path, ''), NULLIF(associated_worktree_path, ''), (SELECT workspace_root FROM projection_projects WHERE project_id=projection_threads.project_id), ''), \
                    COALESCE((SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id=?1), 'full-access'), \
                    model_selection_json \
             FROM projection_threads WHERE thread_id=?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| err(format!("claude prewarm thread lookup: {e}")))?;
    let stored_session: Option<String> = conn
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
    let model_selection = model_selection_json
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let selected_model = selected_claude_model(&model_selection);
    let selected_effort = selected_claude_effort(&model_selection);
    let selected_settings = selected_claude_settings(&model_selection);
    let launch_key = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        claude_binary(),
        cwd,
        runtime_mode,
        selected_model.as_deref().unwrap_or(""),
        selected_effort.as_deref().unwrap_or(""),
        selected_settings,
        model_selection.get("instanceId").and_then(Value::as_str).unwrap_or("")
    );
    let (mut child, stdin_raw, stdout) = spawn_claude(
        &cwd,
        &runtime_mode,
        selected_model.as_deref(),
        selected_effort.as_deref(),
        Some(&selected_settings),
        stored_session.as_deref(),
        Some(thread_id),
        model_selection.get("instanceId").and_then(Value::as_str),
    )?;
    let stdin = Arc::new(Mutex::new(stdin_raw));
    let mut reader = BufReader::new(stdout);
    match initialize_cli(&stdin, &mut reader) {
        Ok(()) => {
            if let Ok(mut parked_map) = PARKED.lock() {
                parked_map.insert(
                    thread_id.to_string(),
                    ParkedClaude {
                        child,
                        stdin,
                        reader,
                        session_id: stored_session,
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
    let turn_runtime_mode: String = conn
        .query_row(
            "SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "full-access".to_string());
    let model_selection: Value = conn
        .query_row(
            "SELECT model_selection_json FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let selected_model = selected_claude_model(&model_selection);
    let selected_effort = selected_claude_effort(&model_selection);
    let selected_settings = selected_claude_settings(&model_selection);
    let cwd = if !cwd.is_empty() && std::path::Path::new(cwd).is_dir() {
        cwd.to_string()
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    };
    let launch_key = format!(
        "{}|{}|{}|{}|{}|{}",
        claude_binary(),
        cwd,
        turn_runtime_mode,
        selected_model.as_deref().unwrap_or(""),
        selected_effort.as_deref().unwrap_or(""),
        selected_settings
    );

    let turn_id = uuid_v4();
    let assistant_msg_id = uuid_v4();
    let now = iso_now_ms();
    conn.execute(
        "INSERT INTO projection_turns (thread_id, turn_id, pending_message_id, assistant_message_id, state, requested_at, started_at, checkpoint_files_json) VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?5, '[]')",
        params![thread_id, turn_id, message_id, assistant_msg_id, now],
    )
    .map_err(|e| err(format!("insert turn: {e}")))?;
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_sessions (thread_id, status, provider_name, provider_session_id, provider_thread_id, active_turn_id, last_error, updated_at, runtime_mode) VALUES (?1, 'running', 'claudeAgent', NULL, ?2, ?3, NULL, ?4, COALESCE((SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1), 'full-access'))",
        params![thread_id, stored_session, turn_id, now],
    )
    .map_err(|e| err(format!("session start: {e}")))?;
    session_set_event(
        &conn,
        thread_id,
        &json!({"threadId": thread_id, "status": "running", "providerName": "claudeAgent", "activeTurnId": turn_id, "runtimeMode": turn_runtime_mode, "lastError": null, "updatedAt": now}),
    )?;

    let parked = take_parked(thread_id, &launch_key);
    let fresh = parked.is_none();
    let mut monitor_cursor = crate::claude_monitor::MonitorCursor::default();
    let turn_started_at = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        crate::claude_monitor::iso_from_ms(now.as_millis() as i64)
    };
    let (mut child, stdin, mut reader, mut session_id) = if let Some(parked) = parked {
        (
            parked.child,
            parked.stdin,
            parked.reader,
            parked.session_id.or_else(|| stored_session.clone()),
        )
    } else {
        let (child, stdin_raw, stdout) = spawn_claude(
            &cwd,
            &turn_runtime_mode,
            selected_model.as_deref(),
            selected_effort.as_deref(),
            Some(&selected_settings),
            stored_session.as_deref(),
            Some(thread_id),
            model_selection.get("instanceId").and_then(Value::as_str),
        )?;
        let stdin = Arc::new(Mutex::new(stdin_raw));
        let mut reader = BufReader::new(stdout);
        initialize_cli(&stdin, &mut reader)?;
        (child, stdin, reader, stored_session.clone())
    };

    let prompt_text = if fresh && stored_session.is_none() {
        format!("{}{}", crate::harness_policy::first_prompt_prefix(), text)
    } else {
        text.to_string()
    };
    let attachments = crate::provider_attachments::message_attachments(thread_id, message_id)?;
    let content = claude_content_blocks(&prompt_text, &attachments)?;
    write_line(&stdin, &user_message(content.clone()))?;
    let cancelled = Arc::new(AtomicBool::new(false));
    {
        let mut map = RUNNING.lock().map_err(|_| err("running map"))?;
        map.insert(
            thread_id.to_string(),
            RunningTurn {
                stdin: stdin.clone(),
                cancelled: cancelled.clone(),
            },
        );
    }

    let mut usage_windows = json!({"windows": {}});
    let mut edge_attempt = 0usize;
    let mut tracked_tasks: HashMap<String, crate::claude_tasks::TrackedTask> = HashMap::new();
    let mut pending_tools: HashMap<String, (String, Value)> = HashMap::new();
    let result = (|| -> Result<String, WsErr> {
        loop {
            let Some(v) = read_json_line(&mut reader)? else {
                return Err(err("claude exited before turn result"));
            };
            if v.get("rate_limit_info").is_some() || v.get("type").and_then(Value::as_str) == Some("rate_limit_info") {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                crate::provider_usage::persist("claudeAgent", &v);
                if let Some(next) = crate::usage_limit::claude_usage_limit_windows(&usage_windows, &v, now_ms) {
                    usage_windows = next;
                }
            }
            match v.get("type").and_then(Value::as_str).unwrap_or("") {
                "system" => {
                    let subtype = v.get("subtype").and_then(Value::as_str).unwrap_or("");
                    if subtype == "init" {
                        if let Some(sid) = v.get("session_id").and_then(Value::as_str) {
                            session_id = Some(sid.to_string());
                            let _ = set_session(&conn, thread_id, "running", Some(&turn_id), None, Some(sid));
                        }
                    } else if subtype == "task_progress" || subtype == "task_notification" {
                        // Subagent progress/terminal, sliced from upstream
                        // ClaudeAdapter: emit only when the event's tool_use_id
                        // matches an in-flight tool call — unattributable events
                        // are dropped exactly as upstream drops them. The deep
                        // layer (SendMessage alias retirement, child-thread
                        // mirroring, workflow pause/kill) is not ported yet.
                        let tool_use_id = v.get("tool_use_id").and_then(Value::as_str).unwrap_or("");
                        let task_id = v.get("task_id").and_then(Value::as_str).unwrap_or("");
                        let attributable = !tool_use_id.is_empty() && pending_tools.contains_key(tool_use_id);
                        if attributable && !task_id.is_empty() {
                            let mut payload = serde_json::Map::new();
                            payload.insert("taskId".into(), json!(task_id));
                            if let Some(d) = v.get("description").and_then(Value::as_str) {
                                if !d.trim().is_empty() {
                                    payload.insert("description".into(), json!(d.trim()));
                                }
                            }
                            if let Some(sum) = v.get("summary").and_then(Value::as_str) {
                                if !sum.trim().is_empty() {
                                    payload.insert("summary".into(), json!(sum.trim()));
                                }
                            }
                            if let Some(u) = v.get("usage").filter(|u| u.is_object()) {
                                payload.insert("usage".into(), u.clone());
                            }
                            if let Some(lt) = v.get("last_tool_name").and_then(Value::as_str) {
                                if !lt.trim().is_empty() {
                                    payload.insert("lastToolName".into(), json!(lt.trim()));
                                }
                            }
                            if let Some(st) = v.get("status").and_then(Value::as_str) {
                                payload.insert("status".into(), json!(st));
                            }
                            let kind = if subtype == "task_progress" { "task.progress" } else { "task.updated" };
                            let summary = if subtype == "task_progress" { "Subagent progress" } else { "Subagent update" };
                            if let Ok(c) = open_state_rw() {
                                let _ = append_activity(&c, thread_id, &turn_id, "info", kind, summary, &Value::Object(payload));
                            }
                        }
                    }
                }
                "assistant" => {
                    let parts = v.pointer("/message/content").and_then(Value::as_array);
                    let text_out: String = parts
                        .map(|parts| parts.iter().filter(|p| p.get("type").and_then(Value::as_str) == Some("text")).filter_map(|p| p.get("text").and_then(Value::as_str)).collect::<Vec<_>>().join(""))
                        .unwrap_or_default();
                    if !text_out.is_empty() {
                        if let Ok(c) = open_state_rw() {
                            let _ = append_assistant_text(&c, thread_id, &assistant_msg_id, &turn_id, &text_out, true);
                        }
                    }
                    if let Some(parts) = parts {
                        for block in parts {
                            if block.get("type").and_then(Value::as_str) != Some("tool_use") {
                                continue;
                            }
                            let name = block.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                            let input = block.get("input").cloned().unwrap_or(json!({}));
                            if let Some(id) = block.get("id").and_then(Value::as_str) {
                                pending_tools.insert(id.to_string(), (name.clone(), input.clone()));
                            }
                            if name == "TodoWrite" {
                                if let Some(payload) = crate::claude_tasks::todo_payload(&input) {
                                    if let Ok(c) = open_state_rw() {
                                        let _ = append_activity(&c, thread_id, &turn_id, "info", "turn.tasks.updated", "Task list updated", &payload);
                                    }
                                }
                            }
                        }
                    }
                }
                "user" => {
                    // Tool results arrive as user-type frames; the task tools
                    // (TaskCreate/Update/Get/List) update the tracked map and
                    // re-emit the whole list, mirroring upstream.
                    if let Some(parts) = v.pointer("/message/content").and_then(Value::as_array) {
                        for block in parts {
                            if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                                continue;
                            }
                            let Some(use_id) = block.get("tool_use_id").and_then(Value::as_str) else { continue };
                            let Some((name, input)) = pending_tools.get(use_id).cloned() else { continue };
                            let is_error = block.get("is_error").and_then(Value::as_bool) == Some(true);
                            let content = block.get("content").cloned().unwrap_or(Value::Null);
                            if crate::claude_tasks::apply_tool_result(&mut tracked_tasks, &name, &input, &content, is_error) {
                                let payload = crate::claude_tasks::tracked_payload(&tracked_tasks);
                                if let Ok(c) = open_state_rw() {
                                    let _ = append_activity(&c, thread_id, &turn_id, "info", "turn.tasks.updated", "Task list updated", &payload);
                                }
                            }
                            if name == "Workflow" && !is_error {
                                // Workflow launch detection, upstream ClaudeAdapter:
                                // structured result first, then text fallback.
                                let text = match &content {
                                    Value::String(t) => t.clone(),
                                    Value::Array(blocks) => blocks
                                        .iter()
                                        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                                        .filter_map(|b| b.get("text").and_then(Value::as_str))
                                        .collect::<Vec<_>>()
                                        .join("\n"),
                                    other => serde_json::to_string(other).unwrap_or_default(),
                                };
                                let launch = crate::claude_workflow_script::parse_workflow_launch(&content)
                                    .or_else(|| {
                                        if text.is_empty() {
                                            None
                                        } else {
                                            crate::claude_workflow_script::parse_workflow_launch_from_text(&text)
                                        }
                                    });
                                if let Some(launch) = launch {
                                    let task_id = launch
                                        .get("taskId")
                                        .and_then(Value::as_str)
                                        .or_else(|| launch.get("runId").and_then(Value::as_str))
                                        .unwrap_or("")
                                        .to_string();
                                    if !task_id.is_empty() {
                                        if let Ok(c) = open_state_rw() {
                                            let _ = append_activity(
                                                &c,
                                                thread_id,
                                                &turn_id,
                                                "info",
                                                "task.updated",
                                                "Workflow launched",
                                                &json!({
                                                    "taskId": task_id,
                                                    "workflowRunId": launch.get("runId"),
                                                    "workflowScriptPath": launch.get("scriptPath"),
                                                }),
                                            );
                                        }
                                        if let Some(dir) = launch.get("transcriptDir").and_then(Value::as_str) {
                                            // Labels zip against journal start order; read
                                            // them from the persisted script when present.
                                            let labels = launch
                                                .get("scriptPath")
                                                .and_then(Value::as_str)
                                                .and_then(|p| std::fs::read_to_string(p).ok())
                                                .map(|s| crate::claude_workflow_script::ordered_agent_labels(&s))
                                                .unwrap_or_default();
                                            crate::claude_workflow_runtime::spawn_workflow_poller(
                                                thread_id.to_string(),
                                                turn_id.clone(),
                                                task_id,
                                                dir.to_string(),
                                                labels,
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                "control_request" => {
                    let request_id = v.get("request_id").and_then(Value::as_str).unwrap_or("").to_string();
                    let subtype = v.pointer("/request/subtype").and_then(Value::as_str).unwrap_or("");
                    if subtype == "can_use_tool" && !request_id.is_empty() {
                        let request = v.get("request").cloned().unwrap_or(json!({}));
                        handle_can_use_tool(&stdin, &request, &request_id, thread_id, &turn_id, &turn_runtime_mode, &cancelled)?;
                    }
                }
                "result" => {
                    // Monitor notifications never reach the SDK stream; read
                    // them back from the session transcript when this turn was
                    // woken by one, upstream emitMonitorEvents.
                    if v.pointer("/origin/kind").and_then(Value::as_str) == Some("task-notification") {
                        if let Some(sid) = session_id.clone().or_else(|| stored_session.clone()) {
                            let config_dir = std::env::var("CLAUDE_CONFIG_DIR")
                                .ok()
                                .filter(|v| !v.is_empty())
                                .unwrap_or_else(|| {
                                    let home = std::env::var("HOME").unwrap_or_default();
                                    format!("{home}/.claude")
                                });
                            let events = crate::claude_monitor::read_monitor_events(
                                &mut monitor_cursor,
                                &sid,
                                &config_dir,
                                &turn_started_at,
                            );
                            if !events.is_empty() {
                                if let Ok(c) = open_state_rw() {
                                    for event in events {
                                        let _ = append_activity(
                                            &c,
                                            thread_id,
                                            &turn_id,
                                            "warning",
                                            "monitor.event",
                                            &event.message,
                                            &json!({
                                                "type": "system",
                                                "subtype": "monitor_event",
                                                "task_id": event.task_id,
                                                "notificationId": event.id,
                                                "name": event.name,
                                                "output": event.output,
                                                "outcome": event.outcome,
                                                "createdAt": event.created_at,
                                            }),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    let subtype = v.get("subtype").and_then(Value::as_str).unwrap_or("");
                    if subtype == "success" {
                        break Ok("completed".to_string());
                    }
                    let msg = v.get("result").and_then(Value::as_str)
                        .or_else(|| v.pointer("/error/message").and_then(Value::as_str))
                        .unwrap_or("claude turn error")
                        .to_string();
                    // Edge block: keep the turn open and re-prompt the same
                    // live CLI after a backoff, mirroring upstream
                    // retryEdgeBlockedTurn. Interrupts and cancel stop it.
                    if is_edge_blocked_text(&msg) && !cancelled.load(Ordering::SeqCst) {
                        let delays = edge_block_retry_delays_ms();
                        if edge_attempt < delays.len() {
                            let delay = delays[edge_attempt];
                            edge_attempt += 1;
                            let mut waited = 0u64;
                            while waited < delay && !cancelled.load(Ordering::SeqCst) {
                                let step = (delay - waited).min(100);
                                std::thread::sleep(std::time::Duration::from_millis(step));
                                waited += step;
                            }
                            if cancelled.load(Ordering::SeqCst) {
                                break Err(err(msg));
                            }
                            if write_line(&stdin, &user_message(content.clone())).is_ok() {
                                continue;
                            }
                        }
                        break Err(err(format!(
                            "Anthropic's Cloudflare edge temporarily blocked the request (HTTP 403). Your Claude login is fine; send a message to continue."
                        )));
                    }
                    break Err(err(msg));
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
    if clean {
        if child.try_wait().ok().flatten().is_none() {
            if let Ok(mut parked_map) = PARKED.lock() {
                parked_map.insert(
                    thread_id.to_string(),
                    ParkedClaude {
                        child,
                        stdin,
                        reader,
                        session_id: session_id.clone(),
                        launch_key,
                    },
                );
            }
        } else {
            let _ = child.kill();
            let _ = child.wait();
        }
    } else {
        let _ = child.kill();
        let _ = child.wait();
    }

    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    match result {
        Ok(_) => {
            let interrupted = cancelled_flag;
            let state = if interrupted { "interrupted" } else { "completed" };
            let status = if interrupted { "interrupted" } else { "ready" };
            set_turn_state(&conn, thread_id, &turn_id, state, true)?;
            set_session(&conn, thread_id, status, Some(""), None, session_id.as_deref())?;
            session_set_event(&conn, thread_id, &json!({"threadId": thread_id, "status": status, "providerName": "claudeAgent", "activeTurnId": null, "runtimeMode": turn_runtime_mode, "lastError": null, "updatedAt": iso_now_ms()}))?;
            crate::orchestration_engine::continue_queued_turn(thread_id);
            Ok(())
        }
        Err(e) => {
            let (state, status) = if cancelled_flag { ("interrupted", "interrupted") } else { ("error", "error") };
            set_turn_state(&conn, thread_id, &turn_id, state, true)?;
            // Usage-limit evidence: schedule automatic recovery while the
            // session still carries the active turn (`source` requires it).
            if !cancelled_flag {
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);
                if let Some(reset_at) = crate::usage_limit::usage_limit_reset_at(&usage_windows, now_ms) {
                    let epoch = format!("claude-{turn_id}");
                    let binding = json!({
                        "turnId": turn_id,
                        "bindingKey": "detection",
                        "policyEpoch": epoch,
                    });
                    let model_sel: Value = conn
                        .query_row(
                            "SELECT model_selection_json FROM projection_threads WHERE thread_id=?1",
                            rusqlite::params![thread_id],
                            |r| r.get::<_, Option<String>>(0),
                        )
                        .ok()
                        .flatten()
                        .and_then(|j| serde_json::from_str(&j).ok())
                        .unwrap_or(Value::Null);
                    let now = iso_now_ms();
                    let mut src = serde_json::Map::new();
                    src.insert("type".into(), json!("thread.usage-limit.source"));
                    src.insert("commandId".into(), json!(format!("ul-source:{turn_id}")));
                    src.insert("threadId".into(), json!(thread_id));
                    src.insert("createdAt".into(), json!(now));
                    src.insert("source".into(), binding.clone());
                    let mut sched = serde_json::Map::new();
                    sched.insert("type".into(), json!("thread.usage-limit.schedule"));
                    sched.insert("commandId".into(), json!(format!("ul-schedule:{turn_id}")));
                    sched.insert("threadId".into(), json!(thread_id));
                    sched.insert("createdAt".into(), json!(now));
                    sched.insert("recovery".into(), json!({
                        "id": format!("ul-{turn_id}"),
                        "turnId": turn_id,
                        "bindingKey": "detection",
                        "policyEpoch": epoch,
                        "resetAt": reset_at,
                        "provider": model_sel.get("provider").cloned().unwrap_or(json!("claudeAgent")),
                        "providerInstanceId": model_sel.get("instanceId").or_else(|| model_sel.get("provider")).cloned().unwrap_or(json!("claudeAgent")),
                        "model": model_sel.get("model").cloned().unwrap_or(Value::Null),
                    }));
                    let _ = crate::orchestration_engine::handle("orchestration.dispatchCommand", &Value::Object(src));
                    let _ = crate::orchestration_engine::handle("orchestration.dispatchCommand", &Value::Object(sched));
                }
            }
            set_session(&conn, thread_id, status, Some(""), Some(&e.0), session_id.as_deref())?;
            session_set_event(&conn, thread_id, &json!({"threadId": thread_id, "status": status, "providerName": "claudeAgent", "activeTurnId": null, "runtimeMode": turn_runtime_mode, "lastError": e.0, "updatedAt": iso_now_ms()}))?;
            if cancelled_flag {
                crate::orchestration_engine::continue_queued_turn(thread_id);
            }
            Err(e)
        }
    }
}
