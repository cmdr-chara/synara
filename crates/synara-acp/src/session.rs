use crate::{backend::AcpSession, wire};
use async_trait::async_trait;
use serde_json::json;
use std::sync::atomic::Ordering;
use synara_agent::*;
use synara_core::*;
use uuid::Uuid;

#[async_trait]
impl AgentSession for AcpSession {
    fn id(&self) -> &str {
        &self.state.id
    }
    fn thread_id(&self) -> ThreadId {
        self.state.thread_id
    }
    fn configuration(&self) -> SessionConfiguration {
        self.state.configuration.read().unwrap().clone()
    }
    async fn fork_session(&self, options: SessionOptions) -> AgentResult<Arc<dyn AgentSession>> {
        self.ensure_open()?;
        self.connection.fork_session(self.id(), options).await
    }
    async fn prompt(&self, prompt: Prompt) -> AgentResult<String> {
        let cancellation = self
            .state
            .cancellation_gate
            .try_lock()
            .map_err(|_| AgentError::Busy)?;
        let _operation = self
            .state
            .prompt_gate
            .try_lock()
            .map_err(|_| AgentError::Busy)?;
        self.connection.ensure_connected()?;
        if self.state.closed.load(Ordering::Acquire) {
            return Err(AgentError::Disconnected("session is closed".into()));
        }
        let content = wire::prompt_parts(&prompt, &self.connection.capabilities())?;
        let turn = Uuid::new_v4().to_string();
        *self.state.turn.lock().unwrap() = self.state.lifetime.child_token();
        self.state.active.store(true, Ordering::Release);
        let guard = ActivePrompt {
            session: self,
            complete: false,
        };
        drop(cancellation);
        self.state
            .emit(
                &self.connection.context,
                ThreadEvent::PromptStarted { turn: turn.clone() },
            )
            .await?;
        let text = prompt
            .parts
            .iter()
            .map(|part| match part {
                PromptPart::Text(text) => text.clone(),
                PromptPart::Image { .. } | PromptPart::MediaImage(_) => "[Image]".into(),
                PromptPart::Audio { .. } => "[Audio]".into(),
                PromptPart::Context { uri, .. } => format!("[Context: {uri}]"),
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.state
            .emit(
                &self.connection.context,
                ThreadEvent::TextDelta {
                    message_id: Some(format!("user-{turn}")),
                    role: Role::User,
                    text,
                },
            )
            .await?;
        // Persist the exact locally submitted bytes with their message before any
        // external write. This survives Recent pruning and never claims delivery.
        for part in &prompt.parts {
            let image = match part {
                PromptPart::MediaImage(image) => Some(image.clone()),
                PromptPart::Image { base64, mime_type } => Some(synara_core::TranscriptImage {
                    source: synara_core::ImageSource::Uploaded,
                    mime_type: mime_type.clone(),
                    base64: base64.clone(),
                }),
                _ => None,
            };
            if let Some(image) = image {
                self.state
                    .emit(
                        &self.connection.context,
                        ThreadEvent::ImageMessage {
                            message_id: Some(format!("user-{turn}")),
                            role: Role::User,
                            image,
                        },
                    )
                    .await?;
            }
        }
        let cancellation = self.state.turn.lock().unwrap().clone();
        // Cancellation may have completed while the durable prompt events were
        // being delivered. Serialize enqueue against cancel, not response wait:
        // either no prompt is queued, or its frame precedes session/cancel.
        let pending = tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(AgentError::Cancelled),
            gate = self.state.cancellation_gate.lock() => {
                let _gate = gate;
                if cancellation.is_cancelled() {
                    Err(AgentError::Cancelled)
                } else {
                    self.connection.begin_call(
                        "session/prompt",
                        json!({"sessionId":self.id(),"prompt":content}),
                        self.connection.timeouts.prompt,
                    ).await
                }
            }
        };
        let response = match pending {
            Ok(pending) => {
                let operation = pending.wait();
                tokio::pin!(operation);
                let result = tokio::select! {
                    result = &mut operation => result,
                    () = cancellation.cancelled() => {
                        match tokio::time::timeout(
                            self.connection.timeouts.cancellation, &mut operation
                        ).await {
                            Ok(result) => result,
                            Err(_) => {
                                self.connection.rpc.fail("agent did not acknowledge cancellation");
                                Err(AgentError::Disconnected(
                                    "Agent did not acknowledge cancellation. Restart the connection.".into()
                                ))
                            }
                        }
                    }
                };
                self.connection.call_result("session/prompt", result)
            }
            Err(error) => Err(error),
        };
        let response = match response {
            Ok(result) => {
                self.connection.rpc.barrier().await?;
                Ok(wire::string(&result, "stopReason")?.to_owned())
            }
            Err(AgentError::Cancelled) if cancellation.is_cancelled() => Ok("cancelled".into()),
            Err(error) => {
                if matches!(error, AgentError::Timeout) {
                    self.connection
                        .rpc
                        .fail("prompt deadline expired before agent completion");
                }
                Err(error)
            }
        };
        self.state.turn.lock().unwrap().cancel();
        self.state.active.store(false, Ordering::Release);
        let mut guard = guard;
        guard.complete = true;
        match response {
            Ok(reason) => {
                self.state
                    .emit(
                        &self.connection.context,
                        ThreadEvent::PromptFinished {
                            reason: reason.clone(),
                        },
                    )
                    .await?;
                Ok(reason)
            }
            Err(error) => {
                self.state
                    .emit(
                        &self.connection.context,
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
    async fn cancel(&self) -> AgentResult<()> {
        // Do not let a late cancellation event clear the next turn's interactions.
        let _cancellation = self.state.cancellation_gate.lock().await;
        if !self.state.active.load(Ordering::Acquire) {
            return Ok(());
        }
        self.state.turn.lock().unwrap().cancel();
        // Control-plane cancellation and resource cleanup must run even when the
        // event consumer is blocked or has failed. Report delivery separately.
        let result = self
            .connection
            .rpc
            .notify("session/cancel", json!({"sessionId":self.id()}))
            .await;
        let cleanup = self
            .connection
            .callbacks
            .stop_terminals(Some(self.id()))
            .await;
        let delivered = self
            .state
            .emit(&self.connection.context, ThreadEvent::CancellationRequested)
            .await;
        result?;
        cleanup?;
        delivered
    }
    async fn set_option(&self, id: &str, value: ConfigValue) -> AgentResult<SessionConfiguration> {
        let _mutation = self.state.mutation_gate.lock().await;
        self.ensure_open()?;
        let configuration = self.configuration();
        let option = configuration
            .options
            .iter()
            .find(|option| option.id == id)
            .ok_or_else(|| AgentError::Unsupported("configuration option".into()))?;
        let (value, value_type) = match (&option.current, value) {
            (ConfigValue::Boolean { .. }, ConfigValue::Boolean { value }) => {
                (json!(value), Some("boolean"))
            }
            (ConfigValue::Select { .. }, ConfigValue::Select { value })
                if option.choices.iter().any(|choice| choice.value == value) =>
            {
                (json!(value), None)
            }
            _ => {
                return Err(wire::invalid(
                    "configuration value is not one of the advertised choices",
                ));
            }
        };
        let mut params = json!({"sessionId":self.id(),"configId":id,"value":value});
        if let Some(value_type) = value_type {
            params["type"] = json!(value_type);
        }
        let result = self
            .connection
            .call(
                "session/set_config_option",
                params,
                self.connection.timeouts.operation,
            )
            .await?;
        self.connection.rpc.barrier().await?;
        let _update = self.state.update_gate.lock().await;
        let configuration = wire::configuration(&result, &self.configuration())?;
        *self.state.configuration.write().unwrap() = configuration.clone();
        self.state
            .emit(
                &self.connection.context,
                ThreadEvent::ConfigurationChanged {
                    configuration: configuration.clone(),
                },
            )
            .await?;
        Ok(configuration)
    }
    async fn set_mode(&self, id: &str) -> AgentResult<()> {
        let _mutation = self.state.mutation_gate.lock().await;
        self.ensure_open()?;
        if !self.configuration().modes.iter().any(|mode| mode.id == id) {
            return Err(AgentError::Unsupported("session mode".into()));
        }
        self.connection
            .call(
                "session/set_mode",
                json!({"sessionId":self.id(),"modeId":id}),
                self.connection.timeouts.operation,
            )
            .await?;
        self.connection.rpc.barrier().await?;
        let _update = self.state.update_gate.lock().await;
        let mut configuration = self.configuration();
        configuration.current_mode = Some(id.into());
        *self.state.configuration.write().unwrap() = configuration.clone();
        self.state
            .emit(
                &self.connection.context,
                ThreadEvent::ConfigurationChanged { configuration },
            )
            .await
    }
    async fn set_model(&self, id: &str) -> AgentResult<()> {
        // ACP removed the never-stabilized session/set_model method. Stable v1
        // model selection is represented by a model-category config option.
        let configuration = self.configuration();
        let option = configuration
            .options
            .iter()
            .find(|option| option.category.as_deref() == Some("model"))
            .ok_or_else(|| AgentError::Unsupported("model configuration option".into()))?;
        self.set_option(&option.id, ConfigValue::Select { value: id.into() })
            .await?;
        Ok(())
    }
    async fn close(&self) -> AgentResult<()> {
        if self.state.closed.load(Ordering::Acquire) {
            return Ok(());
        }
        let _mutation = self.state.mutation_gate.lock().await;
        if self.state.closed.load(Ordering::Acquire) {
            return Ok(());
        }
        self.cancel().await?;
        let _operation = tokio::time::timeout(
            self.connection.timeouts.cancellation,
            self.state.prompt_gate.lock(),
        )
        .await
        .map_err(|_| AgentError::Timeout)?;
        if self.connection.capabilities().close_session {
            self.connection
                .call(
                    "session/close",
                    json!({"sessionId":self.id()}),
                    self.connection.timeouts.operation,
                )
                .await?;
        }
        let cleanup = self
            .connection
            .callbacks
            .stop_terminals(Some(self.id()))
            .await;
        self.state.closed.store(true, Ordering::Release);
        self.state.lifetime.cancel();
        self.connection
            .sessions
            .states
            .lock()
            .unwrap()
            .remove(self.id());
        let delivered = self
            .state
            .emit(
                &self.connection.context,
                ThreadEvent::SessionStatus {
                    status: "Session closed".into(),
                },
            )
            .await;
        cleanup?;
        delivered
    }
}
struct ActivePrompt<'a> {
    session: &'a AcpSession,
    complete: bool,
}
impl Drop for ActivePrompt<'_> {
    fn drop(&mut self) {
        self.session.state.active.store(false, Ordering::Release);
        if !self.complete {
            self.session.state.turn.lock().unwrap().cancel();
            // An aborted caller must not leave an invisible model/tool run in the external process.
            self.session
                .connection
                .rpc
                .fail("prompt owner was dropped before completion");
        }
    }
}

impl AcpSession {
    fn ensure_open(&self) -> AgentResult<()> {
        self.connection.ensure_connected()?;
        if self.state.closed.load(Ordering::Acquire) {
            return Err(AgentError::Disconnected("session is closed".into()));
        }
        Ok(())
    }
}
