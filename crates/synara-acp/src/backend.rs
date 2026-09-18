use crate::{
    callbacks::CallbackServices,
    rpc::{Incoming, PendingResponse, RpcPeer},
    scope::{SessionState, Sessions},
    wire,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use std::{
    sync::{Arc, Weak, atomic::Ordering},
    time::Duration,
};
use synara_agent::*;
use synara_core::*;
use synara_runtime::ProcessHandle;
use tokio::{
    sync::{Mutex, Semaphore, mpsc, watch},
    task::JoinSet,
};

#[derive(Clone, Debug)]
pub struct AcpTimeouts {
    pub initialize: Duration,
    pub operation: Duration,
    pub authentication: Duration,
    pub prompt: Duration,
    pub cancellation: Duration,
}
impl Default for AcpTimeouts {
    fn default() -> Self {
        Self {
            initialize: Duration::from_secs(30),
            operation: Duration::from_secs(30),
            authentication: Duration::from_secs(300),
            prompt: Duration::from_secs(1800),
            cancellation: Duration::from_secs(10),
        }
    }
}
#[derive(Clone, Default)]
pub struct AcpBackend {
    pub timeouts: AcpTimeouts,
}
pub(crate) struct Connection {
    pub rpc: RpcPeer,
    pub context: ConnectionContext,
    pub sessions: Arc<Sessions>,
    pub callbacks: Arc<CallbackServices>,
    pub state: watch::Sender<ConnectionInfo>,
    pub timeouts: AcpTimeouts,
    process: ProcessHandle,
    setup_gate: Mutex<()>,
    auth_gate: Mutex<()>,
    failed: std::sync::atomic::AtomicBool,
}
impl Drop for Connection {
    fn drop(&mut self) {
        self.rpc.fail("connection released");
        self.process.request_stop();
    }
}
#[derive(Clone)]
struct AcpConnection(Arc<Connection>);
pub(crate) struct AcpSession {
    pub connection: Arc<Connection>,
    pub state: Arc<SessionState>,
}

#[async_trait]
impl AgentBackend for AcpBackend {
    async fn connect(
        &self,
        spec: &AgentSpec,
        context: ConnectionContext,
    ) -> AgentResult<Arc<dyn AgentConnection>> {
        spec.validate()?;
        if !context.cwd.is_absolute() {
            return Err(wire::invalid("connection directory must be absolute"));
        }
        let directory = spec.launch_directory.as_deref().unwrap_or(&context.cwd);
        let process = context.host.spawn(&spec.launch, directory).await?;
        let (rpc, incoming) = RpcPeer::start(process.stdout, process.stdin, process.stderr);
        let sessions = Arc::new(Sessions::default());
        let connection_id = ConnectionId::new();
        let callbacks = Arc::new(CallbackServices::new(
            context.clone(),
            sessions.clone(),
            connection_id,
        ));
        let (state, _) = watch::channel(ConnectionInfo {
            id: connection_id,
            state: ConnectionState::Initializing,
            identity: None,
            capabilities: AgentCapabilities::default(),
            authentication: vec![],
            host: context.host.label(),
            error: None,
        });
        let connection = Arc::new(Connection {
            rpc,
            context,
            sessions,
            callbacks,
            state,
            process: process.handle,
            timeouts: self.timeouts.clone(),
            setup_gate: Mutex::new(()),
            auth_gate: Mutex::new(()),
            failed: std::sync::atomic::AtomicBool::new(false),
        });
        tokio::spawn(dispatch_loop(Arc::downgrade(&connection), incoming));
        tokio::spawn(watch_exit(
            Arc::downgrade(&connection),
            connection.process.clone(),
            connection.rpc.failures(),
        ));
        let params = wire::initialize_params(connection.context.host.is_local());
        let initialized = async {
            let response = connection
                .call("initialize", params, connection.timeouts.initialize)
                .await?;
            let (capabilities, identity, authentication) = wire::initialization(&response)?;
            connection.update_live(|state| {
                state.state = ConnectionState::Connected;
                state.capabilities = capabilities;
                state.identity = identity;
                state.authentication = authentication;
                state.error = None;
            })?;
            Ok::<_, AgentError>(())
        }
        .await;
        if let Err(error) = initialized {
            connection.rpc.fail("initialization failed");
            let _ = connection.process.shutdown().await;
            return Err(error);
        }
        Ok(Arc::new(AcpConnection(connection)))
    }
}
impl Connection {
    /// A late completion must never revive a disconnected or failed transport.
    fn update_live(&self, update: impl FnOnce(&mut ConnectionInfo)) -> AgentResult<()> {
        let changed = self.state.send_if_modified(|state| {
            if self.rpc.cancelled().is_cancelled()
                || matches!(
                    state.state,
                    ConnectionState::Disconnected
                        | ConnectionState::Failed
                        | ConnectionState::Exited
                )
            {
                return false;
            }
            update(state);
            true
        });
        if changed {
            Ok(())
        } else {
            Err(AgentError::Disconnected(
                "connection is no longer live".into(),
            ))
        }
    }

    fn ensure_thread_available(&self, thread_id: ThreadId) -> AgentResult<()> {
        let states = self.sessions.states.lock().unwrap();
        if states
            .values()
            .any(|session| session.thread_id == thread_id)
        {
            return Err(AgentError::Busy);
        }
        if states.len() >= 64 {
            return Err(AgentError::Limit);
        }
        Ok(())
    }

    fn register_session(&self, session: Arc<SessionState>) -> AgentResult<()> {
        let mut states = self.sessions.states.lock().unwrap();
        // This check and insertion share the same lock as disconnect's drain.
        self.ensure_connected()?;
        if states.contains_key(&session.id)
            || states
                .values()
                .any(|owned| owned.thread_id == session.thread_id)
        {
            return Err(AgentError::Busy);
        }
        if states.len() >= 64 {
            return Err(AgentError::Limit);
        }
        states.insert(session.id.clone(), session);
        Ok(())
    }

    pub async fn call(&self, method: &str, params: Value, timeout: Duration) -> AgentResult<Value> {
        crate::schema::request(method, &params)?;
        self.call_result(method, self.rpc.request(method, params, timeout).await)
    }
    pub async fn begin_call(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> AgentResult<PendingResponse> {
        crate::schema::request(method, &params)?;
        self.rpc.begin_request(method, params, timeout).await
    }
    pub fn call_result(&self, method: &str, result: AgentResult<Value>) -> AgentResult<Value> {
        match result {
            Ok(value) => {
                if let Err(error) = crate::schema::response(method, &value) {
                    self.rpc.fail("response schema mismatch");
                    return Err(error);
                }
                Ok(value)
            }
            Err(AgentError::Remote { code: -32000, .. }) => {
                self.update_live(|state| {
                    if state.state != ConnectionState::Authenticating || method == "authenticate" {
                        state.state = ConnectionState::AuthenticationRequired;
                    }
                    state.error = Some("Authentication required".into());
                })?;
                Err(AgentError::AuthenticationRequired)
            }
            Err(AgentError::Remote { code: -32800, .. }) => Err(AgentError::Cancelled),
            result => result,
        }
    }
    pub fn ensure_connected(&self) -> AgentResult<()> {
        if self.rpc.cancelled().is_cancelled() {
            return Err(AgentError::Disconnected("agent transport is closed".into()));
        }
        match self.state.borrow().state {
            ConnectionState::Connected => Ok(()),
            ConnectionState::AuthenticationRequired => Err(AgentError::AuthenticationRequired),
            ConnectionState::Authenticating => Err(AgentError::Busy),
            _ => Err(AgentError::Disconnected("agent is not connected".into())),
        }
    }
    pub fn capabilities(&self) -> AgentCapabilities {
        self.state.borrow().capabilities.clone()
    }
    pub async fn notify_session(&self, params: Value) -> AgentResult<()> {
        let id = wire::id(&params, "sessionId")?;
        let update = params
            .get("update")
            .ok_or_else(|| wire::invalid("session update missing"))?;
        let session = self.sessions.states.lock().unwrap().get(&id).cloned();
        if let Some(session) = session {
            if session.closed.load(Ordering::Acquire) || session.lifetime.is_cancelled() {
                return Ok(());
            }
            session.apply_update(&self.context, update).await
        } else if self.sessions.creating.load(Ordering::Acquire) > 0 {
            let mut updates = self.sessions.early_updates.lock().unwrap();
            let bytes: usize = updates
                .iter()
                .map(|(_, value)| value.to_string().len())
                .sum();
            if updates.len() >= 64 || bytes + update.to_string().len() > 1024 * 1024 {
                return Err(AgentError::Limit);
            }
            updates.push((id, update.clone()));
            Ok(())
        } else {
            // Late notifications for a closed session do not acquire new ownership.
            Ok(())
        }
    }
    pub async fn failed(&self, reason: String) {
        if self.failed.swap(true, Ordering::AcqRel)
            || self.state.borrow().state == ConnectionState::Disconnected
        {
            return;
        }
        self.state.send_if_modified(|state| {
            if state.state == ConnectionState::Disconnected {
                return false;
            }
            state.state = ConnectionState::Failed;
            state.error = Some(reason.clone());
            true
        });
        self.process.request_stop();
        self.rpc.fail("connection failed");
        let sessions: Vec<_> = self
            .sessions
            .states
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect();
        for session in &sessions {
            session.lifetime.cancel();
            session.turn.lock().unwrap().cancel();
            session.active.store(false, Ordering::Release);
        }
        let cleanup = self.callbacks.stop_terminals(None).await;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        let mut diagnostics_failed = false;
        for session in sessions {
            let delivered = tokio::time::timeout_at(
                deadline,
                session.emit(
                    &self.context,
                    ThreadEvent::Error {
                        message: reason.clone(),
                        recoverable: false,
                    },
                ),
            )
            .await;
            if !matches!(delivered, Ok(Ok(()))) {
                diagnostics_failed = true;
                break;
            }
        }
        if cleanup.is_err() || diagnostics_failed {
            self.state.send_if_modified(|state| {
                if state.state != ConnectionState::Failed {
                    return false;
                }
                state.error = Some(format!(
                    "{reason}. Cleanup or final diagnostic delivery did not complete."
                ));
                true
            });
        }
        self.process.request_stop();
    }
}

#[async_trait]
impl AgentConnection for AcpConnection {
    fn info(&self) -> ConnectionInfo {
        self.0.state.borrow().clone()
    }
    fn observe(&self) -> watch::Receiver<ConnectionInfo> {
        self.0.state.subscribe()
    }
    fn trace(&self) -> Vec<TraceEntry> {
        self.0.rpc.trace()
    }
    fn clear_trace(&self) {
        self.0.rpc.clear_trace();
    }
    async fn new_session(&self, options: SessionOptions) -> AgentResult<Arc<dyn AgentSession>> {
        let connection = &self.0;
        let _setup = connection.setup_gate.lock().await;
        connection.ensure_connected()?;
        connection.ensure_thread_available(options.thread_id)?;
        let params = wire::session_params(&options, &connection.capabilities())?;
        // Validate filesystem ownership before asking the external agent to start work.
        let prepared = SessionState::build(
            String::new(),
            &options,
            &connection.context,
            connection.rpc.cancelled().child_token(),
        )
        .await?;
        let _creating = CreatingGuard::new(connection.sessions.clone());
        let mut setup_owner = SetupOwner {
            connection,
            completed: false,
        };
        let response = match connection
            .call("session/new", params, connection.timeouts.operation)
            .await
        {
            Ok(value) => value,
            Err(error) => {
                if matches!(
                    error,
                    AgentError::Remote { .. }
                        | AgentError::AuthenticationRequired
                        | AgentError::Unsupported(_)
                ) {
                    setup_owner.completed = true;
                }
                return Err(error);
            }
        };
        connection.rpc.barrier().await?;
        let id = wire::id(&response, "sessionId")?;
        if connection.sessions.states.lock().unwrap().contains_key(&id) {
            return Err(wire::invalid("agent returned an already-owned session ID"));
        }
        let mut prepared = Arc::try_unwrap(prepared).map_err(|_| AgentError::Busy)?;
        prepared.id = id.clone();
        *prepared.configuration.write().unwrap() =
            wire::configuration(&response, &SessionConfiguration::default())?;
        let session = Arc::new(prepared);
        let lock = session.update_gate.lock().await;
        connection.register_session(session.clone())?;
        let configuration = session.configuration.read().unwrap().clone();
        session
            .emit(
                &connection.context,
                ThreadEvent::ConfigurationChanged { configuration },
            )
            .await?;
        let updates = std::mem::take(&mut *connection.sessions.early_updates.lock().unwrap());
        for (owner, update) in updates {
            if owner == id {
                session
                    .apply_update_locked(&connection.context, &update)
                    .await?;
            }
        }
        drop(lock);
        connection.ensure_connected()?;
        setup_owner.completed = true;
        Ok(Arc::new(AcpSession {
            connection: connection.clone(),
            state: session,
        }))
    }
    async fn restore_session(
        &self,
        id: &str,
        options: SessionOptions,
        mode: RestoreMode,
    ) -> AgentResult<Arc<dyn AgentSession>> {
        let connection = &self.0;
        let _setup = connection.setup_gate.lock().await;
        connection.ensure_connected()?;
        let capabilities = connection.capabilities();
        let method = match mode {
            RestoreMode::ReplayHistory if capabilities.load_session => "session/load",
            RestoreMode::ResumeWithoutReplay if capabilities.resume_session => "session/resume",
            _ => {
                return Err(AgentError::Unsupported(
                    "requested session restore mode".into(),
                ));
            }
        };
        wire::id(&json!({"sessionId":id}), "sessionId")?;
        if connection.sessions.states.lock().unwrap().contains_key(id) {
            return Err(AgentError::Busy);
        }
        connection.ensure_thread_available(options.thread_id)?;
        let mut params = wire::session_params(&options, &capabilities)?;
        params["sessionId"] = json!(id);
        let session = SessionState::build(
            id.into(),
            &options,
            &connection.context,
            connection.rpc.cancelled().child_token(),
        )
        .await?;
        let mut setup_owner = SetupOwner {
            connection,
            completed: false,
        };
        let result = async {
            {
                let _update = session.update_gate.lock().await;
                connection.register_session(session.clone())?;
                if mode == RestoreMode::ReplayHistory {
                    session
                        .emit(&connection.context, ThreadEvent::HistoryStarted)
                        .await?;
                }
            }
            let result = connection
                .call(method, params, connection.timeouts.operation)
                .await?;
            connection.rpc.barrier().await?;
            let configuration =
                wire::configuration(&result, &session.configuration.read().unwrap())?;
            *session.configuration.write().unwrap() = configuration.clone();
            session
                .emit(
                    &connection.context,
                    ThreadEvent::ConfigurationChanged { configuration },
                )
                .await?;
            if mode == RestoreMode::ReplayHistory {
                session
                    .emit(&connection.context, ThreadEvent::HistoryCompleted)
                    .await?;
            }
            connection.ensure_connected()?;
            Ok::<_, AgentError>(())
        }
        .await;
        if let Err(error) = result {
            if !matches!(error, AgentError::Timeout | AgentError::Disconnected(_)) {
                setup_owner.completed = true;
            }
            let _ = session
                .emit(
                    &connection.context,
                    ThreadEvent::Error {
                        message: format!("Session history could not be restored: {error}"),
                        recoverable: false,
                    },
                )
                .await;
            session.lifetime.cancel();
            let _ = connection.callbacks.stop_terminals(Some(id)).await;
            connection.sessions.states.lock().unwrap().remove(id);
            return Err(error);
        }
        setup_owner.completed = true;
        Ok(Arc::new(AcpSession {
            connection: connection.clone(),
            state: session,
        }))
    }
    async fn list_sessions(
        &self,
        cwd: Option<String>,
        cursor: Option<String>,
    ) -> AgentResult<SessionPage> {
        self.0.ensure_connected()?;
        if !self.0.capabilities().list_sessions {
            return Err(AgentError::Unsupported("session listing".into()));
        }
        let mut params = json!({});
        if let Some(cwd) = cwd {
            params["cwd"] = json!(cwd);
        }
        if let Some(cursor) = cursor {
            if cursor.len() > 8192 {
                return Err(AgentError::Limit);
            }
            params["cursor"] = json!(cursor);
        }
        let result = self
            .0
            .call("session/list", params, self.0.timeouts.operation)
            .await?;
        let sessions = wire::array(&result, "sessions", 1000)?
            .iter()
            .map(|session| {
                Ok(SessionSummary {
                    id: wire::id(session, "sessionId")?,
                    cwd: wire::optional_string(session, "cwd"),
                    title: wire::optional_string(session, "title"),
                    updated_at: wire::optional_string(session, "updatedAt"),
                })
            })
            .collect::<AgentResult<_>>()?;
        Ok(SessionPage {
            sessions,
            next_cursor: wire::optional_string(&result, "nextCursor"),
        })
    }
    async fn delete_session(&self, id: &str) -> AgentResult<()> {
        let _setup = self.0.setup_gate.lock().await;
        self.0.ensure_connected()?;
        wire::id(&json!({"sessionId":id}), "sessionId")?;
        if !self.0.capabilities().delete_session {
            return Err(AgentError::Unsupported("session deletion".into()));
        }
        if self.0.sessions.states.lock().unwrap().contains_key(id) {
            return Err(AgentError::Busy);
        }
        self.0
            .call(
                "session/delete",
                json!({"sessionId":id}),
                self.0.timeouts.operation,
            )
            .await?;
        Ok(())
    }
    async fn authenticate(&self, method: &str) -> AgentResult<()> {
        let _auth = self.0.auth_gate.try_lock().map_err(|_| AgentError::Busy)?;
        if !self
            .0
            .state
            .borrow()
            .authentication
            .iter()
            .any(|auth| auth.id == method)
        {
            return Err(wire::invalid("authentication method was not advertised"));
        }
        self.0.update_live(|state| {
            state.state = ConnectionState::Authenticating;
            state.error = None;
        })?;
        let mut owner = AuthenticationOwner {
            connection: &self.0,
            completed: false,
        };
        let result = self
            .0
            .call(
                "authenticate",
                json!({"methodId":method}),
                self.0.timeouts.authentication,
            )
            .await;
        owner.completed = true;
        match result {
            Ok(_) => self.0.update_live(|state| {
                state.state = ConnectionState::Connected;
                state.error = None;
            }),
            Err(error) => {
                if matches!(error, AgentError::Timeout) {
                    self.0
                        .rpc
                        .fail("authentication deadline expired before completion");
                }
                let _ = self.0.update_live(|state| {
                    state.state = ConnectionState::AuthenticationRequired;
                    state.error = Some(error.to_string());
                });
                Err(error)
            }
        }
    }
    async fn logout(&self) -> AgentResult<()> {
        let _auth = self.0.auth_gate.try_lock().map_err(|_| AgentError::Busy)?;
        self.0.ensure_connected()?;
        if !self.0.capabilities().logout {
            return Err(AgentError::Unsupported("logout".into()));
        }
        let mut owner = AuthenticationOwner {
            connection: &self.0,
            completed: false,
        };
        let result = self
            .0
            .call("logout", json!({}), self.0.timeouts.operation)
            .await;
        owner.completed = true;
        if matches!(result, Err(AgentError::Timeout)) {
            self.0.rpc.fail("logout deadline expired before completion");
        }
        result?;
        self.0.update_live(|state| {
            state.state = ConnectionState::AuthenticationRequired;
            state.error = Some("Authentication required".into());
        })
    }
    async fn disconnect(&self) -> AgentResult<()> {
        self.0.state.send_modify(|state| {
            state.state = ConnectionState::Disconnected;
            state.error = None;
        });
        self.0.rpc.fail("connection disconnected");
        let sessions = std::mem::take(&mut *self.0.sessions.states.lock().unwrap());
        for session in sessions.values() {
            session.lifetime.cancel();
            session.turn.lock().unwrap().cancel();
            session.closed.store(true, Ordering::Release);
        }
        let cleanup = self.0.callbacks.stop_terminals(None).await;
        self.0.process.shutdown().await?;
        cleanup
    }
}
struct CreatingGuard {
    sessions: Arc<Sessions>,
}
impl CreatingGuard {
    fn new(sessions: Arc<Sessions>) -> Self {
        sessions.early_updates.lock().unwrap().clear();
        sessions.creating.fetch_add(1, Ordering::AcqRel);
        Self { sessions }
    }
}
impl Drop for CreatingGuard {
    fn drop(&mut self) {
        self.sessions.creating.fetch_sub(1, Ordering::AcqRel);
        self.sessions.early_updates.lock().unwrap().clear();
    }
}

async fn dispatch_loop(connection: Weak<Connection>, mut incoming: mpsc::Receiver<Incoming>) {
    let semaphore = Arc::new(Semaphore::new(32));
    let mut callbacks = JoinSet::new();
    loop {
        let Some(owner) = connection.upgrade() else {
            break;
        };
        let cancelled = owner.rpc.cancelled();
        drop(owner);
        tokio::select! {
            () = cancelled.cancelled() => break,
            Some(result) = callbacks.join_next(), if !callbacks.is_empty() => {
                if result.is_err() && let Some(owner) = connection.upgrade() { owner.failed("Client callback worker stopped unexpectedly".into()).await; break; }
            }
            message = incoming.recv() => {
                let (Some(message),Some(owner)) = (message,connection.upgrade()) else { break; };
                match message {
                    Incoming::Barrier(sender) => { let _ = sender.send(()); },
                    Incoming::Notification { method,params } => {
                        let result = match method.as_str() {
                            "session/update" => owner.notify_session(params).await,
                            "elicitation/complete" => { owner.callbacks.elicitation_complete(&params).await; Ok(()) },
                            _ => Ok(()),
                        };
                        if let Err(error) = result { owner.failed(format!("Agent update could not be applied: {error}")).await; break; }
                    }
                    Incoming::Request { id,method,params } => {
                        let Ok(permit) = semaphore.clone().try_acquire_owned() else {
                            let _ = owner.rpc.reply(id,Err(AgentError::Limit)).await;
                            continue;
                        };
                        callbacks.spawn(async move {
                            let _permit = permit;
                            let result = tokio::time::timeout(std::time::Duration::from_secs(310), owner.callbacks.handle(&method,params,&owner.rpc)).await.unwrap_or(Err(AgentError::Timeout));
                            let _ = owner.rpc.reply(id,result).await;
                        });
                    }
                }
            }
        }
    }
    callbacks.abort_all();
    while callbacks.join_next().await.is_some() {}
}
async fn watch_exit(
    connection: Weak<Connection>,
    process: ProcessHandle,
    mut failures: watch::Receiver<Option<String>>,
) {
    let existing = failures.borrow().clone();
    let reason = if let Some(reason) = existing {
        reason
    } else {
        tokio::select! {
            exit = process.wait() => match exit { Ok(exit) => format!("Agent process exited (code {:?}, signal {:?})",exit.code,exit.signal), Err(_) => "Agent process supervision ended".into() },
            result = failures.changed() => if result.is_ok() { failures.borrow().clone().unwrap_or_else(|| "Protocol transport ended".into()) } else { "Protocol transport ended".into() },
        }
    };
    if let Some(connection) = connection.upgrade() {
        if connection.state.borrow().state != ConnectionState::Disconnected {
            connection.failed(reason).await;
        }
        connection.process.request_stop();
    }
}

struct SetupOwner<'a> {
    connection: &'a Connection,
    completed: bool,
}
impl Drop for SetupOwner<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.connection
                .rpc
                .fail("session setup did not complete under an owner");
        }
    }
}

struct AuthenticationOwner<'a> {
    connection: &'a Connection,
    completed: bool,
}
impl Drop for AuthenticationOwner<'_> {
    fn drop(&mut self) {
        if !self.completed {
            self.connection
                .rpc
                .fail("authentication owner dropped before completion");
        }
    }
}
