use async_trait::async_trait;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use synara_acp::{AcpBackend, AcpTimeouts};
use synara_agent::*;
use synara_core::*;
use synara_runtime::{LaunchSpec, LocalHost};

#[derive(Default)]
struct Events(Mutex<Vec<ThreadEvent>>);
#[async_trait]
impl EventSink for Events {
    async fn emit(&self, _: ThreadId, event: ThreadEvent) -> AgentResult<()> {
        self.0.lock().unwrap().push(event);
        Ok(())
    }
}
impl Events {
    fn assistant(&self) -> String {
        self.0
            .lock()
            .unwrap()
            .iter()
            .filter_map(|event| match event {
                ThreadEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                    ..
                } => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
    fn thread(&self, id: ThreadId) -> Thread {
        let mut thread = Thread::new(id);
        for (index, event) in self.0.lock().unwrap().iter().enumerate() {
            let envelope = EventEnvelope {
                id: EventId::new(),
                thread_id: id,
                sequence: index as u64 + 1,
                timestamp_ms: 0,
                event: event.clone(),
            };
            thread.apply(&envelope).unwrap();
        }
        thread
    }
}
struct Consent;
#[async_trait]
impl InteractionHandler for Consent {
    async fn permission(
        &self,
        _: InteractionContext,
        request: PermissionRequest,
    ) -> AgentResult<Option<String>> {
        Ok(request
            .choices
            .iter()
            .find(|choice| choice.kind == PermissionKind::AllowOnce)
            .map(|choice| choice.id.clone()))
    }
    async fn input(
        &self,
        _: InteractionContext,
        _: UserInputRequest,
    ) -> AgentResult<UserInputResponse> {
        Ok(UserInputResponse::Accept {
            values: [("name".into(), InputValue::Text("Ada".into()))].into(),
        })
    }
}
struct Harness {
    connection: Arc<dyn AgentConnection>,
    events: Arc<Events>,
    directory: tempfile::TempDir,
    thread_id: ThreadId,
}
impl Harness {
    async fn start(profile: &str, consent: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let events = Arc::new(Events::default());
        let spec = AgentSpec {
            launch_directory: None,
            id: profile.into(),
            name: profile.into(),
            origin: "test fixture".into(),
            launch: LaunchSpec {
                command: env!("CARGO_BIN_EXE_synara-acp-fixture").into(),
                args: vec!["--integration-fixture".into(), profile.into()],
                env: Default::default(),
            },
        };
        let timeouts = AcpTimeouts {
            initialize: Duration::from_secs(3),
            operation: Duration::from_secs(3),
            authentication: Duration::from_secs(3),
            prompt: Duration::from_millis(1500),
            cancellation: Duration::from_secs(1),
        };
        let connection = AcpBackend { timeouts }
            .connect(
                &spec,
                ConnectionContext {
                    cwd: directory.path().into(),
                    host: Arc::new(LocalHost),
                    events: events.clone(),
                    interactions: if consent {
                        Arc::new(Consent)
                    } else {
                        Arc::new(DenyInteractions)
                    },
                },
            )
            .await
            .unwrap();
        Self {
            connection,
            events,
            directory,
            thread_id: ThreadId::new(),
        }
    }
    fn options(&self) -> SessionOptions {
        SessionOptions::new(self.thread_id, self.directory.path().into())
    }
    async fn session(&self) -> Arc<dyn AgentSession> {
        self.connection.new_session(self.options()).await.unwrap()
    }
    async fn stop(self) {
        self.connection.disconnect().await.unwrap();
    }
}

#[tokio::test]
async fn two_different_agents_share_the_same_backend_and_normalized_transcript() {
    for profile in ["alpha", "beta"] {
        let h = Harness::start(profile, false).await;
        assert_eq!(
            h.connection.info().identity.unwrap().name,
            format!("fixture-{profile}")
        );
        assert_eq!(
            h.connection.info().capabilities.load_session,
            profile == "alpha"
        );
        let session = h.session().await;
        assert_eq!(session.configuration().options[0].choices[0].value, profile);
        assert_eq!(
            session.prompt(Prompt::text("hello")).await.unwrap(),
            "end_turn"
        );
        let thread = h.events.thread(h.thread_id);
        assert_eq!(thread.title, "Fixture task");
        assert_eq!(h.events.assistant(), format!("Hello from {profile}"));
        assert_eq!(thread.tools["tool-1"].status, ToolStatus::Completed);
        assert!(!thread.tools["tool-1"].output.is_empty());
        assert_eq!(thread.state, TaskState::Completed);
        assert!(
            h.connection
                .trace()
                .iter()
                .any(|trace| trace.method.as_deref() == Some("initialize"))
        );
        session.close().await.unwrap();
        h.stop().await;
    }
}
#[tokio::test]
async fn authentication_is_agent_owned_and_retryable() {
    let h = Harness::start("auth", false).await;
    assert!(matches!(
        h.connection.new_session(h.options()).await,
        Err(AgentError::AuthenticationRequired)
    ));
    assert_eq!(h.connection.info().state, ConnectionState::Authenticating);
    h.connection.authenticate("test-login").await.unwrap();
    let session = h.session().await;
    assert!(session.prompt(Prompt::text("hello")).await.is_ok());
    h.stop().await;
}
#[tokio::test]
async fn permissions_never_default_to_approval() {
    for consent in [false, true] {
        let h = Harness::start("alpha", consent).await;
        h.session()
            .await
            .prompt(Prompt::text("permission"))
            .await
            .unwrap();
        assert_eq!(
            h.events.assistant(),
            if consent { "selected" } else { "cancelled" }
        );
        assert!(h.events.thread(h.thread_id).permissions.is_empty());
        h.stop().await;
    }
}
#[tokio::test]
async fn cancellation_waits_for_the_agent_and_keeps_late_updates() {
    let h = Harness::start("alpha", false).await;
    let session = h.session().await;
    let worker = session.clone();
    let task = tokio::spawn(async move { worker.prompt(Prompt::text("hold")).await });
    tokio::time::timeout(Duration::from_secs(2), async {
        while !h.events.assistant().contains("Started") {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        session.prompt(Prompt::text("overlap")).await,
        Err(AgentError::Busy)
    ));
    session.cancel().await.unwrap();
    assert_eq!(task.await.unwrap().unwrap(), "cancelled");
    assert!(h.events.assistant().contains("Late update"));
    assert!(session.prompt(Prompt::text("again")).await.is_ok());
    h.stop().await;
}
#[tokio::test]
async fn replay_finishes_after_history_and_duplicate_ownership_is_rejected() {
    let h = Harness::start("alpha", false).await;
    let session = h
        .connection
        .restore_session("saved", h.options(), RestoreMode::ReplayHistory)
        .await
        .unwrap();
    assert!(matches!(
        h.connection
            .restore_session("saved", h.options(), RestoreMode::ReplayHistory)
            .await,
        Err(AgentError::Busy)
    ));
    let thread = h.events.thread(h.thread_id);
    assert_eq!(thread.messages.len(), 2);
    assert_eq!(thread.messages[1].text, "Earlier answer");
    assert_eq!(thread.state, TaskState::Ready);
    assert_eq!(
        h.connection
            .list_sessions(None, None)
            .await
            .unwrap()
            .sessions
            .len(),
        1
    );
    session.close().await.unwrap();
    h.connection.delete_session("saved").await.unwrap();
    h.stop().await;
}
#[tokio::test]
async fn failed_history_replay_preserves_previous_messages_and_reports_failure() {
    let h = Harness::start("alpha", false).await;
    h.events
        .emit(
            h.thread_id,
            ThreadEvent::TextDelta {
                role: Role::User,
                message_id: None,
                text: "Keep this".into(),
            },
        )
        .await
        .unwrap();
    assert!(
        h.connection
            .restore_session("bad-history", h.options(), RestoreMode::ReplayHistory)
            .await
            .is_err()
    );
    let thread = h.events.thread(h.thread_id);
    assert_eq!(thread.messages.len(), 1);
    assert_eq!(thread.messages[0].text, "Keep this");
    assert_eq!(thread.state, TaskState::Failed);
    h.stop().await;
}
#[tokio::test]
async fn selectors_use_advertised_values_and_update_configuration() {
    let h = Harness::start("alpha", false).await;
    let session = h.session().await;
    assert!(session.set_model("not-offered").await.is_err());
    session.set_model("alternate").await.unwrap();
    assert_eq!(
        session.configuration().options[0].current,
        ConfigValue::Select {
            value: "alternate".into()
        }
    );
    session.set_mode("plan").await.unwrap();
    assert_eq!(
        session.configuration().current_mode.as_deref(),
        Some("plan")
    );
    session.close().await.unwrap();
    assert!(session.set_mode("code").await.is_err());
    h.stop().await;
}
#[tokio::test]
async fn filesystem_callbacks_obey_consent_and_session_containment() {
    for consent in [false, true] {
        let h = Harness::start("alpha", consent).await;
        let session = h.session().await;
        session.prompt(Prompt::text("files")).await.unwrap();
        assert_eq!(h.directory.path().join("created.txt").exists(), consent);
        if consent {
            assert_eq!(
                std::fs::read_to_string(h.directory.path().join("created.txt")).unwrap(),
                "native 🦀\n"
            );
        }
        session.prompt(Prompt::text("escape")).await.unwrap();
        session.prompt(Prompt::text("unknown-owner")).await.unwrap();
        assert!(!h.events.assistant().contains("unexpected access"));
        h.stop().await;
    }
}
#[cfg(unix)]
#[tokio::test]
async fn terminal_callbacks_release_live_resources_but_keep_history() {
    let h = Harness::start("alpha", true).await;
    let session = h.session().await;
    session.prompt(Prompt::text("terminal")).await.unwrap();
    assert!(h.events.assistant().contains("terminal-proof"));
    assert!(h.events.assistant().contains("released"));
    let thread = h.events.thread(h.thread_id);
    assert!(
        thread
            .terminals
            .values()
            .any(|terminal| terminal.text.contains("terminal-proof")
                && terminal.exit_code == Some(0))
    );
    h.stop().await;
}
#[tokio::test]
async fn structured_user_input_round_trips_without_provider_code() {
    let h = Harness::start("alpha", true).await;
    h.session()
        .await
        .prompt(Prompt::text("input"))
        .await
        .unwrap();
    assert!(h.events.assistant().contains("Ada"));
    assert!(h.events.thread(h.thread_id).inputs.is_empty());
    h.stop().await;
}
#[tokio::test]
async fn process_crashes_and_malformed_frames_fail_pending_prompts() {
    for command in ["crash", "malformed"] {
        let h = Harness::start("alpha", false).await;
        let session = h.session().await;
        assert!(session.prompt(Prompt::text(command)).await.is_err());
        h.stop().await;
    }
}
#[tokio::test]
async fn timed_out_prompt_does_not_leave_an_invisible_agent_run() {
    let h = Harness::start("alpha", false).await;
    let session = h.session().await;
    assert!(matches!(
        session.prompt(Prompt::text("timeout")).await,
        Err(AgentError::Timeout)
    ));
    assert!(session.prompt(Prompt::text("again")).await.is_err());
    h.stop().await;
}
