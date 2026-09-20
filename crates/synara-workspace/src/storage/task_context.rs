//! Per-task notes and checklists are user metadata, never agent instructions
//! unless explicitly inserted into a prompt by the user.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_CONTEXT_BYTES: usize = 256 * 1024;
pub const MAX_NOTE_BYTES: usize = 128 * 1024;
pub const MAX_CHECKLIST_ITEMS: usize = 128;
pub const MAX_CHECKLIST_TEXT: usize = 512;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChecklistItem {
    pub id: String,
    pub text: String,
    pub done: bool,
}
impl ChecklistItem {
    pub fn new(text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            text,
            done: false,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskContext {
    pub version: u32,
    pub revision: u64,
    pub notes: String,
    pub checklist: Vec<ChecklistItem>,
}
impl Default for TaskContext {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            notes: String::new(),
            checklist: Vec::new(),
        }
    }
}
impl TaskContext {
    pub fn validate(&self) -> WorkspaceResult<()> {
        let ids = self
            .checklist
            .iter()
            .map(|item| &item.id)
            .collect::<HashSet<_>>();
        if self.version != 1
            || self.notes.len() > MAX_NOTE_BYTES
            || self.notes.contains('\0')
            || self.checklist.len() > MAX_CHECKLIST_ITEMS
            || ids.len() != self.checklist.len()
            || self.checklist.iter().any(|item| {
                uuid::Uuid::parse_str(&item.id).is_err()
                    || item.text.trim().is_empty()
                    || item.text.len() > MAX_CHECKLIST_TEXT
                    || item.text.chars().any(char::is_control)
            })
        {
            return Err(WorkspaceError::Invalid(
                "Notes or checklist are invalid or exceed their size limits.".into(),
            ));
        }
        Ok(())
    }
    pub fn as_prompt_context(&self) -> String {
        let mut text = String::new();
        if !self.notes.trim().is_empty() {
            text.push_str("## My notes\n\n");
            text.push_str(&self.notes);
        }
        if !self.checklist.is_empty() {
            if !text.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str("## My checklist\n\n");
            for item in &self.checklist {
                text.push_str(if item.done { "- [x] " } else { "- [ ] " });
                text.push_str(&item.text);
                text.push('\n');
            }
        }
        text
    }
}
fn context_key(task: TaskId) -> String {
    format!("task-context:{task}")
}
fn read_context(connection: &Connection, task: TaskId) -> WorkspaceResult<TaskContext> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE id=?1)",
            [task.to_string()],
            |row| row.get(0),
        )
        .map_err(StorageError::from)?;
    if !exists {
        return Err(WorkspaceError::NotFound);
    }
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [context_key(task)],
            |row| row.get(0),
        )
        .optional()
        .map_err(StorageError::from)?;
    let context = match raw {
        Some(raw) if raw.len() > MAX_CONTEXT_BYTES => return Err(StorageError::Limit.into()),
        Some(raw) => decode::<TaskContext>(&raw)?,
        None => TaskContext::default(),
    };
    context.validate()?;
    Ok(context)
}
impl WorkspaceService {
    pub async fn task_context(&self, task: TaskId) -> WorkspaceResult<TaskContext> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Deferred)
                .map_err(StorageError::from)?;
            let value = read_context(&tx, task)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(value)
        })
        .await
    }
    /// Optimistic revision check prevents an old editor or second window from
    /// overwriting newer notes. The caller retains the failed draft for review.
    pub async fn save_task_context(
        &self,
        task: TaskId,
        expected_revision: u64,
        mut value: TaskContext,
    ) -> WorkspaceResult<TaskContext> {
        value.validate()?;
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate).map_err(StorageError::from)?;
            let current = read_context(&tx, task)?;
            if current.revision != expected_revision || value.revision != expected_revision {
                return Err(WorkspaceError::Invalid("These notes changed in another window. Copy your edits, then reload the saved version before retrying.".into()));
            }
            if value.notes == current.notes && value.checklist == current.checklist {
                tx.commit().map_err(StorageError::from)?;
                return Ok(current);
            }
            value.revision = expected_revision.checked_add(1).ok_or(StorageError::Limit)?;
            let data = encode(&value)?;
            if data.len() > MAX_CONTEXT_BYTES { return Err(StorageError::Limit.into()); }
            tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data", params![context_key(task), data]).map_err(StorageError::from)?;
            tx.commit().map_err(StorageError::from)?;
            Ok(value)
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    async fn seed(service: &WorkspaceService, path: &Path) -> Task {
        let project = service.add_local_workspace(path.into()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        service
            .create_task(project.id, "Notes".into(), agent)
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn notes_and_ordered_checklist_survive_restart_without_sending_or_changing_drafts() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let service = WorkspaceService::open(path.clone()).await.unwrap();
        let task = seed(&service, dir.path()).await;
        service
            .save_task_draft(task.id, "Unsent prompt".into())
            .await
            .unwrap();
        let mut value = TaskContext {
            notes: "Caffè 日本語\nKeep exact spacing  ".into(),
            ..Default::default()
        };
        value.checklist.push(ChecklistItem::new("Review UI".into()));
        let mut done = ChecklistItem::new("Read source".into());
        done.done = true;
        value.checklist.insert(0, done);
        let value = service.save_task_context(task.id, 0, value).await.unwrap();
        assert_eq!(value.revision, 1);
        assert!(
            value
                .as_prompt_context()
                .contains("- [x] Read source\n- [ ] Review UI")
        );
        drop(service);
        let reopened = WorkspaceService::open(path).await.unwrap();
        assert_eq!(reopened.task_context(task.id).await.unwrap(), value);
        assert_eq!(reopened.task_draft(task.id).await.unwrap(), "Unsent prompt");
        assert!(
            reopened
                .thread(task.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
        assert!(reopened.session(task.thread_id).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn stale_invalid_and_future_edits_preserve_saved_notes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.db");
        let a = WorkspaceService::open(path.clone()).await.unwrap();
        let task = seed(&a, dir.path()).await;
        let b = WorkspaceService::open(path).await.unwrap();
        let stale = b.task_context(task.id).await.unwrap();
        let saved = a
            .save_task_context(
                task.id,
                0,
                TaskContext {
                    notes: "Keep".into(),
                    ..stale.clone()
                },
            )
            .await
            .unwrap();
        assert!(
            b.save_task_context(
                task.id,
                0,
                TaskContext {
                    notes: "Stale overwrite".into(),
                    ..stale
                }
            )
            .await
            .is_err()
        );
        assert!(
            a.save_task_context(
                task.id,
                1,
                TaskContext {
                    notes: "x".repeat(MAX_NOTE_BYTES + 1),
                    ..saved.clone()
                }
            )
            .await
            .is_err()
        );
        assert_eq!(b.task_context(task.id).await.unwrap(), saved);
        a.access(move |store| {
            store.set_preference(&context_key(task.id), &serde_json::json!({"version":99}))?;
            Ok(())
        })
        .await
        .unwrap();
        assert!(a.task_context(task.id).await.is_err());
        assert!(
            a.save_task_context(task.id, 0, TaskContext::default())
                .await
                .is_err()
        );
        a.access(move |store| {
            assert_eq!(
                store
                    .preference::<serde_json::Value>(&context_key(task.id))?
                    .unwrap()["version"],
                99
            );
            Ok(())
        })
        .await
        .unwrap();
    }
    #[tokio::test]
    async fn context_is_task_scoped_and_is_cleaned_up_by_permanent_deletion() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let task = seed(&service, dir.path()).await;
        let other = seed(&service, dir.path()).await;
        service
            .save_task_context(
                task.id,
                0,
                TaskContext {
                    notes: "Only this chat".into(),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        assert_eq!(
            service.task_context(other.id).await.unwrap(),
            TaskContext::default()
        );
        service.archive_task(task.id).await.unwrap();
        service.delete_task(task.id).await.unwrap();
        assert!(service.task_context(task.id).await.is_err());
        assert!(
            service
                .save_task_context(task.id, 0, TaskContext::default())
                .await
                .is_err()
        );
        service
            .access(move |store| {
                assert!(store.preference_raw(&context_key(task.id))?.is_none());
                Ok(())
            })
            .await
            .unwrap();
    }
}
