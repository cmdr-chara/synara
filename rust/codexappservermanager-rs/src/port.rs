//! The Rust port of `apps/server/src/codexAppServerManager.ts`.
//!
//! Behavior is proven against `cases.jsonl`; helper constants were extracted from the
//! source module rather than copied as code (clean-room port).

#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use std::cell::RefCell;
use std::sync::OnceLock;

const CODEX_SPARK_MODEL: &str = "gpt-5.3-codex-spark";
const CODEX_DEFAULT_MODEL: &str = "gpt-6-astra";
const PLAN_INSTRUCTIONS: &str = include_str!("plan_instr.txt");
const DEFAULT_INSTRUCTIONS: &str = include_str!("default_instr.txt");
const HARNESS_BASE: &str = include_str!("harness_base.txt");
const HARNESS_CC: &str = include_str!("harness_cc.txt");

const UNDEF_MSG: &str = "Cannot read properties of undefined";
const NULL_MSG: &str = "Cannot read properties of null";

thread_local! {
    static LOGGED_OAUTH_SERVERS: RefCell<std::collections::HashSet<String>> =
        RefCell::new(std::collections::HashSet::new());
}

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn undefined() -> Value {
    crate::js::undefined()
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !is_undefined(v),
    }
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if is_undefined(v) {
        return None;
    }
    match v {
        Value::Object(m) => m.get(key),
        _ => None,
    }
}

fn prop_or_undef(v: &Value, key: &str) -> Value {
    prop(v, key).cloned().unwrap_or_else(undefined)
}

fn prop_err(v: &Value, key: &str) -> Result<(), String> {
    match v {
        Value::Null => Err(format!("{NULL_MSG} (reading '{key}')")),
        _ if is_undefined(v) => Err(format!("{UNDEF_MSG} (reading '{key}')")),
        _ => Ok(()),
    }
}

fn as_object(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        Value::Object(m) if !is_undefined(v) => Some(m),
        _ => None,
    }
}

fn as_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// Port of `readCodexAccountSnapshot` (`apps/server/src/codexAppServerManager.ts:498`).
pub fn read_codex_account_snapshot(response: Value) -> Result<Value, String> {
    let record = as_object(&response);
    let account = record
        .and_then(|r| r.get("account"))
        .and_then(as_object)
        .or(record);
    let account_type = account.and_then(|a| a.get("type")).and_then(as_string);

    match account_type.as_deref() {
        Some("apiKey") => Ok(serde_json::json!({
            "type": "apiKey",
            "planType": Value::Null,
            "sparkEnabled": true,
        })),
        Some("chatgpt") => {
            let plan = account.and_then(|a| a.get("planType")).cloned();
            let plan_type = match plan {
                None | Some(Value::Null) => Value::from("unknown"),
                Some(p) if is_undefined(&p) => Value::from("unknown"),
                Some(p) => p,
            };
            let disabled = matches!(plan_type.as_str(), Some("free" | "go" | "plus"));
            let mut out = Map::new();
            out.insert("type".into(), Value::from("chatgpt"));
            out.insert("planType".into(), plan_type);
            out.insert("sparkEnabled".into(), Value::from(!disabled));
            Ok(Value::Object(out))
        }
        _ => Ok(serde_json::json!({
            "type": "unknown",
            "planType": Value::Null,
            "sparkEnabled": true,
        })),
    }
}

fn spread_into(params: &mut Map<String, Value>, overrides: &Value) {
    match overrides {
        Value::Object(m) if !is_undefined(overrides) => {
            for (k, v) in m {
                params.insert(k.clone(), v.clone());
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                params.insert(i.to_string(), v.clone());
            }
        }
        Value::String(s) => {
            for (i, c) in s.chars().enumerate() {
                params.insert(i.to_string(), Value::from(c.to_string()));
            }
        }
        _ => {}
    }
}

