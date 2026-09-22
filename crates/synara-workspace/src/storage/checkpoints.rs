//! Explicit rollback of app-owned draft/notes/checklist only. No file, Git,
//! transcript, session, approval, attachment or provider state is copied/restored.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const MAX_SNAPSHOT: usize = 128 * 1024;
const MAX_HISTORY: usize = 512 * 1024;
const MAX_ITEMS: usize = 8;
fn sql(e: rusqlite::Error) -> WorkspaceError {
    StorageError::from(e).into()
}
fn key(task: TaskId) -> String {
    format!("task-checkpoints:{task}")
}
fn stale() -> WorkspaceError {
    WorkspaceError::Invalid(
        "Checkpoint review is stale. Reload and review again. Nothing was restored.".into(),
    )
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCheckpoint {
    pub id: String,
    pub label: String,
    pub created_at_ms: i64,
    pub project: ProjectId,
    pub thread: ThreadId,
    pub working_directory: PathBuf,
    pub draft: String,
    pub context: TaskContext,
}
impl TaskCheckpoint {
    fn capture(
        task: &Task,
        label: &str,
        draft: String,
        context: TaskContext,
    ) -> WorkspaceResult<Self> {
        let value = Self {
            id: uuid::Uuid::new_v4().to_string(),
            label: label.into(),
            created_at_ms: crate::now_ms(),
            project: task.project_id,
            thread: task.thread_id,
            working_directory: task.working_directory.clone(),
            draft,
            context,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> WorkspaceResult<()> {
        self.context.validate()?;
        if uuid::Uuid::parse_str(&self.id).is_err()
            || self.id.len() != 36
            || self.label.is_empty()
            || self.label.len() > 96
            || self.label.chars().any(char::is_control)
            || !self.working_directory.is_absolute()
            || self.working_directory.as_os_str().len() > 4096
            || self.draft.contains('\0')
            || encode(self)?.len() > MAX_SNAPSHOT
        {
            return Err(WorkspaceError::Invalid("Checkpoint exceeds the supported 128 KiB snapshot or contains invalid data. Nothing was replaced.".into()));
        }
        Ok(())
    }
    fn owns(&self, task: &Task) -> bool {
        self.project == task.project_id
            && self.thread == task.thread_id
            && self.working_directory == task.working_directory
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCheckpoints {
    pub version: u32,
    pub revision: u64,
    pub task: TaskId,
    pub items: Vec<TaskCheckpoint>,
}
impl TaskCheckpoints {
    fn empty(task: TaskId) -> Self {
        Self {
            version: 1,
            revision: 0,
            task,
            items: vec![],
        }
    }
    fn validate(&self, task: TaskId) -> WorkspaceResult<()> {
        if self.version != 1
            || self.task != task
            || self.items.len() > MAX_ITEMS
            || encode(self)?.len() > MAX_HISTORY
        {
            return Err(WorkspaceError::Invalid(
                "Unsupported or malformed checkpoint history. Nothing was replaced.".into(),
            ));
        }
        for (index, item) in self.items.iter().enumerate() {
            item.validate()?;
            if self.items[..index].iter().any(|v| v.id == item.id) {
                return Err(StorageError::Identity.into());
            }
        }
        Ok(())
    }
    fn push(&mut self, checkpoint: TaskCheckpoint) -> WorkspaceResult<()> {
        checkpoint.validate()?;
        self.revision = self.revision.checked_add(1).ok_or(StorageError::Limit)?;
        self.items.push(checkpoint);
        while self.items.len() > MAX_ITEMS || encode(self)?.len() > MAX_HISTORY {
            self.items.remove(0);
        }
        self.validate(self.task)
    }
}
#[derive(Clone, Debug)]
pub struct CheckpointReview {
    task: Task,
    history_revision: u64,
    checkpoint: TaskCheckpoint,
    draft: String,
    context: TaskContext,
    reviewed_at: Instant,
}
impl CheckpointReview {
    pub fn task(&self) -> TaskId {
        self.task.id
    }
    pub fn checkpoint(&self) -> &TaskCheckpoint {
        &self.checkpoint
    }
    pub fn current_draft(&self) -> &str {
        &self.draft
    }
    pub fn current_context(&self) -> &TaskContext {
        &self.context
    }
}
#[derive(Clone, Debug)]
pub struct CheckpointRestored {
    pub history: TaskCheckpoints,
    pub draft: String,
    pub context: TaskContext,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    version: u32,
    text: String,
}
fn read_draft(db: &Connection, task: TaskId) -> WorkspaceResult<String> {
    let raw: Option<String> = db
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [format!("task-draft:{task}")],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql)?;
    let value = raw
        .as_deref()
        .map(decode::<Draft>)
        .transpose()?
        .unwrap_or(Draft {
            version: 1,
            text: String::new(),
        });
    if value.version != 1 || value.text.len() > 1024 * 1024 {
        return Err(StorageError::Limit.into());
    }
    Ok(value.text)
}
fn read_task(db: &Connection, task: TaskId) -> WorkspaceResult<Task> {
    let raw: String = db
        .query_row(
            "SELECT data FROM tasks WHERE id=?1",
            [task.to_string()],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql)?
        .ok_or(WorkspaceError::NotFound)?;
    let value: Task = decode(&raw)?;
    if value.id != task {
        return Err(StorageError::Identity.into());
    }
    if value.state == TaskState::Archived {
        return Err(WorkspaceError::Invalid(
            "Restore the archived task before using its checkpoints.".into(),
        ));
    }
    Ok(value)
}
fn read_history(db: &Connection, task: TaskId) -> WorkspaceResult<TaskCheckpoints> {
    read_task(db, task)?;
    let raw: Option<String> = db
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [key(task)],
            |r| r.get(0),
        )
        .optional()
        .map_err(sql)?;
    if raw.as_ref().is_some_and(|v| v.len() > MAX_HISTORY) {
        return Err(StorageError::Limit.into());
    }
    let value = raw
        .as_deref()
        .map(decode::<TaskCheckpoints>)
        .transpose()?
        .unwrap_or_else(|| TaskCheckpoints::empty(task));
    value.validate(task)?;
    Ok(value)
}
fn save(db: &Connection, key: String, data: String) -> WorkspaceResult<()> {
    db.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![key,data]).map_err(sql)?;
    Ok(())
}
impl WorkspaceService {
    pub async fn task_checkpoints(&self, task: TaskId) -> WorkspaceResult<TaskCheckpoints> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Deferred)
                .map_err(sql)?;
            let value = read_history(&tx, task)?;
            tx.commit().map_err(sql)?;
            Ok(value)
        })
        .await
    }
    pub(crate) async fn capture_task_checkpoint(
        &self,
        task: TaskId,
        expected_draft: String,
    ) -> WorkspaceResult<TaskCheckpoints> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(sql)?;
            let owner = read_task(&tx, task)?;
            if matches!(owner.state, TaskState::Running | TaskState::Waiting) {
                return Err(synara_agent::AgentError::Busy.into());
            }
            let draft = read_draft(&tx, task)?;
            if draft != expected_draft {
                return Err(stale());
            }
            let context = task_context::read_context(&tx, task)?;
            let mut history = read_history(&tx, task)?;
            history.push(TaskCheckpoint::capture(
                &owner,
                "Saved draft and notes",
                draft,
                context,
            )?)?;
            save(&tx, key(task), encode(&history)?)?;
            tx.commit().map_err(sql)?;
            Ok(history)
        })
        .await
    }
    pub async fn review_task_checkpoint(
        &self,
        task: TaskId,
        checkpoint: String,
    ) -> WorkspaceResult<CheckpointReview> {
        if uuid::Uuid::parse_str(&checkpoint).is_err() {
            return Err(stale());
        }
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Deferred)
                .map_err(sql)?;
            let owner = read_task(&tx, task)?;
            let history = read_history(&tx, task)?;
            let saved = history
                .items
                .iter()
                .find(|c| c.id == checkpoint && c.owns(&owner))
                .cloned()
                .ok_or_else(stale)?;
            let value = CheckpointReview {
                task: owner,
                history_revision: history.revision,
                checkpoint: saved,
                draft: read_draft(&tx, task)?,
                context: task_context::read_context(&tx, task)?,
                reviewed_at: Instant::now(),
            };
            // Refuse a review whose pre-revert recovery state cannot fit. Never truncate it.
            TaskCheckpoint::capture(
                &value.task,
                "Before revert",
                value.draft.clone(),
                value.context.clone(),
            )?;
            tx.commit().map_err(sql)?;
            Ok(value)
        })
        .await
    }
    pub(crate) async fn restore_task_checkpoint(
        &self,
        review: CheckpointReview,
    ) -> WorkspaceResult<CheckpointRestored> {
        self.access(move |store| {
            if review.reviewed_at.elapsed() > Duration::from_secs(300) {
                return Err(stale());
            }
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(sql)?;
            let task = read_task(&tx, review.task.id)?;
            if matches!(task.state, TaskState::Running | TaskState::Waiting) {
                return Err(synara_agent::AgentError::Busy.into());
            }
            let mut history = read_history(&tx, task.id)?;
            let draft = read_draft(&tx, task.id)?;
            let context = task_context::read_context(&tx, task.id)?;
            if !review.checkpoint.owns(&task)
                || history.revision != review.history_revision
                || !history.items.iter().any(|c| c == &review.checkpoint)
                || draft != review.draft
                || context != review.context
            {
                return Err(stale());
            }
            let mut restored = review.checkpoint.context;
            restored.revision = context.revision.checked_add(1).ok_or(StorageError::Limit)?;
            restored.validate()?;
            history.push(TaskCheckpoint::capture(
                &task,
                "Before revert (recovery)",
                draft,
                context,
            )?)?;
            // One SQLite transaction contains recovery, draft and notes. Failure rolls ALL back.
            save(&tx, key(task.id), encode(&history)?)?;
            save(
                &tx,
                format!("task-draft:{}", task.id),
                encode(&Draft {
                    version: 1,
                    text: review.checkpoint.draft.clone(),
                })?,
            )?;
            save(&tx, format!("task-context:{}", task.id), encode(&restored)?)?;
            tx.commit().map_err(sql)?;
            Ok(CheckpointRestored {
                history,
                draft: review.checkpoint.draft,
                context: restored,
            })
        })
        .await
    }
}
#[cfg(test)]
mod tests;
