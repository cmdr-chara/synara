use super::*;
use async_trait::async_trait;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use synara_agent::*;
use synara_core::{
    AgentCapabilities, ConnectionId, ConnectionState, Role, SessionConfiguration, ThreadEvent,
    ThreadId,
};
use tokio::sync::{Notify, watch};

const TOKEN: &str = "local-web-execution-fixture-token-32-bytes";
const PORT: u16 = 17341;
#[derive(Default)]
struct Evidence {
    connects: AtomicUsize,
    prompts: AtomicUsize,
    cancels: AtomicUsize,
    disconnects: AtomicUsize,
    hold_setup: AtomicBool,
    hold_prompt: AtomicBool,
    fail: AtomicBool,
    setup_started: Notify,
    prompt_started: Notify,
}
struct Backend(Arc<Evidence>);
struct Connection {
    info: watch::Sender<ConnectionInfo>,
    context: ConnectionContext,
    evidence: Arc<Evidence>,
}
struct Session {
    id: String,
    thread: ThreadId,
    context: ConnectionContext,
    evidence: Arc<Evidence>,
    cancellation: StdMutex<CancellationToken>,
}
#[async_trait]
impl AgentBackend for Backend {
    async fn connect(
        &self,
        _: &AgentSpec,
        context: ConnectionContext,
    ) -> AgentResult<Arc<dyn AgentConnection>> {
        self.0.connects.fetch_add(1, Ordering::SeqCst);
        self.0.setup_started.notify_one();
        if self.0.hold_setup.load(Ordering::Acquire) {
            std::future::pending::<()>().await;
        }
        let (info, _) = watch::channel(ConnectionInfo {
            id: ConnectionId::new(),
            state: ConnectionState::Connected,
            identity: None,
            capabilities: AgentCapabilities::default(),
            authentication: vec![],
            host: "Local".into(),
            error: None,
        });
        Ok(Arc::new(Connection {
            info,
            context,
            evidence: self.0.clone(),
        }))
    }
}
#[async_trait]
impl AgentConnection for Connection {
    fn info(&self) -> ConnectionInfo {
        self.info.borrow().clone()
    }
    fn observe(&self) -> watch::Receiver<ConnectionInfo> {
        self.info.subscribe()
    }
    async fn new_session(&self, options: SessionOptions) -> AgentResult<Arc<dyn AgentSession>> {
        Ok(Arc::new(Session {
            id: options.thread_id.to_string(),
            thread: options.thread_id,
            context: self.context.clone(),
            evidence: self.evidence.clone(),
            cancellation: StdMutex::new(CancellationToken::new()),
        }))
    }
    async fn disconnect(&self) -> AgentResult<()> {
        self.evidence.disconnects.fetch_add(1, Ordering::SeqCst);
        self.info
            .send_modify(|info| info.state = ConnectionState::Disconnected);
        Ok(())
    }
}
#[async_trait]
impl AgentSession for Session {
    fn id(&self) -> &str {
        &self.id
    }
    fn thread_id(&self) -> ThreadId {
        self.thread
    }
    fn configuration(&self) -> SessionConfiguration {
        SessionConfiguration::default()
    }
    async fn prompt(&self, prompt: Prompt) -> AgentResult<String> {
        self.evidence.prompts.fetch_add(1, Ordering::SeqCst);
        let cancel = CancellationToken::new();
        *self.cancellation.lock().unwrap() = cancel.clone();
        self.evidence.prompt_started.notify_one();
        if self.evidence.fail.load(Ordering::Acquire) {
            return Err(AgentError::Remote {
                code: -1,
                message: "PRIVATE-CANARY-CREDENTIAL".into(),
            });
        }
        let text = prompt
            .parts
            .into_iter()
            .map(|part| match part {
                PromptPart::Text(text) => text,
                _ => panic!("text-only web run"),
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.context
            .events
            .emit(
                self.thread,
                ThreadEvent::PromptStarted {
                    turn: self.id.clone(),
                },
            )
            .await?;
        self.context
            .events
            .emit(
                self.thread,
                ThreadEvent::TextDelta {
                    message_id: Some("user".into()),
                    role: Role::User,
                    text,
                },
            )
            .await?;
        if self.evidence.hold_prompt.load(Ordering::Acquire) {
            cancel.cancelled().await;
        }
        if !cancel.is_cancelled() {
            self.context
                .events
                .emit(
                    self.thread,
                    ThreadEvent::TextDelta {
                        message_id: Some("assistant".into()),
                        role: Role::Assistant,
                        text: "fixture-answer".into(),
                    },
                )
                .await?;
        }
        self.context
            .events
            .emit(
                self.thread,
                ThreadEvent::PromptFinished {
                    reason: "fixture-finished".into(),
                },
            )
            .await?;
        if cancel.is_cancelled() {
            Err(AgentError::Cancelled)
        } else {
            Ok("fixture-answer".into())
        }
    }
    async fn cancel(&self) -> AgentResult<()> {
        self.evidence.cancels.fetch_add(1, Ordering::SeqCst);
        self.cancellation.lock().unwrap().cancel();
        Ok(())
    }
    async fn close(&self) -> AgentResult<()> {
        self.cancel().await
    }
}
async fn fixture() -> (tempfile::TempDir, Arc<AppState>, Task, Arc<Evidence>) {
    let dir = tempfile::tempdir().unwrap();
    let workspace = WorkspaceService::memory().unwrap();
    let project = workspace
        .add_local_workspace(dir.path().into())
        .await
        .unwrap();
    let agent = workspace.profiles().await.unwrap()[0].id.clone();
    let task = workspace
        .create_task(project.id, "Web fixture".into(), agent)
        .await
        .unwrap();
    let state = Arc::new(AppState::new(TOKEN).unwrap());
    let evidence = Arc::new(Evidence::default());
    let controller = Arc::new(Controller::new(
        workspace.clone(),
        Arc::new(Backend(evidence.clone())),
        Arc::new(DenyInteractions),
    ));
    *state.runtime.write().await = Some(RuntimeServices {
        workspace,
        controller,
        _owner_lock: None,
    });
    state.transition(Lifecycle::Ready);
    (dir, state, task, evidence)
}
fn request(id: TaskId, action: &str, body: Option<serde_json::Value>) -> Request {
    let mut headers = vec![
        ("host".into(), format!("127.0.0.1:{PORT}")),
        ("authorization".into(), format!("Bearer {TOKEN}")),
    ];
    let method = if body.is_some() { "POST" } else { "GET" };
    let body = body
        .map(|value| serde_json::to_vec(&value).unwrap())
        .unwrap_or_default();
    if method == "POST" {
        headers.push(("content-type".into(), "application/json".into()));
        headers.push(("content-length".into(), body.len().to_string()));
    }
    Request {
        method: method.into(),
        target: format!("/api/tasks/{id}/{action}"),
        version: "HTTP/1.1".into(),
        headers,
        body,
    }
}
async fn start(state: &AppState, id: TaskId, expected: &str) -> Response {
    dispatch(
        &request(
            id,
            "run",
            Some(serde_json::json!({"text":"literal 日本語", "expected_draft":expected})),
        ),
        state,
        PORT,
    )
    .await
}
async fn terminal(state: &AppState, id: TaskId) -> serde_json::Value {
    timeout(Duration::from_secs(5), async {
        loop {
            let response = dispatch(&request(id, "run", None), state, PORT).await;
            assert_eq!(response.status, 200);
            let view: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
            if !matches!(view["state"].as_str(), Some("running" | "stopping")) {
                break view;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap()
}
async fn shutdown(state: &AppState) {
    let runtime = state.runtime.read().await;
    state
        .execution
        .shutdown(&runtime.as_ref().unwrap().controller)
        .await
        .unwrap();
}
#[tokio::test]
async fn explicit_run_delivers_durable_text_retains_draft_and_does_not_replay() {
    let (_dir, state, task, evidence) = fixture().await;
    assert_eq!(evidence.connects.load(Ordering::Acquire), 0);
    assert_eq!(start(&state, task.id, "").await.status, 202);
    assert_eq!(terminal(&state, task.id).await["state"], "completed");
    assert_eq!(evidence.prompts.load(Ordering::Acquire), 1);
    let runtime = state.runtime.read().await;
    let workspace = &runtime.as_ref().unwrap().workspace;
    assert_eq!(
        workspace.task_draft(task.id).await.unwrap(),
        "literal 日本語"
    );
    timeout(Duration::from_secs(3), async {
        loop {
            let response = dispatch(&request(task.id, "thread", None), &state, PORT).await;
            if String::from_utf8(response.body)
                .unwrap()
                .contains("fixture-answer")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let restarted = AppState::new(TOKEN).unwrap();
    restarted.install_runtime(workspace.clone(), None).await;
    let idle = dispatch(&request(task.id, "run", None), &restarted, PORT).await;
    assert!(String::from_utf8(idle.body).unwrap().contains("idle"));
    assert_eq!(evidence.prompts.load(Ordering::Acquire), 1);
    drop(runtime);
    shutdown(&state).await;
    shutdown(&restarted).await;
}
#[tokio::test]
async fn admission_rejects_stale_drafts_duplicate_runs_and_malformed_authority() {
    let (_dir, state, task, evidence) = fixture().await;
    evidence.hold_prompt.store(true, Ordering::Release);
    assert_eq!(start(&state, task.id, "stale").await.status, 409);
    assert_eq!(evidence.connects.load(Ordering::Acquire), 0);
    let mut unauth = request(
        task.id,
        "run",
        Some(serde_json::json!({"text":"hello","expected_draft":""})),
    );
    unauth.headers.retain(|(name, _)| name != "authorization");
    assert_eq!(dispatch(&unauth, &state, PORT).await.status, 401);
    let mut foreign = request(
        task.id,
        "run",
        Some(serde_json::json!({"text":"hello","expected_draft":""})),
    );
    foreign
        .headers
        .push(("origin".into(), "http://evil.example".into()));
    assert_eq!(dispatch(&foreign, &state, PORT).await.status, 421);
    assert_eq!(start(&state, task.id, "").await.status, 202);
    timeout(Duration::from_secs(3), evidence.prompt_started.notified())
        .await
        .unwrap();
    assert_eq!(start(&state, task.id, "literal 日本語").await.status, 409);
    assert_eq!(evidence.prompts.load(Ordering::Acquire), 1);
    assert_eq!(
        dispatch(
            &request(task.id, "stop", Some(serde_json::json!({}))),
            &state,
            PORT
        )
        .await
        .status,
        200
    );
    assert_eq!(terminal(&state, task.id).await["state"], "cancelled");
    assert_eq!(evidence.cancels.load(Ordering::Acquire), 1);
    shutdown(&state).await;
}
#[tokio::test]
async fn stop_during_connection_setup_never_submits_a_delayed_prompt() {
    let (_dir, state, task, evidence) = fixture().await;
    evidence.hold_setup.store(true, Ordering::Release);
    assert_eq!(start(&state, task.id, "").await.status, 202);
    timeout(Duration::from_secs(3), evidence.setup_started.notified())
        .await
        .unwrap();
    dispatch(
        &request(task.id, "stop", Some(serde_json::json!({}))),
        &state,
        PORT,
    )
    .await;
    assert_eq!(terminal(&state, task.id).await["state"], "cancelled");
    assert_eq!(evidence.prompts.load(Ordering::Acquire), 0);
    shutdown(&state).await;
}
#[tokio::test]
async fn failures_are_allowlisted_and_stopping_a_completed_run_is_idempotent() {
    let (_dir, state, task, evidence) = fixture().await;
    evidence.fail.store(true, Ordering::Release);
    assert_eq!(start(&state, task.id, "").await.status, 202);
    let view = terminal(&state, task.id).await;
    assert_eq!(view["state"], "failed");
    assert_eq!(view["error"], "agent_failed");
    assert!(!view.to_string().contains("PRIVATE-CANARY"));
    let stop = dispatch(
        &request(task.id, "stop", Some(serde_json::json!({}))),
        &state,
        PORT,
    )
    .await;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&stop.body).unwrap(),
        view
    );
    shutdown(&state).await;
}
#[tokio::test]
async fn execution_deadline_cancels_and_drains_the_native_owner() {
    let (_dir, state, task, evidence) = fixture().await;
    evidence.hold_prompt.store(true, Ordering::Release);
    let view = Arc::new(StdMutex::new(RunView::new("running")));
    let controller = state
        .runtime
        .read()
        .await
        .as_ref()
        .unwrap()
        .controller
        .clone();
    let workspace = state
        .runtime
        .read()
        .await
        .as_ref()
        .unwrap()
        .workspace
        .clone();
    timeout(
        Duration::from_secs(3),
        run_task(
            controller,
            workspace,
            task.id,
            "deadline".into(),
            None,
            None,
            CancellationToken::new(),
            view.clone(),
            Duration::from_millis(100),
        ),
    )
    .await
    .unwrap();
    assert_eq!(view.lock().unwrap().state, "timed_out");
    assert_eq!(evidence.cancels.load(Ordering::Acquire), 1);
    shutdown(&state).await;
}
#[tokio::test]
async fn shutdown_joins_active_runs_before_releasing_process_ownership() {
    let (_dir, state, task, evidence) = fixture().await;
    evidence.hold_prompt.store(true, Ordering::Release);
    assert_eq!(start(&state, task.id, "").await.status, 202);
    timeout(Duration::from_secs(3), evidence.prompt_started.notified())
        .await
        .unwrap();
    timeout(Duration::from_secs(5), shutdown(&state))
        .await
        .unwrap();
    assert!(state.execution.runs.lock().await.is_empty());
    assert_eq!(evidence.disconnects.load(Ordering::Acquire), 1);
}
#[tokio::test]
async fn validation_rejects_empty_oversized_unknown_fields_and_unknown_tasks() {
    let (_dir, state, task, evidence) = fixture().await;
    for value in [
        serde_json::json!({"text":"", "expected_draft":""}),
        serde_json::json!({"text":"x".repeat(MAX_WEB_DRAFT_BYTES+1), "expected_draft":""}),
        serde_json::json!({"text":"ok", "expected_draft":"", "approve_all":true}),
    ] {
        assert_eq!(
            dispatch(&request(task.id, "run", Some(value)), &state, PORT)
                .await
                .status,
            400
        );
    }
    assert_eq!(start(&state, TaskId::new(), "").await.status, 404);
    assert_eq!(evidence.connects.load(Ordering::Acquire), 0);
    shutdown(&state).await;
}