/// Port of `buildCodexThreadOpenRequest` (`apps/server/src/codexAppServerManager.ts:734`).
pub fn build_codex_thread_open_request(input: Value) -> Result<Value, String> {
    prop_err(&input, "forkSourceThreadId")?;
    let fork = prop_or_undef(&input, "forkSourceThreadId");
    let resume = prop_or_undef(&input, "resumeThreadId");
    if truthy(&fork) && truthy(&resume) {
        return Err("A Codex session cannot resume and fork at the same time.".into());
    }
    let mut params = Map::new();
    let overrides = prop_or_undef(&input, "sessionOverrides");
    spread_into(&mut params, &overrides);
    let method = if truthy(&fork) {
        params.insert("threadId".into(), fork);
        params.insert("excludeTurns".into(), Value::from(true));
        "thread/fork"
    } else if truthy(&resume) {
        params.insert("threadId".into(), resume);
        params.insert("excludeTurns".into(), Value::from(true));
        "thread/resume"
    } else {
        params.insert("experimentalRawEvents".into(), Value::from(false));
        "thread/start"
    };
    let mut out = Map::new();
    out.insert("method".into(), Value::from(method));
    out.insert("params".into(), Value::Object(params));
    Ok(Value::Object(out))
}

/// Port of `resolveCodexThreadOpenMinimumVersion` (`apps/server/src/codexAppServerManager.ts:764`).
pub fn resolve_codex_thread_open_minimum_version(input: Value) -> Result<Value, String> {
    prop_err(&input, "runtimeMode")?;
    let mut floors: Vec<&str> = Vec::new();
    if prop_or_undef(&input, "runtimeMode").as_str() == Some("auto") {
        floors.push("0.124.0");
    }
    if prop_or_undef(&input, "threadOpenMethod").as_str() != Some("thread/start") {
        floors.push("0.125.0");
    }
    Ok(if floors.contains(&"0.125.0") {
        Value::from("0.125.0")
    } else if let Some(v) = floors.first() {
        Value::from(*v)
    } else {
        undefined()
    })
}

/// Port of `shouldWarnCodexFreshStartWithoutResume` (`apps/server/src/codexAppServerManager.ts:781`).
pub fn should_warn_codex_fresh_start_without_resume(input: Value) -> Result<bool, String> {
    prop_err(&input, "threadOpenMethod")?;
    let warn = prop_or_undef(&input, "threadOpenMethod").as_str() == Some("thread/start")
        && truthy(&prop_or_undef(&input, "previouslyBound"));
    Ok(warn)
}

/// Port of `resolveCodexModelForAccount` (`apps/server/src/codexAppServerManager.ts:844`).
pub fn resolve_codex_model_for_account(
    model: Option<String>,
    account: Value,
) -> Result<Value, String> {
    if model.as_deref() != Some(CODEX_SPARK_MODEL) {
        return Ok(model.map_or_else(undefined, Value::from));
    }
    prop_err(&account, "sparkEnabled")?;
    if truthy(&prop_or_undef(&account, "sparkEnabled")) {
        return Ok(model.map_or_else(undefined, Value::from));
    }
    Ok(Value::from(CODEX_DEFAULT_MODEL))
}

fn codex_alias(lower: &str) -> Option<&'static str> {
    Some(match lower {
        "sol" => "gpt-6-sol",
        "luna" => "gpt-6-luna",
        "astra" => "gpt-6-astra",
        "6" | "gpt-6" => "gpt-6-astra",
        "5.5" => "gpt-5.5",
        "5.4" => "gpt-5.4",
        "5.3" | "gpt-5.3" => "gpt-5.3-codex",
        "5.3-spark" | "gpt-5.3-spark" => "gpt-5.3-codex-spark",
        _ => return None,
    })
}

fn normalize_model_slug(model: &Option<String>) -> Option<String> {
    let raw = model.as_deref()?;
    let trimmed = js_trim(raw);
    if trimmed.is_empty() {
        return None;
    }
    let lower = trimmed.to_lowercase();
    Some(codex_alias(&lower).unwrap_or(trimmed).to_string())
}

/// Port of `normalizeCodexModelSlug` (`apps/server/src/codexAppServerManager.ts:878`).
pub fn normalize_codex_model_slug(
    model: Option<String>,
    preferred_id: Option<String>,
) -> Result<Value, String> {
    let Some(normalized) = normalize_model_slug(&model) else {
        return Ok(undefined());
    };
    if let Some(pref) = preferred_id
        && pref.ends_with("-codex")
        && pref != normalized
    {
        return Ok(Value::from(pref));
    }
    Ok(Value::from(normalized))
}

