use super::*;
use crate::HandoffReview;

impl Controller {
    /// Create an unsent related task without touching the source live session.
    /// The existing task reservation serializes this with submission and route
    /// changes. The transaction rechecks persisted scope/configuration state.
    pub async fn continue_with(
        &self,
        review: HandoffReview,
        draft: String,
    ) -> WorkspaceResult<Task> {
        let _integrations = self.integrations_gate.read().await;
        let slot = self.slot(review.source().id).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        let _creation = slot.creation.lock().await;
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        self.workspace.create_handoff(review, draft).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HandoffTarget;
    struct NeverLaunch;
    #[async_trait::async_trait]
    impl AgentBackend for NeverLaunch {
        async fn connect(
            &self,
            _: &AgentSpec,
            _: ConnectionContext,
        ) -> AgentResult<Arc<dyn AgentConnection>> {
            panic!("reviewed continuation must not connect or start a provider");
        }
    }
    #[tokio::test]
    async fn handoff_respects_active_reservation_and_shutdown_without_launching() {
        let root = tempfile::tempdir().unwrap();
        let workspace = WorkspaceService::memory().unwrap();
        let project = workspace
            .add_local_workspace(root.path().into())
            .await
            .unwrap();
        let task = workspace
            .create_task(
                project.id,
                "Source".into(),
                crate::default_profiles()[0].id.clone(),
            )
            .await
            .unwrap();
        let controller = Controller::new(
            workspace.clone(),
            Arc::new(NeverLaunch),
            Arc::new(DenyInteractions),
        );
        let review = workspace
            .review_handoff(task.id, HandoffTarget::Agent(task.agent_id.clone()))
            .await
            .unwrap();
        let slot = controller.slot(task.id).await.unwrap();
        slot.active.store(true, Ordering::Release);
        assert!(
            controller
                .continue_with(review.clone(), "draft".into())
                .await
                .is_err()
        );
        slot.active.store(false, Ordering::Release);
        let child = controller
            .continue_with(review, "reviewed request".into())
            .await
            .unwrap();
        assert!(!slot.active.load(Ordering::Acquire));
        assert!(slot.connection().unwrap().is_none());
        assert!(workspace.session(child.thread_id).await.unwrap().is_none());
        let review = workspace
            .review_handoff(task.id, HandoffTarget::Agent(task.agent_id))
            .await
            .unwrap();
        controller.closing.store(true, Ordering::Release);
        assert!(
            controller
                .continue_with(review, "draft".into())
                .await
                .is_err()
        );
        assert_eq!(workspace.catalog().await.unwrap().tasks.len(), 2);
    }
}
