//! Per-task notes and checklists are user metadata, never agent instructions
//! unless explicitly inserted into a prompt by the user.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

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
    /// Absolute paths selected by the user. Paths are stored as text only.
    #[serde(default)]
    pub folder_references: Vec<String>,
}
impl Default for TaskContext {
    fn default() -> Self {
        Self {
            version: Self::CURRENT_VERSION,
            revision: 0,
            notes: String::new(),
            checklist: Vec::new(),
            folder_references: Vec::new(),
        }
    }
}
impl TaskContext {
    pub const CURRENT_VERSION: u32 = 2;
    pub const MAX_FOLDER_REFERENCES: usize = 32;
    pub const MAX_FOLDER_REFERENCE_BYTES: usize = 4096;

    /// Validate a selected folder path without opening it or inspecting its contents.
    pub fn validate_folder_reference(path: &str) -> WorkspaceResult<()> {
        let value = Path::new(path);
        if path.is_empty()
            || path.len() > Self::MAX_FOLDER_REFERENCE_BYTES
            || !value.is_absolute()
            || path.starts_with("//")
            || path.starts_with("\\\\")
            || path.chars().any(char::is_control)
        {
            return Err(WorkspaceError::Invalid(
                "Choose an absolute local folder path within 4096 bytes. Network shares and control characters are not supported.".into(),
            ));
        }
        Ok(())
    }

