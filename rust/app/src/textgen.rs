//! Auxiliary AI text generation through the official Claude CLI, mirroring
//! upstream ClaudeTextGeneration: `claude -p` in safe mode with no tools, no
//! session persistence, strict MCP config, JSON output schema enforcement,
//! prompt on stdin, isolated cwd. The CLI owns all credentials; Synara only
//! ever sees the generated text.

use serde_json::{json, Value};

use crate::todo::WsErr;

fn err(message: &str) -> WsErr {
    WsErr(message.to_string(), None)
}

pub fn handles(tag: &str) -> bool {
    matches!(tag, "server.generateThreadRecap" | "server.generateAutomationIntent")
}

/// Run one JSON-mode generation and return the structured output object.
pub(crate) fn claude_json_generate(prompt: &str, schema: &Value, model: Option<&str>) -> Result<Value, WsErr> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let isolated = std::env::temp_dir().join(format!("synara-textgen-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&isolated);

    let mut args = vec![
        "-p".to_string(),
        "--safe-mode".to_string(),
        "--setting-sources".to_string(),
        String::new(),
        "--strict-mcp-config".to_string(),
        "--no-session-persistence".to_string(),
        "--output-format".to_string(),
        "json".to_string(),
        "--json-schema".to_string(),
        schema.to_string(),
    ];
    if let Some(model) = model {
        args.push("--model".to_string());
        args.push(model.to_string());
    }
    // No tools: untrusted prompt content cannot trigger workspace reads/edits.
    args.push("--tools".to_string());
    args.push(String::new());

    let binary = std::env::var("SYNARA_CLAUDE_CLI")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| std::env::var("CLAUDE_BINARY_PATH").ok().filter(|v| !v.trim().is_empty()))
        .unwrap_or_else(|| "claude".to_string());

    let mut child = Command::new(&binary)
        .args(&args)
        .current_dir(&isolated)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| err(&format!("failed to spawn Claude CLI: {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes());
    }

    // 180s deadline like upstream; kill hard on timeout since auxiliary
    // generation has no state worth preserving.
    let (tx, rx) = std::sync::mpsc::channel();
    let pid = child.id();
    std::thread::spawn(move || {
        let out = child.wait_with_output();
        let _ = tx.send(out);
    });
    let output = match rx.recv_timeout(std::time::Duration::from_secs(180)) {
        Ok(Ok(out)) => out,
        Ok(Err(e)) => return Err(err(&format!("failed to read Claude CLI output: {e}"))),
        Err(_) => {
            let _ = unsafe { libc_kill(pid) };
            return Err(err("Claude CLI generation timed out"));
        }
    };
    if !output.status.success() {
        return Err(err(&format!(
            "Claude CLI generation failed with exit code {:?}",
            output.status.code()
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope: Value = serde_json::from_str(stdout.trim())
        .map_err(|e| err(&format!("Claude CLI returned non-JSON output: {e}")))?;
    if let Some(structured) = envelope.get("structured_output").filter(|v| v.is_object()) {
        return Ok(structured.clone());
    }
    // Raw-text fallback: parse the result text itself as JSON.
    if let Some(text) = envelope.get("result").and_then(Value::as_str) {
        if let Ok(parsed) = serde_json::from_str::<Value>(text.trim()) {
            return Ok(parsed);
        }
    }
    Err(err("Claude CLI generation produced no structured output"))
}

unsafe fn libc_kill(pid: u32) -> i32 {
    unsafe extern "C" {
        fn kill(pid: i32, sig: i32) -> i32;
    }
    unsafe { kill(pid as i32, 9) }
}

fn limit_section(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    text.chars().take(max).collect()
}

fn collapse_ws(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Upstream sanitizeThreadRecap: strip a leading `recap:` prefix, collapse
/// whitespace, fall back to the previous recap, cap at 240 chars.
fn sanitize_thread_recap(raw: &str, previous: Option<&str>) -> String {
    let trimmed = raw.trim();
    let stripped = trimmed
        .strip_prefix("recap:")
        .or_else(|| trimmed.strip_prefix("Recap:"))
        .or_else(|| trimmed.strip_prefix("RECAP:"))
        .map(str::trim)
        .unwrap_or(trimmed);
    let stripped = collapse_ws(stripped);
    let fallback = previous.map(collapse_ws).unwrap_or_default();
    let candidate = if stripped.is_empty() { fallback } else { stripped };
    if candidate.is_empty() {
        return "No meaningful recap yet.".to_string();
    }
    if candidate.chars().count() <= 240 {
        return candidate;
    }
    let clipped: String = candidate.chars().take(237).collect();
    format!("{}...", clipped.trim_end())
}

fn build_thread_recap_prompt(payload: &Value) -> String {
    let previous = payload
        .get("previousRecap")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("(none)");
    let new_material = payload
        .get("newMaterial")
        .and_then(Value::as_str)
        .unwrap_or("");
    let current_state = payload
        .get("currentState")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("(none)");
    [
        "You are writing a compact live recap for Synara's chat side panel.",
        "Return a JSON object with key: recap.",
        "Respond with only the JSON object, no prose and no code fences.",
        "Goal:",
        "Help the user quickly remember what happened in this chat, especially the latest concrete work and the current next step.",
        "",
        "Rules:",
        "- recap must be only the recap text; no title, no prefix, no bullets, no markdown",
        "- use the same language as the active chat",
        "- maximum 220 characters; prefer 150-190 characters",
        "- write one compact paragraph that fits in 3-4 narrow panel lines",
        "- mention the current work area first",
        "- prioritize recent completed changes over old context",
        "- include the next step, blocker, or pending decision if useful",
        "- ignore tool noise unless it changed the outcome",
        "- do not invent completed work, files, tests, or decisions",
        "- if there is no meaningful new information, return the previous recap unchanged",
        "",
        "Previous recap:",
        &limit_section(previous, 600),
        "",
        "New material:",
        &limit_section(new_material, 5_000),
        "",
        "Current state:",
        &limit_section(current_state, 1_500),
    ]
    .join("\n")
}


fn build_automation_intent_prompt(payload: &Value) -> String {
    let default_mode = payload
        .get("defaultMode")
        .and_then(Value::as_str)
        .unwrap_or("heartbeat");
    let now_iso = payload.get("nowIso").and_then(Value::as_str).unwrap_or("");
    let message = payload.get("message").and_then(Value::as_str).unwrap_or("");
    [
        "You extract structured Synara automation creation intents.",
        "Return a JSON object matching the requested schema.",
        "Respond with only the JSON object, no prose and no code fences.",
        "",
        "Context:",
        "- The user already invoked /automation or @automation in the chat composer.",
        "- Still set isAutomation=false if the text is only asking a question about automations or does not request a scheduled task.",
        "- Synara automations run a saved prompt on a schedule.",
        &format!("- Current timestamp for relative timers: {now_iso}."),
        "",
        "Required output fields:",
        "- isAutomation: true only when the user wants to create a scheduled automation.",
        "- confidence: number from 0 to 1.",
        "- language: detected user language, or null.",
        "- name: short automation name, <= 160 chars, or null.",
        "- taskPrompt: the detailed, self-contained recurring instruction to save, without /automation, @automation, schedule, stop, or run-count scaffolding.",
        "- Expand terse tasks into a clear saved automation prompt only using facts the user provided.",
        "- Preserve concrete user-provided workspace paths, commands, files, commit/push rules, verification steps, URLs, accounts, and constraints.",
        "- Do not invent repo-specific files, commands, services, tests, tickets, product context, credentials, or success criteria.",
        "- If the user only gave a tiny task, keep taskPrompt clear and short instead of padding it with fake details.",
        "- schedule: automation cadence, or null when missing/ambiguous.",
        "- mode: heartbeat, dedicated, or standalone.",
        "- maxIterations: positive integer only when the user explicitly says for N times/runs/iterations/volte; otherwise null.",
        "- completionPolicy: use {\"type\":\"ai-evaluated\",\"stopWhen\":\"...\",\"confidenceThreshold\":0.8} only when the user explicitly says until/stop when/if X stop/fino a quando/finche. Otherwise use {\"type\":\"none\"}.",
        "- missingFields: include schedule, taskPrompt, name, or mode when that field is null or too unclear.",
        "- needsConfirmation: true when schedule/task/mode is missing, ambiguous, or confidence < 0.75.",
        "- reason: short explanation when isAutomation=false or needsConfirmation=true; otherwise null.",
        "",
        "Task prompt quality checklist:",
        "- Objective: state the concrete recurring task.",
        "- Source of truth: keep any user-provided URLs, accounts, APIs, commands, files, or public-source constraints.",
        "- Scope: name files, directories, branches, or repositories only when the user provided them.",
        "- Procedure: preserve user-provided commands and ordered steps.",
        "- Publish rules: preserve explicit commit, push, branch, PR, or no-commit rules.",
        "- Non-goals: preserve constraints like do not use APIs, do not change architecture, and do not stage unrelated files.",
        "- Reporting: include concise output expectations when the user asked for them.",
        "",
        "Schedule rules:",
        "- For 'in N seconds/minutes/hours/days', 'tra N secondi/minuti/ore/giorni', or 'fra ...', use {\"type\":\"once\",\"runAt\":\"<ISO timestamp>\"} calculated from the current timestamp.",
        "- For 'every N seconds/minutes/hours/days' or equivalents in any language, use {\"type\":\"interval\",\"everySeconds\":N in seconds}.",
        "- Recurring intervals under 60 seconds require explicit review; keep the interval schedule, set needsConfirmation=true, and explain the fast cadence in reason.",
        "- For daily/weekdays/weekly, use HH:mm 24h timeOfDay. If the user gives no time, use 09:00.",
        "- For weekly, dayOfWeek is 0=Sunday, 1=Monday, ... 6=Saturday.",
        "- Do not invent a cadence or relative base time. If time is missing, approximate, or ambiguous, schedule=null and missingFields includes schedule.",
        "",
        "Mode rules:",
        &format!("- Default mode is {default_mode}."),
        "- heartbeat means continue/report in the current thread on each run.",
        "- dedicated means the automation gets one thread of its own and every run continues it, so each run sees what the previous ones did.",
        "- standalone means every run starts a brand new thread with no history.",
        "- Use the default unless the user clearly asks for the other behavior.",
        "- When the user wants work outside the current thread, prefer dedicated over standalone unless they explicitly want each run isolated.",
        "- Stop clauses work in every mode; never downgrade completionPolicy because of the chosen mode.",
        "",
        "User message:",
        &limit_section(message, 16_000),
    ]
    .join("\n")
}

fn automation_intent_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "isAutomation": {"type": "boolean"},
            "confidence": {"type": "number", "minimum": 0, "maximum": 1},
            "language": {"type": ["string", "null"]},
            "name": {"type": ["string", "null"]},
            "taskPrompt": {"type": ["string", "null"]},
            "schedule": {"type": ["object", "null"]},
            "mode": {"type": ["string", "null"], "enum": ["heartbeat", "dedicated", "standalone", null]},
            "maxIterations": {"type": ["integer", "null"]},
            "completionPolicy": {"type": "object"},
            "missingFields": {"type": "array", "items": {"type": "string"}},
            "needsConfirmation": {"type": "boolean"},
            "reason": {"type": ["string", "null"]},
        },
        "required": ["isAutomation", "confidence", "missingFields", "needsConfirmation"],
    })
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "server.generateThreadRecap" => {
            if payload
                .get("newMaterial")
                .and_then(Value::as_str)
                .is_none_or(|s| s.is_empty())
            {
                return Err(err("server.generateThreadRecap requires newMaterial"));
            }
            let prompt = build_thread_recap_prompt(payload);
            let schema = json!({
                "type": "object",
                "properties": {"recap": {"type": "string"}},
                "required": ["recap"],
                "additionalProperties": false,
            });
            let model = payload
                .get("textGenerationModel")
                .and_then(Value::as_str)
                .or_else(|| payload.pointer("/textGenerationModelSelection/model").and_then(Value::as_str));
            let structured = claude_json_generate(&prompt, &schema, model)?;
            let raw = structured
                .get("recap")
                .and_then(Value::as_str)
                .ok_or_else(|| err("Claude generation did not return a recap string"))?;
            let recap = sanitize_thread_recap(
                raw,
                payload.get("previousRecap").and_then(Value::as_str),
            );
            Ok(json!({"recap": recap}))
        }
        "server.generateAutomationIntent" => {
            if payload
                .get("message")
                .and_then(Value::as_str)
                .is_none_or(|s| s.is_empty())
            {
                return Err(err("server.generateAutomationIntent requires a message"));
            }
            let prompt = build_automation_intent_prompt(payload);
            let schema = automation_intent_schema();
            let model = payload
                .get("textGenerationModel")
                .and_then(Value::as_str)
                .or_else(|| payload.pointer("/textGenerationModelSelection/model").and_then(Value::as_str));
            let structured = claude_json_generate(&prompt, &schema, model)?;
            let is_automation = structured
                .get("isAutomation")
                .and_then(Value::as_bool)
                .ok_or_else(|| err("Claude generation did not return isAutomation"))?;
            let mut result = structured;
            // Contract defaults: maxIterations/completionPolicy fill in when absent.
            if !result.as_object().is_some_and(|m| m.contains_key("maxIterations")) {
                result["maxIterations"] = Value::Null;
            }
            if !result.as_object().is_some_and(|m| m.contains_key("completionPolicy")) {
                result["completionPolicy"] = json!({"type": "none"});
            }
            result["isAutomation"] = json!(is_automation);
            Ok(result)
        }
        _ => Err(err("unhandled text generation tag")),
    }
}
