//! Control-plane teardown must not wait for a failed presentation/event consumer.
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
use tokio_util::sync::CancellationToken;

enum Stall {
    Cancellation,
    Failure,
    PromptStart,
    UserText,
}
struct Events {
    stall: Stall,
    items: Mutex<Vec<(ThreadId, ThreadEvent)>>,
    changed: Notify,
    entered: Notify,
    release: CancellationToken,
}
impl Events {
    fn new(stall: Stall) -> Self {
        Self {
            stall,
            items: Mutex::new(Vec::new()),
            changed: Notify::new(),
            entered: Notify::new(),
            release: CancellationToken::new(),
        }
    }
    async fn text(&self, thread: ThreadId, expected: &str) {
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let changed = self.changed.notified();
                tokio::pin!(changed);
                changed.as_mut().enable();
                if self.items.lock().unwrap().iter().any(|(id, event)| {
                    *id == thread
                        && matches!(event, ThreadEvent::TextDelta { text, .. }
                        if text.contains(expected))
                }) {
                    return;
                }
                changed.await;
            }
        })
        .await
        .expect("fixture text was not received");
    }
}
#[async_trait]
impl EventSink for Events {
    async fn emit(&self, thread: ThreadId, event: ThreadEvent) -> AgentResult<()> {
        if matches!(
            (&self.stall, &event),
            (Stall::Cancellation, ThreadEvent::CancellationRequested)
                | (Stall::Failure, ThreadEvent::Error { .. })
                | (Stall::PromptStart, ThreadEvent::PromptStarted { .. })
                | (
                    Stall::UserText,
                    ThreadEvent::TextDelta {
                        role: Role::User,
                        ..
                    }
                )
        ) {
            self.entered.notify_one();
            self.release.cancelled().await;
            if matches!(self.stall, Stall::Cancellation | Stall::Failure) {
                return Err(AgentError::EventDelivery);
            }
        }
        self.items.lock().unwrap().push((thread, event));
        self.changed.notify_waiters();
        Ok(())
    }
}

struct FixtureConsent;
#[async_trait]
impl InteractionHandler for FixtureConsent {
    async fn permission(
        &self,
        _: InteractionContext,
        _: PermissionRequest,
    ) -> AgentResult<Option<String>> {
        Ok(Some("allow".into()))
    }
    async fn input(
        &self,
        _: InteractionContext,
        _: UserInputRequest,
    ) -> AgentResult<UserInputResponse> {
        Ok(UserInputResponse::Cancel)
    }
}
struct Harness {
    directory: tempfile::TempDir,
    connection: Arc<dyn AgentConnection>,
    events: Arc<Events>,
}
impl Harness {
    async fn new(stall: Stall) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let events = Arc::new(Events::new(stall));
        let mut launch = LaunchSpec::new(env!("CARGO_BIN_EXE_synara-acp-fixture"));
        launch.args = vec!["--integration-fixture".into(), "alpha".into()];
        let spec = AgentSpec {
            launch_directory: None,
            id: "cleanup".into(),
            name: "Cleanup fixture".into(),
            origin: "isolated test".into(),
            launch,
        };
        let backend = AcpBackend {
            timeouts: AcpTimeouts {
                initialize: Duration::from_secs(5),
                operation: Duration::from_secs(5),
                authentication: Duration::from_secs(5),
                prompt: Duration::from_secs(30),
                cancellation: Duration::from_secs(2),
            },
        };
        let connection = backend
            .connect(
                &spec,
                ConnectionContext {
                    cwd: directory.path().into(),
                    host: Arc::new(LocalHost),
                    events: events.clone(),
                    interactions: Arc::new(FixtureConsent),
                },
            )
            .await
            .unwrap();
        Self {
            directory,
            connection,
            events,
        }
    }
    async fn session(&self) -> Arc<dyn AgentSession> {
        self.connection
            .new_session(SessionOptions::new(
                ThreadId::new(),
                self.directory.path().into(),
            ))
            .await
            .unwrap()
    }
}