    pub fn validate(&self) -> WorkspaceResult<()> {
        let ids = self
            .checklist
            .iter()
            .map(|item| &item.id)
            .collect::<HashSet<_>>();
        let folders = self.folder_references.iter().collect::<HashSet<_>>();
        if !(self.version == 1 || self.version == Self::CURRENT_VERSION)
            || self.notes.len() > MAX_NOTE_BYTES
            || self.notes.contains('\0')
            || self.checklist.len() > MAX_CHECKLIST_ITEMS
            || ids.len() != self.checklist.len()
            || self.folder_references.len() > Self::MAX_FOLDER_REFERENCES
            || folders.len() != self.folder_references.len()
            || self
                .folder_references
                .iter()
                .any(|path| Self::validate_folder_reference(path).is_err())
            || self.checklist.iter().any(|item| {
                uuid::Uuid::parse_str(&item.id).is_err()
                    || item.text.trim().is_empty()
                    || item.text.len() > MAX_CHECKLIST_TEXT
                    || item.text.chars().any(char::is_control)
            })
        {
            return Err(WorkspaceError::Invalid(
                "Saved context or folder references are invalid or exceed their size limits.".into(),
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
        if !self.folder_references.is_empty() {
            if !text.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str("## Saved folder path references\n\n");
            text.push_str("These are user-selected path strings only. Folder contents were not read, and no filesystem access or permissions were granted.\n");
            for path in &self.folder_references {
                let escaped: String = path
                    .chars()
                    .map(|character| {
                        if is_bidi_control(character) {
                            format!("\\u{{{:04X}}}", character as u32)
                        } else {
                            character.to_string()
                        }
                    })
                    .collect();
                let quoted = serde_json::to_string(&escaped)
                    .unwrap_or_else(|_| "\"<invalid folder path>\"".into());
                text.push_str("- ");
                text.push_str(&quoted);
                text.push('\n');
            }
        }
        text
    }
}

fn is_bidi_control(character: char) -> bool {
    matches!(
        character as u32,
        0x061c | 0x200e..=0x200f | 0x202a..=0x202e | 0x2066..=0x2069
    )
}
fn context_key(task: TaskId) -> String {
    format!("task-context:{task}")
}
pub(super) fn read_context(connection: &Connection, task: TaskId) -> WorkspaceResult<TaskContext> {
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
            if value.notes == current.notes
                && value.checklist == current.checklist
                && value.folder_references == current.folder_references
            {
                tx.commit().map_err(StorageError::from)?;
                return Ok(current);
            }
            value.version = TaskContext::CURRENT_VERSION;
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
        value.folder_references.push(dir.path().to_string_lossy().into_owned());
        value.checklist.push(ChecklistItem::new("Review UI".into()));
        let mut done = ChecklistItem::new("Read source".into());
        done.done = true;
        value.checklist.insert(0, done);
        let value = service.save_task_context(task.id, 0, value).await.unwrap();
        assert_eq!(value.version, TaskContext::CURRENT_VERSION);
        assert_eq!(value.revision, 1);
        assert!(
            value
                .as_prompt_context()
                .contains("- [x] Read source\n- [ ] Review UI")
        );
        let prompt = value.as_prompt_context();
        assert!(prompt.contains("## Saved folder path references"));
        assert!(prompt.contains("Folder contents were not read"));
        assert!(prompt.contains(&dir.path().to_string_lossy().to_string()));
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
                    folder_references: vec![dir.path().to_string_lossy().into_owned()],
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

    #[test]
    fn legacy_contexts_migrate_with_no_references_and_path_validation_is_bounded() {
        let legacy: TaskContext = serde_json::from_value(serde_json::json!({
            "version": 1,
            "revision": 7,
            "notes": "Existing notes",
            "checklist": []
        }))
        .unwrap();
        assert_eq!(legacy.folder_references, Vec::<String>::new());
        assert_eq!(legacy.version, 1);
        legacy.validate().unwrap();

        let root = tempfile::tempdir().unwrap();
        let valid = root.path().to_string_lossy().into_owned();
        TaskContext::validate_folder_reference(&valid).unwrap();
        for invalid in [
            "relative/path",
            "//server/share",
            "\\\\server\\share",
            "bad\0path",
        ] {
            assert!(TaskContext::validate_folder_reference(invalid).is_err());
        }
        assert!(TaskContext::validate_folder_reference(
            &"/".repeat(TaskContext::MAX_FOLDER_REFERENCE_BYTES + 1)
        )
        .is_err());

        let duplicate = TaskContext {
            folder_references: vec![valid.clone(), valid],
            ..Default::default()
        };
        assert!(duplicate.validate().is_err());
        let too_many = TaskContext {
            folder_references: (0..=TaskContext::MAX_FOLDER_REFERENCES)
                .map(|index| {
                    Path::new(valid.as_str())
                        .join(format!("folder-{index}"))
                        .to_string_lossy()
                        .into_owned()
                })
                .collect(),
            ..Default::default()
        };
        assert!(too_many.validate().is_err());
    }

    #[tokio::test]
    async fn saving_a_legacy_context_upgrades_version_and_persists_folder_paths() {
        let dir = tempfile::tempdir().unwrap();
        let database = dir.path().join("data.db");
        let service = WorkspaceService::open(database.clone()).await.unwrap();
        let task = seed(&service, dir.path()).await;
        let key = context_key(task.id);
        service
            .access(move |store| {
                store.set_preference(
                    &key,
                    &serde_json::json!({
                        "version": 1,
                        "revision": 0,
                        "notes": "Legacy notes",
                        "checklist": []
                    }),
                )?;
                Ok(())
            })
            .await
            .unwrap();

        let legacy = service.task_context(task.id).await.unwrap();
        assert_eq!(legacy.version, 1);
        assert!(legacy.folder_references.is_empty());
        let folder = dir.path().join("chosen-folder").to_string_lossy().into_owned();
        let upgraded = service
            .save_task_context(
                task.id,
                legacy.revision,
                TaskContext {
                    folder_references: vec![folder],
                    ..legacy
                },
            )
            .await
            .unwrap();
        assert_eq!(upgraded.version, TaskContext::CURRENT_VERSION);
        assert_eq!(upgraded.revision, 1);
        assert_eq!(upgraded.notes, "Legacy notes");
        drop(service);

        let reopened = WorkspaceService::open(database).await.unwrap();
        assert_eq!(reopened.task_context(task.id).await.unwrap(), upgraded);
    }
}
