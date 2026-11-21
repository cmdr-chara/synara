//! The Rust port of `packages/shared/src/codexStderr.ts`.

#![forbid(unsafe_code)]

use std::sync::OnceLock;

use regex::Regex;
use serde_json::{Map, Value, json};

use crate::js;

const SYNARA_MCP_SERVER_NAME: &str = "synara";
const MCP_CONNECTOR_SERVER_NAME_SUFFIX: &str = "_connector";
const MULTI_AGENT_DEPRECATION: &str = "`[features].multi_agent` is deprecated. It still controls the V1 fallback. `[agents].enabled = false` disables subagent tools even when the model selects a subagent version, unless `[features].multi_agent_v2` is enabled.";
const BENIGN_ERRORS: [&str; 2] = [
    "state db missing rollout path for thread",
    "state db record_discrepancy: find_thread_path_by_id_str_in_subdir, falling_back",
];

// JS regex atoms are inlined: `.` became `[^\n\r\u{2028}\u{2029}]`, `\s`/`\S`
// became the JS whitespace set (incl. \u{feff}, excl. \u{0085}).

macro_rules! regex {
    ($name:ident, $pat:expr) => {
        fn $name() -> &'static Regex {
            static R: OnceLock<Regex> = OnceLock::new();
            R.get_or_init(|| Regex::new($pat).expect("valid regex"))
        }
    };
}

regex!(
    log_line,
    r"^([0-9]{4}-[0-9]{2}-[0-9]{2}T[^\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]+)[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]+(TRACE|DEBUG|INFO|WARN|ERROR)[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]+([^\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]+):[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]+([^\n\r\u{2028}\u{2029}]*)$"
);
regex!(
    error_like,
    r"(?i)(error|failed|fatal|panic|exception|denied|refused)"
);
regex!(token_usage, r"(?i)^(?:\^C)?Token usage:");
regex!(level_name, r"^(TRACE|DEBUG|INFO|WARN|ERROR)$");
regex!(
    quoted_response,
    r#"UnexpectedServerResponse\(("(?:[^"\\]|\\[^\n\r\u{2028}\u{2029}])*")\)$"#
);
regex!(
    http_error_body,
    r#"HTTP 403: (\{[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*"error"[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*:[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*"(?:[^"\\]|\\[^\n\r\u{2028}\u{2029}])*"[\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*\})"#
);
regex!(
    subscription_prefix,
    r"^(?:subscription_inactive|subscription_required): "
);
regex!(
    duplicate_field,
    r"(?i)failed to parse function arguments: duplicate field `([^`]+)`"
);
regex!(
    oauth_server,
    r"\bcodex_rmcp_client::oauth[^\t-\r \u{a0}\u{1680}\u{2000}-\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}]*:[^\n\r\u{2028}\u{2029}]*?\bfor server ([A-Za-z0-9_.\-]+)"
);

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}'
            | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// `stripTerminalControlSequences`: drops CSI and OSC sequences. Index-based so a
/// failed (unterminated) sequence leaves its text to be rescanned, like the regex.
fn strip_terminal_control_sequences(s: &str) -> String {
    let v: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < v.len() {
        let is_csi = v[i] == '\u{9b}' || (v[i] == '\u{1b}' && v.get(i + 1) == Some(&'['));
        let is_osc = v[i] == '\u{9d}' || (v[i] == '\u{1b}' && v.get(i + 1) == Some(&']'));
        let body = if v[i] == '\u{1b}' { i + 2 } else { i + 1 };
        if is_csi {
            let mut j = body;
            while j < v.len() && ('0'..='?').contains(&v[j]) {
                j += 1;
            }
            while j < v.len() && (' '..='/').contains(&v[j]) {
                j += 1;
            }
            if j < v.len() && ('@'..='~').contains(&v[j]) {
                i = j + 1;
            } else {
                out.push(v[i]);
                i += 1;
            }
            continue;
        }
        if is_osc {
            let mut j = body;
            let mut end = None;
            while j < v.len() {
                match v[j] {
                    '\u{7}' | '\u{9c}' => {
                        end = Some(j + 1);
                        break;
                    }
                    '\u{1b}' => {
                        if v.get(j + 1) == Some(&'\\') {
                            end = Some(j + 2);
                        }
                        break;
                    }
                    _ => j += 1,
                }
            }
            match end {
                Some(e) => i = e,
                None => {
                    out.push(v[i]);
                    i += 1;
                }
            }
            continue;
        }
        out.push(v[i]);
        i += 1;
    }
    out
}
fn js_trim(s: &str) -> String {
    s.trim_matches(is_js_space).to_string()
}

