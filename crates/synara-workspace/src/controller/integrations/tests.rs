use super::*;
use async_trait::async_trait;
use std::sync::atomic::AtomicUsize;
use synara_runtime::{RuntimeError, SecretReference, SecretValue};
use tokio::sync::{Notify, watch};

#[derive(Default)]
struct Evidence {
    connections: AtomicUsize,
    options: StdMutex<Vec<SessionOptions>>,
    closes: AtomicUsize,
    hold: AtomicBool,
    started: Notify,
    release: Notify,
}
struct Backend {
    caps: AgentCapabilities,
    evidence: Arc<Evidence>,
}
struct Connection {
    info: watch::Sender<ConnectionInfo>,
    evidence: Arc<Evidence>,
}
struct Session {
    thread: ThreadId,
    id: String,
    can_close: bool,
    evidence: Arc<Evidence>,
}
#[async_trait]
impl AgentBackend for Backend {
    async fn connect(
        &self,
        _: &AgentSpec,
        _: ConnectionContext,
    ) -> AgentResult<Arc<dyn AgentConnection>> {
        self.evidence.connections.fetch_add(1, Ordering::SeqCst);
        let (info, _) = watch::channel(ConnectionInfo {
            id: ConnectionId::new(),
            state: ConnectionState::Connected,
            identity: None,
            capabilities: self.caps.clone(),
            authentication: vec![],
            host: "Local".into(),
            error: None,
        });
        Ok(Arc::new(Connection {
            info,
            evidence: self.evidence.clone(),
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
        let mut captured = self.evidence.options.lock().unwrap();
        let session = Session {
            thread: options.thread_id,
            id: format!("fixture-{}", captured.len()),
            can_close: self.info().capabilities.close_session,
            evidence: self.evidence.clone(),
        };
        captured.push(options);
        Ok(Arc::new(session))
    }
    async fn disconnect(&self) -> AgentResult<()> {
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
    async fn prompt(&self, _: Prompt) -> AgentResult<String> {
        if self.evidence.hold.load(Ordering::Acquire) {
            self.evidence.started.notify_one();
            self.evidence.release.notified().await;
        }
        Ok("fixture complete".into())
    }
    async fn cancel(&self) -> AgentResult<()> {
        self.evidence.release.notify_one();
        Ok(())
    }
    async fn close(&self) -> AgentResult<()> {
        if !self.can_close {
            return Err(AgentError::Unsupported("fixture close".into()));
        }
        self.evidence.closes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
struct Secrets(SecretStoreState);
#[async_trait]
impl SecretStore for Secrets {
    fn state(&self) -> SecretStoreState {
        self.0
    }
    async fn read(&self, _: &SecretReference) -> Result<Option<SecretValue>, RuntimeError> {
        Ok(Some(SecretValue::new(b"mcp-canary-credential".to_vec())?))
    }
    async fn write(&self, _: &SecretReference, _: SecretValue) -> Result<(), RuntimeError> {
        Err(RuntimeError::Unsupported("fixture".into()))
    }
    async fn delete(&self, _: &SecretReference) -> Result<(), RuntimeError> {
        Err(RuntimeError::Unsupported("fixture".into()))
    }
}
async fn fixture(
    caps: AgentCapabilities,
    secrets: SecretStoreState,
) -> (tempfile::TempDir, Arc<Controller>, Task, Arc<Evidence>) {
    let dir = tempfile::tempdir().unwrap();
    let workspace = WorkspaceService::memory().unwrap();
    let project = workspace
        .add_local_workspace(dir.path().into())
        .await
        .unwrap();
    let agent = workspace.profiles().await.unwrap()[0].id.clone();
    let task = workspace
        .create_task(project.id, "MCP fixture".into(), agent)
        .await
        .unwrap();
    let evidence = Arc::new(Evidence::default());
    let controller = Arc::new(Controller::with_secret_store(
        workspace,
        Arc::new(Backend {
            caps,
            evidence: evidence.clone(),
        }),
        Arc::new(DenyInteractions),
        Arc::new(Secrets(secrets)),
    ));
    (dir, controller, task, evidence)
}
async fn enabled(
    controller: &Controller,
    task: &Task,
    bearer: bool,
) -> (IntegrationSettings, String) {
    let mut config = ManagedMcp::new(task.id, task.agent_id.clone());
    config.name = "Reviewed tools".into();
    config.endpoint = "https://example.com/mcp".into();
    if bearer {
        config.bearer = Some(SecretReference::new("dev.synara", "fixture").unwrap());
    }
    let id = config.id.clone();
    let value = controller
        .configure_mcp(0, McpEdit::Save(config))
        .await
        .unwrap();
    let value = controller
        .configure_mcp(
            value.revision,
            McpEdit::SetEnabled {
                id: id.clone(),
                enabled: true,
            },
        )
        .await
        .unwrap();
    (value, id)
}
#[tokio::test]
async fn integrations_controller_negotiates_exact_scope_and_resolves_only_references() {
    let (_dir, controller, task, evidence) = fixture(
        AgentCapabilities {
            mcp_http: true,
            close_session: true,
            ..Default::default()
        },
        SecretStoreState::Available,
    )
    .await;
    let (value, id) = enabled(&controller, &task, true).await;
    assert_eq!(
        evidence.connections.load(Ordering::SeqCst),
        0,
        "configuration must not launch an agent"
    );
    controller.connect(task.id).await.unwrap();
    let other = controller
        .workspace
        .create_task(
            task.project_id,
            "Unconsented task".into(),
            task.agent_id.clone(),
        )
        .await
        .unwrap();
    controller.connect(other.id).await.unwrap();
    {
        let options = evidence.options.lock().unwrap();
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].context_servers.len(), 1);
        assert!(options[1].context_servers.is_empty());
        assert!(
            matches!(&options[0].context_servers[0],ContextServer::Http{headers,..} if headers["Authorization"]=="Bearer mcp-canary-credential")
        );
        assert!(!format!("{:?}", options[0]).contains("mcp-canary-credential"));
    }
    assert_eq!(
        evidence.connections.load(Ordering::SeqCst),
        1,
        "reuse the existing generic process manager"
    );
    assert!(
        !serde_json::to_string(&controller.workspace.integrations().await.unwrap())
            .unwrap()
            .contains("mcp-canary-credential")
    );
    controller
        .configure_mcp(value.revision, McpEdit::SetEnabled { id, enabled: false })
        .await
        .unwrap();
    assert_eq!(evidence.closes.load(Ordering::SeqCst), 1);
    assert!(
        controller
            .workspace
            .session(task.thread_id)
            .await
            .unwrap()
            .is_none()
    );
    controller.connect(task.id).await.unwrap();
    assert!(
        evidence.options.lock().unwrap()[2]
            .context_servers
            .is_empty()
    );
}
#[tokio::test]
async fn integrations_controller_never_guesses_support_from_the_profile_name() {
    let (_dir, controller, task, evidence) =
        fixture(AgentCapabilities::default(), SecretStoreState::Available).await;
    enabled(&controller, &task, false).await;
    let error = controller.connect(task.id).await.unwrap_err().to_string();
    assert!(error.contains("did not negotiate"));
    assert!(evidence.options.lock().unwrap().is_empty());
    assert!(
        controller
            .workspace
            .session(task.thread_id)
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn integrations_controller_locked_unavailable_secrets_fail_without_test_or_plaintext_fallback()
 {
    for state in [SecretStoreState::Locked, SecretStoreState::Unavailable] {
        let (_dir, controller, task, evidence) = fixture(
            AgentCapabilities {
                mcp_http: true,
                ..Default::default()
            },
            state,
        )
        .await;
        let (value, id) = enabled(&controller, &task, true).await;
        let error = controller
            .test_mcp(value.revision, id)
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains(if state == SecretStoreState::Locked {
            "locked"
        } else {
            "unavailable"
        }));
        assert_eq!(evidence.connections.load(Ordering::SeqCst), 0);
        assert_eq!(controller.workspace.integrations().await.unwrap(), value);
    }
}
#[tokio::test]
async fn integrations_controller_revoke_requires_actual_close_or_explicit_disconnect() {
    let (_dir, controller, task, evidence) = fixture(
        AgentCapabilities {
            mcp_http: true,
            ..Default::default()
        },
        SecretStoreState::Available,
    )
    .await;
    let (value, id) = enabled(&controller, &task, false).await;
    controller.connect(task.id).await.unwrap();
    assert!(
        controller
            .configure_mcp(value.revision, McpEdit::Remove(id.clone()))
            .await
            .unwrap_err()
            .to_string()
            .contains("did not confirm session closure")
    );
    assert_eq!(controller.workspace.integrations().await.unwrap(), value);
    controller.disconnect_mcp_agent(task.id).await.unwrap();
    let removed = controller
        .configure_mcp(value.revision, McpEdit::Remove(id))
        .await
        .unwrap();
    assert!(removed.mcp.is_empty());
    assert_eq!(evidence.connections.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn integrations_controller_active_prompt_blocks_configuration_and_shared_disconnect() {
    let (_dir, controller, task, evidence) = fixture(
        AgentCapabilities {
            mcp_http: true,
            close_session: true,
            ..Default::default()
        },
        SecretStoreState::Available,
    )
    .await;
    let (value, id) = enabled(&controller, &task, false).await;
    controller.connect(task.id).await.unwrap();
    let other = controller
        .workspace
        .create_task(
            task.project_id,
            "Shared process".into(),
            task.agent_id.clone(),
        )
        .await
        .unwrap();
    controller.connect(other.id).await.unwrap();
    evidence.hold.store(true, Ordering::Release);
    let submit = {
        let controller = controller.clone();
        tokio::spawn(async move {
            controller
                .submit(task.id, "Explicit fixture prompt".into())
                .await
        })
    };
    tokio::time::timeout(
        std::time::Duration::from_secs(3),
        evidence.started.notified(),
    )
    .await
    .unwrap();
    assert!(matches!(
        controller
            .configure_mcp(value.revision, McpEdit::Remove(id))
            .await,
        Err(WorkspaceError::Agent(AgentError::Busy))
    ));
    assert!(matches!(
        controller.disconnect_mcp_agent(other.id).await,
        Err(WorkspaceError::Agent(AgentError::Busy))
    ));
    evidence.release.notify_one();
    submit.await.unwrap().unwrap();
    assert_eq!(controller.workspace.integrations().await.unwrap(), value);
}
