//! One versioned scheduler ledger in the existing SQLite store. Every mutation
//! takes the database writer lock before reading. Transcript events never decide
//! whether a scheduled slot has already been claimed.
use super::*;
use crate::automations::*;
use crate::{AgentProfile, WorkspaceResult, default_profiles};
const KEY: &str = "automation-ledger-v1";
fn read(connection: &Connection) -> WorkspaceResult<AutomationLedger> {
    let data: Option<String> = connection
        .query_row("SELECT data FROM preferences WHERE key=?1", [KEY], |row| {
            row.get(0)
        })
        .optional()?;
    let ledger: AutomationLedger = data.map(|s| decode(&s)).transpose()?.unwrap_or_default();
    ledger.validate()?;
    Ok(ledger)
}
fn write(connection: &Connection, ledger: &AutomationLedger) -> WorkspaceResult<()> {
    ledger.validate()?;
    connection.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![KEY, encode(ledger)?])?;
    Ok(())
}
fn context(
    connection: &Connection,
    definition: &AutomationDefinition,
) -> WorkspaceResult<(Project, Workspace)> {
    let data: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key='agent_profiles'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let profiles: Vec<AgentProfile> = data
        .map(|s| decode(&s))
        .transpose()?
        .unwrap_or_else(default_profiles);
    if !profiles.iter().any(|p| p.id == definition.agent_id) {
        return Err(invalid(
            "Automation's explicit agent profile is unavailable. Edit and select an installed profile.",
        ));
    }
    let project: String = connection
        .query_row(
            "SELECT data FROM projects WHERE id=?1",
            [definition.project_id.to_string()],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| invalid("Automation project no longer exists."))?;
    let project: Project = decode(&project)?;
    let workspace: String = connection.query_row(
        "SELECT data FROM workspaces WHERE id=?1",
        [project.workspace_id.to_string()],
        |r| r.get(0),
    )?;
    Ok((project, decode(&workspace)?))
}
impl Store {
    pub(crate) fn automation_ledger(&self) -> WorkspaceResult<AutomationLedger> {
        read(&self.connection)
    }
    pub(crate) fn validate_automation_context(
        &self,
        definition: &AutomationDefinition,
    ) -> WorkspaceResult<()> {
        context(&self.connection, definition).map(|_| ())
    }
    pub(crate) fn edit_automations<R>(
        &mut self,
        edit: impl FnOnce(&mut AutomationLedger) -> WorkspaceResult<R>,
    ) -> WorkspaceResult<R> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut ledger = read(&tx)?;
        let value = edit(&mut ledger)?;
        write(&tx, &ledger)?;
        tx.commit()?;
        Ok(value)
    }
    pub(crate) fn claim_automation(
        &mut self,
        id: AutomationId,
        owner: AutomationId,
        scheduled: bool,
        now: i64,
        revision: Option<u64>,
    ) -> WorkspaceResult<Option<AutomationRun>> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut ledger = read(&tx)?;
        let Some(index) = ledger.definitions.iter().position(|d| d.id == id) else {
            return Err(invalid("Automation no longer exists."));
        };
        let definition = ledger.definitions[index].clone();
        if revision.is_some_and(|expected| expected != definition.revision) {
            return Err(invalid(
                "Automation changed after confirmation. Reload and review its current instructions before running.",
            ));
        }
        if scheduled && (!definition.enabled || definition.next_run_ms > now) {
            return Ok(None);
        }
        if ledger
            .runs
            .iter()
            .any(|r| r.definition.id == id && r.status == AutomationRunStatus::Running)
        {
            return Ok(None);
        }
        if ledger.runs.len() >= 256 {
            return Err(invalid(
                "Automation history is full. No run was started. Retained history is not silently deleted.",
            ));
        }
        let scheduled_ms = scheduled.then_some(definition.next_run_ms);
        if let Some(slot) = scheduled_ms {
            if ledger
                .runs
                .iter()
                .any(|r| r.definition.id == id && r.scheduled_ms == Some(slot))
            {
                return Err(invalid("Scheduled slot was already claimed."));
            }
            ledger.definitions[index].next_run_ms =
                definition
                    .schedule
                    .advance(slot, now, &definition.timezone)?;
            if definition.missed == MissedRunPolicy::Skip && now.saturating_sub(slot) > 30_000 {
                ledger.runs.push(AutomationRun {
                    id: AutomationId::new_v4(),
                    definition,
                    owner,
                    scheduled_ms,
                    started_ms: now,
                    finished_ms: Some(now),
                    status: AutomationRunStatus::Skipped,
                    task_id: None,
                    output: "Missed-run policy skipped an overdue slot. No agent was launched."
                        .into(),
                });
                write(&tx, &ledger)?;
                tx.commit()?;
                return Ok(None);
            }
        }
        let (project, workspace) = context(&tx, &definition)?;
        let task = Task {
            id: TaskId::new(),
            project_id: project.id,
            title: format!("Automation: {}", definition.title),
            state: TaskState::Ready,
            thread_id: ThreadId::new(),
            agent_id: definition.agent_id.clone(),
            working_directory: crate::service::project_directory(&workspace, &project)?,
            updated_at_ms: now,
            scope: TaskScope::Project,
        };
        let run = AutomationRun {
            id: AutomationId::new_v4(),
            definition,
            owner,
            scheduled_ms,
            started_ms: now,
            finished_ms: None,
            status: AutomationRunStatus::Running,
            task_id: Some(task.id),
            output: String::new(),
        };
        // Claimed slot, conversation identity and its visible unsent prompt are one transaction.
        tx.execute(
            "INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?1,?2,?3,?4,?5)",
            params![
                task.id.to_string(),
                task.project_id.to_string(),
                task.thread_id.to_string(),
                task.updated_at_ms,
                encode(&task)?
            ],
        )?;
        tx.execute(
            "INSERT INTO preferences(key,data) VALUES(?1,?2)",
            params![
                format!("task-draft:{}", task.id),
                encode(&serde_json::json!({"version":1,"text":run.definition.instructions}))?
            ],
        )?;
        ledger.runs.push(run.clone());
        write(&tx, &ledger)?;
        tx.commit()?;
        Ok(Some(run))
    }
}
