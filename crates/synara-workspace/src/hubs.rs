//! Optional shared work context. Hub identity is the existing project identity,
//! while tasks, files, agent sessions and permissions retain their existing owners.
//!
//! Upstream parity: a Hub corresponds to an upstream studio project; its only
//! carried content is project-scoped instructions, which upstream seeds into a
//! new thread's notes (EnvironmentProjectInstructionsSection +
//! mergeProjectInstructionsIntoThreadNotes). The earlier local invention —
//! description, curated memory, include-toggle and composer-draft seeding — has
//! no upstream counterpart and is dropped; legacy v1 rows migrate by folding
//! memory into instructions so no user text is lost.
use crate::{WorkspaceError, WorkspaceResult};
use serde::{Deserialize, Serialize};
use synara_core::ProjectId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HubProfile {
    pub version: u32,
    pub revision: u64,
    pub project: ProjectId,
    pub name: String,
    /// Project-scoped instructions, seeded into new threads' notes.
    pub instructions: String,
}
impl HubProfile {
    pub const CURRENT_VERSION: u32 = 2;
    pub const MAX_INSTRUCTIONS_BYTES: usize = 32 * 1024;
    pub fn new(project: ProjectId, name: String) -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            revision: 0,
            project,
            name,
            instructions: String::new(),
        }
    }
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.version != Self::CURRENT_VERSION
            || self.name.trim().is_empty()
            || self.name.len() > 160
            || self.name.chars().any(char::is_control)
            || self.instructions.len() > Self::MAX_INSTRUCTIONS_BYTES
            || self.instructions.contains('\0')
        {
            return Err(WorkspaceError::Invalid(
                "Hub name or instructions are invalid or exceed their limit.".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct HubSummary {
    pub profile: HubProfile,
    pub threads: usize,
    /// Legacy Studio is represented without rewriting any task or file identity.
    pub imported: bool,
}

/// Version-1 rows carried extra invented fields. Decode them once and fold the
/// curated `memory` text into `instructions` so upgrading loses no user content.
#[derive(Deserialize)]
struct HubProfileV1 {
    version: u32,
    revision: u64,
    project: ProjectId,
    name: String,
    #[serde(default)]
    instructions: String,
    #[serde(default)]
    memory: String,
}
pub(crate) fn decode_hub_profile(raw: &str) -> WorkspaceResult<HubProfile> {
    if let Ok(profile) = serde_json::from_str::<HubProfile>(raw) {
        return Ok(profile);
    }
    let legacy: HubProfileV1 = serde_json::from_str(raw).map_err(crate::StorageError::from)?;
    if legacy.version != 1 {
        return Err(crate::StorageError::Identity.into());
    }
    let mut instructions = legacy.instructions;
    let memory = legacy.memory.trim();
    if !memory.is_empty() {
        if !instructions.trim().is_empty() {
            instructions.push_str("\n\n");
        }
        instructions.push_str(memory);
    }
    Ok(HubProfile {
        version: HubProfile::CURRENT_VERSION,
        revision: legacy.revision,
        project: legacy.project,
        name: legacy.name,
        instructions,
    })
}
