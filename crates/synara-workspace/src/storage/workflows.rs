//! Workflow graph, child tasks and unsent drafts commit in one SQLite transaction.
#[cfg(test)]
mod tests;
use super::*;
use crate::autonomy::workflow::{fingerprint, invalid};
use crate::autonomy::{Workflow, WorkflowPhase, WorkflowSpec, WorkflowStep, WorkflowStepState};
use crate::{AgentProfile, WorkspaceError, WorkspaceResult, WorkspaceService};
use uuid::Uuid;

fn key(id: TaskId) -> String {
    format!("task-workflow:{id}")
}
fn parent_key(id: TaskId) -> String {
    format!("task-workflow-parent:{id}")
}
fn raw(db: &Connection, key: &str) -> WorkspaceResult<Option<String>> {
    Ok(db
        .query_row("SELECT data FROM preferences WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}
fn read(db: &Connection, parent: TaskId) -> WorkspaceResult<Option<Workflow>> {
    raw(db, &key(parent))?
        .map(|text| {
            if text.len() > 512 * 1024 {
                return Err(invalid("Workflow exceeds its storage budget"));
            }
            let value: Workflow = decode(&text)?;
            value.validate()?;
            if value.parent != parent {
                return Err(invalid("Workflow ownership mismatch"));
            }
            Ok(value)
        })
        .transpose()
}
fn task(db: &Connection, id: TaskId) -> WorkspaceResult<Task> {
    let value: Option<String> = db
        .query_row(
            "SELECT data FROM tasks WHERE id=?1",
            [id.to_string()],
            |r| r.get(0),
        )
        .optional()?;
    value
        .map(|v| decode(&v).map_err(Into::into))
        .unwrap_or(Err(WorkspaceError::NotFound))
}
fn profiles(db: &Connection) -> WorkspaceResult<Vec<AgentProfile>> {
    match raw(db, "agent_profiles")? {
        Some(v) => Ok(decode(&v)?),
        None => Ok(crate::default_profiles()),
    }
}
fn save(db: &Connection, value: &Workflow) -> WorkspaceResult<()> {
    value.validate()?;
    let text = encode(value)?;
    if text.len() > 512 * 1024 {
        return Err(invalid("Workflow storage budget exceeded"));
    }
    db.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![key(value.parent), text])?;
    Ok(())
}
fn next(value: &mut Workflow) -> WorkspaceResult<()> {
    value.revision = value
        .revision
        .checked_add(1)
        .ok_or_else(|| invalid("Workflow revision exhausted"))?;
    Ok(())
}
impl WorkspaceService {
    pub async fn workflow(&self, parent: TaskId) -> WorkspaceResult<Option<Workflow>> {
        self.access(move |store| {
            task(&store.connection, parent)?;
            read(&store.connection, parent)
        })
        .await
    }
    pub async fn workflow_parent(&self, child: TaskId) -> WorkspaceResult<Option<TaskId>> {
        self.access(move |store| {
            raw(&store.connection, &parent_key(child))?
                .map(|v| decode(&v).map_err(Into::into))
                .transpose()
        })
        .await
    }
    /// Creates unsent children only. No process, inference or permission is started.
    pub async fn create_workflow(
        &self,
        parent: TaskId,
        spec: WorkflowSpec,
    ) -> WorkspaceResult<Workflow> {
        spec.validate()?;
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let source = task(&tx, parent)?;
            if source.state == TaskState::Archived || raw(&tx, &parent_key(parent))?.is_some() {
                return Err(invalid("Archived tasks and delegated children cannot own new workflow graphs"));
            }
            if raw(&tx, &key(parent))?.is_some() { return Err(invalid("This task already owns a workflow. Detach its stopped children before creating another")); }
            let profiles = profiles(&tx)?;
            let mut value = Workflow { version: 1, revision: 1, id: Uuid::new_v4(), parent, spec, phase: WorkflowPhase::Ready, run: None, steps: vec![] };
            for step in &value.spec.steps {
                let profile = profiles.iter().find(|p| p.id == step.agent_id).ok_or_else(|| invalid("A selected agent profile no longer exists"))?;
                let child = Task { id: TaskId::new(), thread_id: ThreadId::new(), title: step.title.clone(), agent_id: step.agent_id.clone(), state: TaskState::Ready, scope: TaskScope::Chat, updated_at_ms: chrono::Utc::now().timestamp_millis(), ..source.clone() };
                tx.execute("INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?1,?2,?3,?4,?5)", params![child.id.to_string(), child.project_id.to_string(), child.thread_id.to_string(), child.updated_at_ms, encode(&child)?])?;
                tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2)", params![format!("task-draft:{}", child.id), encode(&serde_json::json!({"version":1,"text":step.instruction}))?])?;
                tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2)", params![parent_key(child.id), encode(&parent)?])?;
                value.steps.push(WorkflowStep { task: child.id, state: WorkflowStepState::Pending, attempts: 0, profile_fingerprint: fingerprint(profile)?, sequence: 0, output: String::new(), usage: None, error: None });
            }
            save(&tx, &value)?;
            tx.commit()?;
            Ok(value)
        }).await
    }
    pub(crate) async fn change_workflow(
        &self,
        parent: TaskId,
        revision: Option<u64>,
        run: Option<Uuid>,
        change: impl FnOnce(&mut Workflow) -> WorkspaceResult<()> + Send + 'static,
    ) -> WorkspaceResult<Workflow> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut value = read(&tx, parent)?.ok_or(WorkspaceError::NotFound)?;
            if value.run != run || revision.is_some_and(|r| r != value.revision) {
                return Err(invalid("Stale workflow review or run owner"));
            }
            let identities: Vec<_> = value.steps.iter().map(|s| s.task).collect();
            let identity = value.id;
            change(&mut value)?;
            if identity != value.id
                || value.parent != parent
                || identities != value.steps.iter().map(|s| s.task).collect::<Vec<_>>()
            {
                return Err(invalid("Workflow identities cannot be reassigned"));
            }
            next(&mut value)?;
            save(&tx, &value)?;
            tx.commit()?;
            Ok(value)
        })
        .await
    }
    pub(crate) async fn claim_workflow_step(
        &self,
        parent: TaskId,
        run: Uuid,
        index: usize,
    ) -> WorkspaceResult<Workflow> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut value = read(&tx, parent)?.ok_or(WorkspaceError::NotFound)?;
            if value.run != Some(run)
                || value.phase != WorkflowPhase::Running
                || !value.ready_indices().contains(&index)
                || task(&tx, parent)?.state == TaskState::Archived
            {
                return Err(invalid("Workflow step is not owned and ready"));
            }
            let child = task(&tx, value.steps[index].task)?;
            let spec = &value.spec.steps[index];
            let profile = profiles(&tx)?
                .into_iter()
                .find(|p| p.id == spec.agent_id)
                .ok_or_else(|| invalid("Agent profile disappeared"))?;
            let sequence: i64 = tx
                .query_row(
                    "SELECT sequence FROM event_heads WHERE thread_id=?1",
                    [child.thread_id.to_string()],
                    |r| r.get(0),
                )
                .optional()?
                .unwrap_or(0);
            let sequence = u64::try_from(sequence)
                .map_err(|_| invalid("Invalid stored workflow event sequence"))?;
            let draft: serde_json::Value = raw(&tx, &format!("task-draft:{}", child.id))?
                .map(|v| decode(&v))
                .transpose()?
                .unwrap_or(serde_json::Value::Null);
            if matches!(
                child.state,
                TaskState::Running | TaskState::Waiting | TaskState::Archived
            ) || child.agent_id != spec.agent_id
                || sequence != value.steps[index].sequence
                || fingerprint(&profile)? != value.steps[index].profile_fingerprint
                || draft["text"].as_str() != Some(spec.instruction.as_str())
                || raw(&tx, &format!("task-direct-model:{}", child.id))?
                    .as_deref()
                    .is_some_and(|v| v != "null")
                || raw(&tx, &parent_key(child.id))? != Some(encode(&parent)?)
            {
                return Err(invalid(
                    "Child draft, route, profile or transcript changed after workflow review",
                ));
            }
            let step = &mut value.steps[index];
            if step.attempts >= 3 {
                return Err(invalid(
                    "A workflow step permits at most three explicitly reviewed attempts",
                ));
            }
            step.state = WorkflowStepState::Running;
            step.attempts += 1;
            step.error = None;
            next(&mut value)?;
            save(&tx, &value)?;
            tx.commit()?;
            Ok(value)
        })
        .await
    }
    pub async fn steer_workflow_step(
        &self,
        parent: TaskId,
        workflow: Uuid,
        revision: u64,
        index: usize,
        instruction: String,
    ) -> WorkspaceResult<Workflow> {
        crate::autonomy::workflow::bounded(&instruction, 16 * 1024, true)?;
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut value = read(&tx, parent)?.ok_or(WorkspaceError::NotFound)?;
            if value.id != workflow
                || value.revision != revision
                || value.run.is_some()
                || value
                    .steps
                    .get(index)
                    .is_none_or(|s| s.state != WorkflowStepState::Pending)
                || task(&tx, parent)?.state == TaskState::Archived
            {
                return Err(invalid(
                    "Only an idle pending step can be steered after fresh review",
                ));
            }
            let child = task(&tx, value.steps[index].task)?;
            if matches!(
                child.state,
                TaskState::Archived | TaskState::Running | TaskState::Waiting
            ) {
                return Err(invalid("Child is not idle"));
            }
            let draft_key = format!("task-draft:{}", child.id);
            let current: serde_json::Value = raw(&tx, &draft_key)?
                .map(|v| decode(&v))
                .transpose()?
                .unwrap_or(serde_json::Value::Null);
            if current["text"].as_str() != Some(value.spec.steps[index].instruction.as_str()) {
                return Err(invalid("An edited child draft must not be overwritten"));
            }
            value.spec.steps[index].instruction = instruction.clone();
            tx.execute(
                "UPDATE preferences SET data=?1 WHERE key=?2",
                params![
                    encode(&serde_json::json!({"version":1,"text":instruction}))?,
                    draft_key
                ],
            )?;
            next(&mut value)?;
            save(&tx, &value)?;
            tx.commit()?;
            Ok(value)
        })
        .await
    }
    /// Removes orchestration links only. Child tasks, drafts, transcripts and files survive.
    pub async fn detach_workflow(
        &self,
        parent: TaskId,
        workflow: Uuid,
        revision: u64,
    ) -> WorkspaceResult<()> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let value = read(&tx, parent)?.ok_or(WorkspaceError::NotFound)?;
            if value.id != workflow || value.revision != revision || value.run.is_some() {
                return Err(invalid("Stop the workflow before detaching children"));
            }
            for step in &value.steps {
                if matches!(
                    task(&tx, step.task)?.state,
                    TaskState::Running | TaskState::Waiting
                ) {
                    return Err(invalid("An active child must be stopped before detaching"));
                }
                tx.execute(
                    "DELETE FROM preferences WHERE key=?1",
                    [parent_key(step.task)],
                )?;
            }
            tx.execute("DELETE FROM preferences WHERE key=?1", [key(parent)])?;
            tx.commit()?;
            Ok(())
        })
        .await
    }
}