/// Port of `buildCodexCollaborationMode` (`apps/server/src/codexAppServerManager.ts:907`).
pub fn build_codex_collaboration_mode(input: Value) -> Result<Value, String> {
    prop_err(&input, "interactionMode")?;
    let interaction = prop_or_undef(&input, "interactionMode");
    let computer = prop_or_undef(&input, "enableComputerControl");
    if is_undefined(&interaction) && computer != true {
        return Ok(undefined());
    }
    let model_arg = prop(&input, "model")
        .and_then(|v| v.as_str())
        .map(String::from);
    let model = normalize_model_slug(&model_arg).unwrap_or_else(|| "gpt-5.3-codex".into());
    let native_mode = if interaction.as_str() == Some("plan") {
        "plan"
    } else {
        "default"
    };
    let base = if native_mode == "plan" {
        PLAN_INSTRUCTIONS
    } else {
        DEFAULT_INSTRUCTIONS
    };
    let instructions = if computer == true {
        base.replacen(HARNESS_BASE, HARNESS_CC, 1)
    } else {
        base.to_string()
    };
    let effort = match prop(&input, "effort") {
        None | Some(Value::Null) => Value::from("medium"),
        Some(v) if is_undefined(v) => Value::from("medium"),
        Some(v) => v.clone(),
    };
    let mut settings = Map::new();
    settings.insert("model".into(), Value::from(model));
    settings.insert("reasoning_effort".into(), effort);
    settings.insert("developer_instructions".into(), Value::from(instructions));
    let mut out = Map::new();
    out.insert("mode".into(), Value::from(native_mode));
    out.insert("settings".into(), Value::Object(settings));
    Ok(Value::Object(out))
}

/// Port of `parseCodexUserInputQuestions` (`apps/server/src/codexAppServerManager.ts:993`).
pub fn parse_codex_user_input_questions(payload: Option<Value>) -> Result<Value, String> {
    let questions = payload
        .as_ref()
        .and_then(|p| prop(p, "questions"))
        .and_then(|q| q.as_array());
    let Some(questions) = questions else {
        return Ok(undefined());
    };
    let mut parsed = Vec::new();
    for entry in questions {
        let Some(question) = as_object(entry) else {
            continue;
        };
        let empty = String::new();
        let get = |key: &str| -> String {
            question
                .get(key)
                .and_then(as_string)
                .map(|s| js_trim(&s).to_string())
                .unwrap_or_default()
        };
        let id = get("id");
        let header = get("header");
        let prompt = get("question");
        if id.is_empty() || header.is_empty() || prompt.is_empty() {
            continue;
        }
        let _ = &empty;
        let mut options = Vec::new();
        if let Some(opts) = question.get("options").and_then(|o| o.as_array()) {
            for opt in opts {
                let label = opt
                    .get("label")
                    .and_then(as_string)
                    .map(|s| js_trim(&s).to_string())
                    .unwrap_or_default();
                if label.is_empty() {
                    continue;
                }
                let description = opt
                    .get("description")
                    .and_then(as_string)
                    .map(|s| js_trim(&s).to_string())
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| label.clone());
                options.push(serde_json::json!({"label": label, "description": description}));
            }
        }
        let mut q = Map::new();
        q.insert("id".into(), Value::from(id));
        q.insert("header".into(), Value::from(header));
        q.insert("question".into(), Value::from(prompt));
        q.insert("options".into(), Value::Array(options));
        if question.get("multiSelect") == Some(&Value::from(true)) {
            q.insert("multiSelect".into(), Value::from(true));
        }
        parsed.push(Value::Object(q));
    }
    Ok(if parsed.is_empty() {
        undefined()
    } else {
        Value::Array(parsed)
    })
}

fn strip_terminal_control_sequences(value: &str) -> String {
    static CSI: OnceLock<regex::Regex> = OnceLock::new();
    static OSC: OnceLock<regex::Regex> = OnceLock::new();
    let csi = CSI.get_or_init(|| regex::Regex::new(r"(?:\x1B\[|\u{9B})[0-?]*[ -/]*[@-~]").unwrap());
    let osc = OSC.get_or_init(|| {
        regex::Regex::new(r"(?:\x1B\]|\u{9D})[^\u{7}\x1B\u{9C}]*(?:\u{7}|\x1B\\|\u{9C})").unwrap()
    });
    let value = csi.replace_all(value, "");
    osc.replace_all(&value, "").to_string()
}

struct Log {
    level: String,
    target: String,
    message: String,
}

