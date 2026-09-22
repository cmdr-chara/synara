use super::*;
use crate::{DirectModelBinding, ModelSelection, ProviderSettings};
use synara_model::{HttpModelProvider, Message, MessageRole, ModelEvent, ModelProvider};
use synara_runtime::SecretValue;
use tokio_util::sync::CancellationToken;

impl Controller {
    pub async fn save_direct_model_settings(
        &self,
        settings: ProviderSettings,
    ) -> WorkspaceResult<ProviderSettings> {
        let _gate = self
            .integrations_gate
            .try_write()
            .map_err(|_| AgentError::Busy)?;
        if self.closing.load(Ordering::Acquire)
            || self
                .tasks
                .lock()
                .await
                .values()
                .any(|s| s.active.load(Ordering::Acquire))
        {
            return Err(AgentError::Busy.into());
        }
        self.workspace.save_direct_model_settings(settings).await
    }
    /// User-reviewed route change, pinned to the visible transcript sequence and
    /// provider configuration revision. No automatic handoff of hidden state.
    pub async fn select_direct_model(
        &self,
        id: TaskId,
        selection: Option<ModelSelection>,
        settings_revision: u64,
        reviewed_sequence: u64,
    ) -> WorkspaceResult<Option<DirectModelBinding>> {
        let _integrations = self.integrations_gate.read().await;
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        let _creation = slot.creation.lock().await;
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        // Refuse stale review before retiring a live session.
        let task = self.workspace.task(id).await?;
        if self.workspace.thread(task.thread_id).await?.last_sequence != reviewed_sequence {
            return Err(WorkspaceError::Invalid(
                "Conversation changed after review.".into(),
            ));
        }
        if matches!(
            task.state,
            TaskState::Running | TaskState::Waiting | TaskState::Archived
        ) {
            return Err(AgentError::Busy.into());
        }
        let settings = self.workspace.direct_model_settings().await?;
        if settings.revision != settings_revision {
            return Err(WorkspaceError::Invalid(
                "Provider settings changed after review.".into(),
            ));
        }
        if let Some(selection) = &selection {
            let profile = settings
                .providers
                .iter()
                .find(|p| p.id == selection.provider_id)
                .ok_or(WorkspaceError::NotFound)?;
            synara_model::validate_request(
                profile,
                &selection.request(vec![Message::text(
                    MessageRole::User,
                    "Validate options".into(),
                )]),
            )
            .map_err(model_error)?;
        }
        let old = slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
        if let Some(old) = old {
            let closed =
                tokio::time::timeout(std::time::Duration::from_secs(8), old.session.close()).await;
            if !matches!(closed, Ok(Ok(()))) {
                *slot.live.lock().map_err(|_| WorkspaceError::Worker)? = Some(old);
                return Err(match closed {
                    Ok(Err(error)) => error.into(),
                    _ => WorkspaceError::Invalid(
                        "Agent session close timed out. The route was not changed.".into(),
                    ),
                });
            }
        }
        self.revoke_browser_use(id);
        *slot.connection.lock().map_err(|_| WorkspaceError::Worker)? = None;
        self.workspace
            .bind_direct_model(id, selection, settings_revision, reviewed_sequence)
            .await
    }
    pub(super) async fn require_agent_route(&self, id: TaskId) -> WorkspaceResult<()> {
        if self.workspace.direct_model_binding(id).await?.is_some() {
            return Err(WorkspaceError::Invalid("This conversation uses a direct model, not an ACP agent. Review a route change in Direct models settings.".into()));
        }
        Ok(())
    }
    pub async fn direct_model_key(
        &self,
        provider_id: String,
        revision: u64,
        value: Option<SecretValue>,
    ) -> WorkspaceResult<()> {
        let _gate = self
            .integrations_gate
            .try_write()
            .map_err(|_| AgentError::Busy)?;
        if self.closing.load(Ordering::Acquire)
            || self
                .tasks
                .lock()
                .await
                .values()
                .any(|s| s.active.load(Ordering::Acquire))
        {
            return Err(AgentError::Busy.into());
        }
        let settings = self.workspace.direct_model_settings().await?;
        if settings.revision != revision {
            return Err(WorkspaceError::Invalid(
                "Provider settings changed. Review the endpoint again.".into(),
            ));
        }
        let profile = settings
            .providers
            .iter()
            .find(|p| p.id == provider_id)
            .ok_or(WorkspaceError::NotFound)?;
        let reference = profile
            .secret_reference()
            .map_err(|e| WorkspaceError::Invalid(e.to_string()))?;
        if let Some(value) = value {
            if !profile.requires_key
                || value.expose().is_empty()
                || value.expose().len() > 8192
                || !value.expose().iter().all(|b| (33..=126).contains(b))
            {
                return Err(WorkspaceError::Invalid("An API key must be nonempty printable ASCII without whitespace and fit within 8 KiB.".into()));
            }
            self.secrets.write(&reference, value).await?;
        } else {
            self.secrets.delete(&reference).await?;
        }
        Ok(())
    }
    pub async fn discover_direct_models(
        &self,
        provider_id: String,
        revision: u64,
    ) -> WorkspaceResult<Vec<synara_model::ModelInfo>> {
        let _gate = self.integrations_gate.read().await;
        let settings = self.workspace.direct_model_settings().await?;
        if settings.revision != revision {
            return Err(WorkspaceError::Invalid(
                "Provider settings changed. Reload before discovery.".into(),
            ));
        }
        let profile = settings
            .providers
            .iter()
            .find(|p| p.id == provider_id)
            .ok_or(WorkspaceError::NotFound)?;
        HttpModelProvider::new()
            .map_err(model_error)?
            .discover_models(profile, self.secrets.as_ref(), CancellationToken::new())
            .await
            .map_err(model_error)
    }
    pub(super) async fn submit_direct(
        &self,
        id: TaskId,
        binding: DirectModelBinding,
        text: String,
        cancellation: CancellationToken,
    ) -> WorkspaceResult<String> {
        let _integrations = self.integrations_gate.read().await;
        let slot = self.slot(id).await?;
        let _creation = slot.creation.lock().await;
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) || cancellation.is_cancelled() {
            return Err(AgentError::Cancelled.into());
        }
        let settings = self.workspace.direct_model_settings().await?;
        let profile = binding.profile(&settings)?;
        let task = self.workspace.task(id).await?;
        if matches!(
            task.state,
            TaskState::Archived | TaskState::Running | TaskState::Waiting
        ) {
            return Err(AgentError::Busy.into());
        }
        let thread = self.workspace.thread(task.thread_id).await?;
        let mut messages = Vec::new();
        for message in thread.messages {
            let role = match message.role {
                Role::User => MessageRole::User,
                Role::Assistant => MessageRole::Assistant,
                _ => continue,
            };
            messages.push(Message::text(role, message.text));
        }
        messages.push(Message::text(MessageRole::User, text.clone()));
        let request = binding.selection.request(messages);
        synara_model::validate_request(profile, &request).map_err(model_error)?;
        let provider = HttpModelProvider::new().map_err(model_error)?;
        let turn = uuid::Uuid::new_v4().to_string();
        self.workspace
            .record(
                task.thread_id,
                ThreadEvent::PromptStarted { turn: turn.clone() },
            )
            .await?;
        self.workspace
            .record(
                task.thread_id,
                ThreadEvent::TextDelta {
                    message_id: Some(format!("direct:{turn}:user")),
                    role: Role::User,
                    text,
                },
            )
            .await?;
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        let produce = provider.stream(
            profile,
            request,
            self.secrets.as_ref(),
            cancellation.clone(),
            tx,
        );
        let workspace = self.workspace.clone();
        let event_turn = turn.clone();
        let context_limit = profile
            .model(&binding.selection.model_id)
            .map_err(model_error)?
            .capabilities
            .context_window;
        let consume = async move {
            let turn = event_turn;
            let mut finish = None;
            while let Some(event) = rx.recv().await {
                let event = match event {
                    ModelEvent::Text(text) => ThreadEvent::TextDelta {
                        message_id: Some(format!("direct:{turn}:assistant")),
                        role: Role::Assistant,
                        text,
                    },
                    ModelEvent::Reasoning(text) => ThreadEvent::TextDelta {
                        message_id: Some(format!("direct:{turn}:thought")),
                        role: Role::Reasoning,
                        text,
                    },
                    ModelEvent::Usage(usage) => ThreadEvent::UsageChanged {
                        usage: Usage {
                            context_used: usage
                                .input_tokens
                                .zip(usage.output_tokens)
                                .and_then(|(i, o)| i.checked_add(o)),
                            context_limit,
                            input_tokens: usage.input_tokens,
                            output_tokens: usage.output_tokens,
                            cost_amount: None,
                            cost_currency: None,
                        },
                    },
                    ModelEvent::ToolCall(_) => {
                        return Err(WorkspaceError::Invalid(
                            "Direct chat does not grant tools. No tool was executed.".into(),
                        ));
                    }
                    ModelEvent::Finished { reason } => {
                        finish = Some(reason);
                        continue;
                    }
                };
                workspace.record(task.thread_id, event).await?;
            }
            finish.ok_or_else(|| model_error(synara_model::ModelError::Protocol))
        };
        // Both futures belong to this prompt. No detached worker can write into a
        // later turn after Stop, task deletion, route changes or shutdown.
        let (produced, consumed) = tokio::join!(produce, consume);
        let result = produced.map_err(model_error).and(consumed);
        match result {
            Ok(reason) => {
                self.workspace
                    .record(
                        task.thread_id,
                        ThreadEvent::PromptFinished {
                            reason: reason.clone(),
                        },
                    )
                    .await?;
                Ok(reason)
            }
            Err(error) => {
                self.workspace
                    .record(
                        task.thread_id,
                        ThreadEvent::Error {
                            message: error.to_string(),
                            recoverable: false,
                        },
                    )
                    .await?;
                Err(error)
            }
        }
    }
}
fn model_error(error: synara_model::ModelError) -> WorkspaceError {
    WorkspaceError::Invalid(error.to_string())
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
            panic!("a direct-model workflow must never launch an ACP agent")
        }
    }
    async fn setup() -> (tempfile::TempDir, WorkspaceService, Arc<Controller>, Task) {
        let root = tempfile::tempdir().unwrap();
        let workspace = WorkspaceService::open(root.path().join("data.sqlite3"))
            .await
            .unwrap();
        let project = workspace
            .add_local_workspace(root.path().to_path_buf())
            .await
            .unwrap();
        let task = workspace
            .create_task(
                project.id,
                "Direct test".into(),
                crate::default_profiles()[0].id.clone(),
            )
            .await
            .unwrap();
        let controller = Arc::new(Controller::new(
            workspace.clone(),
            Arc::new(NeverLaunch),
            Arc::new(DenyInteractions),
        ));
        (root, workspace, controller, task)
    }
    async fn configure(controller: &Controller, task: &Task, endpoint: String) -> ProviderSettings {
        let mut profile = synara_model::custom_profile_example();
        profile.endpoint = endpoint;
        profile.models[0].id = "fixture".into();
        profile.models[0].capabilities.context_window = Some(8192);
        let settings = controller
            .save_direct_model_settings(ProviderSettings {
                revision: 0,
                providers: vec![profile],
            })
            .await
            .unwrap();
        controller
            .select_direct_model(
                task.id,
                Some(ModelSelection {
                    provider_id: "local-compatible".into(),
                    model_id: "fixture".into(),
                    max_output_tokens: 32,
                    reasoning_effort: None,
                    output: Default::default(),
                }),
                settings.revision,
                0,
            )
            .await
            .unwrap();
        settings
    }
    async fn server(stall: bool) -> (String, tokio::task::JoinHandle<serde_json::Value>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut headers = Vec::new();
            let mut byte = [0];
            while !headers.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).await.unwrap();
                headers.push(byte[0]);
                assert!(headers.len() < 16384);
            }
            let headers = String::from_utf8(headers).unwrap();
            assert!(headers.starts_with("POST /v1/chat/completions"));
            assert!(!headers.to_lowercase().contains("authorization:"));
            let len = headers
                .lines()
                .find_map(|l| {
                    l.to_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|v| v.trim().parse::<usize>().ok())
                })
                .unwrap();
            assert!(len < 1024 * 1024);
            let mut body = vec![0; len];
            socket.read_exact(&mut body).await.unwrap();
            let body = serde_json::from_slice(&body).unwrap();
            let data = concat!(
                "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Native reply\"},\"finish_reason\":null}]}\n\n",
                "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
                "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":12,\"completion_tokens\":3}}\n\n",
                "data: [DONE]\n\n"
            );
            if stall {
                socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n").await.unwrap();
                let part = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n\n";
                socket
                    .write_all(format!("{:x}\r\n{part}\r\n", part.len()).as_bytes())
                    .await
                    .unwrap();
                let _ =
                    tokio::time::timeout(std::time::Duration::from_secs(5), socket.read(&mut byte))
                        .await;
            } else {
                socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{data}",data.len()).as_bytes()).await.unwrap();
            }
            body
        });
        (format!("http://{addr}/v1"), handle)
    }
    #[tokio::test]
    async fn direct_models_settings_selection_and_restart_are_inert_and_scoped() {
        let (root, workspace, controller, task) = setup().await;
        let settings = configure(&controller, &task, "http://127.0.0.1:1/v1".into()).await;
        assert_eq!(settings.revision, 1);
        assert!(
            workspace
                .thread(task.thread_id)
                .await
                .unwrap()
                .messages
                .is_empty()
        );
        assert!(workspace.session(task.thread_id).await.unwrap().is_none());
        let another = workspace
            .create_task(task.project_id, "Other task".into(), task.agent_id.clone())
            .await
            .unwrap();
        assert!(
            workspace
                .direct_model_binding(another.id)
                .await
                .unwrap()
                .is_none()
        );
        assert!(controller.connect(task.id).await.is_err());
        assert!(
            controller
                .authenticate(task.id, "not-an-agent".into())
                .await
                .is_err()
        );
        assert!(controller.restart(task.id).await.is_err());
        assert!(controller.fresh_session(task.id).await.is_err());
        assert!(
            controller
                .switch_agent(task.id, task.agent_id.clone())
                .await
                .is_err()
        );
        let reopened = WorkspaceService::open(root.path().join("data.sqlite3"))
            .await
            .unwrap();
        assert!(
            reopened
                .direct_model_binding(task.id)
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(
            reopened.thread(task.thread_id).await.unwrap().last_sequence,
            0
        );
        assert_eq!(reopened.recover_interrupted().await.unwrap(), 0);
        controller.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn direct_models_stream_through_existing_transcript_and_never_create_agent_sessions() {
        let (_root, workspace, controller, task) = setup().await;
        let (endpoint, server) = server(false).await;
        configure(&controller, &task, endpoint).await;
        assert_eq!(
            controller
                .submit(task.id, "A visible prompt".into())
                .await
                .unwrap(),
            "stop"
        );
        let body = server.await.unwrap();
        assert_eq!(body["messages"][0]["role"], "user");
        assert!(body.get("tools").is_none());
        let thread = workspace.thread(task.thread_id).await.unwrap();
        assert_eq!(thread.messages.len(), 2);
        assert_eq!(thread.messages[0].text, "A visible prompt");
        assert_eq!(thread.messages[1].text, "Native reply");
        assert_eq!(thread.usage.context_used, Some(15));
        assert_eq!(thread.usage.context_limit, Some(8192));
        assert_eq!(thread.usage.cost_amount, None);
        assert!(workspace.session(task.thread_id).await.unwrap().is_none());
        assert!(controller.details(task.id).await.unwrap().is_none());
        assert!(
            !controller
                .slot(task.id)
                .await
                .unwrap()
                .active
                .load(Ordering::Acquire)
        );
    }
    #[tokio::test]
    async fn direct_models_stale_metadata_and_review_do_not_send_or_clone_state() {
        let (_root, workspace, controller, task) = setup().await;
        let mut settings = configure(&controller, &task, "http://127.0.0.1:1/v1".into()).await;
        let binding = workspace
            .direct_model_binding(task.id)
            .await
            .unwrap()
            .unwrap();
        settings.providers[0].endpoint = "http://127.0.0.1:2/v1".into();
        let changed = controller
            .save_direct_model_settings(settings.clone())
            .await
            .unwrap();
        assert!(
            controller
                .save_direct_model_settings(settings)
                .await
                .is_err()
        );
        assert!(
            controller
                .submit(task.id, "Do not send".into())
                .await
                .is_err()
        );
        assert_eq!(
            workspace
                .thread(task.thread_id)
                .await
                .unwrap()
                .last_sequence,
            0
        );
        workspace
            .record(
                task.thread_id,
                ThreadEvent::Notice {
                    message: "New reviewed context".into(),
                },
            )
            .await
            .unwrap();
        assert!(
            controller
                .select_direct_model(task.id, Some(binding.selection), changed.revision, 0)
                .await
                .is_err()
        );
        assert_eq!(
            workspace
                .direct_model_binding(task.id)
                .await
                .unwrap()
                .unwrap()
                .reviewed_profile_sha256,
            binding.reviewed_profile_sha256
        );
    }
    #[tokio::test]
    async fn direct_models_cancel_and_shutdown_release_task_ownership_without_late_completion() {
        for shutdown in [false, true] {
            let (_root, workspace, controller, task) = setup().await;
            let (endpoint, server) = server(true).await;
            let settings = configure(&controller, &task, endpoint).await;
            let running = {
                let c = controller.clone();
                tokio::spawn(async move { c.submit(task.id, "Hold this reply".into()).await })
            };
            tokio::time::timeout(std::time::Duration::from_secs(5), async {
                loop {
                    if workspace
                        .thread(task.thread_id)
                        .await
                        .unwrap()
                        .messages
                        .iter()
                        .any(|m| m.text == "partial")
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            assert!(controller.submit(task.id, "another".into()).await.is_err());
            assert!(
                controller
                    .save_direct_model_settings(settings)
                    .await
                    .is_err()
            );
            if shutdown {
                tokio::time::timeout(std::time::Duration::from_secs(2), controller.shutdown())
                    .await
                    .unwrap()
                    .unwrap();
            } else {
                controller.cancel(task.id).await.unwrap();
            }
            assert!(
                tokio::time::timeout(std::time::Duration::from_secs(2), running)
                    .await
                    .unwrap()
                    .unwrap()
                    .is_err()
            );
            server.await.unwrap();
            let thread = workspace.thread(task.thread_id).await.unwrap();
            assert_eq!(thread.state, TaskState::Failed);
            assert!(thread.messages.iter().any(|m| m.text == "partial"));
            assert!(workspace.session(task.thread_id).await.unwrap().is_none());
        }
    }
    #[tokio::test]
    async fn direct_models_archived_deletion_cleans_binding_and_keeps_source_directory() {
        let (root, workspace, controller, task) = setup().await;
        configure(&controller, &task, "http://127.0.0.1:1/v1".into()).await;
        workspace.archive_task(task.id).await.unwrap();
        controller.delete_archived_task(task.id).await.unwrap();
        let preference = workspace
            .access(move |store| {
                Ok(store.preference_raw(&format!("task-direct-model:{}", task.id))?)
            })
            .await
            .unwrap();
        assert!(preference.is_none());
        assert!(root.path().is_dir());
    }
}
