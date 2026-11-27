//! Oh My Pi local session-history import, port of upstream
//! `provider/OmpSessionHistory.ts`. OMP persists one JSONL transcript per
//! workspace/session; this reader validates the session id, discovers the
//! file below the account's agent dir, keeps only user-visible user/assistant
//! messages, and strips Synara's injected host-context block from prompts.

use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OmpSessionMessage {
    pub id: String,
    pub role: String,
    pub text: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OmpSessionHistory {
    pub session_id: String,
    pub cwd: Option<String>,
    pub last_model: Option<String>,
    pub last_thinking_level: Option<String>,
    pub messages: Vec<OmpSessionMessage>,
}

fn valid_session_id(session_id: &str) -> bool {
    !session_id.is_empty()
        && session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn env_value<'a>(environment: &'a [(String, String)], key: &str) -> Option<&'a str> {
    environment
        .iter()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn expand_home(path: &str, home: &str) -> PathBuf {
    if path == "~" {
        return PathBuf::from(home);
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return Path::new(home).join(rest);
    }
    PathBuf::from(path)
}

/// Resolve the OMP profile directory for the selected instance, mirroring
/// `resolveOmpAgentDir`: configured `agentDir`, then PI_CODING_AGENT_DIR,
/// then `<HOME>/<PI_CONFIG_DIR|.omp>/agent` in that account's environment.
pub(crate) fn resolve_omp_agent_dir(instance_id: Option<&str>) -> PathBuf {
    let settings = crate::server_settings::current();
    let instance = crate::provider_instances::resolve_instance(&settings, instance_id, Some("omp"));
    let environment = instance
        .as_ref()
        .map(|instance| instance.environment.clone())
        .unwrap_or_default();
    let env_or_process = |key: &str| -> Option<String> {
        env_value(&environment, key)
            .map(str::to_string)
            .or_else(|| std::env::var(key).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty()))
    };
    let home = env_or_process("HOME")
        .or_else(|| env_or_process("USERPROFILE"))
        .unwrap_or_else(|| std::env::var("HOME").unwrap_or_default());
    let configured = instance
        .as_ref()
        .and_then(|instance| instance.config.get("agentDir"))
        .or_else(|| settings.pointer("/providers/omp/agentDir"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(configured) = configured {
        return expand_home(configured, &home);
    }
    if let Some(dir) = env_or_process("PI_CODING_AGENT_DIR") {
        return expand_home(&dir, &home);
    }
    let config_dir = env_or_process("PI_CONFIG_DIR").unwrap_or_else(|| ".omp".to_string());
    Path::new(&home).join(config_dir).join("agent")
}

fn find_session_path(sessions_dir: &Path, session_id: &str) -> Option<PathBuf> {
    if !valid_session_id(session_id) {
        return None;
    }
    for workspace in std::fs::read_dir(sessions_dir).ok()?.flatten() {
        if workspace.file_type().map(|kind| !kind.is_dir()).unwrap_or(true) {
            continue;
        }
        let dir = workspace.path();
        for entry in std::fs::read_dir(&dir).ok()?.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type().map(|kind| kind.is_file()).unwrap_or(false)
                && name.ends_with(&format!("_{session_id}.jsonl"))
            {
                return Some(entry.path());
            }
        }
    }
    None
}

fn strip_host_context(text: &str) -> String {
    const START: &str = "<synara_host_context>";
    const END: &str = "</synara_host_context>";
    let mut rest = text;
    let mut out = String::new();
    while let Some(start) = rest.find(START) {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + START.len()..];
        if let Some(end) = after_start.find(END) {
            rest = &after_start[end + END.len()..];
        } else {
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out
}

fn visible_message_text(content: Option<&Value>) -> String {
    let Some(content) = content else { return String::new() };
    let joined = if let Some(text) = content.as_str() {
        text.to_string()
    } else if let Some(parts) = content.as_array() {
        parts
            .iter()
            .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n\n")
    } else {
        String::new()
    };
    strip_host_context(&joined).trim().to_string()
}

pub(crate) fn read_omp_session_history(agent_dir: &Path, session_id: &str) -> Option<OmpSessionHistory> {
    let session_id = session_id.trim();
    let path = find_session_path(&agent_dir.join("sessions"), session_id)?;
    let raw = std::fs::read_to_string(path).ok()?;
    let mut cwd = None;
    let mut last_model = None;
    let mut last_thinking_level = None;
    let mut messages = Vec::new();
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<Value>(line) else { continue };
        match entry.get("type").and_then(Value::as_str) {
            Some("session") => {
                if let Some(value) = entry.get("cwd").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
                    cwd = Some(value.to_string());
                }
            }
            Some("model_change") => {
                if let Some(value) = entry.get("model").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
                    last_model = Some(value.to_string());
                }
            }
            Some("thinking_level_change") => {
                if let Some(value) = entry.get("thinkingLevel").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
                    last_thinking_level = Some(value.to_string());
                }
            }
            Some("message") => {
                let Some(message) = entry.get("message") else { continue };
                let role = message.get("role").and_then(Value::as_str).unwrap_or("");
                if role != "user" && role != "assistant" {
                    continue;
                }
                let text = visible_message_text(message.get("content"));
                if text.is_empty() {
                    continue;
                }
                let id = entry
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| messages.len().to_string());
                messages.push(OmpSessionMessage {
                    id,
                    role: role.to_string(),
                    text,
                    timestamp: entry
                        .get("timestamp")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_string),
                });
            }
            _ => {}
        }
    }
    Some(OmpSessionHistory {
        session_id: session_id.to_string(),
        cwd,
        last_model,
        last_thinking_level,
        messages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_history(dir: &Path, session_id: &str, lines: &[&str]) {
        let session_dir = dir.join("sessions").join("-tmp-project");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(
            session_dir.join(format!("2026-01-01T00-00-00_{session_id}.jsonl")),
            lines.join("\n"),
        )
        .unwrap();
    }

    #[test]
    fn reads_only_visible_messages_and_session_metadata() {
        let root = std::env::temp_dir().join(format!("omp-import-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        write_history(&root, "sess-abc", &[
            r#"{"type":"session","cwd":"/tmp/project"}"#,
            r#"{"type":"model_change","model":"gpt-5"}"#,
            r#"{"type":"thinking_level_change","thinkingLevel":"high"}"#,
            r#"{"type":"message","id":"m1","timestamp":"2026-01-01T00:00:01.000Z","message":{"role":"user","content":[{"type":"text","text":"hello"},{"type":"tool","text":"hidden"}]}}"#,
            r#"{"type":"message","id":"tool","message":{"role":"tool","content":"hidden"}}"#,
            r#"{"type":"message","id":"m2","timestamp":"2026-01-01T00:00:02.000Z","message":{"role":"assistant","content":"answer"}}"#,
        ]);
        let history = read_omp_session_history(&root, "sess-abc").unwrap();
        assert_eq!(history.cwd.as_deref(), Some("/tmp/project"));
        assert_eq!(history.last_model.as_deref(), Some("gpt-5"));
        assert_eq!(history.last_thinking_level.as_deref(), Some("high"));
        assert_eq!(history.messages.len(), 2);
        assert_eq!(history.messages[0].text, "hello");
        assert_eq!(history.messages[1].role, "assistant");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn strips_host_context_and_rejects_path_escape_ids() {
        assert_eq!(
            strip_host_context("before<synara_host_context>secret</synara_host_context>after"),
            "beforeafter"
        );
        assert!(read_omp_session_history(Path::new("/tmp"), "../escape").is_none());
    }
}
