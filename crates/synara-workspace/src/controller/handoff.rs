use super::*;
use crate::{HandoffReview, HandoffTarget};

impl Controller {
    /// Create an unsent related task without touching the source live session.
    /// The existing task reservation serializes this with submission and route
    /// changes. The transaction rechecks persisted scope/configuration state.
    pub async fn continue_here(
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
        let task = self
            .workspace
            .continue_handoff_in_place(review, draft)
            .await?;

        // Storage is authoritative once committed. Retire any old ACP session
        // best-effort, but never turn a successful route mutation into a
        // retryable error that could duplicate the user's handoff.
        let old = slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
        *slot.connection.lock().map_err(|_| WorkspaceError::Worker)? = None;
        if let Some(old) = old {
            let _ =
                tokio::time::timeout(std::time::Duration::from_secs(8), old.session.close()).await;
        }
        Ok(task)
    }

    /// Create the normal retained-context child first, then opportunistically
    /// attach an advertised provider-native ACP session fork. Any unsupported,
    /// rejected or ambiguous fork leaves the child intact as the safe fallback.
    pub async fn fork_current_provider(
        &self,
        id: TaskId,
    ) -> WorkspaceResult<(Task, bool, Option<String>)> {
        let _integrations = self.integrations_gate.read().await;
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        self.require_agent_route(id).await?;
        let source = self.workspace.task(id).await?;
        let review = self
            .workspace
            .review_handoff(id, HandoffTarget::Agent(source.agent_id.clone()))
            .await?;
        let mut fallback_draft = review.context().to_owned();
        fallback_draft.push_str(
            "Continue from this point in the new conversation. Review this retained context before sending.",
        );
        let child = self
            .workspace
            .create_handoff(review, fallback_draft)
            .await?;

        let Some(saved_source) = self.workspace.session(source.thread_id).await? else {
            return Ok((
                child,
                false,
                Some(
                    "The source has no saved provider session to copy; retained-context fallback was created."
                        .into(),
                ),
            ));
        };
        if saved_source.agent_id != source.agent_id
            || saved_source.working_directory != source.working_directory
        {
            return Ok((
                child,
                false,
                Some(
                    "The source provider session no longer matches this task; retained-context fallback was created."
                        .into(),
                ),
            ));
        }

        let source_session = match self.session_for(id).await {
            Ok(session) => session,
            Err(error) => {
                return Ok((
                    child,
                    false,
                    Some(format!(
                        "Provider session could not be restored for native fork ({error}); retained-context fallback was created."
                    )),
                ));
            }
        };
        let (profile, connection) = {
            let live = slot.live.lock().map_err(|_| WorkspaceError::Worker)?;
            let live = live.as_ref().ok_or(WorkspaceError::Worker)?;
            (live.profile.clone(), live.connection.clone())
        };
        let capabilities = connection.info().capabilities;
        if !capabilities.fork_session {
            return Ok((
                child,
                false,
                Some(
                    "This ACP provider does not advertise session/fork; retained-context fallback was created."
                        .into(),
                ),
            ));
        }
        if !capabilities.resume_session && !capabilities.load_session {
            return Ok((
                child,
                false,
                Some(
                    "This ACP provider advertises session/fork but cannot reopen forked sessions; retained-context fallback was created."
                        .into(),
                ),
            ));
        }

        let child_slot = match self.slot(child.id).await {
            Ok(slot) => slot,
            Err(error) => {
                return Ok((
                    child,
                    false,
                    Some(format!(
                        "Native fork could not acquire child session ownership ({error}); retained-context fallback was created."
                    )),
                ));
            }
        };
        let forked = match source_session
            .fork_session(SessionOptions::new(
                child.thread_id,
                child.working_directory.clone(),
            ))
            .await
        {
            Ok(session) => session,
            Err(error) => {
                return Ok((
                    child,
                    false,
                    Some(format!(
                        "Provider-native fork failed or was ambiguous ({error}); it was not retried. Retained-context fallback was created."
                    )),
                ));
            }
        };

        let reference = SessionReference {
            agent_id: child.agent_id.clone(),
            remote_id: forked.id().to_owned(),
            working_directory: child.working_directory.clone(),
            title: None,
        };
        if let Err(error) = self
            .workspace
            .save_session(child.thread_id, reference)
            .await
        {
            let _ = forked.close().await;
            return Ok((
                child,
                false,
                Some(format!(
                    "Forked provider session could not be persisted ({error}); it was closed and the retained-context fallback remains."
                )),
            ));
        }

        let native_draft =
            "Continue from the provider-native forked session here. Review before sending."
                .to_string();
        if let Err(error) = self.workspace.save_task_draft(child.id, native_draft).await {
            let _ = self.workspace.forget_session(child.thread_id).await;
            let _ = forked.close().await;
            return Ok((
                child,
                false,
                Some(format!(
                    "Forked provider session was not attached because its visible draft could not be updated ({error}); retained-context fallback remains."
                )),
            ));
        }

        *child_slot
            .connection
            .lock()
            .map_err(|_| WorkspaceError::Worker)? = Some(connection.clone());
        *child_slot.live.lock().map_err(|_| WorkspaceError::Worker)? = Some(LiveSession {
            profile,
            session: forked,
            connection,
        });
        Ok((child, true, None))
    }

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
    struct ForkBackend {
        fork_ok: bool,
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }
    struct ForkConnection {
        info: tokio::sync::watch::Sender<ConnectionInfo>,
        fork_ok: bool,
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }
    struct ForkSession {
        id: String,
        thread: ThreadId,
        fork_ok: bool,
        calls: Arc<std::sync::atomic::AtomicUsize>,
    }
    #[async_trait::async_trait]
    impl AgentBackend for ForkBackend {
        async fn connect(
            &self,
            _: &AgentSpec,
            _: ConnectionContext,
        ) -> AgentResult<Arc<dyn AgentConnection>> {
            let (info, _) = tokio::sync::watch::channel(ConnectionInfo {
                id: ConnectionId::new(),
                state: ConnectionState::Connected,
                identity: None,
                capabilities: AgentCapabilities {
                    fork_session: true,
                    resume_session: true,
                    ..AgentCapabilities::default()
                },
                authentication: vec![],
                host: "Local".into(),
                error: None,
            });
            Ok(Arc::new(ForkConnection {
                info,
                fork_ok: self.fork_ok,
                calls: self.calls.clone(),
            }))
        }
    }
    #[async_trait::async_trait]
    impl AgentConnection for ForkConnection {
        fn info(&self) -> ConnectionInfo {
            self.info.borrow().clone()
        }
        fn observe(&self) -> tokio::sync::watch::Receiver<ConnectionInfo> {
            self.info.subscribe()
        }
        async fn new_session(&self, options: SessionOptions) -> AgentResult<Arc<dyn AgentSession>> {
            Ok(Arc::new(ForkSession {
                id: "source-native".into(),
                thread: options.thread_id,
                fork_ok: self.fork_ok,
                calls: self.calls.clone(),
            }))
        }
        async fn disconnect(&self) -> AgentResult<()> {
            Ok(())
        }
    }
    #[async_trait::async_trait]
    impl AgentSession for ForkSession {
        fn id(&self) -> &str {
            &self.id
        }
        fn thread_id(&self) -> ThreadId {
            self.thread
        }
        fn configuration(&self) -> SessionConfiguration {
            SessionConfiguration::default()
        }
        async fn fork_session(
            &self,
            options: SessionOptions,
        ) -> AgentResult<Arc<dyn AgentSession>> {
            self.calls.fetch_add(1, Ordering::AcqRel);
            if !self.fork_ok {
                return Err(AgentError::Unsupported("fixture fork rejected".into()));
            }
            Ok(Arc::new(ForkSession {
                id: "forked-native".into(),
                thread: options.thread_id,
                fork_ok: self.fork_ok,
                calls: self.calls.clone(),
            }))
        }
        async fn prompt(&self, _: Prompt) -> AgentResult<String> {
            Err(AgentError::Unsupported("fixture prompt".into()))
        }
        async fn cancel(&self) -> AgentResult<()> {
            Ok(())
        }
        async fn close(&self) -> AgentResult<()> {
            Ok(())
        }
    }

