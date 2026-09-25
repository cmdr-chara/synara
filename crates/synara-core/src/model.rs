use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}
id_type!(WorkspaceId);
id_type!(ProjectId);
id_type!(TaskId);
id_type!(ThreadId);
id_type!(ConnectionId);
id_type!(EventId);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceLocation {
    Local {
        root: PathBuf,
    },
    Ssh {
        host: String,
        port: u16,
        user: Option<String>,
        root: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub location: WorkspaceLocation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub relative_directory: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    #[default]
    Ready,
    Running,
    Waiting,
    Completed,
    Failed,
    Archived,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub title: String,
    pub state: TaskState,
    pub thread_id: ThreadId,
    pub agent_id: String,
    pub working_directory: PathBuf,
    pub updated_at_ms: i64,
    #[serde(default)]
    pub scope: TaskScope,
}

/// Sidebar ownership. Older native records are project conversations.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskScope {
    #[default]
    Project,
    Chat,
    Studio,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionReference {
    pub agent_id: String,
    pub remote_id: String,
    pub working_directory: PathBuf,
    pub title: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Starting,
    Initializing,
    AuthenticationRequired,
    Authenticating,
    Connected,
    Failed,
    Restarting,
    Exited,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentCapabilities {
    pub load_session: bool,
    pub resume_session: bool,
    #[serde(default)]
    pub fork_session: bool,
    pub close_session: bool,
    pub list_sessions: bool,
    pub delete_session: bool,
    pub logout: bool,
    pub image_prompts: bool,
    pub audio_prompts: bool,
    pub embedded_context: bool,
    pub mcp_http: bool,
    pub mcp_sse: bool,
    pub additional_directories: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub name: String,
    pub title: Option<String>,
    pub version: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuthMethod {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ConfigValue {
    Boolean { value: bool },
    Select { value: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SelectChoice {
    pub value: String,
    pub label: String,
    pub group: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SessionOption {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub current: ConfigValue,
    pub choices: Vec<SelectChoice>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionMode {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionConfiguration {
    pub options: Vec<SessionOption>,
    pub modes: Vec<SessionMode>,
    pub current_mode: Option<String>,
    pub models: Vec<SelectChoice>,
    pub current_model: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub argument_hint: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    Reasoning,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    #[default]
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToolOutput {
    Text {
        text: String,
    },
    Diff {
        path: String,
        before: Option<String>,
        after: Option<String>,
    },
    Terminal {
        id: String,
    },
    Resource {
        uri: String,
        name: String,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolPatch {
    pub id: String,
    pub title: Option<String>,
    pub status: Option<ToolStatus>,
    pub kind: Option<String>,
    pub output: Option<Vec<ToolOutput>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    AllowOnce,
    AllowAlways,
    DenyOnce,
    DenyAlways,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PermissionChoice {
    pub id: String,
    pub label: String,
    pub kind: PermissionKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PermissionRequest {
    pub id: String,
    pub tool_id: Option<String>,
    pub title: String,
    pub choices: Vec<PermissionChoice>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputFieldKind {
    Text {
        min_length: Option<usize>,
        max_length: Option<usize>,
        format: Option<String>,
    },
    Boolean,
    Number {
        integer: bool,
        minimum: Option<f64>,
        maximum: Option<f64>,
    },
    Choice {
        options: Vec<SelectChoice>,
    },
    MultiChoice {
        options: Vec<SelectChoice>,
        minimum: Option<usize>,
        maximum: Option<usize>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InputField {
    pub id: String,
    pub label: String,
    pub required: bool,
    pub kind: InputFieldKind,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UserInputRequest {
    pub id: String,
    pub message: String,
    pub fields: Vec<InputField>,
    pub url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InputValue {
    Text(String),
    Boolean(bool),
    Number(f64),
    Strings(Vec<String>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum UserInputResponse {
    Accept {
        values: BTreeMap<String, InputValue>,
    },
    Decline,
    Cancel,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlanEntry {
    pub text: String,
    pub status: String,
    pub priority: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub context_used: Option<u64>,
    pub context_limit: Option<u64>,
    pub cost_amount: Option<f64>,
    pub cost_currency: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TerminalRecord {
    pub text: String,
    pub truncated: bool,
    pub exit_code: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ThreadEvent {
    HistoryStarted,
    HistoryCompleted,
    CancellationRequested,
    TerminalOutput {
        id: String,
        text: String,
        truncated: bool,
        exit_code: Option<u32>,
    },
    PromptStarted {
        turn: String,
    },
    TextDelta {
        message_id: Option<String>,
        role: Role,
        text: String,
    },
    ImageMessage {
        message_id: Option<String>,
        role: Role,
        image: crate::TranscriptImage,
    },
    ToolChanged {
        patch: ToolPatch,
    },
    PermissionRequested {
        request: PermissionRequest,
    },
    PermissionResolved {
        id: String,
        selected: Option<String>,
    },
    UserInputRequested {
        request: UserInputRequest,
    },
    UserInputResolved {
        id: String,
    },
    PlanChanged {
        entries: Vec<PlanEntry>,
    },
    UsageChanged {
        usage: Usage,
    },
    DirectModelRoute {
        provider_id: String,
        model_id: String,
    },
    AcpTurnRoute {
        turn: String,
        agent_id: String,
        model_id: Option<String>,
    },
    ConfigurationChanged {
        configuration: SessionConfiguration,
    },
    CommandsChanged {
        commands: Vec<SlashCommand>,
    },
    TitleChanged {
        title: String,
    },
    PromptFinished {
        reason: String,
    },
    SessionStatus {
        status: String,
    },
    ContextCompaction {
        message: String,
    },
    Error {
        message: String,
        recoverable: bool,
    },
    Notice {
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: EventId,
    pub thread_id: ThreadId,
    pub sequence: u64,
    pub timestamp_ms: i64,
    pub event: ThreadEvent,
}
