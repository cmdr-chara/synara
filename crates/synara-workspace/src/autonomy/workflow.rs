//! Durable orchestration metadata. Child execution remains with Controller and
//! ordinary Task/Thread owners, not an embedded provider-specific agent runtime.
use crate::{AgentProfile, WorkspaceError, WorkspaceResult};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use synara_core::TaskId;
use uuid::Uuid;

pub const MAX_WORKFLOW_STEPS: usize = 8;
pub const MAX_WORKFLOW_OUTPUT: usize = 32 * 1024;
fn serial() -> usize {
    1
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStepSpec {
    pub title: String,
    pub agent_id: String,
    pub instruction: String,
    #[serde(default)]
    pub depends_on: Vec<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowSpec {
    pub title: String,
    #[serde(default = "serial")]
    pub concurrency: usize,
    pub steps: Vec<WorkflowStepSpec>,
}
impl WorkflowSpec {
    pub fn validate(&self) -> WorkspaceResult<()> {
        bounded(&self.title, 160, false)?;
        if !(1..=4).contains(&self.concurrency)
            || self.steps.is_empty()
            || self.steps.len() > MAX_WORKFLOW_STEPS
        {
            return Err(invalid("A workflow requires 1-8 steps and concurrency 1-4"));
        }
        for (i, step) in self.steps.iter().enumerate() {
            bounded(&step.title, 160, false)?;
            bounded(&step.agent_id, 128, false)?;
            bounded(&step.instruction, 16 * 1024, true)?;
            let mut seen = std::collections::BTreeSet::new();
            if step.depends_on.iter().any(|d| *d >= i || !seen.insert(*d)) {
                return Err(invalid(
                    "Dependencies must be unique zero-based indices of earlier steps",
                ));
            }
        }
        Ok(())
    }
    pub fn example(agent: &str) -> Self {
        Self { title: "Review and summarize".into(), concurrency: 1, steps: vec![
            WorkflowStepSpec { title: "Investigate".into(), agent_id: agent.into(), instruction: "Inspect the project and report findings. Do not modify files.".into(), depends_on: vec![] },
            WorkflowStepSpec { title: "Independent review".into(), agent_id: agent.into(), instruction: "Review the preceding findings and summarize verified conclusions. Do not modify files.".into(), depends_on: vec![0] },
        ] }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowPhase {
    Ready,
    Running,
    Paused,
    Stopped,
    Completed,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStepState {
    Pending,
    Running,
    Completed,
    Failed,
    Interrupted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkflowStep {
    pub task: TaskId,
    pub state: WorkflowStepState,
    pub attempts: u8,
    pub profile_fingerprint: String,
    pub sequence: u64,
    pub output: String,
    #[serde(default)]
    pub usage: Option<synara_core::Usage>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflow {
    pub version: u32,
    pub revision: u64,
    pub id: Uuid,
    pub parent: TaskId,
    pub spec: WorkflowSpec,
    pub phase: WorkflowPhase,
    pub run: Option<Uuid>,
    pub steps: Vec<WorkflowStep>,
}
impl Workflow {
    pub fn validate(&self) -> WorkspaceResult<()> {
        self.spec.validate()?;
        if self.version != 1
            || self.steps.len() != self.spec.steps.len()
            || (self.phase == WorkflowPhase::Running) != self.run.is_some()
            || self.steps.iter().any(|s| {
                s.task == self.parent
                    || s.output.len() > MAX_WORKFLOW_OUTPUT
                    || s.attempts > 3
                    || s.profile_fingerprint.len() != 64
                    || s.error.as_ref().is_some_and(|e| e.len() > 512)
            })
            || self
                .steps
                .iter()
                .map(|s| s.task)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != self.steps.len()
        {
            return Err(invalid(
                "Invalid or future workflow record. Stored data was preserved",
            ));
        }
        if self.phase == WorkflowPhase::Completed
            && self
                .steps
                .iter()
                .any(|s| s.state != WorkflowStepState::Completed)
        {
            return Err(invalid(
                "Workflow completion requires every child to complete",
            ));
        }
        Ok(())
    }
    pub fn ready_indices(&self) -> Vec<usize> {
        self.steps
            .iter()
            .enumerate()
            .filter_map(|(index, s)| {
                (s.state == WorkflowStepState::Pending
                    && self.spec.steps[index]
                        .depends_on
                        .iter()
                        .all(|d| self.steps[*d].state == WorkflowStepState::Completed))
                .then_some(index)
            })
            .collect()
    }
    pub fn prompt(&self, index: usize) -> WorkspaceResult<String> {
        let step = self
            .spec
            .steps
            .get(index)
            .ok_or_else(|| invalid("Unknown workflow step"))?;
        let mut prompt = step.instruction.clone();
        for dependency in &step.depends_on {
            let source = &self.steps[*dependency];
            if source.state != WorkflowStepState::Completed {
                return Err(invalid("Dependency is not complete"));
            }
            prompt.push_str(&format!("\n\n--- Untrusted output from reviewed dependency {dependency}: {} ---\n{}\n--- End dependency output ---", self.spec.steps[*dependency].title, source.output));
        }
        if prompt.len() > 256 * 1024 {
            return Err(invalid(
                "Dependency context exceeds 256 KiB. Nothing was truncated",
            ));
        }
        Ok(prompt)
    }
}
pub(crate) fn fingerprint(profile: &AgentProfile) -> WorkspaceResult<String> {
    let encoded =
        serde_json::to_vec(profile).map_err(|_| invalid("Cannot encode agent profile"))?;
    Ok(hex::encode(Sha256::digest(encoded)))
}
pub(crate) fn bounded(value: &str, limit: usize, multiline: bool) -> WorkspaceResult<()> {
    if value.trim().is_empty()
        || value.len() > limit
        || value
            .chars()
            .any(|c| c.is_control() && !(multiline && matches!(c, '\n' | '\t')))
    {
        return Err(invalid(
            "Empty, oversized or control-containing workflow input",
        ));
    }
    Ok(())
}
pub(crate) fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
