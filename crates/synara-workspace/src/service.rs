mod new_worktree;
use crate::storage::ManagedWorktreeOwnership;
use crate::{
    AgentProfile, GitOperation, GitOperationError, GitOperationOptions, GitOperations,
    NewSshWorkspace, SshWorkspaceProfile, StorageError, Store, default_profiles, parse_profiles,
    upsert_ssh_profile, validate_profiles,
};
use async_trait::async_trait;
pub use new_worktree::NewWorktreePlan;
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
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error(transparent)]
    Agent(#[from] AgentError),
    #[error(transparent)]
    Git(#[from] GitOperationError),
    #[error("workspace object was not found")]
    NotFound,
    #[error("invalid workspace operation: {0}")]
    Invalid(String),
    #[error("workspace worker stopped")]
    Worker,
}

impl From<rusqlite::Error> for crate::WorkspaceError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Storage(StorageError::Database(error))
    }
}
pub type WorkspaceResult<T> = Result<T, WorkspaceError>;
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    pub workspaces: Vec<Workspace>,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
}
/// A Git-linked worktree mapped to the registered project's directory within it.
/// `path` is the directory Synara will use as this task's working directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectWorktree {
    /// Git's registered worktree root, used by Git lifecycle operations.
    pub repository_path: PathBuf,
    /// The registered project's execution directory within this worktree.
    pub path: PathBuf,
    pub branch: Option<String>,
    pub detached: bool,
    pub locked: bool,
    pub bare: bool,
    pub prunable: bool,
    pub project_root: bool,
    pub assigned_task: Option<TaskId>,
    pub assigned_task_title: Option<String>,
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
    worktree_lifecycle: Arc<tokio::sync::Mutex<()>>,
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
            worktree_lifecycle: Arc::new(tokio::sync::Mutex::new(())),
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
        self.access(move |store| store.archive_task_with_workflow_guard(id, now_ms()))
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
        let managed = self
            .access(move |store| Ok(store.managed_worktree(id)?))
            .await?;
        self.access(move |store| {
            if !store.delete_task(id)? {
                return Err(WorkspaceError::NotFound);
            }
            Ok(())
        })
        .await?;
        if let Some(managed) = managed
            && let Err(error) = self
                .cleanup_deleted_managed_worktree(managed.clone(), CancellationToken::new())
                .await
        {
            eprintln!(
                "managed worktree retained after task deletion: task={} branch={} repository={} error={}",
                managed.task,
                managed.branch,
                managed.repository_path.display(),
                error
            );
        }
        Ok(())
    }

    pub async fn delete_project(&self, id: ProjectId) -> WorkspaceResult<()> {
        self.retry_pending_managed_worktree_cleanup(Some(id), None)
            .await?;
        self.access(move |store| {
            if !store.delete_project(id)? {
                return Err(WorkspaceError::NotFound);
            }
            Ok(())
        })
        .await
    }

    pub async fn delete_workspace(&self, id: WorkspaceId) -> WorkspaceResult<()> {
        self.retry_pending_managed_worktree_cleanup(None, Some(id))
            .await?;
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
        self.create_scoped_task_at(project, title, agent_id, scope, draft, None, None)
            .await
    }

    /// Create a new unsent branch draft in an existing, unassigned linked
    /// worktree. The worktree remains user-managed and is never removed here.
    pub async fn create_scoped_task_in_worktree(
        &self,
        project: ProjectId,
        title: String,
        agent_id: String,
        scope: TaskScope,
        draft: String,
        worktree_directory: PathBuf,
    ) -> WorkspaceResult<Task> {
        let _lifecycle = self.lock_worktree_lifecycle().await;
        let directory = normalized_absolute(&worktree_directory).ok_or_else(|| {
            WorkspaceError::Invalid("select a valid linked worktree directory".into())
        })?;
        let worktrees = self.project_worktrees(project).await?;
        let worktree = worktrees
            .into_iter()
            .find(|item| item.path == directory && !item.project_root)
            .ok_or_else(|| {
                WorkspaceError::Invalid(
                    "the selected directory is no longer a linked worktree for this project".into(),
                )
            })?;
        if worktree.bare || worktree.prunable || worktree.locked {
            return Err(WorkspaceError::Invalid(
                "the selected linked worktree is locked or unavailable".into(),
            ));
        }
        if worktree.assigned_task.is_some() {
            return Err(WorkspaceError::Invalid(
                "this linked worktree already belongs to a task".into(),
            ));
        }
        self.validate_task_directory(
            project,
            worktree.path.clone(),
            worktree.repository_path.clone(),
        )
        .await?;
        self.create_scoped_task_at(
            project,
            title,
            agent_id,
            scope,
            draft,
            Some((worktree.path, worktree.repository_path)),
            None,
        )
        .await
    }

    /// Serialize task assignment with explicit Git worktree removal in this
    /// process. Callers must hold the guard from ownership validation through
    /// completion of the Git operation.
    pub async fn lock_worktree_lifecycle(&self) -> tokio::sync::OwnedMutexGuard<()> {
        self.worktree_lifecycle.clone().lock_owned().await
    }

    async fn create_scoped_task_at(
        &self,
        project: ProjectId,
        title: String,
        agent_id: String,
        scope: TaskScope,
        draft: String,
        selected_directory: Option<(PathBuf, PathBuf)>,
        managed_branch: Option<(String, bool)>,
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
            let project_directory = normalized_absolute(&project_directory(workspace, project)?)
                .ok_or_else(|| WorkspaceError::Invalid("invalid project directory".into()))?;
            let workspace_project_ids = c
                .projects
                .iter()
                .filter(|candidate| candidate.workspace_id == project.workspace_id)
                .map(|candidate| candidate.id)
                .collect::<Vec<_>>();
            let working_directory = selected_directory
                .as_ref()
                .map(|(directory, repository_path)| {
                    if *directory == project_directory
                        || c.tasks.iter().any(|task| {
                            workspace_project_ids.contains(&task.project_id)
                                && normalized_absolute(&task.working_directory)
                                    .is_some_and(|task_path| task_path.starts_with(repository_path))
                        })
                    {
                        return Err(WorkspaceError::Invalid(
                            "this linked worktree already belongs to a task".into(),
                        ));
                    }
                    Ok(directory.clone())
                })
                .transpose()?
                .unwrap_or(project_directory);
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
            let managed = managed_branch
                .map(|(branch, remote)| {
                    let (_, repository_path) =
                        selected_directory.as_ref().ok_or(WorkspaceError::Invalid(
                            "managed worktree task is missing its repository root".into(),
                        ))?;
                    let managed = ManagedWorktreeOwnership {
                        version: 1,
                        task: task.id,
                        project: project.id,
                        workspace: workspace.id,
                        repository_path: repository_path.clone(),
                        task_path: task.working_directory.clone(),
                        branch,
                        remote,
                    };
                    managed.validate().map_err(WorkspaceError::Storage)?;
                    Ok::<_, WorkspaceError>(managed)
                })
                .transpose()?;
            store.insert_task_with_draft_and_managed_worktree(&task, draft, managed)?;
            Ok(task)
        })
        .await
    }

    /// Return linked Git worktrees for a project's repository. Paths are
    /// mapped through the project-relative subdirectory and occupied entries
    /// stay visible to explain why they cannot be selected for a new branch.
    pub async fn project_worktrees(
        &self,
        project_id: ProjectId,
    ) -> WorkspaceResult<Vec<ProjectWorktree>> {
        let (workspace, project, tasks) = self
            .access(move |store| {
                let catalog = catalog(store)?;
                let project = catalog
                    .projects
                    .iter()
                    .find(|project| project.id == project_id)
                    .cloned()
                    .ok_or(WorkspaceError::NotFound)?;
                let workspace = catalog
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.id == project.workspace_id)
                    .cloned()
                    .ok_or(WorkspaceError::NotFound)?;
                let workspace_project_ids = catalog
                    .projects
                    .iter()
                    .filter(|candidate| candidate.workspace_id == workspace.id)
                    .map(|candidate| candidate.id)
                    .collect::<Vec<_>>();
                let tasks: Vec<Task> = catalog
                    .tasks
                    .into_iter()
                    .filter(|task| workspace_project_ids.contains(&task.project_id))
                    .collect();
                Ok((workspace, project, tasks))
            })
            .await?;
        let project_directory = normalized_absolute(&project_directory(&workspace, &project)?)
            .ok_or_else(|| WorkspaceError::Invalid("invalid project directory".into()))?;
        let git = match &workspace.location {
            WorkspaceLocation::Local { .. } => GitOperations::new(project_directory.clone()),
            WorkspaceLocation::Ssh { .. } => {
                let profile = self
                    .ssh_profile(workspace.id)
                    .await?
                    .ok_or_else(|| WorkspaceError::Invalid("SSH profile is missing".into()))?;
                GitOperations::with_host(
                    project_directory.clone(),
                    Arc::new(profile.host(&workspace)?),
                )
            }
        };
        let output = git
            .execute(
                GitOperation::Worktrees,
                GitOperationOptions::default(),
                tokio_util::sync::CancellationToken::new(),
                None,
            )
            .await?;
        let parsed = parse_git_worktrees(&output.stdout)?;
        let repository_root = parsed
            .iter()
            .filter(|worktree| project_directory.starts_with(&worktree.path))
            .max_by_key(|worktree| worktree.path.components().count())
            .ok_or_else(|| {
                WorkspaceError::Invalid(
                    "the registered project directory is not inside a listed Git worktree".into(),
                )
            })?;
        let relative_project = project_directory
            .strip_prefix(&repository_root.path)
            .map_err(|_| WorkspaceError::Invalid("invalid project directory".into()))?;
        let mut result = Vec::with_capacity(parsed.len().min(256));
        for worktree in parsed.into_iter().take(256) {
            let repository_path = worktree.path;
            let path = normalized_absolute(&repository_path.join(relative_project))
                .ok_or_else(|| WorkspaceError::Invalid("invalid Git worktree path".into()))?;
            let assigned_task = tasks.iter().find(|task| {
                normalized_absolute(&task.working_directory)
                    .is_some_and(|task_path| task_path.starts_with(&repository_path))
            });
            result.push(ProjectWorktree {
                repository_path,
                project_root: path == project_directory,
                path,
                branch: worktree.branch,
                detached: worktree.detached,
                locked: worktree.locked,
                bare: worktree.bare,
                prunable: worktree.prunable,
                assigned_task: assigned_task.map(|task| task.id),
                assigned_task_title: assigned_task.map(|task| task.title.clone()),
            });
        }
        Ok(result)
    }

    async fn validate_task_directory(
        &self,
        project_id: ProjectId,
        directory: PathBuf,
        repository_path: PathBuf,
    ) -> WorkspaceResult<()> {
        let workspace = self
            .access(move |store| {
                let catalog = catalog(store)?;
                let project = catalog
                    .projects
                    .iter()
                    .find(|project| project.id == project_id)
                    .cloned()
                    .ok_or(WorkspaceError::NotFound)?;
                let workspace = catalog
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.id == project.workspace_id)
                    .cloned()
                    .ok_or(WorkspaceError::NotFound)?;
                Ok(workspace)
            })
            .await?;
        match &workspace.location {
            WorkspaceLocation::Local { .. } => {
                let directory = directory.clone();
                let repository_path = repository_path.clone();
                tokio::task::spawn_blocking(move || {
                    let repository = WorkspaceFs::open(&repository_path)?;
                    if repository.root() != repository_path.as_path()
                        || !directory.starts_with(repository.root())
                    {
                        return Err(RuntimeError::Denied(
                            "the selected project directory resolves through a symlink".into(),
                        ));
                    }
                    if directory != repository_path {
                        let relative = repository.relative(&directory)?;
                        repository.entries(&relative)?;
                    }
                    Ok(())
                })
                .await
                .map_err(|_| WorkspaceError::Worker)??;
            }
            WorkspaceLocation::Ssh { .. } => {
                let profile = self
                    .ssh_profile(workspace.id)
                    .await?
                    .ok_or_else(|| WorkspaceError::Invalid("SSH profile is missing".into()))?;
                let repository = profile.filesystem(&workspace, &repository_path).await?;
                if Path::new(repository.root_identity()) != repository_path.as_path()
                    || !directory.starts_with(&repository_path)
                {
                    return Err(WorkspaceError::Invalid(
                        "the selected project directory resolves through a symlink".into(),
                    ));
                }
                if directory != repository_path {
                    repository.entries(&directory).await?;
                }
            }
        }
        Ok(())
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
        let (workspace, project, persisted) = self
            .access(move |store| {
                let c = catalog(store)?;
                let persisted = c
                    .tasks
                    .iter()
                    .find(|candidate| candidate.id == task.id)
                    .cloned()
                    .ok_or(WorkspaceError::NotFound)?;
                if persisted.project_id != task.project_id
                    || persisted.working_directory != task.working_directory
                {
                    return Err(WorkspaceError::Invalid(
                        "task workspace metadata is stale".into(),
                    ));
                }
                let project = c
                    .projects
                    .iter()
                    .find(|p| p.id == persisted.project_id)
                    .cloned()
                    .ok_or(WorkspaceError::NotFound)?;
                let workspace = c
                    .workspaces
                    .iter()
                    .find(|w| w.id == project.workspace_id)
                    .cloned()
                    .ok_or(WorkspaceError::NotFound)?;
                Ok((workspace, project, persisted))
            })
            .await?;
        let project_path = normalized_absolute(&project_directory(&workspace, &project)?)
            .ok_or_else(|| WorkspaceError::Invalid("invalid project directory".into()))?;
        let task_path = normalized_absolute(&persisted.working_directory)
            .ok_or_else(|| WorkspaceError::Invalid("invalid task working directory".into()))?;
        if task_path != project_path {
            let worktrees = self.project_worktrees(project.id).await?;
            let worktree = worktrees
                .iter()
                .find(|worktree| {
                    worktree.path == task_path
                        && !worktree.project_root
                        && !worktree.bare
                        && !worktree.prunable
                })
                .ok_or_else(|| {
                    WorkspaceError::Invalid(
                        "task directory is not a linked worktree for its project".into(),
                    )
                })?;
            self.validate_task_directory(project.id, task_path, worktree.repository_path.clone())
                .await?;
        }
        Ok(workspace)
    }
}

