use super::*;
use crate::{CheckpointRestored, CheckpointReview, TaskCheckpoints};
impl Controller {
    /// Reserve the existing task, without creating a session or inheriting authority.
    pub async fn capture_task_checkpoint(
        &self,
        task: TaskId,
        expected_draft: String,
    ) -> WorkspaceResult<TaskCheckpoints> {
        let slot = self.slot(task).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        let _creation = slot.creation.try_lock().map_err(|_| AgentError::Busy)?;
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        self.workspace
            .capture_task_checkpoint(task, expected_draft)
            .await
    }
    pub async fn restore_task_checkpoint(
        &self,
        review: CheckpointReview,
    ) -> WorkspaceResult<CheckpointRestored> {
        let slot = self.slot(review.task()).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        let _creation = slot.creation.try_lock().map_err(|_| AgentError::Busy)?;
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        self.workspace.restore_task_checkpoint(review).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct NeverLaunch;
    #[async_trait::async_trait]
    impl AgentBackend for NeverLaunch {
        async fn connect(
            &self,
            _: &AgentSpec,
            _: ConnectionContext,
        ) -> AgentResult<Arc<dyn AgentConnection>> {
            panic!("Checkpoint operations must never launch an agent");
        }
    }
    #[tokio::test]
    async fn checkpoints_reserve_existing_owner_and_refuse_active_setup_and_shutdown() {
        let root = tempfile::tempdir().unwrap();
        let workspace = WorkspaceService::memory().unwrap();
        let project = workspace
            .add_local_workspace(root.path().into())
            .await
            .unwrap();
        let task = workspace
            .create_task(
                project.id,
                "Original".into(),
                crate::default_profiles()[0].id.clone(),
            )
            .await
            .unwrap();
        let controller = Controller::new(
            workspace.clone(),
            Arc::new(NeverLaunch),
            Arc::new(DenyInteractions),
        );
        let history = controller
            .capture_task_checkpoint(task.id, String::new())
            .await
            .unwrap();
        let review = workspace
            .review_task_checkpoint(task.id, history.items[0].id.clone())
            .await
            .unwrap();
        let slot = controller.slot(task.id).await.unwrap();
        slot.active.store(true, Ordering::Release);
        assert!(
            controller
                .capture_task_checkpoint(task.id, String::new())
                .await
                .is_err()
        );
        assert!(
            controller
                .restore_task_checkpoint(review.clone())
                .await
                .is_err()
        );
        assert!(slot.active.load(Ordering::Acquire));
        slot.active.store(false, Ordering::Release);
        let setup = slot.creation.lock().await;
        assert!(
            controller
                .restore_task_checkpoint(review.clone())
                .await
                .is_err()
        );
        assert!(!slot.active.load(Ordering::Acquire));
        drop(setup);
        controller
            .restore_task_checkpoint(review.clone())
            .await
            .unwrap();
        assert!(!slot.active.load(Ordering::Acquire));
        controller.closing.store(true, Ordering::Release);
        assert!(controller.restore_task_checkpoint(review).await.is_err());
        assert!(workspace.session(task.thread_id).await.unwrap().is_none());
        assert!(
            workspace
                .thread(task.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
    }
}
