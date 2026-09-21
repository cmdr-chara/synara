//! User-owned integrations. Repository configuration is never discovered or trusted
//! implicitly. Provider extensions remain outside this catalog and its authority.
use crate::{WorkspaceError, WorkspaceResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use synara_core::TaskId;
use synara_runtime::SecretReference;

mod skills;
pub use skills::SkillReview;
pub(crate) mod probe;
pub use probe::{McpProbeReport, McpToolInfo};

pub const MAX_SKILL_BYTES: usize = 64 * 1024;
pub(crate) const INTEGRATIONS_KEY: &str = "integrations";

pub(crate) fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
fn text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
pub(crate) fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}
fn identity(id: &str) -> bool { uuid::Uuid::parse_str(id).is_ok() }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillOrigin {
    /// Explicitly selected local document, not a verified publisher identity.
    pub path: String,
    pub sha256: String,
    pub version: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledSkill {
    pub id: String,
    pub title: String,
    pub description: String,
    pub origin: SkillOrigin,
    pub previous_origins: Vec<SkillOrigin>,
    pub markdown: String,
    /// Enables explicit insertion into a draft, never automatic prompt injection.
    pub enabled: bool,
}
impl InstalledSkill {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if !identity(&self.id) || !text(&self.title, 160)
            || self.description.len() > 1024 || self.description.chars().any(char::is_control)
            || self.markdown.trim().is_empty() || self.markdown.len() > MAX_SKILL_BYTES
            || self.markdown.chars().any(|c| c.is_control() && !matches!(c, '\n'|'\r'|'\t'))
            || digest(&self.markdown) != self.origin.sha256 || self.previous_origins.len() > 8
        { return Err(invalid("Invalid skill document or checksum. The stored library was not replaced.")); }
        for origin in std::iter::once(&self.origin).chain(&self.previous_origins) {
            if !text(&origin.path, 8192) || !std::path::Path::new(&origin.path).is_absolute()
                || origin.sha256.len() != 64 || !origin.sha256.bytes().all(|c| c.is_ascii_hexdigit())
                || origin.version.as_ref().is_some_and(|v| !text(v, 128)) {
                return Err(invalid("Invalid skill origin receipt."));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedMcp {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub task: TaskId,
    /// Exact profile ID, not a guessed vendor compatibility list.
    pub agent_id: String,
    pub enabled: bool,
    /// Optional bearer credential reference. Never a token or an arbitrary header.
    pub bearer: Option<SecretReference>,
}
impl ManagedMcp {
    pub fn new(task: TaskId, agent_id: String) -> Self {
        Self {id:uuid::Uuid::new_v4().to_string(),name:String::new(),endpoint:String::new(),task,agent_id,enabled:false,bearer:None}
    }
    pub fn validate(&self) -> WorkspaceResult<()> {
        if !identity(&self.id) || !text(&self.name, 160) || !text(&self.agent_id, 128) {
            return Err(invalid("A connection needs a valid ID, name and agent profile."));
        }
        endpoint(&self.endpoint)?;
        if let Some(reference) = &self.bearer { reference.validate()?; }
        Ok(())
    }
    pub fn applies_to(&self, task: TaskId, agent: &str) -> bool {
        self.enabled && self.task == task && self.agent_id == agent
    }
}
/// Deliberately disallow URL credentials, query tokens, fragments, redirects and
/// non-loopback cleartext. Localhost DNS is not a substitute for literal loopback.
pub(crate) fn endpoint(value: &str) -> WorkspaceResult<url::Url> {
    if value.len() > 4096 || value.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(invalid("Invalid MCP endpoint."));
    }
    let url = url::Url::parse(value).map_err(|_| invalid("Enter an absolute MCP HTTP endpoint."))?;
    let loopback = match url.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    if !url.username().is_empty() || url.password().is_some() || url.query().is_some()
        || url.fragment().is_some() || url.host().is_none()
        || !(url.scheme() == "https" || (url.scheme() == "http" && loopback)) {
        return Err(invalid("Use HTTPS, or HTTP on literal loopback. URL credentials, queries and fragments are forbidden. Use a credential-store reference for bearer authentication."));
    }
    Ok(url)
}
#[derive(Clone, Debug)]
pub enum McpEdit {
    /// Adding or editing always disables the connection until a separate action.
    Save(ManagedMcp),
    SetEnabled { id: String, enabled: bool },
    Remove(String),
}
impl McpEdit {
    pub(crate) fn target(&self, value: &IntegrationSettings) -> WorkspaceResult<TaskId> {
        match self {
            Self::Save(item) => { item.validate()?; Ok(item.task) },
            Self::SetEnabled { id,.. } | Self::Remove(id) => value.mcp.iter()
                .find(|item| &item.id == id).map(|item|item.task).ok_or(WorkspaceError::NotFound),
        }
    }
    pub(crate) fn apply(self, value: &mut IntegrationSettings) -> WorkspaceResult<()> {
        match self {
            Self::Save(mut item) => {
                item.validate()?;
                item.enabled = false;
                item.endpoint = endpoint(&item.endpoint)?.to_string();
                if let Some(old) = value.mcp.iter_mut().find(|old| old.id == item.id) {
                    if old.task != item.task || old.agent_id != item.agent_id {
                        return Err(invalid("Connection ownership cannot be changed. Add a separately reviewed connection for the new task or agent."));
                    }
                    *old = item;
                } else { value.mcp.push(item); }
            },
            Self::SetEnabled { id, enabled } => {
                value.mcp.iter_mut().find(|item| item.id == id)
                    .ok_or(WorkspaceError::NotFound)?.enabled = enabled;
            },
            Self::Remove(id) => {
                let index = value.mcp.iter().position(|item| item.id == id).ok_or(WorkspaceError::NotFound)?;
                value.mcp.remove(index);
            },
        }
        value.validate()
    }
}
#[derive(Clone, Debug)]
pub enum SkillEdit {
    SetEnabled { id: String, enabled: bool },
    Remove(String),
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrationSettings {
    pub version: u32,
    pub revision: u64,
    pub skills: Vec<InstalledSkill>,
    pub mcp: Vec<ManagedMcp>,
}
impl Default for IntegrationSettings {
    fn default() -> Self { Self { version: 1, revision: 0, skills: vec![], mcp: vec![] } }
}
impl IntegrationSettings {
    pub fn validate(&self) -> WorkspaceResult<()> {
        let mut ids = HashSet::new();
        if self.version != 1 || self.skills.len() > 48 || self.mcp.len() > 32 {
            return Err(invalid("Unsupported or oversized integrations catalog. The saved value was not replaced."));
        }
        for item in &self.skills {
            item.validate()?;
            if !ids.insert(&item.id) { return Err(invalid("Duplicate integration identity.")); }
        }
        let mut scopes = HashSet::new();
        for item in &self.mcp {
            item.validate()?;
            if !scopes.insert((item.task, &item.agent_id, &item.name)) { return Err(invalid("Duplicate MCP name in the same task and agent scope.")); }
            if !ids.insert(&item.id) { return Err(invalid("Duplicate integration identity.")); }
        }
        Ok(())
    }
}
