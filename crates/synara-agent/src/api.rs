use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use synara_core::{
    AgentCapabilities, AgentIdentity, AuthMethod, ConfigValue, ConnectionId, ConnectionState,
    SessionConfiguration, ThreadEvent, ThreadId,
};
use synara_runtime::{ExecutionHost, LaunchSpec, RuntimeError};
use tokio::sync::{mpsc, watch};

pub type AgentResult<T> = Result<T, AgentError>;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("invalid agent operation: {0}")]
    Invalid(String),
    #[error("agent operation is not supported: {0}")]
    Unsupported(String),
    #[error("agent request failed ({code}): {message}")]
    Remote { code: i64, message: String },
    #[error("agent authentication is required")]
    AuthenticationRequired,
    #[error("agent transport disconnected: {0}")]
    Disconnected(String),
    #[error("agent operation timed out")]
    Timeout,
    #[error("agent operation was cancelled")]
    Cancelled,
    #[error("the session already has an active operation")]
    Busy,
    #[error("agent resource limit reached")]
    Limit,
    #[error("conversation event could not be delivered")]
    EventDelivery,
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
}

#[derive(Clone, Debug)]
pub struct AgentSpec {
    pub id: String,
    pub name: String,
    pub origin: String,
    pub launch: LaunchSpec,
}
impl AgentSpec {
    pub fn validate(&self) -> AgentResult<()> {
        if self.id.is_empty()
            || self.id.len() > 128
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            || self.name.is_empty()
        {
            return Err(AgentError::Invalid("invalid agent identity".into()));
        }
        self.launch.validate()?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub id: ConnectionId,
    pub state: ConnectionState,
    pub identity: Option<AgentIdentity>,
    pub capabilities: AgentCapabilities,
    pub authentication: Vec<AuthMethod>,
    pub host: String,
    pub error: Option<String>,
}

/// The inspector receives an already-redacted representation, never raw wire buffers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceEntry {
    pub sequence: u64,
    pub timestamp_ms: u64,
    pub direction: String,
    pub kind: String,
    pub request_id: Option<String>,
    pub method: Option<String>,
    pub shape: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ContextServer {
    Process {
        name: String,
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
    },
    Http {
        name: String,
        url: String,
        headers: BTreeMap<String, String>,
    },
    ServerSentEvents {
        name: String,
        url: String,
        headers: BTreeMap<String, String>,
    },
}

#[derive(Clone, Debug)]
pub struct SessionOptions {
    pub thread_id: ThreadId,
    pub cwd: PathBuf,
    pub additional_directories: Vec<PathBuf>,
    pub context_servers: Vec<ContextServer>,
}
impl SessionOptions {
    pub fn new(thread_id: ThreadId, cwd: PathBuf) -> Self {
        Self {
            thread_id,
            cwd,
            additional_directories: Vec::new(),
            context_servers: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RestoreMode {
    ReplayHistory,
    ResumeWithoutReplay,
}

#[derive(Clone, Debug)]
pub enum PromptPart {
    Text(String),
    Image {
        base64: String,
        mime_type: String,
    },
    Audio {
        base64: String,
        mime_type: String,
    },
    Context {
        uri: String,
        text: String,
        mime_type: String,
    },
}
#[derive(Clone, Debug)]
pub struct Prompt {
    pub parts: Vec<PromptPart>,
}
impl Prompt {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            parts: vec![PromptPart::Text(text.into())],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub cwd: Option<String>,
    pub title: Option<String>,
    pub updated_at: Option<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SessionPage {
    pub sessions: Vec<SessionSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AgentEvent {
    pub thread_id: ThreadId,
    pub event: ThreadEvent,
}

#[async_trait]
pub trait EventSink: Send + Sync {
    /// Success means ownership passed to an ordered, lossless event consumer.
    async fn emit(&self, thread_id: ThreadId, event: ThreadEvent) -> AgentResult<()>;
}
pub struct ChannelEvents(pub mpsc::Sender<AgentEvent>);
#[async_trait]
impl EventSink for ChannelEvents {
    async fn emit(&self, thread_id: ThreadId, event: ThreadEvent) -> AgentResult<()> {
        self.0
            .send(AgentEvent { thread_id, event })
            .await
            .map_err(|_| AgentError::EventDelivery)
    }
}

#[derive(Clone)]
pub struct ConnectionContext {
    pub host: Arc<dyn ExecutionHost>,
    pub cwd: PathBuf,
    pub events: Arc<dyn EventSink>,
    pub interactions: Arc<dyn crate::InteractionHandler>,
}

#[async_trait]
pub trait AgentBackend: Send + Sync {
    async fn connect(
        &self,
        spec: &AgentSpec,
        context: ConnectionContext,
    ) -> AgentResult<Arc<dyn AgentConnection>>;
}

#[async_trait]
pub trait AgentConnection: Send + Sync {
    fn info(&self) -> ConnectionInfo;
    fn observe(&self) -> watch::Receiver<ConnectionInfo>;
    fn trace(&self) -> Vec<TraceEntry> {
        Vec::new()
    }
    fn clear_trace(&self) {}
    async fn new_session(&self, options: SessionOptions) -> AgentResult<Arc<dyn AgentSession>>;
    async fn restore_session(
        &self,
        _id: &str,
        _options: SessionOptions,
        _mode: RestoreMode,
    ) -> AgentResult<Arc<dyn AgentSession>> {
        Err(AgentError::Unsupported("session restore".into()))
    }
    async fn list_sessions(
        &self,
        _cwd: Option<String>,
        _cursor: Option<String>,
    ) -> AgentResult<SessionPage> {
        Err(AgentError::Unsupported("session listing".into()))
    }
    async fn delete_session(&self, _id: &str) -> AgentResult<()> {
        Err(AgentError::Unsupported("session deletion".into()))
    }
    async fn authenticate(&self, _method: &str) -> AgentResult<()> {
        Err(AgentError::Unsupported("authentication".into()))
    }
    async fn logout(&self) -> AgentResult<()> {
        Err(AgentError::Unsupported("logout".into()))
    }
    async fn disconnect(&self) -> AgentResult<()>;
}

#[async_trait]
pub trait AgentSession: Send + Sync {
    fn id(&self) -> &str;
    fn thread_id(&self) -> ThreadId;
    fn configuration(&self) -> SessionConfiguration;
    async fn prompt(&self, prompt: Prompt) -> AgentResult<String>;
    async fn cancel(&self) -> AgentResult<()>;
    async fn set_option(
        &self,
        _id: &str,
        _value: ConfigValue,
    ) -> AgentResult<SessionConfiguration> {
        Err(AgentError::Unsupported("session configuration".into()))
    }
    async fn set_mode(&self, _id: &str) -> AgentResult<()> {
        Err(AgentError::Unsupported("session modes".into()))
    }
    async fn set_model(&self, _id: &str) -> AgentResult<()> {
        Err(AgentError::Unsupported("model selection".into()))
    }
    async fn close(&self) -> AgentResult<()>;
}
