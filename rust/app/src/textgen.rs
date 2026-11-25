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
    tag == "server.generateThreadRecap"
}

/// Run one JSON-mode generation and return the structured output object.
fn claude_json_generate(prompt: &str, schema: &Value, model: Option<&str>) -> Result<Value, WsErr> {
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
        _ => Err(err("unhandled text generation tag")),
    }
}