fn js_trim_start(s: &str) -> &str {
    s.trim_start_matches(is_js_space)
}

#[derive(Clone)]
struct LogEntry {
    level: String,
    target: String,
    message: String,
}

fn read_log(line: &str) -> Option<LogEntry> {
    if let Some(m) = log_line().captures(line) {
        return Some(LogEntry {
            level: m[2].to_string(),
            target: m[3].to_string(),
            message: m[4].to_string(),
        });
    }
    if !line.starts_with('{') {
        return None;
    }
    let log: Value = serde_json::from_str(line).ok()?;
    let fields = log.get("fields");
    let field = |k: &str| {
        fields
            .and_then(|f| f.get(k))
            .filter(|v| !v.is_null() && !js::is_undefined(v))
    };
    let message = field("message")
        .or_else(|| field("error"))
        .and_then(Value::as_str);
    let level = log.get("level").and_then(Value::as_str);
    let target = log.get("target").and_then(Value::as_str);
    let timestamp = log.get("timestamp").and_then(Value::as_str);
    match (timestamp, level, target, message) {
        (Some(_), Some(level), Some(target), Some(message)) if level_name().is_match(level) => {
            Some(LogEntry {
                level: level.to_string(),
                target: target.to_string(),
                message: message.to_string(),
            })
        }
        _ => None,
    }
}

fn is_user_configured_mcp_server(server: &str) -> bool {
    server != SYNARA_MCP_SERVER_NAME && !server.ends_with(MCP_CONNECTOR_SERVER_NAME_SUFFIX)
}

/// Port of `normalizeCodexProcessLine` (`packages/shared/src/codexStderr.ts:19`).
pub fn normalize_codex_process_line(line: String) -> Result<String, String> {
    Ok(js_trim(&strip_terminal_control_sequences(&line)))
}

/// Port of `isIgnorableCodexProcessLine` (`packages/shared/src/codexStderr.ts:23`).
pub fn is_ignorable_codex_process_line(line: String) -> Result<bool, String> {
    Ok(line.is_empty() || token_usage().is_match(&line))
}

/// Port of `isCodexUnstableFeaturesWarning` (`packages/shared/src/codexStderr.ts:28`).
pub fn is_codex_unstable_features_warning(message: String) -> Result<bool, String> {
    Ok(
        js_trim_start(&message).starts_with("Under-development features enabled:")
            && message.contains("suppress_unstable_features_warning = true"),
    )
}

/// Port of `isCodexMultiAgentDeprecationWarning` (`packages/shared/src/codexStderr.ts:36`).
pub fn is_codex_multi_agent_deprecation_warning(message: String) -> Result<bool, String> {
    Ok(message == MULTI_AGENT_DEPRECATION)
}

/// Port of `isCodexUserMcpOAuthStartupError` (`packages/shared/src/codexStderr.ts:47`).
pub fn is_codex_user_mcp_o_auth_startup_error(
    server: String,
    error: String,
) -> Result<bool, String> {
    Ok(is_user_configured_mcp_server(&server)
        && error
            == format!(
                "The {server} MCP server requires OAuth reauthentication. Run `codex mcp login {server}`."
            ))
}

/// Port of `readCodexMcpSubscriptionError` (`packages/shared/src/codexStderr.ts:56`).
pub fn read_codex_mcp_subscription_error(message: String) -> Result<Option<String>, String> {
    let mut message = message.as_str();
    let mut parsed = None;
    if let Some(m) = quoted_response().captures(message) {
        let quoted = &m[1];
        match serde_json::from_str::<Value>(quoted) {
            Ok(v) => {
                parsed = v.as_str().map(|s| s.to_string());
                if parsed.is_none() {
                    return Ok(None);
                }
            }
            Err(_) => return Ok(None),
        }
    }
    if let Some(p) = &parsed {
        message = p;
    }
    let Some(bm) = http_error_body().captures(message) else {
        return Ok(None);
    };
    let body: Value = match serde_json::from_str(&bm[1]) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    let error = body.get("error").and_then(Value::as_str);
    match error {
        Some(e) if subscription_prefix().is_match(e) => Ok(Some(e.to_string())),
        _ => Ok(None),
    }
}