#[derive(Default)]
struct ParsedGitWorktree {
    path: PathBuf,
    head: Option<String>,
    branch: Option<String>,
    detached: bool,
    locked: bool,
    bare: bool,
    prunable: bool,
}

fn parse_git_worktrees(output: &[u8]) -> WorkspaceResult<Vec<ParsedGitWorktree>> {
    let output = std::str::from_utf8(output)
        .map_err(|_| WorkspaceError::Invalid("Git worktree paths are not UTF-8".into()))?;
    let mut worktrees = Vec::new();
    let mut current = ParsedGitWorktree::default();
    for field in output.split('\0') {
        if field.is_empty() {
            if !current.path.as_os_str().is_empty() {
                worktrees.push(std::mem::take(&mut current));
            }
            continue;
        }
        if let Some(path) = field.strip_prefix("worktree ") {
            if !current.path.as_os_str().is_empty() {
                worktrees.push(std::mem::take(&mut current));
            }
            let path = normalized_absolute(Path::new(path)).ok_or_else(|| {
                WorkspaceError::Invalid("Git returned an invalid worktree path".into())
            })?;
            current.path = path;
        } else if let Some(head) = field.strip_prefix("HEAD ") {
            current.head = Some(head.to_owned());
        } else if let Some(branch) = field.strip_prefix("branch refs/heads/") {
            current.branch = Some(branch.to_owned());
        } else if field == "detached" {
            current.detached = true;
        } else if field == "locked" || field.starts_with("locked ") {
            current.locked = true;
        } else if field == "bare" {
            current.bare = true;
        } else if field == "prunable" || field.starts_with("prunable ") {
            current.prunable = true;
        }
    }
    if !current.path.as_os_str().is_empty() {
        worktrees.push(current);
    }
    if worktrees.is_empty() || worktrees.len() > 256 {
        return Err(WorkspaceError::Invalid(
            "Git returned an empty or oversized worktree list".into(),
        ));
    }
    Ok(worktrees)
}

