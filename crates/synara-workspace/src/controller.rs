mod handoff;
mod direct_models;
mod integrations;
mod browser;
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
use synara_runtime::{ExecutionHost, LocalHost, SecretStore, UnavailableSecretStore};
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
    pub browser: crate::BrowserService,
    browser_endpoints: StdMutex<HashMap<TaskId, crate::browser::mcp::Endpoint>>,
    backend: Arc<dyn AgentBackend>,
    interactions: Arc<dyn InteractionHandler>,
    secrets: Arc<dyn SecretStore>,
    manager: ConnectionManager,
    integrations_gate: tokio::sync::RwLock<()>,
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
        Self::with_secret_store(
            workspace,
            backend,
            interactions,
            Arc::new(UnavailableSecretStore::unavailable()),
        )
    }

    pub fn with_secret_store(
        workspace: WorkspaceService,
        backend: Arc<dyn AgentBackend>,
        interactions: Arc<dyn InteractionHandler>,
        secrets: Arc<dyn SecretStore>,
    ) -> Self {
        Self {
            browser: crate::BrowserService::default(),
            browser_endpoints: StdMutex::new(HashMap::new()),
            workspace,
            backend,
            interactions,
            secrets,
            manager: ConnectionManager::default(),
            integrations_gate: tokio::sync::RwLock::new(()),
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
        let host: Arc<dyn ExecutionHost> = match &workspace.location {
            WorkspaceLocation::Local { .. } => Arc::new(LocalHost),
            WorkspaceLocation::Ssh { .. } => {
                let profile = self
                    .workspace
                    .ssh_profile(workspace.id)
                    .await?
                    .ok_or_else(|| {
                        WorkspaceError::Invalid(
                            "remote workspace is missing its pinned SSH connection profile".into(),
                        )
                    })?;
                Arc::new(profile.host(&workspace)?)
            }
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
        if profile.registry.is_some()
            && matches!(
                self.workspace.workspace_for_task(task).await?.location,
                WorkspaceLocation::Ssh { .. }
            )
        {
            return Err(WorkspaceError::Invalid("registry installations are local. Configure an agent installed on the SSH host instead".into()));
        }
        let context = self.context(task).await?;
        let spec = profile
            .launch_spec_with_secret_store(self.secrets.as_ref())
            .await?;
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
        let _integrations = self.integrations_gate.read().await;
        let slot = self.slot(id).await?;
        let _gate = slot.creation.lock().await;
        self.require_agent_route(id).await?;
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
        let mut options = SessionOptions::new(task.thread_id, task.working_directory.clone());
        options.context_servers = self.managed_mcp_context(&task, connection.as_ref()).await?;
        if let Some(context) = self.browser_context(id, &profile, connection.as_ref())? { options.context_servers.push(context); }
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
        self.submit_prompt(id, text, None).await
    }

    /// Explicit composer send. Other callers retain text-only submission.
    pub async fn submit_with_attachments(&self, id: TaskId, text: String, revision: u64) -> WorkspaceResult<String> {
        self.submit_prompt(id, text, Some(revision)).await
    }

    async fn submit_prompt(&self, id: TaskId, text: String, attachments: Option<u64>) -> WorkspaceResult<String> {
        self.submit_prompt_owned(id, text, attachments, tokio_util::sync::CancellationToken::new()).await
    }

    /// A revocable foreground continuation still uses the one existing task owner.
    pub async fn submit_interruptible(&self, id: TaskId, text: String, cancellation: tokio_util::sync::CancellationToken) -> WorkspaceResult<String> {
        self.submit_prompt_owned(id, text, None, cancellation).await
    }

    async fn submit_prompt_owned(&self, id: TaskId, text: String, attachments: Option<u64>, cancellation: tokio_util::sync::CancellationToken) -> WorkspaceResult<String> {
        if cancellation.is_cancelled() { return Err(AgentError::Cancelled.into()); }
        if text.trim().is_empty() || text.len() > 1024 * 1024 {
            return Err(WorkspaceError::Invalid("prompt must contain text and fit within 1 MiB".into()));
        }
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _guard = PromptOwnership(slot.clone());
        *slot.setup_cancel.lock().map_err(|_| WorkspaceError::Worker)? = Some(cancellation.clone());
        if let Some(binding) = self.workspace.direct_model_binding(id).await? {
            if attachments.is_some() {
                return Err(WorkspaceError::Invalid("Direct chat attachment delivery is not available in this slice. Remove attachments or use an ACP agent.".into()));
            }
            return self.submit_direct(id, binding, text, cancellation).await;
        }
        // Reserve cancellation BEFORE reading/decoding images. Stop during intake
        // must not turn into a delayed agent launch when the worker completes.
        let prompt = if let Some(revision) = attachments {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(AgentError::Cancelled.into()),
                result = self.workspace.attached_prompt(id, text, revision) => result?,
            }
        } else { Prompt::text(text) };
        let session = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(AgentError::Cancelled.into()),
            result = self.session_for(id) => result?,
        };
        if cancellation.is_cancelled() { return Err(AgentError::Cancelled.into()); }
        session.prompt(prompt).await.map_err(Into::into)
    }

    pub async fn cancel(&self, id: TaskId) -> WorkspaceResult<()> {
        self.revoke_browser_use(id);
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
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        {
            let _creation = slot.creation.lock().await;
            self.require_agent_route(id).await?;
            let task = self.workspace.task(id).await?;
            let profile = self.profile(&task.agent_id).await?;
            let connection = self.connection_for(&task, &slot, &profile, false).await?;
            if !connection.info().authentication.iter().any(|auth| auth.id == method) {
                return Err(AgentError::Invalid("unknown authentication method".into()).into());
            }
            connection.authenticate(&method).await?;
        }
        self.connect(id).await
    }
    pub async fn set_option(
        &self,
        id: TaskId,
        key: String,
        value: ConfigValue,
    ) -> WorkspaceResult<()> {
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) { return Err(AgentError::Busy.into()); }
        let _ownership = PromptOwnership(slot);
        let session = self.session_for(id).await?;
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) { return Err(AgentError::Busy.into()); }
        session.set_option(&key, value).await?;
        Ok(())
    }
    pub async fn set_mode(&self, id: TaskId, mode: String) -> WorkspaceResult<()> {
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) { return Err(AgentError::Busy.into()); }
        let _ownership = PromptOwnership(slot);
        let session = self.session_for(id).await?;
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) { return Err(AgentError::Busy.into()); }
        session.set_mode(&mode).await?;
        Ok(())
    }
    pub async fn set_model(&self, id: TaskId, model: String) -> WorkspaceResult<()> {
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) { return Err(AgentError::Busy.into()); }
        let _ownership = PromptOwnership(slot);
        let session = self.session_for(id).await?;
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) { return Err(AgentError::Busy.into()); }
        session.set_model(&model).await?;
        Ok(())
    }
    pub async fn switch_agent(&self, id: TaskId, agent: String) -> WorkspaceResult<Task> {
        self.revoke_browser_use(id);
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        let _gate = slot.creation.lock().await;
        self.require_agent_route(id).await?;
        let old = slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
        if let Some(old) = old {
            old.session.close().await?;
        }
        *slot.connection.lock().map_err(|_| WorkspaceError::Worker)? = None;
        self.workspace.set_task_agent(id, agent).await
    }
    pub async fn restart(&self, id: TaskId) -> WorkspaceResult<SessionDetails> {
        self.revoke_browser_use(id);
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        {
            let _gate = slot.creation.lock().await;
            self.require_agent_route(id).await?;
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
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        {
            let _gate = slot.creation.lock().await;
            self.require_agent_route(id).await?;
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
    /// User-confirmed permanent deletion. Reserve the task against new prompts,
    /// serialize with session setup, close its session (not the shared process),
    /// then use storage's archived-only atomic deletion invariant.
    pub async fn delete_archived_task(&self, id: TaskId) -> WorkspaceResult<()> {
        self.revoke_browser_use(id);
        let slot = self.slot(id).await?;
        if slot.active.swap(true, Ordering::AcqRel) {
            return Err(AgentError::Busy.into());
        }
        let _ownership = PromptOwnership(slot.clone());
        let _gate = slot.creation.lock().await;
        // Match session_for -> connection_for lock order. Holding a lifetime
        // read lock while waiting for creation can deadlock with a queued
        // shutdown writer and an in-flight setup waiting for its read lock.
        let _lifetime = self.lifetime.read().await;
        if self.closing.load(Ordering::Acquire) {
            return Err(WorkspaceError::Invalid("application is shutting down".into()));
        }
        if self.workspace.task(id).await?.state != TaskState::Archived {
            return Err(WorkspaceError::Invalid("Only an archived task may be permanently deleted".into()));
        }
        let old = slot.live.lock().map_err(|_| WorkspaceError::Worker)?.take();
        if let Some(old) = old {
            let result = tokio::time::timeout(std::time::Duration::from_secs(8), old.session.close()).await;
            if !matches!(result, Ok(Ok(()))) {
                *slot.live.lock().map_err(|_| WorkspaceError::Worker)? = Some(old);
                return Err(match result {
                    Ok(Err(error)) => error.into(),
                    _ => WorkspaceError::Invalid("Session close timed out. Archived data was retained".into()),
                });
            }
        }
        self.workspace.delete_task(id).await?;
        *slot.connection.lock().map_err(|_| WorkspaceError::Worker)? = None;
        self.tasks.lock().await.remove(&id);
        Ok(())
    }

    pub fn secret_store_state(&self) -> synara_runtime::SecretStoreState {
        self.secrets.state()
    }

    pub async fn shutdown(&self) -> WorkspaceResult<()> {
        self.closing.store(true, Ordering::Release);
        self.browser.shutdown();
        self.browser_endpoints.lock().map_err(|_| WorkspaceError::Worker)?.clear();
        for slot in self.tasks.lock().await.values() {
            if let Some(token) = slot.setup_cancel.lock().map_err(|_| WorkspaceError::Worker)?.as_ref() { token.cancel(); }
        }
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

#[cfg(test)]
mod device_settings_tests {
    use super::*;
    struct NeverLaunch;
    #[async_trait::async_trait]
    impl AgentBackend for NeverLaunch {
        async fn connect(&self, _: &AgentSpec, _: ConnectionContext) -> AgentResult<Arc<dyn AgentConnection>> {
            panic!("archived deletion must never start an agent")
        }
    }
    #[tokio::test]
    async fn goals_cancelled_preparation_never_launches_or_retries() {
        let root=tempfile::tempdir().unwrap(); let workspace=WorkspaceService::memory().unwrap();
        let project=workspace.add_local_workspace(root.path().into()).await.unwrap();
        let task=workspace.create_task(project.id,"Goal cancellation".into(),crate::default_profiles()[0].id.clone()).await.unwrap();
        let controller=Arc::new(Controller::new(workspace.clone(),Arc::new(NeverLaunch),Arc::new(DenyInteractions)));
        let token=tokio_util::sync::CancellationToken::new();token.cancel();
        assert!(controller.submit_interruptible(task.id,"not sent".into(),token).await.is_err());
        let slot=controller.slot(task.id).await.unwrap();let gate=slot.creation.lock().await;
        let token=tokio_util::sync::CancellationToken::new();let cancel=token.clone();let c=controller.clone();
        let submission=tokio::spawn(async move{c.submit_interruptible(task.id,"cancel before session creation".into(),token).await});
        tokio::time::timeout(std::time::Duration::from_secs(2),async {while !slot.active.load(Ordering::Acquire){tokio::task::yield_now().await;}}).await.unwrap();
        cancel.cancel();drop(gate);assert!(submission.await.unwrap().is_err());
        assert!(!slot.active.load(Ordering::Acquire));assert!(workspace.session(task.thread_id).await.unwrap().is_none());assert!(workspace.thread(task.thread_id).await.unwrap().messages.is_empty());
    }
    #[tokio::test]
    async fn queued_archived_deletion_does_not_block_shutdown_or_delete_after_closing() {
        let root = tempfile::tempdir().unwrap();
        let workspace = WorkspaceService::memory().unwrap();
        let project = workspace.add_local_workspace(root.path().to_path_buf()).await.unwrap();
        let task = workspace.create_task(project.id, "Shutdown deletion".into(), crate::default_profiles()[0].id.clone()).await.unwrap();
        workspace.archive_task(task.id).await.unwrap();
        let controller = Arc::new(Controller::new(workspace.clone(), Arc::new(NeverLaunch), Arc::new(DenyInteractions)));
        let slot = controller.slot(task.id).await.unwrap();
        let setup = slot.creation.lock().await;
        let deletion = {
            let controller = controller.clone();
            tokio::spawn(async move { controller.delete_archived_task(task.id).await })
        };
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while !slot.active.load(Ordering::Acquire) {
                tokio::task::yield_now().await;
            }
        }).await.unwrap();
        // A queued deletion must not retain a lifetime read lock while another
        // setup owns creation. Otherwise this shutdown writer cannot progress.
        tokio::time::timeout(std::time::Duration::from_secs(2), controller.shutdown()).await.unwrap().unwrap();
        drop(setup);
        assert!(tokio::time::timeout(std::time::Duration::from_secs(2), deletion).await.unwrap().unwrap().is_err());
        assert!(workspace.task(task.id).await.is_ok());
        assert!(!slot.active.load(Ordering::Acquire));
    }
    #[tokio::test]
    async fn permanent_deletion_rejects_unarchived_and_active_tasks_then_removes_archived_data() {
        let root = tempfile::tempdir().unwrap();
        let workspace = WorkspaceService::memory().unwrap();
        let project = workspace.add_local_workspace(root.path().to_path_buf()).await.unwrap();
        let task = workspace.create_task(project.id, "Archived deletion".into(), crate::default_profiles()[0].id.clone()).await.unwrap();
        let controller = Controller::new(workspace.clone(), Arc::new(NeverLaunch), Arc::new(DenyInteractions));
        assert!(controller.delete_archived_task(task.id).await.is_err());
        assert!(workspace.task(task.id).await.is_ok());
        workspace.archive_task(task.id).await.unwrap();
        let slot = controller.slot(task.id).await.unwrap();
        slot.active.store(true, Ordering::Release);
        assert!(matches!(controller.delete_archived_task(task.id).await, Err(WorkspaceError::Agent(AgentError::Busy))));
        assert!(workspace.task(task.id).await.is_ok());
        slot.active.store(false, Ordering::Release);
        controller.delete_archived_task(task.id).await.unwrap();
        assert!(matches!(workspace.task(task.id).await, Err(WorkspaceError::NotFound)));
        assert!(root.path().is_dir());
    }
}
