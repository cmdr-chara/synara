use super::*;
use async_trait::async_trait;
use std::{collections::BTreeMap, sync::atomic::Ordering};
use synara_agent::{DenyInteractions, EventSink, InteractionHandler, SessionOptions};
use synara_core::UserInputRequest;
use synara_runtime::LocalHost;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, duplex, split};
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;

struct Events;
#[async_trait]
impl EventSink for Events {
    async fn emit(&self, _: ThreadId, _: ThreadEvent) -> AgentResult<()> {
        Ok(())
    }
}

fn context(directory: &std::path::Path) -> ConnectionContext {
    ConnectionContext {
        host: Arc::new(LocalHost),
        cwd: directory.into(),
        events: Arc::new(Events),
        interactions: Arc::new(DenyInteractions),
    }
}

struct TurnEndingConsent(Arc<SessionState>);
#[async_trait]
impl InteractionHandler for TurnEndingConsent {
    async fn permission(
        &self,
        context: InteractionContext,
        _: PermissionRequest,
    ) -> AgentResult<Option<String>> {
        context.cancelled.cancel();
        self.0.active.store(false, Ordering::Release);
        *self.0.turn.lock().unwrap() = self.0.lifetime.child_token();
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

#[tokio::test]
async fn completed_turn_cannot_promote_stale_approval_to_session_consent() {
    let directory = tempfile::tempdir().unwrap();
    let mut context = context(directory.path());
    let options = SessionOptions::new(ThreadId::new(), directory.path().into());
    let state = SessionState::build(
        "session".into(),
        &options,
        &context,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    state.active.store(true, Ordering::Release);
    context.interactions = Arc::new(TurnEndingConsent(state.clone()));
    let services =
        CallbackServices::new(context, Arc::new(Sessions::default()), ConnectionId::new());
    assert!(matches!(
        services
            .local_approval(&state, "Write one file".into())
            .await,
        Err(AgentError::Cancelled)
    ));
    assert!(!state.lifetime.is_cancelled());
    assert!(!state.turn.lock().unwrap().is_cancelled());
}

struct ExpiredInputConsent(Arc<Notify>);
#[async_trait]
impl InteractionHandler for ExpiredInputConsent {
    async fn permission(
        &self,
        _: InteractionContext,
        _: PermissionRequest,
    ) -> AgentResult<Option<String>> {
        Ok(None)
    }
    async fn input(
        &self,
        context: InteractionContext,
        _: UserInputRequest,
    ) -> AgentResult<UserInputResponse> {
        assert!(matches!(
            context.scope,
            synara_agent::InteractionScope::Connection(_)
        ));
        self.0.notify_one();
        context.cancelled.cancelled().await;
        // A custom handler is untrusted even when it returns an affirmative result.
        Ok(UserInputResponse::Accept {
            values: BTreeMap::new(),
        })
    }
}

#[tokio::test]
async fn login_form_expires_when_its_parent_request_completes() {
    let directory = tempfile::tempdir().unwrap();
    let mut context = context(directory.path());
    let entered = Arc::new(Notify::new());
    context.interactions = Arc::new(ExpiredInputConsent(entered.clone()));
    let services = Arc::new(CallbackServices::new(
        context,
        Arc::new(Sessions::default()),
        ConnectionId::new(),
    ));
    let (client, agent) = duplex(65536);
    let (reader, writer) = split(client);
    let (peer, _incoming) = RpcPeer::start(
        Box::new(reader),
        Box::new(writer),
        Box::new(tokio::io::empty()),
    );
    let caller = peer.clone();
    let pending = tokio::spawn(async move {
        caller
            .request(
                "authenticate",
                json!({"methodId":"login"}),
                Duration::from_secs(5),
            )
            .await
    });
    let mut agent = BufReader::new(agent);
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(2), agent.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    let parent: Value = serde_json::from_str(&line).unwrap();
    let callback_peer = peer.clone();
    let callback = tokio::spawn(async move {
        services
            .elicit(
                json!({
                    "requestId":parent["id"], "mode":"url", "elicitationId":"login-url",
                    "message":"Login", "url":"https://example.com/login"
                }),
                &callback_peer,
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(2), entered.notified())
        .await
        .unwrap();
    agent.get_mut().write_all(format!("{}\n", json!({
        "jsonrpc":"2.0", "id":serde_json::from_str::<Value>(&line).unwrap()["id"], "result":{}
    })).as_bytes()).await.unwrap();
    pending.await.unwrap().unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), callback)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result, json!({"action":"cancel"}));
    assert!(!peer.cancelled().is_cancelled());
}

#[derive(Default)]
struct RecordingEvents(std::sync::Mutex<Vec<(ThreadId, ThreadEvent)>>);
#[async_trait]
impl EventSink for RecordingEvents {
    async fn emit(&self, thread: ThreadId, event: ThreadEvent) -> AgentResult<()> {
        self.0.lock().unwrap().push((thread, event));
        Ok(())
    }
}
#[tokio::test]
async fn opaque_session_named_connection_is_not_mistaken_for_login_scope() {
    let directory = tempfile::tempdir().unwrap();
    let mut context = context(directory.path());
    let events = Arc::new(RecordingEvents::default());
    context.events = events.clone();
    let options = SessionOptions::new(ThreadId::new(), directory.path().into());
    let state = SessionState::build(
        "connection".into(),
        &options,
        &context,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let sessions = Arc::new(Sessions::default());
    sessions
        .states
        .lock()
        .unwrap()
        .insert(state.id.clone(), state);
    let services = CallbackServices::new(context, sessions, ConnectionId::new());
    let (client, _agent) = duplex(65536);
    let (reader, writer) = split(client);
    let (peer, _incoming) = RpcPeer::start(
        Box::new(reader),
        Box::new(writer),
        Box::new(tokio::io::empty()),
    );
    let response = services
        .elicit(
            json!({"sessionId":"connection","mode":"url","elicitationId":"url",
        "url":"https://example.com/login?state=synthetic-canary", "message":"synthetic-canary"}),
            &peer,
        )
        .await
        .unwrap();
    assert_eq!(response, json!({"action":"cancel"}));
    let stored = events.0.lock().unwrap();
    assert_eq!(stored.len(), 2);
    assert!(
        stored
            .iter()
            .all(|(thread, _)| *thread == options.thread_id)
    );
    let ThreadEvent::UserInputRequested { request } = &stored[0].1 else {
        panic!("request expected")
    };
    assert!(request.url.is_none());
    assert!(!request.message.contains("synthetic-canary"));
    assert!(matches!(
        &stored[1].1,
        ThreadEvent::UserInputResolved { .. }
    ));
}

struct BlockedTerminalEvents {
    entered: Notify,
}
#[async_trait]
impl EventSink for BlockedTerminalEvents {
    async fn emit(&self, _: ThreadId, event: ThreadEvent) -> AgentResult<()> {
        if matches!(event, ThreadEvent::TerminalOutput { .. }) {
            self.entered.notify_one();
            std::future::pending().await
        } else {
            Ok(())
        }
    }
}

// Both NativeTerminal implementations make kill a nonblocking stop request.
// A slow output consumer must not keep any of the other owned PTYs alive.
#[tokio::test]
async fn terminal_cleanup_stops_every_process_before_delivering_output() {
    let directory = tempfile::tempdir().unwrap();
    let mut context = context(directory.path());
    let events = Arc::new(BlockedTerminalEvents {
        entered: Notify::new(),
    });
    context.events = events.clone();
    let options = SessionOptions::new(ThreadId::new(), directory.path().into());
    let session = SessionState::build(
        "cleanup".into(),
        &options,
        &context,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let sessions = Arc::new(Sessions::default());
    sessions
        .states
        .lock()
        .unwrap()
        .insert(session.id.clone(), session);
    let services = Arc::new(CallbackServices::new(
        context,
        sessions,
        ConnectionId::new(),
    ));
    let mut terminals = Vec::new();
    for id in ["first", "second"] {
        let mut launch = LaunchSpec::new(std::env::current_exe().unwrap());
        launch.args = vec![
            "--exact".into(),
            "callbacks::lifecycle_tests::terminal_cleanup_child".into(),
            "--ignored".into(),
            "--nocapture".into(),
        ];
        launch
            .env
            .insert("SYNARA_TERMINAL_CLEANUP_CHILD".into(), "1".into());
        let cwd = directory.path().to_owned();
        let terminal = Arc::new(
            tokio::task::spawn_blocking(move || NativeTerminal::spawn(&launch, &cwd, 24, 80))
                .await
                .unwrap()
                .unwrap(),
        );
        services.terminals.lock().await.insert(
            id.into(),
            TerminalEntry {
                session_id: "cleanup".into(),
                terminal: terminal.clone(),
                output_limit: 1024,
            },
        );
        terminals.push(terminal);
    }
    tokio::time::timeout(Duration::from_secs(10), async {
        for terminal in &terminals {
            while !terminal
                .snapshot()
                .unwrap()
                .text
                .contains("terminal-cleanup-ready")
            {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    })
    .await
    .unwrap();
    let worker_services = services.clone();
    let cleanup = tokio::spawn(async move { worker_services.stop_terminals(None).await });
    tokio::time::timeout(Duration::from_secs(5), events.entered.notified())
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), async {
        for terminal in &terminals {
            terminal.wait().await.unwrap();
        }
    })
    .await;
    // Always stop the isolated fixture, including when the unfixed path fails.
    for terminal in &terminals {
        terminal.kill().unwrap();
    }
    cleanup.abort();
    let _ = cleanup.await;
    for terminal in &terminals {
        tokio::time::timeout(Duration::from_secs(5), terminal.wait())
            .await
            .unwrap()
            .unwrap();
    }
    assert!(
        result.is_ok(),
        "blocked diagnostics delayed another terminal's cleanup"
    );
    assert!(services.terminals.lock().await.is_empty());
}

#[test]
#[ignore = "native PTY/ConPTY child fixture invoked only by terminal cleanup tests"]
fn terminal_cleanup_child() {
    use std::io::Write;
    assert_eq!(
        std::env::var("SYNARA_TERMINAL_CLEANUP_CHILD").as_deref(),
        Ok("1")
    );
    writeln!(std::io::stdout(), "terminal-cleanup-ready").unwrap();
    std::io::stdout().flush().unwrap();
    std::thread::sleep(Duration::from_secs(60));
}
