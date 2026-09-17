use crate::{AgentProfile, WorkspaceError, WorkspaceResult, WorkspaceService};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
};
use synara_agent::*;
use synara_core::*;
use synara_runtime::{ExecutionHost, LocalHost, SshHost, SshTarget};
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct SessionDetails {
    pub connection: ConnectionInfo,
    pub configuration: SessionConfiguration,
    pub session_id: Option<String>,
}
struct LiveSession {
    profile: AgentProfile,
    session: Arc<dyn AgentSession>,
    connection: Arc<dyn AgentConnection>,
}
#[derive(Default)]
struct TaskSlot {
    creation: Mutex<()>,
    live: StdMutex<Option<LiveSession>>,
    connection: StdMutex<Option<Arc<dyn AgentConnection>>>,
    active: AtomicBool,
    setup_cancel: StdMutex<Option<tokio_util::sync::CancellationToken>>,
}
impl TaskSlot {
    fn connection(&self) -> WorkspaceResult<Option<Arc<dyn AgentConnection>>> {
        Ok(self
            .connection
            .lock()
            .map_err(|_| WorkspaceError::Worker)?
            .clone())
    }
    fn session(&self) -> WorkspaceResult<Option<Arc<dyn AgentSession>>> {
        Ok(self
            .live
            .lock()
            .map_err(|_| WorkspaceError::Worker)?
            .as_ref()
            .map(|l| l.session.clone()))
    }
}
/// Orchestrates durable tasks through the protocol-independent agent interface.
/// Each task owns its session. The connection manager shares processes by launch and workspace.
pub struct Controller {
    pub workspace: WorkspaceService,
    backend: Arc<dyn AgentBackend>,
    interactions: Arc<dyn InteractionHandler>,
    manager: ConnectionManager,
    tasks: Mutex<HashMap<TaskId, Arc<TaskSlot>>>,
    closing: AtomicBool,
    lifetime: tokio::sync::RwLock<()>,
}
impl Controller {
    pub fn new(
        workspace: WorkspaceService,
        backend: Arc<dyn AgentBackend>,
        interactions: Arc<dyn InteractionHandler>,
    ) -> Self {
        Self {
            workspace,
            backend,
            interactions,
            manager: ConnectionManager::default(),
            tasks: Mutex::new(HashMap::new()),
            closing: AtomicBool::new(false),
            lifetime: tokio::sync::RwLock::new(()),
        }
    }
    async fn slot(&self, id: TaskId) -> WorkspaceResult<Arc<TaskSlot>> {
        if self.closing.load(Ordering::Acquire) {
            return Err(WorkspaceError::Invalid(
                "application is shutting down".into(),
            ));
        }
        let mut tasks = self.tasks.lock().await;
        if tasks.len() >= 256 && !tasks.contains_key(&id) {
            return Err(AgentError::Limit.into());
        }
        Ok(tasks.entry(id).or_default().clone())
    }
    async fn context(&self, task: &Task) -> WorkspaceResult<ConnectionContext> {
        let workspace = self.workspace.workspace_for_task(task).await?;
        let host: Arc<dyn ExecutionHost> = match workspace.location {
            WorkspaceLocation::Local { .. } => Arc::new(LocalHost),
            WorkspaceLocation::Ssh {
                host, port, user, ..
            } => Arc::new(SshHost {
                target: SshTarget { host, port, user },
            }),
        };
        Ok(ConnectionContext {
            host,
            cwd: task.working_directory.clone(),
            events: Arc::new(self.workspace.clone()),
            interactions: self.interactions.clone(),
        })
    }
    async fn profile(&self, id: &str) -> WorkspaceResult<AgentProfile> {
        self.workspace
            .profiles()
            .await?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| {
                WorkspaceError::Invalid(
                    "the task's agent profile is missing. Select another agent in Settings.".into(),
                )
            })
    }
    async fn connection_for(
        &self,
        task: &Task,
        slot: &TaskSlot,
        profile: &AgentProfile,
        restart: bool,
    ) -> WorkspaceResult<Arc<dyn AgentConnection>> {
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) {
            return Err(WorkspaceError::Invalid(
                "application is shutting down".into(),
            ));
        }
        let context = self.context(task).await?;
        let spec = profile.launch_spec()?;
        let connection = if restart {
            self.manager
                .restart(self.backend.as_ref(), &spec, context)
                .await?
        } else {
            self.manager
                .connection(self.backend.as_ref(), &spec, context)
                .await?
        };
        *slot.connection.lock().map_err(|_| WorkspaceError::Worker)? = Some(connection.clone());
        Ok(connection)
    }
    async fn session_for(&self, id: TaskId) -> WorkspaceResult<Arc<dyn AgentSession>> {
        let slot = self.slot(id).await?;
        let _gate = slot.creation.lock().await;
        let task = self.workspace.task(id).await?;
        let profile = self.profile(&task.agent_id).await?;
        {
            let live = slot.live.lock().map_err(|_| WorkspaceError::Worker)?;
            if let Some(live) = live.as_ref()
                && live.profile == profile
                && live.connection.info().state == ConnectionState::Connected
            {
                return Ok(live.session.clone());
            }
        }
        let old = slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
        if let Some(old) = old {
            let _ = old.session.close().await;
        }
        let connection = self.connection_for(&task, &slot, &profile, false).await?;
        let options = SessionOptions::new(task.thread_id, task.working_directory.clone());
        let previous = self.workspace.session(task.thread_id).await?;
        let session = if let Some(previous) = previous.filter(|r| {
            r.agent_id == task.agent_id && r.working_directory == task.working_directory
        }) {
            let capabilities = connection.info().capabilities;
            if capabilities.resume_session {
                // A restore failure is not a license to silently discard the session reference.
                connection
                    .restore_session(
                        &previous.remote_id,
                        options,
                        RestoreMode::ResumeWithoutReplay,
                    )
                    .await?
            } else if capabilities.load_session {
                connection
                    .restore_session(&previous.remote_id, options, RestoreMode::ReplayHistory)
                    .await?
            } else {
                self.workspace.record(task.thread_id,ThreadEvent::Notice{message:"This agent does not advertise session restoration. Starting a new agent session. Your saved transcript remains available.".into()}).await?;
                connection.new_session(options).await?
            }
        } else {
            connection.new_session(options).await?
        };
        if let Err(error) = self
            .workspace
            .save_session(
                task.thread_id,
                SessionReference {
                    agent_id: task.agent_id,
                    remote_id: session.id().into(),
                    working_directory: task.working_directory,
                    title: None,
                },
            )
            .await
        {
            let _ = session.close().await;
            return Err(error);
        }
        *slot.live.lock().map_err(|_| WorkspaceError::Worker)? = Some(LiveSession {
            profile,
            session: session.clone(),
            connection,
        });
        Ok(session)
    }
    pub async fn connect(&self, id: TaskId) -> WorkspaceResult<SessionDetails> {
        self.session_for(id).await?;
        self.details(id).await?.ok_or(WorkspaceError::Worker)
    }
    pub async fn details(&self, id: TaskId) -> WorkspaceResult<Option<SessionDetails>> {
        let slot = self.slot(id).await?;
        let Some(connection) = slot.connection()? else {
            return Ok(None);
        };
        let session = slot.session()?;
        Ok(Some(SessionDetails {
            connection: connection.info(),
            configuration: session
                .as_ref()
                .map_or_else(SessionConfiguration::default, |s| s.configuration()),
            session_id: session.as_ref().map(|s| s.id().to_owned()),
        }))
    }
    pub async fn submit(&self, id: TaskId, text: String) -> WorkspaceResult<String> {
        if text.trim().is_empty() || text.len() > 1024 * 1024 {
            return Err(WorkspaceError::Invalid(
                "prompt must contain text and fit within 1 MiB".into(),
            ));
        }
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let cancellation = tokio_util::sync::CancellationToken::new();
        *slot
            .setup_cancel
            .lock()
            .map_err(|_| WorkspaceError::Worker)? = Some(cancellation.clone());
        let _guard = PromptOwnership(slot);
        let session = tokio::select! {
            result=self.session_for(id)=>result?,
            ()=cancellation.cancelled()=>return Err(AgentError::Cancelled.into()),
        };
        if cancellation.is_cancelled() {
            return Err(AgentError::Cancelled.into());
        }
        session.prompt(Prompt::text(text)).await.map_err(Into::into)
    }

    pub async fn cancel(&self, id: TaskId) -> WorkspaceResult<()> {
        let slot = self.slot(id).await?;
        if let Some(token) = slot
            .setup_cancel
            .lock()
            .map_err(|_| WorkspaceError::Worker)?
            .as_ref()
        {
            token.cancel();
        }
        if let Some(session) = slot.session()? {
            session.cancel().await?;
        }
        Ok(())
    }
    pub async fn authenticate(
        &self,
        id: TaskId,
        method: String,
    ) -> WorkspaceResult<SessionDetails> {
        let slot = self.slot(id).await?;
        let task = self.workspace.task(id).await?;
        let profile = self.profile(&task.agent_id).await?;
        let connection = self.connection_for(&task, &slot, &profile, false).await?;
        if !connection
            .info()
            .authentication
            .iter()
            .any(|auth| auth.id == method)
        {
            return Err(AgentError::Invalid("unknown authentication method".into()).into());
        }
        connection.authenticate(&method).await?;
        self.connect(id).await
    }
    pub async fn set_option(
        &self,
        id: TaskId,
        key: String,
        value: ConfigValue,
    ) -> WorkspaceResult<()> {
        self.session_for(id).await?.set_option(&key, value).await?;
        Ok(())
    }
    pub async fn set_mode(&self, id: TaskId, mode: String) -> WorkspaceResult<()> {
        self.session_for(id).await?.set_mode(&mode).await?;
        Ok(())
    }
    pub async fn set_model(&self, id: TaskId, model: String) -> WorkspaceResult<()> {
        self.session_for(id).await?.set_model(&model).await?;
        Ok(())
    }
    pub async fn switch_agent(&self, id: TaskId, agent: String) -> WorkspaceResult<Task> {
        let slot = self.slot(id).await?;
        if slot.active.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        let _gate = slot.creation.lock().await;
        let old = slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
        if let Some(old) = old {
            old.session.close().await?;
        }
        *slot.connection.lock().map_err(|_| WorkspaceError::Worker)? = None;
        self.workspace.set_task_agent(id, agent).await
    }
    pub async fn restart(&self, id: TaskId) -> WorkspaceResult<SessionDetails> {
        let slot = self.slot(id).await?;
        {
            let _gate = slot.creation.lock().await;
            let task = self.workspace.task(id).await?;
            let profile = self.profile(&task.agent_id).await?;
            slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
            self.connection_for(&task, &slot, &profile, true).await?;
        }
        self.connect(id).await
    }
    /// Explicitly start fresh without deleting the durable transcript.
    pub async fn fresh_session(&self, id: TaskId) -> WorkspaceResult<SessionDetails> {
        let slot = self.slot(id).await?;
        if slot.active.load(Ordering::Acquire) {
            return Err(AgentError::Busy.into());
        }
        {
            let _gate = slot.creation.lock().await;
            let old = slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
            if let Some(old) = old {
                old.session.close().await?;
            }
            let task = self.workspace.task(id).await?;
            self.workspace.forget_session(task.thread_id).await?;
            self.workspace.record(task.thread_id,ThreadEvent::Notice{message:"A new agent session was requested. Saved conversation history has not been deleted.".into()}).await?;
        }
        self.connect(id).await
    }
    pub async fn trace(&self, id: TaskId, clear: bool) -> WorkspaceResult<Vec<TraceEntry>> {
        let Some(connection) = self.slot(id).await?.connection()? else {
            return Ok(vec![]);
        };
        if clear {
            connection.clear_trace();
        }
        Ok(connection.trace())
    }
    pub async fn shutdown(&self) -> WorkspaceResult<()> {
        self.closing.store(true, Ordering::Release);
        let _lifetime = self.lifetime.write().await;
        self.manager.disconnect_all().await?;
        self.tasks.lock().await.clear();
        Ok(())
    }
}
struct PromptOwnership(Arc<TaskSlot>);
impl Drop for PromptOwnership {
    fn drop(&mut self) {
        if let Ok(mut token) = self.0.setup_cancel.lock() {
            token.take();
        }
        self.0.active.store(false, Ordering::Release);
    }
}
