//! Optional shared work context. Hub identity is the existing project identity,
//! while tasks, files, agent sessions and permissions retain their existing owners.
use crate::{WorkspaceError, WorkspaceResult};
use serde::{Deserialize, Serialize};
use synara_core::{ProjectId, TaskId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubProfile {
    pub version: u32,
    pub revision: u64,
    pub project: ProjectId,
    pub name: String,
    pub description: String,
    pub instructions: String,
    /// User-maintained knowledge, never automatically harvested from transcripts.
    pub memory: String,
    pub include_in_new_threads: bool,
    pub archived: bool,
    pub main_task: TaskId,
}
impl HubProfile {
    pub fn new(project: ProjectId, main_task: TaskId, name: String) -> Self {
        Self {
            version: 1,
            revision: 0,
            project,
            main_task,
            name,
            description: String::new(),
            instructions: String::new(),
            memory: String::new(),
            include_in_new_threads: true,
            archived: false,
        }
    }
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1
            || self.name.trim().is_empty()
            || self.name.len() > 160
            || self.name.chars().any(char::is_control)
            || self.description.len() > 2048
            || self.instructions.len() > 32 * 1024
            || self.memory.len() > 64 * 1024
            || [&self.description, &self.instructions, &self.memory]
                .iter()
                .any(|text| text.contains('\0'))
        {
            return Err(WorkspaceError::Invalid(
                "Hub name or context is invalid or exceeds its limit.".into(),
            ));
        }
        Ok(())
    }
    /// A visible, editable initial draft. Never a hidden prompt or automatic send.
    pub fn context_draft(&self) -> String {
        let mut text = String::new();
        if !self.instructions.trim().is_empty() {
            text.push_str("## Hub instructions\n\n");
            text.push_str(&self.instructions);
            text.push_str("\n\n");
        }
        if !self.memory.trim().is_empty() {
            text.push_str("## Shared Hub knowledge\n\n");
            text.push_str(&self.memory);
            text.push_str("\n\n");
        }
        if !text.is_empty() {
            text.push_str("---\n\nTask:\n");
        }
        text
    }
}
#[derive(Clone, Debug)]
pub struct HubSummary {
    pub profile: HubProfile,
    pub threads: usize,
    /// Legacy Studio is represented without rewriting any task or file identity.
    pub imported: bool,
}
