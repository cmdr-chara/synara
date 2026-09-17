use crate::wire;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, AtomicUsize},
    },
};
use synara_agent::{
    AgentError, AgentResult, ConnectionContext, InteractionContext, SessionOptions,
};
use synara_core::{SessionConfiguration, ThreadEvent, ThreadId};
use synara_runtime::WorkspaceFs;
use tokio_util::sync::CancellationToken;

pub(crate) struct SessionState {
    pub id: String,
    pub thread_id: ThreadId,
    pub cwd: PathBuf,
    pub roots: Vec<Arc<WorkspaceFs>>,
    pub configuration: RwLock<SessionConfiguration>,
    pub prompt_gate: tokio::sync::Mutex<()>,
    pub mutation_gate: tokio::sync::Mutex<()>,
    pub update_gate: tokio::sync::Mutex<()>,
    pub active: AtomicBool,
    pub closed: AtomicBool,
    pub turn: Mutex<CancellationToken>,
    pub lifetime: CancellationToken,
}
impl SessionState {
    pub async fn build(
        id: String,
        options: &SessionOptions,
        context: &ConnectionContext,
        lifetime: CancellationToken,
    ) -> AgentResult<Arc<Self>> {
        if !options.cwd.is_absolute() {
            return Err(wire::invalid("session directory must be absolute"));
        }
        if options
            .additional_directories
            .iter()
            .any(|path| !path.is_absolute())
        {
            return Err(wire::invalid("additional directories must be absolute"));
        }
        let roots = if context.host.is_local() {
            let base = context.cwd.clone();
            let cwd = options.cwd.clone();
            let directories = options.additional_directories.clone();
            tokio::task::spawn_blocking(move || {
                let base = base
                    .canonicalize()
                    .map_err(synara_runtime::RuntimeError::from)?;
                let cwd = cwd
                    .canonicalize()
                    .map_err(synara_runtime::RuntimeError::from)?;
                if !cwd.starts_with(&base) {
                    return Err(wire::invalid(
                        "session directory is outside the selected workspace",
                    ));
                }
                let mut roots = vec![Arc::new(WorkspaceFs::open(&cwd)?)];
                // Additional roots come only from the trusted UI/API caller, never from agent callbacks.
                for directory in directories {
                    roots.push(Arc::new(WorkspaceFs::open(&directory)?));
                }
                Ok::<_, AgentError>(roots)
            })
            .await
            .map_err(|_| AgentError::Disconnected("filesystem worker stopped".into()))??
        } else {
            vec![]
        };
        Ok(Arc::new(Self {
            id,
            thread_id: options.thread_id,
            cwd: options.cwd.clone(),
            roots,
            configuration: RwLock::new(SessionConfiguration::default()),
            prompt_gate: tokio::sync::Mutex::new(()),
            mutation_gate: tokio::sync::Mutex::new(()),
            update_gate: tokio::sync::Mutex::new(()),
            active: AtomicBool::new(false),
            closed: AtomicBool::new(false),
            turn: Mutex::new(lifetime.child_token()),
            lifetime,
        }))
    }
    pub fn interaction(&self) -> InteractionContext {
        InteractionContext {
            thread_id: self.thread_id,
            session_id: self.id.clone(),
            cancelled: if self.active.load(std::sync::atomic::Ordering::Acquire) {
                self.turn.lock().unwrap().clone()
            } else {
                self.lifetime.clone()
            },
        }
    }
    pub fn filesystem(&self, path: &Path) -> AgentResult<Arc<WorkspaceFs>> {
        if !path.is_absolute() {
            return Err(wire::invalid("callback paths must be absolute"));
        }
        self.roots
            .iter()
            .filter(|root| path.starts_with(root.root()))
            .max_by_key(|root| root.root().as_os_str().len())
            .cloned()
            .ok_or_else(|| {
                AgentError::Runtime(synara_runtime::RuntimeError::Denied(
                    "path outside session roots".into(),
                ))
            })
    }
    pub async fn emit(&self, context: &ConnectionContext, event: ThreadEvent) -> AgentResult<()> {
        tokio::time::timeout(
            std::time::Duration::from_secs(15),
            context.events.emit(self.thread_id, event),
        )
        .await
        .map_err(|_| AgentError::EventDelivery)?
    }
    pub async fn apply_update(
        &self,
        context: &ConnectionContext,
        value: &serde_json::Value,
    ) -> AgentResult<()> {
        let _lock = self.update_gate.lock().await;
        self.apply_update_locked(context, value).await
    }
    pub async fn apply_update_locked(
        &self,
        context: &ConnectionContext,
        value: &serde_json::Value,
    ) -> AgentResult<()> {
        let configuration = self.configuration.read().unwrap().clone();
        for event in wire::update(value, &configuration)? {
            if let ThreadEvent::ConfigurationChanged { configuration } = &event {
                *self.configuration.write().unwrap() = configuration.clone();
            }
            self.emit(context, event).await?;
        }
        Ok(())
    }
}
#[derive(Default)]
pub(crate) struct Sessions {
    pub states: Mutex<HashMap<String, Arc<SessionState>>>,
    pub creating: AtomicUsize,
    pub early_updates: Mutex<Vec<(String, serde_json::Value)>>,
}
impl Sessions {
    pub fn get(&self, id: &str) -> AgentResult<Arc<SessionState>> {
        self.states
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .filter(|session| !session.closed.load(std::sync::atomic::Ordering::Acquire))
            .ok_or_else(|| wire::invalid("session is not owned by this connection"))
    }
}
