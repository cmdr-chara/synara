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

fn spawn_claude(
    cwd: &str,
    runtime_mode: &str,
    model: Option<&str>,
    resume: Option<&str>,
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
    if let Some(sid) = resume {
        args.push("--resume".to_string());
        args.push(sid.to_string());
    }
    let mcp_config = json!({
        "mcpServers": {"synara": {
            "type": "http",
            "url": crate::agent_gateway_mcp::agent_gateway_url(),
            "headers": {"Authorization": format!("Bearer {}", crate::agent_gateway_mcp::agent_gateway_token())},
        }}
    });
    args.push("--mcp-config".to_string());
    args.push(mcp_config.to_string());

    let mut child = Command::new(claude_binary())
        .args(&args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
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
    let (mut child, stdin_raw, stdout) = spawn_claude(&cwd, "approval-required", None, None)?;
    let stdin = Arc::new(Mutex::new(stdin_raw));
    let mut reader = BufReader::new(stdout);
    let result = initialize_cli_response(&stdin, &mut reader).map(|init| claude_catalog_from_initialize(&init));
    let _ = child.kill();
    let _ = child.wait();
    result
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
    let selected_model = model_selection_json
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|value| value.get("model").and_then(Value::as_str).map(str::to_string));
    let launch_key = format!(
        "{}|{}|{}|{}",
        claude_binary(),
        cwd,
        runtime_mode,
        selected_model.as_deref().unwrap_or("")
    );
    let (mut child, stdin_raw, stdout) = spawn_claude(
        &cwd,
        &runtime_mode,
        selected_model.as_deref(),
        stored_session.as_deref(),
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
    let selected_model: Option<String> = conn
        .query_row(
            "SELECT model_selection_json FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("model").and_then(Value::as_str).map(str::to_string));
    let cwd = if !cwd.is_empty() && std::path::Path::new(cwd).is_dir() {
        cwd.to_string()
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    };
    let launch_key = format!(
        "{}|{}|{}|{}",
        claude_binary(),
        cwd,
        turn_runtime_mode,
        selected_model.as_deref().unwrap_or("")
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
            stored_session.as_deref(),
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
    write_line(&stdin, &user_message(content))?;
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

    let result = (|| -> Result<String, WsErr> {
        loop {
            let Some(v) = read_json_line(&mut reader)? else {
                return Err(err("claude exited before turn result"));
            };
            match v.get("type").and_then(Value::as_str).unwrap_or("") {
                "system" => {
                    if v.get("subtype").and_then(Value::as_str) == Some("init") {
                        if let Some(sid) = v.get("session_id").and_then(Value::as_str) {
                            session_id = Some(sid.to_string());
                            let _ = set_session(&conn, thread_id, "running", Some(&turn_id), None, Some(sid));
                        }
                    }
                }
                "assistant" => {
                    let text_out: String = v
                        .pointer("/message/content")
                        .and_then(Value::as_array)
                        .map(|parts| parts.iter().filter(|p| p.get("type").and_then(Value::as_str) == Some("text")).filter_map(|p| p.get("text").and_then(Value::as_str)).collect::<Vec<_>>().join(""))
                        .unwrap_or_default();
                    if !text_out.is_empty() {
                        if let Ok(c) = open_state_rw() {
                            let _ = append_assistant_text(&c, thread_id, &assistant_msg_id, &turn_id, &text_out, true);
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
                    let subtype = v.get("subtype").and_then(Value::as_str).unwrap_or("");
                    if subtype == "success" {
                        break Ok("completed".to_string());
                    }
                    let msg = v.get("result").and_then(Value::as_str)
                        .or_else(|| v.pointer("/error/message").and_then(Value::as_str))
                        .unwrap_or("claude turn error");
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
            set_session(&conn, thread_id, status, Some(""), Some(&e.0), session_id.as_deref())?;
            session_set_event(&conn, thread_id, &json!({"threadId": thread_id, "status": status, "providerName": "claudeAgent", "activeTurnId": null, "runtimeMode": turn_runtime_mode, "lastError": e.0, "updatedAt": iso_now_ms()}))?;
            if cancelled_flag {
                crate::orchestration_engine::continue_queued_turn(thread_id);
            }
            Err(e)
        }
    }
}
