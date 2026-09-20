use crate::{
    AgentProfile, NewSshWorkspace, SshWorkspaceProfile, StorageError, Store, default_profiles,
    parse_profiles, upsert_ssh_profile, validate_profiles,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use synara_agent::{AgentError, AgentResult, EventSink};
use synara_core::*;
use synara_runtime::{PinnedSshHost, RemoteWorkspaceFs, RuntimeError, WorkspaceFs};
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
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
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
    /// Explicitly requested backup. No automatic export, upload, or process launch.
    pub async fn backup_to(
        &self,
        path: PathBuf,
        options: crate::RecoveryOptions,
    ) -> WorkspaceResult<crate::RecoveryReceipt> {
        self.access(move |store| Ok(store.backup_to(&path, &options)?))
            .await
    }
    /// Return a new database file for a later explicit open. Never replace the active store.
    pub async fn restore_to(
        backup: PathBuf,
        destination: PathBuf,
        options: crate::RecoveryOptions,
    ) -> WorkspaceResult<crate::RecoveryReceipt> {
        tokio::task::spawn_blocking(move || Store::restore_to(&backup, &destination, &options))
            .await
            .map_err(|_| WorkspaceError::Worker)?
            .map_err(Into::into)
    }

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

    /// Enroll a remote workspace only after the pinned host and remote helper
    /// prove the requested root. Private key contents are never persisted.
    pub async fn create_project(
        &self,
        workspace_id: WorkspaceId,
        name: String,
        relative_directory: PathBuf,
    ) -> WorkspaceResult<Project> {
        let name = catalog_name(&name, "project")?;
        self.access(move |store| {
            let existing = catalog(store)?;
            let workspace = existing
                .workspaces
                .iter()
                .find(|workspace| workspace.id == workspace_id)
                .ok_or(WorkspaceError::NotFound)?;
            if existing.projects.iter().any(|project| {
                project.workspace_id == workspace_id
                    && project.relative_directory == relative_directory
            }) {
                return Err(WorkspaceError::Invalid(
                    "this project directory is already registered".into(),
                ));
            }
            let project = Project {
                id: ProjectId::new(),
                workspace_id,
                name,
                relative_directory,
            };
            project_directory(workspace, &project)?;
            store.save_project(&project)?;
            Ok(project)
        })
        .await
    }

    pub async fn rename_workspace(
        &self,
        id: WorkspaceId,
        name: String,
    ) -> WorkspaceResult<Workspace> {
        let name = catalog_name(&name, "workspace")?;
        self.access(move |store| {
            let mut workspace = catalog(store)?
                .workspaces
                .into_iter()
                .find(|workspace| workspace.id == id)
                .ok_or(WorkspaceError::NotFound)?;
            workspace.name = name;
            store.save_workspace(&workspace)?;
            Ok(workspace)
        })
        .await
    }

    pub async fn rename_project(&self, id: ProjectId, name: String) -> WorkspaceResult<Project> {
        let name = catalog_name(&name, "project")?;
        self.access(move |store| {
            let mut project = catalog(store)?
                .projects
                .into_iter()
                .find(|project| project.id == id)
                .ok_or(WorkspaceError::NotFound)?;
            project.name = name;
            store.save_project(&project)?;
            Ok(project)
        })
        .await
    }

    pub async fn rename_task(&self, id: TaskId, title: String) -> WorkspaceResult<Task> {
        let title = catalog_name(&title, "task")?;
        let task = self.task(id).await?;
        self.record(task.thread_id, ThreadEvent::TitleChanged { title })
            .await?;
        self.task(id).await
    }

    pub async fn archive_task(&self, id: TaskId) -> WorkspaceResult<Task> {
        self.access(move |store| {
            let mut task = store.task(id)?.ok_or(WorkspaceError::NotFound)?;
            if matches!(task.state, TaskState::Running | TaskState::Waiting) {
                return Err(AgentError::Busy.into());
            }
            if task.state != TaskState::Archived {
                task.state = TaskState::Archived;
                task.updated_at_ms = now_ms();
                store.save_task(&task)?;
            }
            Ok(task)
        })
        .await
    }

    /// Restore the last durable conversation state without replaying any actions.
    pub async fn unarchive_task(&self, id: TaskId) -> WorkspaceResult<Task> {
        self.access(move |store| {
            let mut task = store.task(id)?.ok_or(WorkspaceError::NotFound)?;
            if task.state == TaskState::Archived {
                task.state = store.replay(task.thread_id)?.state;
                task.updated_at_ms = now_ms();
                store.save_task(&task)?;
            }
            Ok(task)
        })
        .await
    }

    pub async fn delete_task(&self, id: TaskId) -> WorkspaceResult<()> {
        self.access(move |store| {
            if !store.delete_task(id)? {
                return Err(WorkspaceError::NotFound);
            }
            Ok(())
        })
        .await
    }

    pub async fn delete_project(&self, id: ProjectId) -> WorkspaceResult<()> {
        self.access(move |store| {
            if !store.delete_project(id)? {
                return Err(WorkspaceError::NotFound);
            }
            Ok(())
        })
        .await
    }

    pub async fn delete_workspace(&self, id: WorkspaceId) -> WorkspaceResult<()> {
        self.access(move |store| {
            if !store.delete_workspace(id)? {
                return Err(WorkspaceError::NotFound);
            }
            Ok(())
        })
        .await
    }

    pub async fn recent_tasks(&self, limit: usize) -> WorkspaceResult<Vec<Task>> {
        if limit == 0 || limit > 500 {
            return Err(WorkspaceError::Invalid(
                "recent-task limit must be between 1 and 500".into(),
            ));
        }
        Ok(self
            .catalog()
            .await?
            .tasks
            .into_iter()
            .filter(|task| task.state != TaskState::Archived)
            .take(limit)
            .collect())
    }

    pub async fn add_ssh_workspace(&self, request: NewSshWorkspace) -> WorkspaceResult<Project> {
        request.validate()?;
        let host = PinnedSshHost::new(
            request.target.clone(),
            &request.known_hosts,
            &request.identity_file,
        )?;
        let remote =
            RemoteWorkspaceFs::connect(host, PathBuf::from(&request.root), &request.helper).await?;
        let root = remote.root_identity().to_owned();
        let target = request.target;
        let name = if request.name.trim().is_empty() {
            target.host.clone()
        } else {
            request.name.trim().to_owned()
        };
        let known_hosts = request.known_hosts;
        let identity_file = request.identity_file;
        let helper = request.helper;

        self.access(move |store| {
            let existing = catalog(store)?;
            if let Some(workspace) = existing.workspaces.iter().find(|workspace| {
                matches!(
                    &workspace.location,
                    WorkspaceLocation::Ssh {
                        host,
                        port,
                        user,
                        root: candidate,
                    } if host == &target.host
                        && *port == target.port
                        && user == &target.user
                        && candidate == &root
                )
            }) {
                let project = existing
                    .projects
                    .iter()
                    .find(|project| {
                        project.workspace_id == workspace.id
                            && project.relative_directory.as_os_str().is_empty()
                    })
                    .ok_or(WorkspaceError::NotFound)?
                    .clone();
                let mut profiles: Vec<SshWorkspaceProfile> =
                    store.preference("ssh_profiles")?.unwrap_or_default();
                upsert_ssh_profile(
                    &mut profiles,
                    SshWorkspaceProfile {
                        workspace_id: workspace.id,
                        known_hosts,
                        identity_file,
                        helper,
                    },
                )?;
                store.set_preference("ssh_profiles", &profiles)?;
                return Ok(project);
            }

            let workspace = Workspace {
                id: WorkspaceId::new(),
                name: name.clone(),
                location: WorkspaceLocation::Ssh {
                    host: target.host,
                    port: target.port,
                    user: target.user,
                    root,
                },
            };
            let project = Project {
                id: ProjectId::new(),
                workspace_id: workspace.id,
                name,
                relative_directory: PathBuf::new(),
            };
            let mut profiles: Vec<SshWorkspaceProfile> =
                store.preference("ssh_profiles")?.unwrap_or_default();
            upsert_ssh_profile(
                &mut profiles,
                SshWorkspaceProfile {
                    workspace_id: workspace.id,
                    known_hosts,
                    identity_file,
                    helper,
                },
            )?;
            store.create_workspace_project_with_preference(
                &workspace,
                &project,
                "ssh_profiles",
                &profiles,
            )?;
            Ok(project)
        })
        .await
    }

    pub async fn ssh_profile(
        &self,
        workspace: WorkspaceId,
    ) -> WorkspaceResult<Option<SshWorkspaceProfile>> {
        self.access(move |store| {
            let profiles: Vec<SshWorkspaceProfile> =
                store.preference("ssh_profiles")?.unwrap_or_default();
            Ok(profiles
                .into_iter()
                .find(|profile| profile.workspace_id == workspace))
        })
        .await
    }

    pub async fn create_task(
        &self,
        project: ProjectId,
        title: String,
        agent_id: String,
    ) -> WorkspaceResult<Task> {
        self.create_scoped_task(project, title, agent_id, TaskScope::Project)
            .await
    }

    pub async fn create_scoped_task(
        &self,
        project: ProjectId,
        title: String,
        agent_id: String,
        scope: TaskScope,
    ) -> WorkspaceResult<Task> {
        self.create_scoped_task_with_draft(project, title, agent_id, scope, String::new())
            .await
    }

    /// Save an unsent prompt with its task atomically. This never connects or runs an agent.
    pub async fn create_scoped_task_with_draft(
        &self,
        project: ProjectId,
        title: String,
        agent_id: String,
        scope: TaskScope,
        draft: String,
    ) -> WorkspaceResult<Task> {
        if draft.len() > 1024 * 1024 {
            return Err(WorkspaceError::Invalid("Task draft exceeds 1 MiB".into()));
        }
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
                scope,
            };
            store.insert_task_with_draft(&task, draft)?;
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
        self.access(|store| {
            let mut value: Selection = store.preference("selection")?.unwrap_or_default();
            let c = catalog(store)?;
            if value
                .project
                .is_some_and(|id| !c.projects.iter().any(|project| project.id == id))
            {
                value = Selection::default();
            }
            if let Some(task_id) = value.task {
                let valid = value.project.is_some_and(|project_id| {
                    c.tasks.iter().any(|task| {
                        task.id == task_id
                            && task.project_id == project_id
                            && task.state != TaskState::Archived
                    })
                });
                if !valid {
                    value.task = None;
                }
            }
            store.set_preference("selection", &value)?;
            Ok(value)
        })
        .await
    }
    pub async fn save_selection(&self, value: Selection) -> WorkspaceResult<()> {
        self.access(move |store| {
            let c = catalog(store)?;
            if let Some(project) = value.project
                && !c.projects.iter().any(|item| item.id == project)
            {
                return Err(WorkspaceError::NotFound);
            }
            if let Some(task) = value.task {
                let Some(project) = value.project else {
                    return Err(WorkspaceError::Invalid(
                        "a selected task must belong to a selected project".into(),
                    ));
                };
                if !c.tasks.iter().any(|item| {
                    item.id == task
                        && item.project_id == project
                        && item.state != TaskState::Archived
                }) {
                    return Err(WorkspaceError::NotFound);
                }
            }
            store.set_preference("selection", &value)?;
            Ok(())
        })
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
        self.access(move |store| {
            let current: Vec<AgentProfile> = store
                .preference("agent_profiles")?
                .unwrap_or_else(default_profiles);
            let catalog = catalog(store)?;
            for task in &catalog.tasks {
                let previous = current.iter().find(|profile| profile.id == task.agent_id);
                let replacement = profiles.iter().find(|profile| profile.id == task.agent_id);
                if replacement.is_none() {
                    return Err(WorkspaceError::Invalid(
                        "select another agent for assigned tasks before removing this profile"
                            .into(),
                    ));
                }
                if previous != replacement {
                    if matches!(task.state, TaskState::Running | TaskState::Waiting) {
                        return Err(AgentError::Busy.into());
                    }
                    store.forget_session(task.thread_id)?;
                }
            }
            store.set_preference("agent_profiles", &profiles)?;
            Ok(())
        })
        .await
    }

    pub async fn upsert_custom_profile(
        &self,
        profile: AgentProfile,
    ) -> WorkspaceResult<Vec<AgentProfile>> {
        if profile.registry.is_some() {
            return Err(WorkspaceError::Invalid(
                "managed registry profiles cannot be edited as custom profiles".into(),
            ));
        }
        profile.validate()?;
        let mut profiles = self.profiles().await?;
        if let Some(existing) = profiles.iter_mut().find(|item| item.id == profile.id) {
            if existing.registry.is_some() {
                return Err(WorkspaceError::Invalid(
                    "this ID belongs to a managed registry profile".into(),
                ));
            }
            *existing = profile;
        } else {
            profiles.push(profile);
        }
        self.save_profiles(profiles.clone()).await?;
        Ok(profiles)
    }

    pub async fn delete_custom_profile(&self, id: String) -> WorkspaceResult<Vec<AgentProfile>> {
        let mut profiles = self.profiles().await?;
        let Some(profile) = profiles.iter().find(|profile| profile.id == id) else {
            return Err(WorkspaceError::NotFound);
        };
        if profile.registry.is_some() {
            return Err(WorkspaceError::Invalid(
                "managed registry profiles must be removed through the installation workflow"
                    .into(),
            ));
        }
        if self
            .catalog()
            .await?
            .tasks
            .iter()
            .any(|task| task.agent_id == id)
        {
            return Err(WorkspaceError::Invalid(
                "select another agent for assigned tasks before deleting this profile".into(),
            ));
        }
        profiles.retain(|profile| profile.id != id);
        self.save_profiles(profiles.clone()).await?;
        Ok(profiles)
    }

    pub async fn export_custom_profiles(&self) -> WorkspaceResult<String> {
        let profiles: Vec<_> = self
            .profiles()
            .await?
            .into_iter()
            .filter(|profile| profile.registry.is_none())
            .collect();
        let encoded = serde_json::to_string_pretty(&profiles)
            .map_err(|_| WorkspaceError::Invalid("profiles could not be exported".into()))?;
        if encoded.len() > 1024 * 1024 {
            return Err(AgentError::Limit.into());
        }
        Ok(encoded)
    }

    pub async fn import_custom_profiles(
        &self,
        encoded: String,
    ) -> WorkspaceResult<Vec<AgentProfile>> {
        let imported = parse_profiles(&encoded)?;
        if imported.iter().any(|profile| profile.registry.is_some()) {
            return Err(WorkspaceError::Invalid(
                "custom profile imports cannot contain managed registry receipts".into(),
            ));
        }
        let mut profiles = self.profiles().await?;
        for profile in imported {
            if let Some(existing) = profiles.iter_mut().find(|item| item.id == profile.id) {
                if existing.registry.is_some() {
                    return Err(WorkspaceError::Invalid(
                        "an imported custom profile collides with a managed registry ID".into(),
                    ));
                }
                *existing = profile;
            } else {
                profiles.push(profile);
            }
        }
        self.save_profiles(profiles.clone()).await?;
        Ok(profiles)
    }
    /// Publish one managed profile without losing concurrent edits to other profiles.
    pub async fn register_installation(
        &self,
        reference: synara_registry::RegistryReference,
    ) -> WorkspaceResult<Vec<AgentProfile>> {
        let profile = tokio::task::spawn_blocking(move || AgentProfile::from_registry(reference))
            .await
            .map_err(|_| WorkspaceError::Worker)??;
        self.access(move |store| {
            let mut profiles: Vec<AgentProfile> = store
                .preference("agent_profiles")?
                .unwrap_or_else(default_profiles);
            if catalog(store)?.tasks.iter().any(|task| {
                task.agent_id == profile.id
                    && matches!(task.state, TaskState::Running | TaskState::Waiting)
            }) {
                return Err(AgentError::Busy.into());
            }
            if let Some(old) = profiles.iter_mut().find(|p| p.id == profile.id) {
                if old.registry.is_none() {
                    return Err(WorkspaceError::Invalid(
                        "this ID belongs to a custom agent profile".into(),
                    ));
                }
                *old = profile;
            } else {
                profiles.push(profile);
            }
            validate_profiles(&profiles)?;
            store.set_preference("agent_profiles", &profiles)?;
            Ok(profiles)
        })
        .await
    }
    /// Unregister only the exact approved receipt, and never strand an assigned task.
    pub async fn unregister_installation(
        &self,
        reference: synara_registry::RegistryReference,
    ) -> WorkspaceResult<Vec<AgentProfile>> {
        self.access(move |store| {
            let mut profiles: Vec<AgentProfile> = store
                .preference("agent_profiles")?
                .unwrap_or_else(default_profiles);
            if let Some(profile) = profiles
                .iter()
                .find(|p| p.registry.as_ref() == Some(&reference))
            {
                if catalog(store)?
                    .tasks
                    .iter()
                    .any(|task| task.agent_id == profile.id)
                {
                    return Err(WorkspaceError::Invalid(
                        "select another agent for this installation's tasks before removing it"
                            .into(),
                    ));
                }
                profiles.retain(|p| p.registry.as_ref() != Some(&reference));
                validate_profiles(&profiles)?;
                store.set_preference("agent_profiles", &profiles)?;
            }
            Ok(profiles)
        })
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
fn catalog_name(value: &str, kind: &str) -> WorkspaceResult<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 400
        || value.chars().any(|character| character.is_control())
    {
        return Err(WorkspaceError::Invalid(format!(
            "{kind} name must contain text and fit within 400 bytes"
        )));
    }
    Ok(value.to_owned())
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
    async fn scoped_tasks_survive_reopen_and_legacy_tasks_default_to_project() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("workspace.sqlite3");
        let service = WorkspaceService::open(db.clone()).await.unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let legacy = service
            .create_task(project.id, "Legacy".into(), "opencode".into())
            .await
            .unwrap();
        let mut json = serde_json::to_value(&legacy).unwrap();
        json.as_object_mut().unwrap().remove("scope");
        let old: Task = serde_json::from_value(json).unwrap();
        assert_eq!(old.scope, TaskScope::Project);
        let mut ids = vec![(legacy.id, TaskScope::Project)];
        for scope in [TaskScope::Chat, TaskScope::Studio] {
            let task = service
                .create_scoped_task(project.id, "New chat".into(), "opencode".into(), scope)
                .await
                .unwrap();
            ids.push((task.id, scope));
        }
        service.archive_task(ids[2].0).await.unwrap();
        drop(service);
        let reopened = WorkspaceService::open(db).await.unwrap();
        for (id, scope) in &ids {
            assert_eq!(reopened.task(*id).await.unwrap().scope, *scope);
        }
        assert_eq!(reopened.catalog().await.unwrap().tasks.len(), 3);
        let restored = reopened.unarchive_task(ids[2].0).await.unwrap();
        assert_eq!(restored.state, TaskState::Ready);
        assert_eq!(restored.scope, TaskScope::Studio);
        assert!(
            reopened
                .thread(restored.thread_id)
                .await
                .unwrap()
                .timeline
                .is_empty()
        );
    }

    #[tokio::test]
    async fn catalog_metadata_tracks_committed_events_before_broadcast() {
        let service = WorkspaceService::memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let task = service
            .create_task(project.id, "Initial title".into(), "opencode".into())
            .await
            .unwrap();
        let mut changes = service.subscribe();
        service
            .record(
                task.thread_id,
                ThreadEvent::PromptStarted { turn: "t".into() },
            )
            .await
            .unwrap();
        changes.recv().await.unwrap();
        assert_eq!(
            service.task(task.id).await.unwrap().state,
            TaskState::Running
        );
        service
            .record(
                task.thread_id,
                ThreadEvent::TitleChanged {
                    title: "Updated title".into(),
                },
            )
            .await
            .unwrap();
        service
            .record(
                task.thread_id,
                ThreadEvent::PromptFinished {
                    reason: "end_turn".into(),
                },
            )
            .await
            .unwrap();
        let catalog = service.catalog().await.unwrap();
        assert_eq!(catalog.tasks[0].title, "Updated title");
        assert_eq!(catalog.tasks[0].state, TaskState::Completed);
    }
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
    async fn catalog_lifecycle_renames_archives_deletes_and_repairs_selection() {
        let service = WorkspaceService::memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let root = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let workspace_id = service.catalog().await.unwrap().workspaces[0].id;
        let sub = service
            .create_project(workspace_id, "Sub".into(), PathBuf::from("sub"))
            .await
            .unwrap();
        service
            .rename_workspace(workspace_id, "Renamed workspace".into())
            .await
            .unwrap();
        service
            .rename_project(sub.id, "Renamed project".into())
            .await
            .unwrap();
        let task = service
            .create_task(sub.id, "Initial".into(), "opencode".into())
            .await
            .unwrap();
        let task = service
            .rename_task(task.id, "Renamed task".into())
            .await
            .unwrap();
        assert_eq!(task.title, "Renamed task");
        service
            .save_selection(Selection {
                project: Some(sub.id),
                task: Some(task.id),
            })
            .await
            .unwrap();
        assert_eq!(service.recent_tasks(10).await.unwrap().len(), 1);
        service.archive_task(task.id).await.unwrap();
        assert!(service.recent_tasks(10).await.unwrap().is_empty());
        let selection = service.selection().await.unwrap();
        assert_eq!(selection.project, Some(sub.id));
        assert_eq!(selection.task, None);
        service.delete_task(task.id).await.unwrap();
        service.delete_project(sub.id).await.unwrap();
        assert_eq!(service.selection().await.unwrap(), Selection::default());
        service.delete_project(root.id).await.unwrap();
        service.delete_workspace(workspace_id).await.unwrap();
        assert!(service.catalog().await.unwrap().workspaces.is_empty());
    }

    #[tokio::test]
    async fn active_tasks_cannot_be_archived_or_deleted_without_archival() {
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
        assert!(service.delete_task(task.id).await.is_err());
        service
            .record(
                task.thread_id,
                ThreadEvent::PromptStarted {
                    turn: "turn".into(),
                },
            )
            .await
            .unwrap();
        assert!(matches!(
            service.archive_task(task.id).await,
            Err(WorkspaceError::Agent(AgentError::Busy))
        ));
    }

    #[tokio::test]
    async fn custom_profile_crud_exports_only_secret_references_and_invalidates_stale_sessions() {
        let service = WorkspaceService::memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let project = service
            .add_local_workspace(dir.path().into())
            .await
            .unwrap();
        let profile = AgentProfile {
            registry: None,
            id: "custom-secret".into(),
            name: "Custom".into(),
            command: "/opt/custom-agent".into(),
            args: vec!["acp".into()],
            inherit_env: vec![],
            secret_env: std::collections::BTreeMap::from([(
                "API_TOKEN".into(),
                synara_runtime::SecretReference::new("dev.synara", "agent/custom-secret").unwrap(),
            )]),
        };
        service
            .upsert_custom_profile(profile.clone())
            .await
            .unwrap();
        let exported = service.export_custom_profiles().await.unwrap();
        assert!(exported.contains("agent/custom-secret"));
        assert!(!exported.contains("secret-canary"));

        let task = service
            .create_task(project.id, "Task".into(), profile.id.clone())
            .await
            .unwrap();
        service
            .save_session(
                task.thread_id,
                SessionReference {
                    agent_id: profile.id.clone(),
                    remote_id: "old-session".into(),
                    working_directory: task.working_directory.clone(),
                    title: None,
                },
            )
            .await
            .unwrap();
        let mut edited = profile.clone();
        edited.name = "Edited".into();
        service.upsert_custom_profile(edited.clone()).await.unwrap();
        assert!(service.session(task.thread_id).await.unwrap().is_none());
        assert_eq!(
            service
                .profiles()
                .await
                .unwrap()
                .into_iter()
                .find(|item| item.id == profile.id)
                .unwrap()
                .name,
            "Edited"
        );
        assert!(
            service
                .delete_custom_profile(profile.id.clone())
                .await
                .is_err()
        );

        let mut running = edited.clone();
        running.name = "Blocked".into();
        service
            .record(
                task.thread_id,
                ThreadEvent::PromptStarted {
                    turn: "running".into(),
                },
            )
            .await
            .unwrap();
        assert!(matches!(
            service.upsert_custom_profile(running).await,
            Err(WorkspaceError::Agent(AgentError::Busy))
        ));

        service
            .record(
                task.thread_id,
                ThreadEvent::PromptFinished {
                    reason: "end_turn".into(),
                },
            )
            .await
            .unwrap();
        service
            .set_task_agent(task.id, "opencode".into())
            .await
            .unwrap();
        service
            .delete_custom_profile(profile.id.clone())
            .await
            .unwrap();
        assert!(
            service
                .profiles()
                .await
                .unwrap()
                .iter()
                .all(|item| item.id != profile.id)
        );

        let imported = service.import_custom_profiles(exported).await.unwrap();
        assert!(imported.iter().any(|item| item.id == profile.id));
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
