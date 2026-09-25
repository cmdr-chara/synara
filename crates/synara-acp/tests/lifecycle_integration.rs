//! Lifecycle and cross-task ownership proofs through the generic external-process backend.
use async_trait::async_trait;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use synara_acp::{AcpBackend, AcpTimeouts};
use synara_agent::*;
use synara_core::*;
use synara_runtime::{LaunchSpec, LocalHost};
use tokio::sync::Notify;

#[derive(Default)]
struct RoutedEvents {
    items: Mutex<Vec<(ThreadId, ThreadEvent)>>,
    changed: Notify,
}
#[async_trait]
impl EventSink for RoutedEvents {
    async fn emit(&self, id: ThreadId, event: ThreadEvent) -> AgentResult<()> {
        self.items.lock().unwrap().push((id, event));
        self.changed.notify_waiters();
        Ok(())
    }
}
impl RoutedEvents {
    fn assistant(&self, thread: ThreadId) -> String {
        self.items
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(id, event)| match event {
                ThreadEvent::TextDelta {
                    role: Role::Assistant,
                    text,
                    ..
                } if *id == thread => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
    async fn wait_text(&self, thread: ThreadId, needle: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if self.assistant(thread).contains(needle) {
                    break;
                }
                changed.await;
            }
        })
        .await
        .expect("fixture event was not delivered");
    }
}
struct Harness {
    directory: tempfile::TempDir,
    events: Arc<RoutedEvents>,
    connection: Arc<dyn AgentConnection>,
}
fn spec(profile: &str) -> AgentSpec {
    AgentSpec {
        launch_directory: None,
        id: profile.into(),
        name: profile.into(),
        origin: "lifecycle fixture".into(),
        launch: LaunchSpec {
            command: env!("CARGO_BIN_EXE_synara-acp-fixture").into(),
            args: vec!["--integration-fixture".into(), profile.into()],
            env: Default::default(),
        },
    }
}
fn backend() -> AcpBackend {
    AcpBackend {
        timeouts: AcpTimeouts {
            initialize: Duration::from_secs(5),
            operation: Duration::from_secs(5),
            authentication: Duration::from_secs(5),
            prompt: Duration::from_secs(5),
            cancellation: Duration::from_secs(2),
        },
    }
}
impl Harness {
    async fn start(profile: &str) -> Self {
        Self::with(profile, backend(), Arc::new(DenyInteractions)).await
    }
    async fn with(
        profile: &str,
        backend: AcpBackend,
        interactions: Arc<dyn InteractionHandler>,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let events = Arc::new(RoutedEvents::default());
        let connection = backend
            .connect(
                &spec(profile),
                ConnectionContext {
                    cwd: directory.path().into(),
                    host: Arc::new(LocalHost),
                    events: events.clone(),
                    interactions,
                },
            )
            .await
            .unwrap();
        Self {
            directory,
            events,
            connection,
        }
    }
    fn options(&self, thread: ThreadId) -> SessionOptions {
        SessionOptions::new(thread, self.directory.path().into())
    }
    async fn session(&self, thread: ThreadId) -> Arc<dyn AgentSession> {
        self.connection
            .new_session(self.options(thread))
            .await
            .unwrap()
    }
}
async fn observed_state(connection: &Arc<dyn AgentConnection>, expected: ConnectionState) {
    let mut states = connection.observe();
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if states.borrow_and_update().state == expected {
                break;
            }
            states.changed().await.unwrap();
        }
    })
    .await
    .expect("connection transition was not observed");
}
async fn observed_fixture_handshake(connection: &Arc<dyn AgentConnection>) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if connection.trace().iter().any(|entry| {
                entry.direction == "in"
                    && entry.kind == "notification"
                    && entry.method.as_deref() == Some("<extension method>")
            }) {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("fixture method was not observed");
}
fn calls(connection: &Arc<dyn AgentConnection>, method: &str) -> usize {
    connection
        .trace()
        .iter()
        .filter(|entry| entry.direction == "out" && entry.method.as_deref() == Some(method))
        .count()
}

