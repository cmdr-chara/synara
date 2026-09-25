//! Creation and unsent content commit together. A partial task is never visible.
use super::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagedWorktreeOwnership {
    pub version: u32,
    pub task: TaskId,
    pub project: ProjectId,
    pub workspace: WorkspaceId,
    pub repository_path: PathBuf,
    pub task_path: PathBuf,
    pub branch: String,
    pub remote: bool,
}
impl ManagedWorktreeOwnership {
    pub(crate) fn validate(&self) -> StorageResult<()> {
        let token = self
            .branch
            .strip_prefix("synara/")
            .and_then(|value| uuid::Uuid::parse_str(value).ok())
            .filter(|id| format!("synara/{id}") == self.branch)
            .ok_or(StorageError::Identity)?;
        let expected = format!("worktree-{token}");
        let safe_path = |path: &Path| {
            path.is_absolute()
                && !path.components().any(|component| {
                    matches!(
                        component,
                        std::path::Component::ParentDir | std::path::Component::CurDir
                    )
                })
        };
        if self.version != 1
            || !safe_path(&self.repository_path)
            || !safe_path(&self.task_path)
            || !self.task_path.starts_with(&self.repository_path)
            || self
                .repository_path
                .file_name()
                .and_then(|name| name.to_str())
                != Some(expected.as_str())
        {
            return Err(StorageError::Identity);
        }
        Ok(())
    }
}
pub(crate) fn managed_worktree_key(task: TaskId) -> String {
    format!("managed-worktree:{task}")
}