/// Port of `normalizeCodexUserVisibleErrorMessage` (`packages/shared/src/codexStderr.ts:100`).
pub fn normalize_codex_user_visible_error_message(raw_message: String) -> Result<String, String> {
    let line = js_trim(&strip_terminal_control_sequences(&raw_message));
    let message = if line.starts_with('{') {
        read_log(&line).map(|l| l.message).unwrap_or(line)
    } else {
        line
    };
    if let Some(d) = duplicate_field().captures(&message) {
        return Ok(format!(
            "Tool call failed because the same argument was sent twice ({}).",
            &d[1]
        ));
    }
    Ok(message)
}

/// Port of `classifyCodexStderr` (`packages/shared/src/codexStderr.ts:113`).
pub fn classify_codex_stderr(
    line: String,
    mirrored_user_config_path: Option<String>,
    options: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let line = js_trim(&strip_terminal_control_sequences(&line));
    if line.is_empty() || token_usage().is_match(&line) {
        return Ok(Value::Null);
    }
    let log = read_log(&line);
    let unstable_msg = log.as_ref().map(|l| l.message.as_str()).unwrap_or(&line);
    if js_trim_start(unstable_msg).starts_with("Under-development features enabled:")
        && unstable_msg.contains("suppress_unstable_features_warning = true")
    {
        return Ok(Value::Null);
    }
    if let Some(l) = &log {
        if let Some(path) = mirrored_user_config_path
            .as_deref()
            .filter(|p| !p.is_empty())
            && l.target == "codex_app_server"
            && l.message.starts_with(&format!(
                "Ignored unsupported project-local config keys in {path}: "
            ))
            && l.message.ends_with(
                "If you want these settings to apply, manually set them in your user-level config.toml.",
            )
        {
            return Ok(Value::Null);
        }
        if l.level != "ERROR" || BENIGN_ERRORS.iter().any(|b| l.message.contains(b)) {
            return Ok(Value::Null);
        }
        let joined = format!("{}: {}", l.target, l.message);
        if let Some(m) = oauth_server().captures(&joined) {
            let server = &m[1];
            if is_user_configured_mcp_server(server) {
                return Ok(json!({"kind": "user-mcp-oauth", "server": server}));
            }
        }
    }
    let only_flag = options
        .as_ref()
        .and_then(|o| o.get("errorLikePlainLinesOnly"))
        .map(|v| {
            !v.is_null()
                && !js::is_undefined(v)
                && match v {
                    Value::Bool(b) => *b,
                    Value::Number(n) => n.as_f64() != Some(0.0),
                    Value::String(s) => !s.is_empty(),
                    _ => true,
                }
        })
        .unwrap_or(false);
    if log.is_none() && only_flag && !error_like().is_match(&line) {
        return Ok(Value::Null);
    }
    let sub_error = match &log {
        Some(l)
            if l.target == "rmcp::transport::worker"
                && l.message.starts_with(
                    "worker quit with fatal: Transport channel closed, when UnexpectedServerResponse(",
                ) =>
        {
            read_codex_mcp_subscription_error(l.message.clone())?
        }
        _ => None,
    };
    let message = normalize_codex_user_visible_error_message(line)?;
    let mut obj = Map::new();
    obj.insert("kind".into(), json!("notice"));
    obj.insert("message".into(), Value::String(message));
    if let Some(l) = &log
        && l.target == "codex_app_server"
        && l.message == MULTI_AGENT_DEPRECATION
    {
        obj.insert("message".into(), Value::String(l.message.clone()));
        obj.insert("configWarning".into(), json!(true));
    }
    if let Some(e) = sub_error {
        obj.insert("mcpSubscriptionError".into(), Value::String(e));
    }
    Ok(Value::Object(obj))
}
