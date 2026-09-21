use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
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
    /// Agent startup may use a private directory without changing session workspace scope.
    pub launch_directory: Option<PathBuf>,
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
        if self
            .launch_directory
            .as_ref()
            .is_some_and(|path| !path.is_absolute())
        {
            return Err(AgentError::Invalid(
                "agent startup directory must be absolute".into(),
            ));
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

const MAX_INSPECTOR_ENTRIES: usize = 512;
const MAX_INSPECTOR_FIELD_BYTES: usize = 4096;
const MAX_INSPECTOR_EXPORT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectorHostKind {
    Local,
    Remote,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InspectorIdentity {
    pub name: String,
    pub title: Option<String>,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectorConnection {
    pub id: ConnectionId,
    pub state: ConnectionState,
    pub identity: Option<InspectorIdentity>,
    pub capabilities: AgentCapabilities,
    pub authentication_method_count: usize,
    pub host: InspectorHostKind,
    pub process_id: Option<u32>,
    pub error_present: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InspectorSnapshot {
    pub generated_at_ms: u64,
    pub connection: InspectorConnection,
    pub entries: Vec<TraceEntry>,
    pub truncated: bool,
    pub privacy_notice: String,
}

fn bounded_inspector_text(value: &str) -> String {
    if value.len() <= MAX_INSPECTOR_FIELD_BYTES && !value.chars().any(char::is_control) {
        value.to_owned()
    } else {
        "<redacted>".into()
    }
}

fn sanitize_trace(mut entries: Vec<TraceEntry>) -> (Vec<TraceEntry>, bool) {
    let truncated = entries.len() > MAX_INSPECTOR_ENTRIES;
    if truncated {
        entries.drain(..entries.len() - MAX_INSPECTOR_ENTRIES);
    }
    for entry in &mut entries {
        entry.direction = bounded_inspector_text(&entry.direction);
        entry.kind = bounded_inspector_text(&entry.kind);
        entry.request_id = entry
            .request_id
            .take()
            .map(|value| bounded_inspector_text(&value));
        entry.method = entry
            .method
            .take()
            .map(|value| bounded_inspector_text(&value));
        entry.shape = bounded_inspector_text(&entry.shape);
    }
    (entries, truncated)
}

#[derive(Clone, Serialize, Deserialize)]
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

// Resolved credentials may cross this in-memory protocol boundary. They must not
// leak through SessionOptions/transport debug diagnostics.
impl std::fmt::Debug for ContextServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, name) = match self {
            Self::Process { name, .. } => ("Process", name),
            Self::Http { name, .. } => ("Http", name),
            Self::ServerSentEvents { name, .. } => ("ServerSentEvents", name),
        };
        f.debug_struct(kind).field("name", name).field("configuration", &"[REDACTED]").finish()
    }
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
    fn process_id(&self) -> Option<u32> {
        None
    }
    fn inspector_snapshot(&self) -> InspectorSnapshot {
        let info = self.info();
        let (entries, truncated) = sanitize_trace(self.trace());
        let identity = info.identity.map(|identity| InspectorIdentity {
            name: bounded_inspector_text(&identity.name),
            title: identity.title.as_deref().map(bounded_inspector_text),
            version: bounded_inspector_text(&identity.version),
        });
        InspectorSnapshot {
            generated_at_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
            connection: InspectorConnection {
                id: info.id,
                state: info.state,
                identity,
                capabilities: info.capabilities,
                authentication_method_count: info.authentication.len(),
                host: if info.host == "Local" {
                    InspectorHostKind::Local
                } else {
                    InspectorHostKind::Remote
                },
                process_id: self.process_id(),
                error_present: info.error.is_some(),
            },
            entries,
            truncated,
            privacy_notice: "Protocol payloads and stderr contents are excluded. Conversation or file-content exports are separate and may contain sensitive user data.".into(),
        }
    }
    fn export_inspector_json(&self) -> AgentResult<Vec<u8>> {
        let encoded = serde_json::to_vec_pretty(&self.inspector_snapshot())
            .map_err(|_| AgentError::Invalid("inspector export could not be encoded".into()))?;
        if encoded.len() > MAX_INSPECTOR_EXPORT_BYTES {
            return Err(AgentError::Limit);
        }
        Ok(encoded)
    }
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

#[cfg(test)]
mod inspector_tests {
    use super::*;

    struct FixtureConnection {
        info: watch::Sender<ConnectionInfo>,
        trace: Vec<TraceEntry>,
    }

    #[async_trait]
    impl AgentConnection for FixtureConnection {
        fn info(&self) -> ConnectionInfo {
            self.info.borrow().clone()
        }

        fn observe(&self) -> watch::Receiver<ConnectionInfo> {
            self.info.subscribe()
        }

        fn trace(&self) -> Vec<TraceEntry> {
            self.trace.clone()
        }

        fn process_id(&self) -> Option<u32> {
            Some(42)
        }

        async fn new_session(
            &self,
            _options: SessionOptions,
        ) -> AgentResult<Arc<dyn AgentSession>> {
            Err(AgentError::Unsupported("fixture".into()))
        }

        async fn disconnect(&self) -> AgentResult<()> {
            Ok(())
        }
    }

    fn fixture(entries: usize) -> FixtureConnection {
        let (info, _) = watch::channel(ConnectionInfo {
            id: ConnectionId::new(),
            state: ConnectionState::Failed,
            identity: Some(AgentIdentity {
                name: "fixture".into(),
                title: Some("Fixture".into()),
                version: "1.2.3".into(),
            }),
            capabilities: AgentCapabilities {
                load_session: true,
                ..AgentCapabilities::default()
            },
            authentication: vec![AuthMethod {
                id: "login".into(),
                name: "Login".into(),
                description: Some("secret-canary-description".into()),
            }],
            host: "SSH private-user@example.invalid:22".into(),
            error: Some("secret-canary-error".into()),
        });
        FixtureConnection {
            info,
            trace: (0..entries)
                .map(|sequence| TraceEntry {
                    sequence: sequence as u64 + 1,
                    timestamp_ms: sequence as u64,
                    direction: "in".into(),
                    kind: "notification".into(),
                    request_id: None,
                    method: Some("session/update".into()),
                    shape: "none".into(),
                })
                .collect(),
        }
    }

    #[test]
    fn inspector_export_is_bounded_and_excludes_raw_connection_errors() {
        let connection = fixture(600);
        let snapshot = connection.inspector_snapshot();
        assert_eq!(snapshot.entries.len(), MAX_INSPECTOR_ENTRIES);
        assert!(snapshot.truncated);
        assert_eq!(snapshot.connection.host, InspectorHostKind::Remote);
        assert_eq!(snapshot.connection.process_id, Some(42));
        assert!(snapshot.connection.error_present);
        assert_eq!(snapshot.connection.authentication_method_count, 1);
        assert_eq!(
            snapshot
                .connection
                .identity
                .as_ref()
                .map(|identity| identity.version.as_str()),
            Some("1.2.3")
        );
        let encoded = String::from_utf8(connection.export_inspector_json().unwrap()).unwrap();
        assert!(!encoded.contains("secret-canary"));
        assert!(!encoded.contains("private-user"));
        assert!(encoded.contains("session/update"));
        assert!(encoded.contains("Protocol payloads"));
    }
}
