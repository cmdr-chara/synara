//! Direct model inference. This crate has no ACP, agent-process, Git or tool-execution owner.
//! Catalog metadata describes capabilities, not authorization or verified account access.
mod catalog;
mod config;
mod google;
mod protocol;
mod schema;
mod stream;
mod transport;
pub use catalog::*;
pub use config::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use transport::*;

pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("Invalid direct-model configuration: {0}")]
    Invalid(&'static str),
    #[error("Unsupported or unreported model capability: {0}")]
    Unsupported(&'static str),
    #[error("The OS credential store is unavailable, locked, or has no key for this endpoint")]
    Credential,
    #[error("Provider output does not match the reviewed JSON schema")]
    SchemaMismatch,
    #[error("Provider stopped generation without a complete answer")]
    Incomplete,
    #[error("The provider returned HTTP {0}. No automatic retry was made")]
    Http(u16),
    #[error(
        "The provider connection failed or timed out. The request may have been processed. No automatic retry was made"
    )]
    Transport,
    #[error("Invalid or incomplete provider stream")]
    Protocol,
    #[error("Direct-model input or output exceeded its safety limit")]
    Limit,
    #[error("Direct-model request stopped. Remote processing may already have occurred")]
    Cancelled,
}
pub type ModelResult<T> = Result<T, ModelError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Content {
    Text {
        text: String,
    },
    /// Inline images only. No provider-initiated URL or filesystem fetches.
    Image {
        media_type: String,
        base64: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub role: MessageRole,
    pub content: Vec<Content>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
}
impl Message {
    pub fn text(role: MessageRole, text: String) -> Self {
        Self {
            role,
            content: vec![Content::Text { text }],
            tool_calls: vec![],
            tool_call_id: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutputFormat {
    #[default]
    Text,
    JsonSchema {
        name: String,
        schema: Value,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub tools: Vec<ToolDefinition>,
    #[serde(default)]
    pub output: OutputFormat,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    pub max_output_tokens: u32,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelUsage {
    /// Total input tokens, including cache reads and writes when reported.
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ModelEvent {
    Text(String),
    Reasoning(String),
    /// A proposal, never an executed tool. The caller owns approval and execution.
    ToolCall(ToolCall),
    Usage(ModelUsage),
    Finished {
        reason: String,
    },
}

pub(crate) fn bounded_identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}
pub(crate) fn tool_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[cfg(test)]
mod tests;
