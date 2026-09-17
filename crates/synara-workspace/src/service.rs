use crate::{AgentProfile, StorageError, Store, default_profiles, validate_profiles};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use synara_agent::{AgentError, AgentResult, EventSink};
use synara_core::*;
use synara_runtime::{RuntimeError, WorkspaceFs};
use tokio::sync::broadcast;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Agent(#[from] AgentError),
    #[error("workspace object was not found")]
    NotFound,
    #[error("invalid workspace operation: {0}")]
    Invalid(String),
    #[error("workspace worker stopped")]
    Worker,
}
pub type WorkspaceResult<T> = Result<T, WorkspaceError>;
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub workspaces: Vec<Workspace>,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Selection {
    pub project: Option<ProjectId>,
    pub task: Option<TaskId>,
}
#[derive(Clone)]
pub struct WorkspaceService {
    store: Arc<Mutex<Store>>,
    events: broadcast::Sender<EventEnvelope>,
}
impl WorkspaceService {
    pub async fn open(path: PathBuf) -> WorkspaceResult<Self> {
        let store = tokio::task::spawn_blocking(move || Store::open(&path))
            .await
            .map_err(|_| WorkspaceError::Worker)??;
        Ok(Self::from_store(store))
    }
    pub fn memory() -> WorkspaceResult<Self> {
        Ok(Self::from_store(Store::memory()?))
    }
    fn from_store(store: Store) -> Self {
        let (events, _) = broadcast::channel(64);
        Self {
            store: Arc::new(Mutex::new(store)),
            events,
        }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<EventEnvelope> {
        self.events.subscribe()
    }
    pub(crate) async fn access<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Store) -> WorkspaceResult<R> + Send + 'static,
    ) -> WorkspaceResult<R> {
        let store = self.store.clone();
        tokio::task::spawn_blocking(move || {
            let mut guard = store.lock().map_err(|_| WorkspaceError::Worker)?;
            f(&mut guard)
        })
        .await
        .map_err(|_| WorkspaceError::Worker)?
    }
    pub async fn catalog(&self) -> WorkspaceResult<Catalog> {
        self.access(|store| catalog(store)).await
    }
    pub async fn add_local_workspace(&self, root: PathBuf) -> WorkspaceResult<Project> {
        let root = tokio::task::spawn_blocking(move || {
            WorkspaceFs::open(&root).map(|fs| fs.root().to_owned())
        })
        .await
        .map_err(|_| WorkspaceError::Worker)??;
        self.access(move |store| {
            let existing = catalog(store)?;
            if let Some(workspace) = existing
                .workspaces
                .iter()
                .find(|w| matches!(&w.location,WorkspaceLocation::Local{root:r} if r==&root))
                && let Some(project) = existing.projects.iter().find(|p| {
                    p.workspace_id == workspace.id && p.relative_directory.as_os_str().is_empty()
                })
            {
                return Ok(project.clone());
            }
            let name = root
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Workspace")
                .to_owned();
            let workspace = Workspace {
                id: WorkspaceId::new(),
                name: name.clone(),
                location: WorkspaceLocation::Local { root },
            };
            let project = Project {
                id: ProjectId::new(),
                workspace_id: workspace.id,
                name,
                relative_directory: PathBuf::new(),
            };
            store.create_workspace_project(&workspace, &project)?;
            Ok(project)
        })
        .await
    }
    pub async fn create_task(
        &self,
        project: ProjectId,
        title: String,
        agent_id: String,
    ) -> WorkspaceResult<Task> {
        if title.trim().is_empty() || title.len() > 400 || title.contains('\0') {
            return Err(WorkspaceError::Invalid(
                "a task needs a title of at most 400 bytes".into(),
            ));
        }
        self.access(move |store| {
            let profiles: Vec<AgentProfile> = store
                .preference("agent_profiles")?
                .unwrap_or_else(default_profiles);
            if !profiles.iter().any(|p| p.id == agent_id) {
                return Err(WorkspaceError::Invalid("unknown agent profile".into()));
            }
            let c = catalog(store)?;
            let project = c
                .projects
                .iter()
                .find(|p| p.id == project)
                .ok_or(WorkspaceError::NotFound)?;
            let workspace = c
                .workspaces
                .iter()
                .find(|w| w.id == project.workspace_id)
                .ok_or(WorkspaceError::NotFound)?;
            let working_directory = project_directory(workspace, project)?;
            let task = Task {
                id: TaskId::new(),
                project_id: project.id,
                title: title.trim().into(),
                state: TaskState::Ready,
                thread_id: ThreadId::new(),
                agent_id,
                working_directory,
                updated_at_ms: now_ms(),
            };
            store.save_task(&task)?;
            Ok(task)
        })
        .await
    }
    pub async fn task(&self, id: TaskId) -> WorkspaceResult<Task> {
        self.access(move |store| store.task(id)?.ok_or(WorkspaceError::NotFound))
            .await
    }
    pub async fn thread(&self, id: ThreadId) -> WorkspaceResult<Thread> {
        self.access(move |store| Ok(store.replay(id)?)).await
    }
    pub async fn session(&self, id: ThreadId) -> WorkspaceResult<Option<SessionReference>> {
        self.access(move |store| Ok(store.session(id)?)).await
    }
    pub async fn save_session(
        &self,
        id: ThreadId,
        session: SessionReference,
    ) -> WorkspaceResult<()> {
        self.access(move |store| Ok(store.save_session(id, &session)?))
            .await
    }
    pub async fn forget_session(&self, id: ThreadId) -> WorkspaceResult<()> {
        self.access(move |store| Ok(store.forget_session(id)?))
            .await
    }
    pub async fn selection(&self) -> WorkspaceResult<Selection> {
        self.access(|store| Ok(store.preference("selection")?.unwrap_or_default()))
            .await
    }
    pub async fn save_selection(&self, value: Selection) -> WorkspaceResult<()> {
        self.access(move |store| Ok(store.set_preference("selection", &value)?))
            .await
    }
    pub async fn profiles(&self) -> WorkspaceResult<Vec<AgentProfile>> {
        self.access(|store| {
            let profiles = store
                .preference("agent_profiles")?
                .unwrap_or_else(default_profiles);
            validate_profiles(&profiles)?;
            Ok(profiles)
        })
        .await
    }
    pub async fn save_profiles(&self, profiles: Vec<AgentProfile>) -> WorkspaceResult<()> {
        validate_profiles(&profiles)?;
        self.access(move |store| Ok(store.set_preference("agent_profiles", &profiles)?))
            .await
    }
    pub async fn set_task_agent(&self, id: TaskId, agent: String) -> WorkspaceResult<Task> {
        self.access(move |store| {
            let profiles = store
                .preference("agent_profiles")?
                .unwrap_or_else(default_profiles);
            if !profiles.iter().any(|p| p.id == agent) {
                return Err(WorkspaceError::Invalid("unknown agent profile".into()));
            }
            let mut task = store.task(id)?.ok_or(WorkspaceError::NotFound)?;
            if task.agent_id != agent {
                task.agent_id = agent;
                task.updated_at_ms = now_ms();
                store.save_task(&task)?;
                store.forget_session(task.thread_id)?;
            }
            Ok(task)
        })
        .await
    }
    /// Called once before accepting any new agent operations, never while sessions are live.
    pub async fn recover_interrupted(&self) -> WorkspaceResult<usize> {
        let tasks = self.catalog().await?.tasks;
        let mut recovered = 0;
        for task in tasks {
            let thread = self.thread(task.thread_id).await?;
            if matches!(thread.state, TaskState::Running | TaskState::Waiting)
                || thread.history_in_progress()
            {
                self.record(task.thread_id,ThreadEvent::Error{message:"The previous session was interrupted. Stored history is preserved. Reconnect to continue.".into(),recoverable:false}).await?;
                recovered += 1;
            }
        }
        Ok(recovered)
    }
    pub async fn record(
        &self,
        thread_id: ThreadId,
        event: ThreadEvent,
    ) -> WorkspaceResult<EventEnvelope> {
        let events = self.events.clone();
        self.access(move |store| {
            let sequence = store
                .last_sequence(thread_id)?
                .checked_add(1)
                .ok_or(StorageError::Sequence)?;
            let envelope = EventEnvelope {
                id: EventId::new(),
                thread_id,
                sequence,
                timestamp_ms: now_ms(),
                event,
            };
            store.append(&envelope)?;
            // Synchronous publication under the same lock preserves durable commit order.
            // A lagging observer must reload from SQLite instead of losing history.
            let _ = events.send(envelope.clone());
            Ok(envelope)
        })
        .await
    }
    pub async fn workspace_for_task(&self, task: &Task) -> WorkspaceResult<Workspace> {
        let task = task.clone();
        self.access(move |store| {
            let c = catalog(store)?;
            let project = c
                .projects
                .iter()
                .find(|p| p.id == task.project_id)
                .ok_or(WorkspaceError::NotFound)?;
            let workspace = c
                .workspaces
                .iter()
                .find(|w| w.id == project.workspace_id)
                .ok_or(WorkspaceError::NotFound)?;
            if project_directory(workspace, project)? != task.working_directory {
                return Err(WorkspaceError::Invalid(
                    "task directory does not match its project".into(),
                ));
            }
            Ok(workspace.clone())
        })
        .await
    }
}
#[async_trait]
impl EventSink for WorkspaceService {
    async fn emit(&self, thread_id: ThreadId, event: ThreadEvent) -> AgentResult<()> {
        self.record(thread_id, event)
            .await
            .map(|_| ())
            .map_err(|_| AgentError::EventDelivery)
    }
}
fn catalog(store: &Store) -> WorkspaceResult<Catalog> {
    let workspaces = store.workspaces()?;
    let mut projects = vec![];
    let mut tasks = vec![];
    for workspace in &workspaces {
        for project in store.projects(workspace.id)? {
            tasks.extend(store.tasks(project.id)?);
            projects.push(project);
        }
    }
    tasks.sort_by_key(|task| std::cmp::Reverse(task.updated_at_ms));
    Ok(Catalog {
        workspaces,
        projects,
        tasks,
    })
}
fn project_directory(workspace: &Workspace, project: &Project) -> WorkspaceResult<PathBuf> {
    if project.relative_directory.components().any(|c| {
        !matches!(
            c,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    }) {
        return Err(WorkspaceError::Invalid("invalid project directory".into()));
    }
    match &workspace.location {
        WorkspaceLocation::Local { root } => {
            let fs = WorkspaceFs::open(root)?;
            let path = fs.root().join(&project.relative_directory);
            if !project.relative_directory.as_os_str().is_empty() {
                fs.entries(&project.relative_directory)?;
            }
            Ok(path)
        }
        WorkspaceLocation::Ssh { root, .. } => {
            if !root.starts_with('/') || root.contains('\0') {
                return Err(WorkspaceError::Invalid("invalid remote root".into()));
            }
            Ok(Path::new(root).join(&project.relative_directory))
        }
    }
}
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn concurrent_emissions_are_durable_ordered_and_replayable() {
        let service = WorkspaceService::memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let task = service
            .create_task(project.id, "Task".into(), "opencode".into())
            .await
            .unwrap();
        let mut receiver = service.subscribe();
        let mut handles = vec![];
        for n in 0..32 {
            let service = service.clone();
            handles.push(tokio::spawn(async move {
                service
                    .record(
                        task.thread_id,
                        ThreadEvent::Notice {
                            message: n.to_string(),
                        },
                    )
                    .await
                    .unwrap()
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        for n in 1..=32 {
            assert_eq!(receiver.recv().await.unwrap().sequence, n);
        }
        assert_eq!(
            service.thread(task.thread_id).await.unwrap().last_sequence,
            32
        );
    }
    #[tokio::test]
    async fn interrupted_permissions_do_not_reappear_as_actionable_after_restart() {
        let service = WorkspaceService::memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let task = service
            .create_task(project.id, "Task".into(), "opencode".into())
            .await
            .unwrap();
        service
            .record(
                task.thread_id,
                ThreadEvent::PromptStarted {
                    turn: "turn".into(),
                },
            )
            .await
            .unwrap();
        service
            .record(
                task.thread_id,
                ThreadEvent::PermissionRequested {
                    request: PermissionRequest {
                        id: "p".into(),
                        tool_id: None,
                        title: "Run?".into(),
                        choices: vec![],
                    },
                },
            )
            .await
            .unwrap();
        assert_eq!(service.recover_interrupted().await.unwrap(), 1);
        let thread = service.thread(task.thread_id).await.unwrap();
        assert_eq!(thread.state, TaskState::Failed);
        assert!(thread.permissions.is_empty());
        assert_eq!(service.recover_interrupted().await.unwrap(), 0);
    }
    #[tokio::test]
    async fn readding_workspace_does_not_duplicate_projects() {
        let service = WorkspaceService::memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let first = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let second = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(service.catalog().await.unwrap().workspaces.len(), 1);
    }
}