#[tokio::test]
async fn cancellation_reaches_agent_before_blocked_diagnostics_and_excludes_next_turn() {
    let h = Harness::new(Stall::Cancellation).await;
    let session = h.session().await;
    let owner = session.clone();
    let mut prompt = tokio::spawn(async move { owner.prompt(Prompt::text("hold")).await });
    h.events.text(session.thread_id(), "Started waiting").await;
    let owner = session.clone();
    let cancel = tokio::spawn(async move { owner.cancel().await });
    tokio::time::timeout(Duration::from_secs(5), h.events.entered.notified())
        .await
        .unwrap();
    let sent =
        h.connection.trace().iter().any(|entry| {
            entry.direction == "out" && entry.method.as_deref() == Some("session/cancel")
        });
    let acknowledged = tokio::time::timeout(Duration::from_secs(2), &mut prompt).await;
    let next_turn_busy = matches!(
        session.prompt(Prompt::text("next")).await,
        Err(AgentError::Busy)
    );
    h.events.release.cancel();
    let cancel_result = cancel.await.unwrap();
    // Never leave fixture work behind on the pre-fix path.
    prompt.abort();
    h.connection.disconnect().await.unwrap();
    assert!(sent, "event backpressure suppressed the wire cancellation");
    assert!(matches!(acknowledged, Ok(Ok(Ok(reason))) if reason == "cancelled"));
    assert!(
        next_turn_busy,
        "late cancellation could modify a newer turn"
    );
    assert!(matches!(cancel_result, Err(AgentError::EventDelivery)));
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn connection_failure_reaps_callback_terminal_before_blocked_error_delivery() {
    let h = Harness::new(Stall::Failure).await;
    let session = h.session().await;
    let owner = session.clone();
    let prompt = tokio::spawn(async move { owner.prompt(Prompt::text("terminal-hold")).await });
    h.events.text(session.thread_id(), "Terminal waiting").await;
    let pid_path = h.directory.path().join("cleanup-terminal.pid");
    let pid = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(text) = tokio::fs::read_to_string(&pid_path).await
                && let Ok(pid) = text.trim().parse::<u32>()
            {
                break pid;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    let other = h.session().await;
    let crash = tokio::spawn(async move { other.prompt(Prompt::text("crash")).await });
    tokio::time::timeout(Duration::from_secs(5), h.events.entered.notified())
        .await
        .unwrap();
    let process = std::path::PathBuf::from(format!("/proc/{pid}"));
    let gone = tokio::time::timeout(Duration::from_secs(5), async {
        while process.exists() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .is_ok();
    h.events.release.cancel();
    h.connection.disconnect().await.unwrap();
    prompt.abort();
    crash.abort();
    let _ = prompt.await;
    let _ = crash.await;
    assert!(gone, "failure diagnostics delayed owned terminal cleanup");
}

async fn cancel_before_dispatch(stall: Stall) {
    let h = Harness::new(stall).await;
    let session = h.session().await;
    let owner = session.clone();
    let mut prompt = tokio::spawn(async move { owner.prompt(Prompt::text("hold")).await });
    tokio::time::timeout(Duration::from_secs(5), h.events.entered.notified())
        .await
        .expect("initial prompt delivery did not reach the test barrier");
    let cancelled = tokio::time::timeout(Duration::from_secs(2), session.cancel()).await;
    h.events.release.cancel();
    let completion = tokio::time::timeout(Duration::from_secs(5), &mut prompt).await;
    let dispatched =
        h.connection.trace().iter().any(|entry| {
            entry.direction == "out" && entry.method.as_deref() == Some("session/prompt")
        });
    let live = h.connection.info().state == ConnectionState::Connected;
    prompt.abort();
    let reuse = if live {
        session.prompt(Prompt::text("next")).await
    } else {
        Err(AgentError::Disconnected(
            "regression closed the connection".into(),
        ))
    };
    let _ = h.connection.disconnect().await;
    assert!(
        matches!(cancelled, Ok(Ok(()))),
        "pre-dispatch cancellation was blocked"
    );
    assert!(
        !dispatched,
        "a prompt was launched after its cancellation had already completed"
    );
    assert!(matches!(completion, Ok(Ok(Ok(reason))) if reason == "cancelled"));
    assert!(live, "local pre-dispatch cancellation broke the connection");
    assert!(
        reuse.is_ok(),
        "the cancelled local turn prevented session reuse"
    );
}

#[tokio::test]
async fn cancelling_during_prompt_start_delivery_never_launches_late_work() {
    cancel_before_dispatch(Stall::PromptStart).await;
}

#[tokio::test]
async fn cancelling_during_user_text_delivery_never_launches_late_work() {
    cancel_before_dispatch(Stall::UserText).await;
}

#[tokio::test]
async fn crash_rejects_new_sessions_and_disconnects_before_blocked_error_delivery() {
    let h = Harness::new(Stall::Failure).await;
    let waiting = h.session().await;
    let owner = waiting.clone();
    let prompt = tokio::spawn(async move { owner.prompt(Prompt::text("hold")).await });
    h.events.text(waiting.thread_id(), "Started waiting").await;
    let other = h.session().await;
    let crash = tokio::spawn(async move { other.prompt(Prompt::text("crash")).await });
    let entered = tokio::time::timeout(Duration::from_secs(5), h.events.entered.notified()).await;
    let failed = matches!(
        h.connection.info().state,
        ConnectionState::Failed | ConnectionState::Exited
    );
    let new_session = h
        .connection
        .new_session(SessionOptions::new(
            ThreadId::new(),
            h.directory.path().into(),
        ))
        .await;
    let disconnect = tokio::time::timeout(Duration::from_secs(5), h.connection.disconnect()).await;
    // Release and collect every fixture owner even on a regression failure.
    h.events.release.cancel();
    prompt.abort();
    crash.abort();
    let _ = prompt.await;
    let _ = crash.await;
    assert!(
        entered.is_ok(),
        "crash never reached the diagnostic barrier"
    );
    assert!(
        failed,
        "blocked diagnostics hid the failed connection state"
    );
    assert!(matches!(new_session, Err(AgentError::Disconnected(_))));
    assert!(
        matches!(disconnect, Ok(Ok(()))),
        "diagnostics blocked explicit disconnect"
    );
}