fn normalized_absolute(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                normalized.push(component.as_os_str());
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() || !normalized.is_absolute() {
                    return None;
                }
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    normalized.is_absolute().then_some(normalized)
}
#[async_trait]
impl EventSink for WorkspaceService {
    async fn emit(&self, thread_id: ThreadId, event: ThreadEvent) -> AgentResult<()> {
        let event = if let ThreadEvent::ImageMessage { image, .. } = &event {
            match Self::validate_transcript_image(image.clone()).await {
                Ok(()) => event,
                Err(error) => ThreadEvent::Notice {
                    message: format!("Image unavailable: {error}"),
                },
            }
        } else {
            event
        };
        let is_image = matches!(event, ThreadEvent::ImageMessage { .. });
        let result = self.record(thread_id, event).await;
        if is_image && matches!(&result, Err(WorkspaceError::Storage(StorageError::Limit))) {
            return self.record(thread_id,ThreadEvent::Notice {message:"Image not retained: this thread reached the 256-image or 32 MiB media-history boundary. Existing history was preserved.".into()})
                .await.map(|_|()).map_err(|_|AgentError::EventDelivery);
        }
        result.map(|_| ()).map_err(|_| AgentError::EventDelivery)
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

pub(crate) fn project_directory(
    workspace: &Workspace,
    project: &Project,
) -> WorkspaceResult<PathBuf> {
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
    use std::process::Command;

    pub(super) fn git(root: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .expect("git executable available for worktree test");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    pub(super) fn repository(root: &Path) {
        std::fs::create_dir_all(root).unwrap();
        let output = Command::new("git")
            .arg("init")
            .arg("--quiet")
            .arg(root)
            .output()
            .expect("git executable available for worktree test");
        assert!(output.status.success());
        git(root, &["config", "user.name", "Synara tests"]);
        git(
            root,
            &["config", "user.email", "synara-test@example.invalid"],
        );
        git(
            root,
            &["commit", "--quiet", "--allow-empty", "-m", "initial"],
        );
    }

    #[test]
    fn git_worktree_parser_preserves_spaces_newlines_and_status_fields() {
        let parsed = parse_git_worktrees(
            b"worktree /tmp/repo with spaces\0HEAD abc\0branch refs/heads/main\0\0worktree /tmp/linked\nname\0HEAD def\0detached\0locked in use\0prunable missing\0\0",
        )
        .unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].path, Path::new("/tmp/repo with spaces"));
        assert_eq!(parsed[0].branch.as_deref(), Some("main"));
        assert!(parsed[1].detached && parsed[1].locked && parsed[1].prunable);
        assert!(parse_git_worktrees(b"worktree relative\0\0").is_err());
    }

    #[tokio::test]
    async fn linked_worktree_branch_is_persisted_and_cannot_be_shared() {
        let directory = tempfile::tempdir().unwrap();
        let repository_root = directory.path().join("repo");
        let linked = directory.path().join("linked worktree");
        repository(&repository_root);
        git(
            &repository_root,
            &[
                "worktree",
                "add",
                "--quiet",
                "--detach",
                linked.to_str().unwrap(),
                "HEAD",
            ],
        );

        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(repository_root.clone())
            .await
            .unwrap();
        let worktrees = service.project_worktrees(project.id).await.unwrap();
        assert!(worktrees.iter().any(|item| item.project_root));
        assert!(worktrees.iter().any(|item| item.path == linked));

        let agent = service.profiles().await.unwrap()[0].id.clone();
        assert!(
            service
                .create_scoped_task_in_worktree(
                    project.id,
                    "Invalid path".into(),
                    agent.clone(),
                    TaskScope::Project,
                    String::new(),
                    directory.path().join("ordinary directory"),
                )
                .await
                .is_err()
        );
        assert!(
            service
                .create_scoped_task_in_worktree(
                    project.id,
                    "Project root".into(),
                    agent.clone(),
                    TaskScope::Project,
                    String::new(),
                    repository_root.clone(),
                )
                .await
                .is_err()
        );

        let first_service = service.clone();
        let second_service = service.clone();
        let first_agent = agent.clone();
        let second_agent = agent.clone();
        let first_linked = linked.clone();
        let second_linked = linked.clone();
        let first = async move {
            first_service
                .create_scoped_task_in_worktree(
                    project.id,
                    "Branch in linked worktree".into(),
                    first_agent,
                    TaskScope::Project,
                    "Quoted source context".into(),
                    first_linked,
                )
                .await
        };
        let second = async move {
            second_service
                .create_scoped_task_in_worktree(
                    project.id,
                    "Competing branch".into(),
                    second_agent,
                    TaskScope::Project,
                    "Other context".into(),
                    second_linked,
                )
                .await
        };
        let (first, second) = tokio::join!(first, second);
        assert_ne!(first.is_ok(), second.is_ok());
        let task = first.or(second).unwrap();
        assert_eq!(task.working_directory, linked);
        assert_eq!(
            service.task_draft(task.id).await.unwrap(),
            "Quoted source context"
        );
        assert_eq!(
            service.workspace_for_task(&task).await.unwrap().id,
            project.workspace_id
        );
        assert_eq!(
            service
                .project_worktrees(project.id)
                .await
                .unwrap()
                .into_iter()
                .find(|item| item.path == task.working_directory)
                .unwrap()
                .assigned_task,
            Some(task.id)
        );
        assert!(
            service
                .create_scoped_task_in_worktree(
                    project.id,
                    "Second task".into(),
                    agent,
                    TaskScope::Project,
                    String::new(),
                    task.working_directory.clone(),
                )
                .await
                .is_err()
        );

        git(
            &repository_root,
            &[
                "worktree",
                "remove",
                task.working_directory.to_str().unwrap(),
            ],
        );
        assert!(service.workspace_for_task(&task).await.is_err());
    }

    #[tokio::test]
    async fn linked_worktree_maps_nested_registered_project_directory() {
        let directory = tempfile::tempdir().unwrap();
        let repository_root = directory.path().join("repo");
        let linked = directory.path().join("linked");
        repository(&repository_root);
        std::fs::create_dir(repository_root.join("nested")).unwrap();
        std::fs::write(repository_root.join("nested/README.md"), "project").unwrap();
        git(&repository_root, &["add", "nested/README.md"]);
        git(
            &repository_root,
            &["commit", "--quiet", "-m", "add nested project"],
        );
        git(
            &repository_root,
            &[
                "worktree",
                "add",
                "--quiet",
                "--detach",
                linked.to_str().unwrap(),
                "HEAD",
            ],
        );

        let service = WorkspaceService::memory().unwrap();
        let root_project = service
            .add_local_workspace(repository_root.clone())
            .await
            .unwrap();
        let workspace_id = service.catalog().await.unwrap().workspaces[0].id;
        let nested_project = service
            .create_project(workspace_id, "Nested".into(), PathBuf::from("nested"))
            .await
            .unwrap();
        let linked_directory = linked.join("nested");
        let agent = service.profiles().await.unwrap()[0].id.clone();
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let outside = directory.path().join("outside");
            std::fs::create_dir(&outside).unwrap();
            std::fs::remove_dir_all(&linked_directory).unwrap();
            symlink(&outside, &linked_directory).unwrap();
            assert!(
                service
                    .create_scoped_task_in_worktree(
                        nested_project.id,
                        "Symlinked project".into(),
                        agent.clone(),
                        TaskScope::Project,
                        String::new(),
                        linked_directory.clone(),
                    )
                    .await
                    .is_err()
            );
            std::fs::remove_file(&linked_directory).unwrap();
            std::fs::create_dir(&linked_directory).unwrap();
        }
        assert!(
            service
                .project_worktrees(nested_project.id)
                .await
                .unwrap()
                .iter()
                .any(|worktree| worktree.path == linked_directory)
        );
        let task = service
            .create_scoped_task_in_worktree(
                nested_project.id,
                "Nested branch".into(),
                agent,
                TaskScope::Project,
                String::new(),
                linked_directory.clone(),
            )
            .await
            .unwrap();
        assert_eq!(task.working_directory, linked_directory);
        let owning_repository_worktree = service
            .project_worktrees(root_project.id)
            .await
            .unwrap()
            .into_iter()
            .find(|worktree| worktree.repository_path == linked)
            .unwrap();
        assert_eq!(owning_repository_worktree.assigned_task, Some(task.id));
        assert_eq!(
            owning_repository_worktree.assigned_task_title.as_deref(),
            Some("Nested branch")
        );
        assert_eq!(
            service.workspace_for_task(&task).await.unwrap().id,
            workspace_id
        );
        let _ = root_project;
    }

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