    async fn fork_fixture(
        fork_ok: bool,
    ) -> (
        Controller,
        WorkspaceService,
        Task,
        Arc<std::sync::atomic::AtomicUsize>,
    ) {
        let root = tempfile::tempdir().unwrap();
        let workspace = WorkspaceService::memory().unwrap();
        let project = workspace
            .add_local_workspace(root.path().into())
            .await
            .unwrap();
        let task = workspace
            .create_task(
                project.id,
                "Native fork source".into(),
                crate::default_profiles()[0].id.clone(),
            )
            .await
            .unwrap();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let controller = Controller::new(
            workspace.clone(),
            Arc::new(ForkBackend {
                fork_ok,
                calls: calls.clone(),
            }),
            Arc::new(DenyInteractions),
        );
        controller.connect(task.id).await.unwrap();
        (controller, workspace, task, calls)
    }

    #[tokio::test]
    async fn native_provider_fork_attaches_child_session_without_sending() {
        let (controller, workspace, task, calls) = fork_fixture(true).await;
        let source_session = workspace.session(task.thread_id).await.unwrap().unwrap();
        assert_eq!(source_session.remote_id, "source-native");

        let (child, native, reason) = controller.fork_current_provider(task.id).await.unwrap();
        assert!(native);
        assert_eq!(reason, None);
        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert_eq!(workspace.catalog().await.unwrap().tasks.len(), 2);
        assert_eq!(
            workspace
                .session(child.thread_id)
                .await
                .unwrap()
                .unwrap()
                .remote_id,
            "forked-native"
        );
        assert!(
            workspace
                .task_draft(child.id)
                .await
                .unwrap()
                .contains("provider-native forked session")
        );
        assert!(
            workspace
                .thread(child.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
        assert_eq!(
            workspace
                .session(task.thread_id)
                .await
                .unwrap()
                .unwrap()
                .remote_id,
            "source-native"
        );
    }

    #[tokio::test]
    async fn failed_native_provider_fork_keeps_single_retained_context_fallback() {
        let (controller, workspace, task, calls) = fork_fixture(false).await;
        let (child, native, reason) = controller.fork_current_provider(task.id).await.unwrap();
        assert!(!native);
        assert!(reason.unwrap().contains("not retried"));
        assert_eq!(calls.load(Ordering::Acquire), 1);
        assert_eq!(workspace.catalog().await.unwrap().tasks.len(), 2);
        assert!(workspace.session(child.thread_id).await.unwrap().is_none());
        assert!(
            workspace
                .task_draft(child.id)
                .await
                .unwrap()
                .contains("Reviewed continuation from Synara conversation")
        );
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
