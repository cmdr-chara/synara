//! Durable automation definitions and scheduling, separate from transcript state.
//! Loading records never executes them. The scheduler must be explicitly armed
//! each process session. Claims and owned conversation creation are atomic.
mod scheduler;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService, now_ms};
pub use scheduler::AutomationScheduler;
use serde::{Deserialize, Serialize};
use synara_core::{ProjectId, TaskId};
pub use uuid::Uuid as AutomationId;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationSchedule {
    Interval { minutes: u32 },
    Daily { hour: u8, minute: u8 },
}
impl AutomationSchedule {
    pub fn parse(value: &str) -> WorkspaceResult<Self> {
        let value = value.trim();
        let result = if let Some(minutes) = value
            .strip_prefix("every ")
            .and_then(|s| s.strip_suffix('m'))
            .and_then(|s| s.parse().ok())
        {
            Self::Interval { minutes }
        } else if let Some((hour, minute)) =
            value.strip_prefix("daily ").and_then(|s| s.split_once(':'))
        {
            Self::Daily {
                hour: hour.parse().map_err(|_| invalid("Invalid daily hour."))?,
                minute: minute
                    .parse()
                    .map_err(|_| invalid("Invalid daily minute."))?,
            }
        } else {
            return Err(invalid("Use 'every 60m' or 'daily 09:00'."));
        };
        result.validate()?;
        Ok(result)
    }
    pub fn label(&self) -> String {
        match self {
            Self::Interval { minutes } => format!("every {minutes}m"),
            Self::Daily { hour, minute } => format!("daily {hour:02}:{minute:02}"),
        }
    }
    pub fn validate(&self) -> WorkspaceResult<()> {
        match self {
            Self::Interval { minutes: 1..=10080 }
            | Self::Daily {
                hour: 0..=23,
                minute: 0..=59,
            } => Ok(()),
            _ => Err(invalid("Schedule is outside supported limits.")),
        }
    }
    pub fn next_after(&self, now: i64, timezone: &str) -> WorkspaceResult<i64> {
        self.validate()?;
        if !(0..=253_402_214_400_000_i64).contains(&now) {
            return Err(invalid("Clock is outside supported range."));
        }
        let offset = timezone_offset(timezone)? as i64 * 60_000;
        match self {
            Self::Interval { minutes } => now
                .checked_add(i64::from(*minutes) * 60_000)
                .ok_or_else(|| invalid("Schedule overflow.")),
            Self::Daily { hour, minute } => {
                let local = now + offset;
                let candidate = local.div_euclid(86_400_000) * 86_400_000
                    + (i64::from(*hour) * 60 + i64::from(*minute)) * 60_000
                    - offset;
                Ok(if candidate <= now {
                    candidate + 86_400_000
                } else {
                    candidate
                })
            }
        }
    }
    pub(crate) fn advance(&self, due: i64, now: i64, timezone: &str) -> WorkspaceResult<i64> {
        match self {
            Self::Interval { minutes } => {
                self.validate()?;
                let step = i64::from(*minutes) * 60_000;
                due.checked_add(((now - due).max(0) / step + 1) * step)
                    .ok_or_else(|| invalid("Schedule overflow."))
            }
            _ => self.next_after(now, timezone),
        }
    }
}
/// Fixed offsets are explicit. IANA/DST zones are rejected, never approximated.
pub fn timezone_offset(value: &str) -> WorkspaceResult<i32> {
    if value == "UTC" {
        return Ok(0);
    }
    if value.len() != 6
        || !matches!(value.as_bytes()[0], b'+' | b'-')
        || value.as_bytes()[3] != b':'
    {
        return Err(invalid(
            "Timezone must be UTC or a fixed offset such as +02:00. IANA/DST zones are not supported yet.",
        ));
    }
    let hours: i32 = value
        .get(1..3)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| invalid("Invalid timezone offset."))?;
    let minutes: i32 = value
        .get(4..6)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| invalid("Invalid timezone offset."))?;
    if hours > 14 || minutes > 59 || hours < 0 || minutes < 0 || (hours == 14 && minutes != 0) {
        return Err(invalid("Invalid timezone offset."));
    }
    Ok((hours * 60 + minutes) * if value.starts_with('-') { -1 } else { 1 })
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MissedRunPolicy {
    Skip,
    CatchUpOnce,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationDefinition {
    pub id: AutomationId,
    pub revision: u64,
    pub title: String,
    pub instructions: String,
    pub agent_id: String,
    pub project_id: ProjectId,
    pub schedule: AutomationSchedule,
    pub timezone: String,
    pub enabled: bool,
    pub next_run_ms: i64,
    pub missed: MissedRunPolicy,
}
impl AutomationDefinition {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.title.trim().is_empty()
            || self.title.len() > 200
            || self.title.chars().any(char::is_control)
            || self.instructions.trim().is_empty()
            || self.instructions.len() > 16 * 1024
            || self.instructions.contains('\0')
            || self.agent_id.is_empty()
            || self.agent_id.len() > 256
            || self.agent_id.chars().any(char::is_control)
            || !(0..=253_402_300_799_000_i64).contains(&self.next_run_ms)
        {
            return Err(invalid(
                "Automation needs a title, instructions (up to 16 KiB), explicit agent and project.",
            ));
        }
        self.schedule.validate()?;
        timezone_offset(&self.timezone)?;
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AutomationRunStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Interrupted,
    Skipped,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationRun {
    pub id: AutomationId,
    pub definition: AutomationDefinition,
    pub owner: AutomationId,
    pub scheduled_ms: Option<i64>,
    pub started_ms: i64,
    pub finished_ms: Option<i64>,
    pub status: AutomationRunStatus,
    pub task_id: Option<TaskId>,
    pub output: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AutomationLedger {
    pub version: u32,
    pub definitions: Vec<AutomationDefinition>,
    pub runs: Vec<AutomationRun>,
}
impl Default for AutomationLedger {
    fn default() -> Self {
        Self {
            version: 1,
            definitions: vec![],
            runs: vec![],
        }
    }
}
impl AutomationLedger {
    pub(crate) fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1 || self.definitions.len() > 64 || self.runs.len() > 256 {
            return Err(invalid(
                "Unsupported automation ledger or capacity reached (64 definitions / 256 retained runs).",
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for d in &self.definitions {
            d.validate()?;
            if !ids.insert(d.id) {
                return Err(invalid("Duplicate automation identity."));
            }
        }
        ids.clear();
        for r in &self.runs {
            r.definition.validate()?;
            if !ids.insert(r.id) || r.output.len() > 16 * 1024 {
                return Err(invalid("Invalid run history."));
            }
        }
        Ok(())
    }
}
pub(crate) fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
impl WorkspaceService {
    pub async fn automations(&self) -> WorkspaceResult<AutomationLedger> {
        self.access(|store| store.automation_ledger()).await
    }
    /// Saving always pauses. Re-enabling is a separate explicit action.
    pub async fn save_automation(
        &self,
        mut definition: AutomationDefinition,
        expected_revision: Option<u64>,
    ) -> WorkspaceResult<()> {
        definition.validate()?;
        self.access(move |store| {
            store.validate_automation_context(&definition)?;
            store.edit_automations(move |ledger| {
                definition.enabled = false;
                definition.next_run_ms = definition
                    .schedule
                    .next_after(now_ms(), &definition.timezone)?;
                match (
                    ledger
                        .definitions
                        .iter()
                        .position(|d| d.id == definition.id),
                    expected_revision,
                ) {
                    (None, None) => {
                        definition.revision = 1;
                        ledger.definitions.push(definition);
                    }
                    (Some(index), Some(revision))
                        if ledger.definitions[index].revision == revision =>
                    {
                        if ledger.runs.iter().any(|r| {
                            r.definition.id == definition.id
                                && r.status == AutomationRunStatus::Running
                        }) {
                            return Err(invalid("Stop the active run before editing."));
                        }
                        definition.revision = revision
                            .checked_add(1)
                            .ok_or_else(|| invalid("Revision overflow."))?;
                        ledger.definitions[index] = definition;
                    }
                    _ => {
                        return Err(invalid(
                            "Automation changed in another window. Reload before saving.",
                        ));
                    }
                }
                Ok(())
            })
        })
        .await
    }
    pub async fn enable_automation(
        &self,
        id: AutomationId,
        revision: u64,
        enabled: bool,
    ) -> WorkspaceResult<()> {
        self.access(move |store| {
            store.edit_automations(move |ledger| {
                let definition = ledger
                    .definitions
                    .iter_mut()
                    .find(|d| d.id == id && d.revision == revision)
                    .ok_or_else(|| invalid("Automation changed. Reload first."))?;
                definition.enabled = enabled;
                if enabled {
                    definition.next_run_ms = definition
                        .schedule
                        .next_after(now_ms(), &definition.timezone)?;
                }
                definition.revision = definition
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| invalid("Revision overflow."))?;
                Ok(())
            })
        })
        .await
    }
    pub async fn delete_automation(
        &self,
        id: AutomationId,
        revision: u64,
        confirmed: bool,
    ) -> WorkspaceResult<()> {
        if !confirmed {
            return Err(invalid("Deletion requires explicit confirmation."));
        }
        self.access(move |store| {
            store.edit_automations(move |ledger| {
                if ledger
                    .runs
                    .iter()
                    .any(|r| r.definition.id == id && r.status == AutomationRunStatus::Running)
                {
                    return Err(invalid("Stop or resolve the active run before deleting."));
                }
                let index = ledger
                    .definitions
                    .iter()
                    .position(|d| d.id == id && d.revision == revision)
                    .ok_or_else(|| invalid("Automation changed. Reload first."))?;
                ledger.definitions.remove(index);
                // Historical snapshots and generated tasks remain owned and inspectable.
                Ok(())
            })
        })
        .await
    }
    #[cfg(test)]
    pub(crate) async fn claim_automation(
        &self,
        id: AutomationId,
        owner: AutomationId,
        scheduled: bool,
        now: i64,
    ) -> WorkspaceResult<Option<AutomationRun>> {
        self.claim_automation_revision(id, owner, scheduled, now, None)
            .await
    }
    pub(crate) async fn claim_automation_revision(
        &self,
        id: AutomationId,
        owner: AutomationId,
        scheduled: bool,
        now: i64,
        revision: Option<u64>,
    ) -> WorkspaceResult<Option<AutomationRun>> {
        self.access(move |store| store.claim_automation(id, owner, scheduled, now, revision))
            .await
    }
    pub(crate) async fn finish_automation(
        &self,
        id: AutomationId,
        owner: AutomationId,
        status: AutomationRunStatus,
        output: String,
    ) -> WorkspaceResult<()> {
        self.access(move |store| {
            store.edit_automations(move |ledger| {
                let run = ledger
                    .runs
                    .iter_mut()
                    .find(|r| {
                        r.id == id && r.owner == owner && r.status == AutomationRunStatus::Running
                    })
                    .ok_or_else(|| invalid("Run ownership changed."))?;
                if matches!(
                    status,
                    AutomationRunStatus::Running | AutomationRunStatus::Skipped
                ) {
                    return Err(invalid("Invalid run completion."));
                }
                run.status = status;
                run.finished_ms = Some(now_ms());
                run.output = output.chars().take(4096).collect();
                Ok(())
            })
        })
        .await
    }
    /// Explicit recovery after the user has verified the prior app/process stopped.
    /// Never resubmits a claimed slot. Also pauses its definition.
    pub async fn resolve_interrupted_automation(
        &self,
        id: AutomationId,
        current_owner: AutomationId,
        confirmed_stopped: bool,
    ) -> WorkspaceResult<()> {
        if !confirmed_stopped {
            return Err(invalid(
                "Verify that the previous process has stopped, then confirm.",
            ));
        }
        self.access(move |store| {
            store.edit_automations(move |ledger| {
                let run = ledger
                    .runs
                    .iter_mut()
                    .find(|r| {
                        r.id == id
                            && r.owner != current_owner
                            && r.status == AutomationRunStatus::Running
                    })
                    .ok_or_else(|| {
                        invalid("Only a previous process's unresolved run can be recovered.")
                    })?;
                run.status = AutomationRunStatus::Interrupted;
                run.finished_ms = Some(now_ms());
                run.output =
                    "Previous process stopped. External outcome is unknown. No automatic retry."
                        .into();
                if let Some(definition) = ledger
                    .definitions
                    .iter_mut()
                    .find(|d| d.id == run.definition.id)
                {
                    definition.enabled = false;
                    definition.revision = definition
                        .revision
                        .checked_add(1)
                        .ok_or_else(|| invalid("Revision overflow."))?;
                }
                Ok(())
            })
        })
        .await
    }
}

#[cfg(test)]
mod tests;
