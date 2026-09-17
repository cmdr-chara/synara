use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    time::{SystemTime, UNIX_EPOCH},
};
use synara_agent::TraceEntry;

/// Payload values are never retained. Debug export cannot leak prompts, tokens or file contents.
pub(crate) struct TraceLog {
    entries: VecDeque<TraceEntry>,
    next: u64,
    capacity: usize,
}
impl Default for TraceLog {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            next: 1,
            capacity: 512,
        }
    }
}
impl TraceLog {
    pub fn push(&mut self, direction: &str, value: &Value) {
        let kind = if value.get("method").is_some() {
            if value.get("id").is_some() {
                "request"
            } else {
                "notification"
            }
        } else if value.get("error").is_some() {
            "error"
        } else {
            "response"
        };
        let request_id = value.get("id").map(|id| {
            let digest = Sha256::digest(id.to_string().as_bytes());
            format!("id:{}", hex::encode(&digest[..6]))
        });
        let method = value
            .get("method")
            .and_then(Value::as_str)
            .map(|method| match method {
                "initialize"
                | "authenticate"
                | "logout"
                | "session/new"
                | "session/load"
                | "session/resume"
                | "session/close"
                | "session/list"
                | "session/delete"
                | "session/prompt"
                | "session/cancel"
                | "session/update"
                | "session/set_mode"
                | "session/set_model"
                | "session/set_config_option"
                | "session/request_permission"
                | "fs/read_text_file"
                | "fs/write_text_file"
                | "terminal/create"
                | "terminal/output"
                | "terminal/wait_for_exit"
                | "terminal/kill"
                | "terminal/release"
                | "elicitation/create"
                | "elicitation/complete" => method.into(),
                _ => "<extension method>".into(),
            });
        let payload = value
            .get("params")
            .or_else(|| value.get("result"))
            .or_else(|| value.get("error"));
        let shape = payload.map_or_else(|| "none".into(), |value| describe(value, 0));
        self.record(direction, kind, request_id, method, shape);
    }
    pub fn stderr(&mut self, bytes: usize) {
        self.record(
            "agent",
            "stderr",
            None,
            None,
            format!("{bytes} bytes captured, content redacted"),
        );
    }
    pub fn lifecycle(&mut self, state: &str) {
        self.record("host", "lifecycle", None, None, state.into());
    }
    fn record(
        &mut self,
        direction: &str,
        kind: &str,
        request_id: Option<String>,
        method: Option<String>,
        shape: String,
    ) {
        if self.entries.len() >= self.capacity {
            self.entries.pop_front();
        }
        self.entries.push_back(TraceEntry {
            sequence: self.next,
            timestamp_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
            direction: direction.into(),
            kind: kind.into(),
            request_id,
            method,
            shape,
        });
        self.next = self.next.saturating_add(1);
    }
    pub fn snapshot(&self) -> Vec<TraceEntry> {
        self.entries.iter().cloned().collect()
    }
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
fn describe(value: &Value, depth: usize) -> String {
    if depth > 2 {
        return "...".into();
    }
    match value {
        Value::Null => "null".into(),
        Value::Bool(_) => "boolean".into(),
        Value::Number(_) => "number".into(),
        Value::String(text) => format!("string({} bytes)", text.len()),
        Value::Array(items) => format!("array({})", items.len()),
        Value::Object(object) => {
            // Keys may themselves be user-supplied secrets. Only schema-owned keys are shown.
            let names: Vec<_> = object
                .iter()
                .take(32)
                .map(|(key, value)| {
                    let name = match key.as_str() {
                        "sessionId"
                        | "protocolVersion"
                        | "clientCapabilities"
                        | "clientInfo"
                        | "agentCapabilities"
                        | "agentInfo"
                        | "authMethods"
                        | "cwd"
                        | "mcpServers"
                        | "additionalDirectories"
                        | "prompt"
                        | "stopReason"
                        | "update"
                        | "content"
                        | "toolCall"
                        | "options"
                        | "outcome"
                        | "action"
                        | "configOptions"
                        | "modes"
                        | "models"
                        | "code"
                        | "message"
                        | "data"
                        | "terminalId"
                        | "path"
                        | "text" => key.as_str(),
                        _ => "<key>",
                    };
                    format!("{name}: {}", describe(value, depth + 1))
                })
                .collect();
            format!("{{{}}}", names.join(", "))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn secrets_in_values_keys_and_extension_methods_are_not_retained() {
        let mut log = TraceLog::default();
        log.push("in", &json!({"id":"secret-token", "method":"secret-method", "params":{"secret-key":"secret-value", "prompt":"secret-prompt"}}));
        log.stderr(40);
        let encoded = serde_json::to_string(&log.snapshot()).unwrap();
        assert!(!encoded.contains("secret"));
        assert!(encoded.contains("content redacted"));
    }
    #[test]
    fn history_is_bounded_and_clear_does_not_reuse_sequence_numbers() {
        let mut log = TraceLog::default();
        for _ in 0..600 {
            log.lifecycle("connected");
        }
        assert_eq!(log.snapshot().len(), 512);
        assert_eq!(log.snapshot()[0].sequence, 89);
        log.clear();
        log.lifecycle("disconnected");
        assert_eq!(log.snapshot()[0].sequence, 601);
    }
}