impl Store {
    pub(crate) fn insert_task_with_draft_and_managed_worktree(
        &mut self,
        task: &Task,
        text: String,
        managed: Option<ManagedWorktreeOwnership>,
    ) -> StorageResult<()> {
        if task.state != TaskState::Ready || text.len() > 1024 * 1024 {
            return Err(StorageError::Limit);
        }
        if let Some(managed) = managed.as_ref() {
            managed.validate()?;
            if managed.task != task.id
                || managed.project != task.project_id
                || managed.task_path != task.working_directory
            {
                return Err(StorageError::Identity);
            }
        }
        let data = encode(task)?;
        let draft = encode(&serde_json::json!({"version": 1, "text": text}))?;
        let managed = managed
            .as_ref()
            .map(|managed| encode(managed))
            .transpose()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Creation only, never UPSERT. Existing identity/content must survive a retry.
        tx.execute(
            "INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?1,?2,?3,?4,?5)",
            params![
                task.id.to_string(),
                task.project_id.to_string(),
                task.thread_id.to_string(),
                task.updated_at_ms,
                data
            ],
        )?;
        tx.execute(
            "INSERT INTO preferences(key,data) VALUES(?1,?2)",
            params![format!("task-draft:{}", task.id), draft],
        )?;
        if let Some(managed) = managed {
            tx.execute(
                "INSERT INTO preferences(key,data) VALUES(?1,?2)",
                params![managed_worktree_key(task.id), managed],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub(crate) fn managed_worktree(
        &self,
        task: TaskId,
    ) -> StorageResult<Option<ManagedWorktreeOwnership>> {
        let raw: Option<String> = self
            .connection
            .query_row(
                "SELECT data FROM preferences WHERE key=?1",
                [managed_worktree_key(task)],
                |row| row.get(0),
            )
            .optional()?;
        let value = raw
            .as_deref()
            .map(decode::<ManagedWorktreeOwnership>)
            .transpose()?;
        if let Some(value) = value.as_ref() {
            value.validate()?;
            if value.task != task {
                return Err(StorageError::Identity);
            }
        }
        Ok(value)
    }

    pub(crate) fn managed_worktrees(&self) -> StorageResult<Vec<ManagedWorktreeOwnership>> {
        let mut query = self.connection.prepare(
            "SELECT data FROM preferences WHERE key LIKE 'managed-worktree:%' ORDER BY key",
        )?;
        let rows = query.query_map([], |row| row.get::<_, String>(0))?;
        let mut values = Vec::new();
        for row in rows {
            if values.len() >= 10_000 {
                return Err(StorageError::Limit);
            }
            let value: ManagedWorktreeOwnership = decode(&row?)?;
            value.validate()?;
            values.push(value);
        }
        Ok(values)
    }

    pub(crate) fn forget_managed_worktree(
        &self,
        expected: &ManagedWorktreeOwnership,
    ) -> StorageResult<()> {
        expected.validate()?;
        let current = self.managed_worktree(expected.task)?;
        if current.as_ref() != Some(expected) {
            return Err(StorageError::Identity);
        }
        let changed = self.connection.execute(
            "DELETE FROM preferences WHERE key=?1",
            [managed_worktree_key(expected.task)],
        )?;
        if changed != 1 {
            return Err(StorageError::Identity);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::WorkspaceService;
    #[tokio::test]
    async fn scoped_creation_and_unicode_draft_restore_without_any_agent_events() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("task.db");
        let service = WorkspaceService::open(db.clone()).await.unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let text = "Caffè 日本語\nPlan this work first";
        let mut created = vec![];
        for scope in [TaskScope::Project, TaskScope::Chat, TaskScope::Studio] {
            let task = service
                .create_scoped_task_with_draft(
                    project.id,
                    "Plan".into(),
                    agent.clone(),
                    scope,
                    text.into(),
                )
                .await
                .unwrap();
            assert_eq!(task.scope, scope);
            assert_eq!(task.state, TaskState::Ready);
            assert_eq!(service.task_draft(task.id).await.unwrap(), text);
            assert!(
                service
                    .thread(task.thread_id)
                    .await
                    .unwrap()
                    .messages
                    .is_empty()
            );
            assert!(service.session(task.thread_id).await.unwrap().is_none());
            created.push(task);
        }
        drop(service);
        let service = WorkspaceService::open(db).await.unwrap();
        for task in created {
            assert_eq!(service.task_draft(task.id).await.unwrap(), text);
            assert!(
                service
                    .thread(task.thread_id)
                    .await
                    .unwrap()
                    .turns
                    .is_empty()
            );
            assert!(service.session(task.thread_id).await.unwrap().is_none());
        }
    }
    #[tokio::test]
    async fn invalid_creation_leaves_no_task_and_legacy_creation_still_works() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        for (project, title, agent, draft) in [
            (project.id, "".to_owned(), agent.clone(), "text".into()),
            (project.id, "x".repeat(401), agent.clone(), "text".into()),
            (project.id, "Title".into(), "unknown".into(), "text".into()),
            (
                ProjectId::new(),
                "Title".into(),
                agent.clone(),
                "text".into(),
            ),
            (
                project.id,
                "Title".into(),
                agent.clone(),
                "x".repeat(1024 * 1024 + 1),
            ),
        ] {
            assert!(
                service
                    .create_scoped_task_with_draft(project, title, agent, TaskScope::Project, draft)
                    .await
                    .is_err()
            );
        }
        assert!(service.catalog().await.unwrap().tasks.is_empty());
        let task = service
            .create_task(project.id, "Existing caller".into(), agent)
            .await
            .unwrap();
        assert_eq!(service.task_draft(task.id).await.unwrap(), "");
    }
    #[tokio::test]
    async fn preference_failure_rolls_back_task_insert() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        service.access(|store| {
            store.connection.execute_batch("CREATE TEMP TRIGGER fail_draft BEFORE INSERT ON preferences BEGIN SELECT RAISE(ABORT, 'injected draft write failure'); END;").map_err(StorageError::from)?;
            Ok(())
        }).await.unwrap();
        assert!(
            service
                .create_scoped_task_with_draft(
                    project.id,
                    "Plan".into(),
                    agent,
                    TaskScope::Project,
                    "retain me".into()
                )
                .await
                .is_err()
        );
        assert!(service.catalog().await.unwrap().tasks.is_empty());
    }
    #[tokio::test]
    async fn duplicate_creation_never_overwrites_original_text_or_identity() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service
            .create_scoped_task_with_draft(
                project.id,
                "Plan".into(),
                agent,
                TaskScope::Project,
                "original".into(),
            )
            .await
            .unwrap();
        let id = task.id;
        service
            .access(move |store| {
                assert!(
                    store
                        .insert_task_with_draft(&task, "replacement".into())
                        .is_err()
                );
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(service.task_draft(id).await.unwrap(), "original");
        assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
    }
}
