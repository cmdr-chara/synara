//! Factory Droid local session-history import, port of upstream
//! `provider/FactorySessionHistory.ts`. Droid persists one JSONL transcript
//! per workspace/session below `~/.factory/sessions`; this reader validates
//! the session id, keeps only user-visible user/assistant messages, and
//! filters model-only rows.

use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FactorySessionMessage {
    pub id: String,
    pub role: String,
    pub text: String,
    pub timestamp: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FactorySessionHistory {
    pub session_id: String,
    pub cwd: Option<String>,
    pub messages: Vec<FactorySessionMessage>,
}

fn valid_session_id(session_id: &str) -> bool {
    !session_id.is_empty()
        && session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn find_session_path(sessions_dir: &Path, session_id: &str) -> Option<PathBuf> {
    if !valid_session_id(session_id) {
        return None;
    }
    for entry in std::fs::read_dir(sessions_dir).ok()?.flatten() {
        if entry.file_type().map(|kind| !kind.is_dir()).unwrap_or(true) {
            continue;
        }
        let candidate = entry.path().join(format!("{session_id}.jsonl"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn visible_message_text(content: Option<&Value>) -> String {
    let Some(content) = content else { return String::new() };
    if let Some(text) = content.as_str() {
        return text.trim().to_string();
    }
    content
        .as_array()
        .map(|parts| {
            parts
                .iter()
                .filter(|part| part.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n\n")
                .trim()
                .to_string()
        })
        .unwrap_or_default()
}

fn user_visible(message: &Value) -> bool {
    message.get("isUserVisible").and_then(Value::as_bool) != Some(false)
        && message.get("visibility").and_then(Value::as_str) != Some("llm_only")
}

pub(crate) fn read_factory_session_history(home_dir: &Path, session_id: &str) -> Option<FactorySessionHistory> {
    let session_id = session_id.trim();
    let path = find_session_path(&home_dir.join(".factory").join("sessions"), session_id)?;
    let raw = std::fs::read_to_string(path).ok()?;
    let mut cwd = None;
    let mut messages = Vec::new();
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<Value>(line) else { continue };
        match entry.get("type").and_then(Value::as_str) {
            Some("session_start") => {
                if let Some(value) = entry.get("cwd").and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty()) {
                    cwd = Some(value.to_string());
                }
            }
            Some("message") => {
                let Some(message) = entry.get("message") else { continue };
                if !user_visible(message) {
                    continue;
                }
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
                messages.push(FactorySessionMessage {
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
    Some(FactorySessionHistory {
        session_id: session_id.to_string(),
        cwd,
        messages,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_user_visible_messages_and_metadata() {
        let root = std::env::temp_dir().join(format!("factory-import-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let session_dir = root.join(".factory/sessions/workspace");
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("sess-abc.jsonl"), [
            r#"{"type":"session_start","cwd":"/tmp/project"}"#,
            r#"{"type":"message","id":"m1","timestamp":"2026-01-01T00:00:01.000Z","message":{"role":"user","content":[{"type":"text","text":"hello"}]}}"#,
            r#"{"type":"message","id":"hidden","message":{"role":"assistant","visibility":"llm_only","content":"hidden"}}"#,
            r#"{"type":"message","id":"m2","timestamp":"2026-01-01T00:00:02.000Z","message":{"role":"assistant","content":"answer"}}"#,
        ].join("\n")).unwrap();
        let history = read_factory_session_history(&root, "sess-abc").unwrap();
        assert_eq!(history.cwd.as_deref(), Some("/tmp/project"));
        assert_eq!(history.messages.len(), 2);
        assert_eq!(history.messages[0].text, "hello");
        assert_eq!(history.messages[1].role, "assistant");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_path_escape_ids() {
        assert!(read_factory_session_history(Path::new("/tmp"), "../escape").is_none());
    }
}
