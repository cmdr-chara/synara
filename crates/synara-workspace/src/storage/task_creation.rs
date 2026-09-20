//! Creation and unsent content commit together. A partial task is never visible.
use super::*;
impl Store {
    pub(crate) fn insert_task_with_draft(&mut self, task: &Task, text: String) -> StorageResult<()> {
        if task.state != TaskState::Ready || text.len() > 1024 * 1024 {
            return Err(StorageError::Limit);
        }
        let data = encode(task)?;
        let draft = encode(&serde_json::json!({"version": 1, "text": text}))?;
        let tx = self.connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Creation only, never UPSERT. Existing identity/content must survive a retry.
        tx.execute("INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?1,?2,?3,?4,?5)",
            params![task.id.to_string(), task.project_id.to_string(), task.thread_id.to_string(), task.updated_at_ms, data])?;
        tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2)",
            params![format!("task-draft:{}", task.id), draft])?;
        tx.commit()?;
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
        let project = service.add_local_workspace(dir.path().into()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let text = "Caffè 日本語\nPlan this work first";
        let mut created = vec![];
        for scope in [TaskScope::Project, TaskScope::Chat, TaskScope::Studio] {
            let task = service.create_scoped_task_with_draft(project.id, "Plan".into(), agent.clone(), scope, text.into()).await.unwrap();
            assert_eq!(task.scope, scope);
            assert_eq!(task.state, TaskState::Ready);
            assert_eq!(service.task_draft(task.id).await.unwrap(), text);
            assert!(service.thread(task.thread_id).await.unwrap().messages.is_empty());
            assert!(service.session(task.thread_id).await.unwrap().is_none());
            created.push(task);
        }
        drop(service);
        let service = WorkspaceService::open(db).await.unwrap();
        for task in created {
            assert_eq!(service.task_draft(task.id).await.unwrap(), text);
            assert!(service.thread(task.thread_id).await.unwrap().turns.is_empty());
            assert!(service.session(task.thread_id).await.unwrap().is_none());
        }
    }
    #[tokio::test]
    async fn invalid_creation_leaves_no_task_and_legacy_creation_still_works() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(dir.path().into()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        for (project, title, agent, draft) in [
            (project.id, "".to_owned(), agent.clone(), "text".into()),
            (project.id, "x".repeat(401), agent.clone(), "text".into()),
            (project.id, "Title".into(), "unknown".into(), "text".into()),
            (ProjectId::new(), "Title".into(), agent.clone(), "text".into()),
            (project.id, "Title".into(), agent.clone(), "x".repeat(1024 * 1024 + 1)),
        ] {
            assert!(service.create_scoped_task_with_draft(project, title, agent, TaskScope::Project, draft).await.is_err());
        }
        assert!(service.catalog().await.unwrap().tasks.is_empty());
        let task = service.create_task(project.id, "Existing caller".into(), agent).await.unwrap();
        assert_eq!(service.task_draft(task.id).await.unwrap(), "");
    }
    #[tokio::test]
    async fn preference_failure_rolls_back_task_insert() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(dir.path().into()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        service.access(|store| {
            store.connection.execute_batch("CREATE TEMP TRIGGER fail_draft BEFORE INSERT ON preferences BEGIN SELECT RAISE(ABORT, 'injected draft write failure'); END;")?;
            Ok(())
        }).await.unwrap();
        assert!(service.create_scoped_task_with_draft(project.id, "Plan".into(), agent, TaskScope::Project, "retain me".into()).await.is_err());
        assert!(service.catalog().await.unwrap().tasks.is_empty());
    }
    #[tokio::test]
    async fn duplicate_creation_never_overwrites_original_text_or_identity() {
        let dir = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(dir.path().into()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service.create_scoped_task_with_draft(project.id, "Plan".into(), agent, TaskScope::Project, "original".into()).await.unwrap();
        let id = task.id;
        service.access(move |store| {
            assert!(store.insert_task_with_draft(&task, "replacement".into()).is_err());
            Ok(())
        }).await.unwrap();
        assert_eq!(service.task_draft(id).await.unwrap(), "original");
        assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
    }
}