fn read_log(line: &str) -> Option<Log> {
    static LOG_LINE: OnceLock<regex::Regex> = OnceLock::new();
    let re = LOG_LINE.get_or_init(|| {
        regex::Regex::new(
            r"^(\d{4}-\d{2}-\d{2}T\S+)\s+(TRACE|DEBUG|INFO|WARN|ERROR)\s+(\S+):\s+(.*)$",
        )
        .unwrap()
    });
    if let Some(m) = re.captures(line) {
        return Some(Log {
            level: m[2].to_string(),
            target: m[3].to_string(),
            message: m[4].to_string(),
        });
    }
    if !line.starts_with('{') {
        return None;
    }
    let log: Value = serde_json::from_str(line).ok()?;
    let obj = log.as_object()?;
    let message = obj
        .get("fields")
        .and_then(|f| f.get("message"))
        .or_else(|| obj.get("fields").and_then(|f| f.get("error")));
    if !obj.get("timestamp").is_some_and(|v| v.is_string())
        || !obj.get("target").is_some_and(|v| v.is_string())
        || !matches!(
            obj.get("level").and_then(|v| v.as_str()),
            Some("TRACE" | "DEBUG" | "INFO" | "WARN" | "ERROR")
        )
        || !message.is_some_and(|m| m.is_string())
    {
        return None;
    }
    Some(Log {
        level: obj["level"].as_str().unwrap().to_string(),
        target: obj["target"].as_str().unwrap().to_string(),
        message: message.unwrap().as_str().unwrap().to_string(),
    })
}

fn normalize_process_line(line: &str) -> String {
    js_trim(&strip_terminal_control_sequences(line)).to_string()
}

fn is_ignorable_process_line(line: &str) -> bool {
    if line.is_empty() {
        return true;
    }
    static TOKEN_USAGE: OnceLock<regex::Regex> = OnceLock::new();
    let re = TOKEN_USAGE.get_or_init(|| regex::Regex::new(r"(?i)^(?:\^C)?Token usage:").unwrap());
    re.is_match(line)
}

fn is_unstable_features_warning(message: &str) -> bool {
    message
        .trim_start()
        .starts_with("Under-development features enabled:")
        && message.contains("suppress_unstable_features_warning = true")
}

const MULTI_AGENT_DEPRECATION: &str = "`[features].multi_agent` is deprecated. It still controls the V1 fallback. `[agents].enabled = false` disables subagent tools even when the model selects a subagent version, unless `[features].multi_agent_v2` is enabled.";

const BENIGN_ERRORS: [&str; 2] = [
    "state db missing rollout path for thread",
    "state db record_discrepancy: find_thread_path_by_id_str_in_subdir, falling_back",
];

fn is_user_configured_mcp_server(server: &str) -> bool {
    server != "synara" && !server.ends_with("_connector")
}