#[tokio::test]
async fn authentication_requirement_is_distinct_from_an_active_login() {
    let h = Harness::start("auth").await;
    assert!(matches!(
        h.connection.new_session(h.options(ThreadId::new())).await,
        Err(AgentError::AuthenticationRequired)
    ));
    assert_eq!(
        h.connection.info().state,
        ConnectionState::AuthenticationRequired
    );
    let before = calls(&h.connection, "session/new");
    assert!(matches!(
        h.connection.new_session(h.options(ThreadId::new())).await,
        Err(AgentError::AuthenticationRequired)
    ));
    assert_eq!(calls(&h.connection, "session/new"), before);
    h.connection.authenticate("test-login").await.unwrap();
    assert_eq!(h.connection.info().state, ConnectionState::Connected);
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn a_second_login_is_busy_and_disconnect_cannot_be_undone_by_authentication() {
    let h = Harness::start("auth-hold").await;
    let connection = h.connection.clone();
    let authentication = tokio::spawn(async move { connection.authenticate("test-login").await });
    observed_fixture_handshake(&h.connection).await;
    assert_eq!(h.connection.info().state, ConnectionState::Authenticating);
    assert!(matches!(
        h.connection.authenticate("test-login").await,
        Err(AgentError::Busy)
    ));
    assert!(matches!(
        h.connection.new_session(h.options(ThreadId::new())).await,
        Err(AgentError::Busy)
    ));
    h.connection.disconnect().await.unwrap();
    assert!(matches!(
        authentication.await.unwrap(),
        Err(AgentError::Disconnected(_))
    ));
    assert_eq!(h.connection.info().state, ConnectionState::Disconnected);
    assert!(h.connection.info().error.is_none());
    assert!(matches!(
        h.connection.authenticate("test-login").await,
        Err(AgentError::Disconnected(_))
    ));
    assert_eq!(h.connection.info().state, ConnectionState::Disconnected);
}

#[tokio::test]
async fn rejected_login_remains_retryable_and_never_claims_connected() {
    let h = Harness::start("auth-denied").await;
    for _ in 0..2 {
        assert!(matches!(
            h.connection.authenticate("test-login").await,
            Err(AgentError::AuthenticationRequired)
        ));
        assert_eq!(
            h.connection.info().state,
            ConnectionState::AuthenticationRequired
        );
        assert!(h.connection.info().error.is_some());
    }
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn dropped_login_owner_invalidates_the_unacknowledged_operation() {
    let h = Harness::start("auth-hold").await;
    let connection = h.connection.clone();
    let authentication = tokio::spawn(async move { connection.authenticate("test-login").await });
    observed_fixture_handshake(&h.connection).await;
    authentication.abort();
    let _ = authentication.await;
    observed_state(&h.connection, ConnectionState::Failed).await;
    assert!(matches!(
        h.connection.new_session(h.options(ThreadId::new())).await,
        Err(AgentError::Disconnected(_))
    ));
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn authentication_timeout_requires_restart_instead_of_leaving_a_hidden_login() {
    let mut backend = backend();
    backend.timeouts.authentication = Duration::from_millis(60);
    let h = Harness::with("auth-hold", backend, Arc::new(DenyInteractions)).await;
    assert!(matches!(
        h.connection.authenticate("test-login").await,
        Err(AgentError::Timeout)
    ));
    observed_state(&h.connection, ConnectionState::Failed).await;
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn process_exit_during_authentication_is_not_a_successful_login() {
    let h = Harness::start("auth-crash").await;
    assert!(matches!(
        h.connection.authenticate("test-login").await,
        Err(AgentError::Disconnected(_))
    ));
    observed_state(&h.connection, ConnectionState::Failed).await;
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn logout_is_capability_driven_and_returns_to_authentication_required() {
    let h = Harness::start("alpha").await;
    assert!(h.connection.info().capabilities.logout);
    h.connection.logout().await.unwrap();
    assert_eq!(
        h.connection.info().state,
        ConnectionState::AuthenticationRequired
    );
    assert!(matches!(
        h.connection.new_session(h.options(ThreadId::new())).await,
        Err(AgentError::AuthenticationRequired)
    ));
    h.connection.authenticate("test-login").await.unwrap();
    h.session(ThreadId::new()).await.close().await.unwrap();
    h.connection.disconnect().await.unwrap();
    let h = Harness::start("beta").await;
    assert!(matches!(
        h.connection.logout().await,
        Err(AgentError::Unsupported(_))
    ));
    assert_eq!(calls(&h.connection, "logout"), 0);
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn a_thread_cannot_acquire_two_sessions_on_one_connection() {
    let h = Harness::start("alpha").await;
    let thread = ThreadId::new();
    let first = h.session(thread).await;
    assert!(matches!(
        h.connection.new_session(h.options(thread)).await,
        Err(AgentError::Busy)
    ));
    assert!(matches!(
        h.connection
            .restore_session(
                "other-session",
                h.options(thread),
                RestoreMode::ReplayHistory
            )
            .await,
        Err(AgentError::Busy)
    ));
    assert_eq!(calls(&h.connection, "session/new"), 1);
    assert_eq!(calls(&h.connection, "session/load"), 0);
    first.close().await.unwrap();
    h.session(thread).await.close().await.unwrap();
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn sessions_share_a_connection_but_cancellation_and_events_do_not_cross_tasks() {
    let h = Harness::start("alpha").await;
    let one = ThreadId::new();
    let two = ThreadId::new();
    let first = h.session(one).await;
    let second = h.session(two).await;
    assert_ne!(first.id(), second.id());
    let task = first.clone();
    let prompt = tokio::spawn(async move { task.prompt(Prompt::text("hold")).await });
    h.events.wait_text(one, "Started waiting").await;
    assert_eq!(
        second.prompt(Prompt::text("hello")).await.unwrap(),
        "end_turn"
    );
    first.cancel().await.unwrap();
    assert_eq!(prompt.await.unwrap().unwrap(), "cancelled");
    assert!(
        h.events
            .assistant(one)
            .contains("Late update before cancellation")
    );
    assert!(!h.events.assistant(one).contains("Hello"));
    assert_eq!(h.events.assistant(two), "Hello from alpha");
    assert_eq!(
        second.prompt(Prompt::text("hello again")).await.unwrap(),
        "end_turn"
    );
    assert_eq!(calls(&h.connection, "initialize"), 1);
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn overlapping_permissions_are_owned_by_their_original_task() {
    let (broker, mut ui) = InteractionBroker::new();
    let h = Harness::with("alpha", backend(), Arc::new(broker)).await;
    let one = ThreadId::new();
    let two = ThreadId::new();
    let first = h.session(one).await;
    let second = h.session(two).await;
    let a = first.clone();
    let b = second.clone();
    let first_prompt = tokio::spawn(async move { a.prompt(Prompt::text("permission")).await });
    let second_prompt = tokio::spawn(async move { b.prompt(Prompt::text("permission")).await });
    let mut responses = std::collections::HashMap::new();
    for _ in 0..2 {
        let Some(UiInteraction::Permission {
            context, response, ..
        }) = tokio::time::timeout(Duration::from_secs(5), ui.recv())
            .await
            .unwrap()
        else {
            panic!("expected an owned permission");
        };
        responses.insert(context.thread_id, response);
    }
    first.cancel().await.unwrap();
    first_prompt.await.unwrap().unwrap();
    assert!(
        responses
            .remove(&one)
            .unwrap()
            .send(Some("allow".into()))
            .is_err()
    );
    assert!(!responses[&two].is_closed());
    responses
        .remove(&two)
        .unwrap()
        .send(Some("allow".into()))
        .unwrap();
    second_prompt.await.unwrap().unwrap();
    assert_eq!(h.events.assistant(one), "cancelled");
    assert_eq!(h.events.assistant(two), "selected");
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn negotiated_session_lifecycle_covers_titles_restore_list_delete_and_extra_directories() {
    let h = Harness::start("alpha").await;
    let thread = ThreadId::new();
    let extra = h.directory.path().join("extra");
    std::fs::create_dir(&extra).unwrap();
    let mut options = h.options(thread);
    options.additional_directories.push(extra);
    let created = h.connection.new_session(options).await.unwrap();
    assert!(
        h.events
            .items
            .lock()
            .unwrap()
            .iter()
            .any(|(owner, event)| *owner == thread
                && matches!(
                    event,
                    ThreadEvent::TitleChanged { title } if title == "Fixture task"
                ))
    );

    let page = h.connection.list_sessions(None, None).await.unwrap();
    assert!(
        page.sessions
            .iter()
            .any(|item| item.id == created.id() && item.title.as_deref() == Some("Fixture task"))
    );

    let history_thread = ThreadId::new();
    let resume_thread = ThreadId::new();
    let (history, resumed) = tokio::join!(
        h.connection.restore_session(
            "saved-history",
            h.options(history_thread),
            RestoreMode::ReplayHistory
        ),
        h.connection.restore_session(
            "saved-resume",
            h.options(resume_thread),
            RestoreMode::ResumeWithoutReplay
        )
    );
    let history = history.unwrap();
    let resumed = resumed.unwrap();
    assert_eq!(h.events.assistant(history_thread), "Earlier answer");
    assert_eq!(h.events.assistant(resume_thread), "");

    assert!(h.connection.info().capabilities.fork_session);
    let fork_thread = ThreadId::new();
    let forked = created.fork_session(h.options(fork_thread)).await.unwrap();
    assert_ne!(forked.id(), created.id());
    assert_eq!(forked.thread_id(), fork_thread);

    let page = h.connection.list_sessions(None, None).await.unwrap();
    for id in [created.id(), history.id(), resumed.id(), forked.id()] {
        assert!(page.sessions.iter().any(|item| item.id == id));
    }

    forked.close().await.unwrap();
    created.close().await.unwrap();
    history.close().await.unwrap();
    resumed.close().await.unwrap();
    h.connection
        .delete_session("detached-session")
        .await
        .unwrap();
    assert_eq!(calls(&h.connection, "session/load"), 1);
    assert_eq!(calls(&h.connection, "session/resume"), 1);
    assert_eq!(calls(&h.connection, "session/fork"), 1);
    assert_eq!(calls(&h.connection, "session/delete"), 1);
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn fork_requires_both_advertised_fork_and_session_recovery() {
    let h = Harness::start("fork-no-recovery").await;
    assert!(h.connection.info().capabilities.fork_session);
    assert!(!h.connection.info().capabilities.load_session);
    assert!(!h.connection.info().capabilities.resume_session);
    let source = h.session(ThreadId::new()).await;
    assert!(matches!(
        source.fork_session(h.options(ThreadId::new())).await,
        Err(AgentError::Unsupported(_))
    ));
    assert_eq!(calls(&h.connection, "session/fork"), 0);
    source.close().await.unwrap();
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn model_selection_uses_stable_config_options() {
    let h = Harness::start("alpha").await;
    let session = h.session(ThreadId::new()).await;
    session.set_model("alternate").await.unwrap();
    assert_eq!(calls(&h.connection, "session/set_config_option"), 1);
    let option = session
        .configuration()
        .options
        .into_iter()
        .find(|option| option.category.as_deref() == Some("model"))
        .unwrap();
    assert_eq!(
        option.current,
        ConfigValue::Select {
            value: "alternate".into()
        }
    );
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn concurrent_close_is_idempotent_and_does_not_send_two_remote_closes() {
    let h = Harness::start("alpha").await;
    let session = h.session(ThreadId::new()).await;
    let (one, two) = tokio::join!(session.close(), session.close());
    one.unwrap();
    two.unwrap();
    assert_eq!(calls(&h.connection, "session/close"), 1);
    assert!(matches!(
        session.prompt(Prompt::text("no")).await,
        Err(AgentError::Disconnected(_))
    ));
    h.connection.disconnect().await.unwrap();
}

#[tokio::test]
async fn disconnect_during_session_setup_never_returns_an_owned_session() {
    let h = Harness::start("new-hold").await;
    let connection = h.connection.clone();
    let options = h.options(ThreadId::new());
    let setup = tokio::spawn(async move { connection.new_session(options).await });
    observed_fixture_handshake(&h.connection).await;
    h.connection.disconnect().await.unwrap();
    assert!(matches!(
        setup.await.unwrap(),
        Err(AgentError::Disconnected(_))
    ));
    assert_eq!(h.connection.info().state, ConnectionState::Disconnected);
    assert!(matches!(
        h.connection.list_sessions(None, None).await,
        Err(AgentError::Disconnected(_))
    ));
    assert!(matches!(
        h.connection.delete_session("anything").await,
        Err(AgentError::Disconnected(_))
    ));
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn initialization_failure_reaps_the_external_fixture() {
    let directory = tempfile::tempdir().unwrap();
    let pid_file = directory.path().join("fixture.pid");
    let mut spec = spec("init-reject");
    spec.launch.env.insert(
        "SYNARA_FIXTURE_PID_FILE".into(),
        pid_file.to_str().unwrap().into(),
    );
    let result = backend()
        .connect(
            &spec,
            ConnectionContext {
                cwd: directory.path().into(),
                host: Arc::new(LocalHost),
                events: Arc::new(RoutedEvents::default()),
                interactions: Arc::new(DenyInteractions),
            },
        )
        .await;
    assert!(matches!(
        result,
        Err(AgentError::Remote { code: -32603, .. })
    ));
    let pid: u32 = std::fs::read_to_string(pid_file).unwrap().parse().unwrap();
    assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
}