impl Store {
    pub(crate) fn archive_task_with_workflow_guard(
        &mut self,
        id: TaskId,
        timestamp: i64,
    ) -> WorkspaceResult<Task> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut value = task(&tx, id)?;
        if matches!(value.state, TaskState::Running | TaskState::Waiting) {
            return Err(synara_agent::AgentError::Busy.into());
        }
        if read(&tx, id)?.is_some_and(|w| w.run.is_some()) {
            return Err(invalid(
                "Stop or recover this workflow before archiving its parent",
            ));
        }
        if let Some(parent) = raw(&tx, &parent_key(id))? {
            let parent: TaskId = decode(&parent)?;
            let graph = read(&tx, parent)?
                .ok_or_else(|| invalid("Workflow ownership record is incomplete"))?;
            if !graph.steps.iter().any(|s| s.task == id) || graph.run.is_some() {
                return Err(invalid(
                    "Stop or recover the owning workflow before archiving this child",
                ));
            }
        }
        if value.state != TaskState::Archived {
            value.state = TaskState::Archived;
            value.updated_at_ms = timestamp;
            tx.execute(
                "UPDATE tasks SET updated_ms=?2,data=?3 WHERE id=?1",
                params![id.to_string(), timestamp, encode(&value)?],
            )?;
        }
        tx.commit()?;
        Ok(value)
    }
}