fn read_mcp_subscription_error(message: &str) -> Option<String> {
    static QUOTED: OnceLock<regex::Regex> = OnceLock::new();
    let quoted = QUOTED.get_or_init(|| {
        regex::Regex::new(r#"UnexpectedServerResponse\(("(?:[^"\\]|\\.)*")\)$"#).unwrap()
    });
    let mut message = message.to_string();
    if let Some(c) = quoted.captures(&message) {
        match serde_json::from_str::<String>(&c[1]) {
            Ok(parsed) => message = parsed,
            Err(_) => return None,
        }
    }
    static BODY: OnceLock<regex::Regex> = OnceLock::new();
    let body_re = BODY.get_or_init(|| {
        regex::Regex::new(r#"HTTP 403: (\{\s*"error"\s*:\s*"(?:[^"\\]|\\.)*"\s*\})"#).unwrap()
    });
    let body = body_re.captures(&message)?[1].to_string();
    let parsed: Value = serde_json::from_str(&body).ok()?;
    let error = parsed.get("error")?.as_str()?;
    if error.starts_with("subscription_inactive: ") || error.starts_with("subscription_required: ")
    {
        Some(error.to_string())
    } else {
        None
    }
}

fn normalize_user_visible_error_message(raw: &str) -> String {
    let line = normalize_process_line(raw);
    let message = if line.starts_with('{') {
        read_log(&line).map(|l| l.message).unwrap_or(line)
    } else {
        line
    };
    static DUP: OnceLock<regex::Regex> = OnceLock::new();
    let dup = DUP.get_or_init(|| {
        regex::Regex::new(r"(?i)failed to parse function arguments: duplicate field `([^`]+)`")
            .unwrap()
    });
    if let Some(c) = dup.captures(&message) {
        format!(
            "Tool call failed because the same argument was sent twice ({}).",
            &c[1]
        )
    } else {
        message
    }
}

fn classify_codex_stderr(
    line: &str,
    mirrored: Option<&str>,
) -> Option<(String, bool, Option<String>, Option<String>)> {
    let line = normalize_process_line(line);
    if is_ignorable_process_line(&line) {
        return None;
    }
    let log = read_log(&line);
    if is_unstable_features_warning(log.as_ref().map(|l| l.message.as_str()).unwrap_or(&line)) {
        return None;
    }
    let mut oauth_server = None;
    if let Some(l) = &log {
        if let Some(mirrored) = mirrored
            && l.target == "codex_app_server"
            && l.message
                .starts_with(&format!("Ignored unsupported project-local config keys in {mirrored}: "))
            && l.message.ends_with(
                "If you want these settings to apply, manually set them in your user-level config.toml.",
            )
        {
            return None;
        }
        if l.level != "ERROR" || BENIGN_ERRORS.iter().any(|s| l.message.contains(s)) {
            return None;
        }
        static OAUTH: OnceLock<regex::Regex> = OnceLock::new();
        let oauth = OAUTH.get_or_init(|| {
            regex::Regex::new(r"\bcodex_rmcp_client::oauth\S*:.*?\bfor server ([A-Za-z0-9_.-]+)")
                .unwrap()
        });
        let combined = format!("{}: {}", l.target, l.message);
        if let Some(c) = oauth.captures(&combined)
            && is_user_configured_mcp_server(&c[1])
        {
            oauth_server = Some(c[1].to_string());
        }
    }
    if let Some(server) = oauth_server {
        return Some((String::new(), false, None, Some(server)));
    }
    if log.is_none() {
        static ERROR_LIKE: OnceLock<regex::Regex> = OnceLock::new();
        let re = ERROR_LIKE.get_or_init(|| {
            regex::Regex::new(r"(?i)error|failed|fatal|panic|exception|denied|refused").unwrap()
        });
        if !re.is_match(&line) {
            return None;
        }
    }
    let subscription = log.as_ref().and_then(|l| {
        if l.target == "rmcp::transport::worker"
            && l.message.starts_with(
                "worker quit with fatal: Transport channel closed, when UnexpectedServerResponse(",
            )
        {
            read_mcp_subscription_error(&l.message)
        } else {
            None
        }
    });
    let deprecation = log
        .as_ref()
        .is_some_and(|l| l.target == "codex_app_server" && l.message == MULTI_AGENT_DEPRECATION);
    let message = if deprecation {
        log.as_ref().unwrap().message.clone()
    } else {
        normalize_user_visible_error_message(&line)
    };
    Some((message, deprecation, subscription, None))
}

/// Port of `classifyCodexStderrLine` (`apps/server/src/codexAppServerManager.ts:1038`).
pub fn classify_codex_stderr_line(
    raw_line: String,
    mirrored_user_config_path: Option<String>,
) -> Result<Value, String> {
    let Some((message, config_warning, subscription, oauth_server)) =
        classify_codex_stderr(&raw_line, mirrored_user_config_path.as_deref())
    else {
        return Ok(Value::Null);
    };
    if let Some(server) = oauth_server {
        LOGGED_OAUTH_SERVERS.with(|s| {
            s.borrow_mut().insert(server);
        });
        return Ok(Value::Null);
    }
    let mut out = Map::new();
    out.insert("message".into(), Value::from(message));
    if config_warning {
        out.insert("configWarning".into(), Value::from(true));
    }
    if let Some(sub) = subscription {
        out.insert("mcpSubscriptionError".into(), Value::from(sub));
    }
    Ok(Value::Object(out))
}

/// Port of `readResumeCursorThreadId` (`apps/server/src/codexAppServerManager.ts:5995`).
pub fn read_resume_cursor_thread_id(resume_cursor: Value) -> Result<Value, String> {
    if is_undefined(&resume_cursor) {
        return Ok(undefined());
    }
    let Value::Object(m) = &resume_cursor else {
        return Ok(undefined());
    };
    let Some(raw) = m.get("threadId").and_then(|v| v.as_str()) else {
        return Ok(undefined());
    };
    let trimmed = js_trim(raw);
    Ok(if trimmed.is_empty() {
        undefined()
    } else {
        Value::from(trimmed)
    })
}
