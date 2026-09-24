//! Reviewed new-worktree forks. Git owns checkout; task creation owns the unsent
//! draft. An interrupted/failed checkout is never "repaired" by deleting files.
use super::*;
use crate::GitOperationPolicy;
use tokio_util::sync::CancellationToken;

/// Not deserializable: only an explicit native review may authorize checkout.
#[derive(Clone, Debug)]
pub struct NewWorktreePlan {
    source: Task,
    repository: PathBuf,
    relative_project: PathBuf,
    destination: PathBuf,
    branch: String,
    head: String,
}
impl NewWorktreePlan {
    pub fn source(&self) -> TaskId {
        self.source.id
    }
    pub fn repository(&self) -> &Path {
        &self.repository
    }
    pub fn destination(&self) -> &Path {
        &self.destination
    }
    pub fn branch(&self) -> &str {
        &self.branch
    }
    pub fn head(&self) -> &str {
        &self.head
    }
}
fn quiet(task: &Task) -> bool {
    !matches!(
        task.state,
        TaskState::Running | TaskState::Waiting | TaskState::Archived
    )
}
fn unchanged(source: &Task, current: &Task) -> bool {
    source.id == current.id
        && source.project_id == current.project_id
        && source.working_directory == current.working_directory
        && source.agent_id == current.agent_id
        && source.scope == current.scope
        && quiet(current)
}
fn invalid(message: impl Into<String>) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
fn validate_parent(parent: &Path, repository: &Path) -> WorkspaceResult<PathBuf> {
    if !parent.is_absolute()
        || parent
            .components()
            .any(|p| matches!(p, std::path::Component::ParentDir))
    {
        return Err(invalid(
            "the worktree parent must be an absolute local directory",
        ));
    }
    let canonical = parent.canonicalize().map_err(RuntimeError::Io)?;
    if canonical != parent || !canonical.is_dir() || canonical.starts_with(repository) {
        return Err(invalid(
            "the worktree parent must be an existing non-symlink directory outside the source repository",
        ));
    }
    Ok(canonical)
}

impl WorkspaceService {
    pub async fn prepare_new_worktree_fork(
        &self,
        source: TaskId,
        parent: PathBuf,
    ) -> WorkspaceResult<NewWorktreePlan> {
        let source = self.task(source).await?;
        if !quiet(&source) {
            return Err(invalid(
                "stop or restore the source task before creating an isolated fork",
            ));
        }
        let workspace = self.workspace_for_task(&source).await?;
        if !matches!(workspace.location, WorkspaceLocation::Local { .. }) {
            return Err(invalid(
                "automatic worktree creation currently requires a local project; existing SSH worktree selection remains available",
            ));
        }
        let output = GitOperations::new(source.working_directory.clone())
            .execute(
                GitOperation::Worktrees,
                GitOperationOptions::default(),
                CancellationToken::new(),
                None,
            )
            .await?;
        let entries = parse_git_worktrees(&output.stdout)?;
        let entry = entries
            .iter()
            .filter(|entry| source.working_directory.starts_with(&entry.path))
            .max_by_key(|entry| entry.path.components().count())
            .ok_or_else(|| invalid("the source task is not inside a Git worktree"))?;
        if entry.bare || entry.prunable || entry.locked {
            return Err(invalid("the source worktree is unavailable or locked"));
        }
        let head = entry
            .head
            .clone()
            .filter(|head| {
                matches!(head.len(), 40 | 64)
                    && head.bytes().all(|b| b.is_ascii_hexdigit())
                    && head.bytes().any(|b| b != b'0')
            })
            .ok_or_else(|| {
                invalid("commit the repository's initial revision before creating a worktree")
            })?;
        let parent = validate_parent(&parent, &entry.path)?;
        let token = uuid::Uuid::new_v4();
        let relative_project = source
            .working_directory
            .strip_prefix(&entry.path)
            .map_err(|_| invalid("invalid source project directory"))?
            .to_path_buf();
        Ok(NewWorktreePlan {
            source,
            repository: entry.path.clone(),
            relative_project,
            destination: parent.join(format!("worktree-{token}")),
            branch: format!("synara/{token}"),
            head,
        })
    }

    /// Checkout is separately consented because repository filters can execute.
    /// No prompt, source edit, force operation or automatic cleanup is performed.
    pub async fn create_new_worktree_fork(
        &self,
        plan: NewWorktreePlan,
        title: String,
        draft: String,
        policy: GitOperationPolicy,
        cancel: CancellationToken,
    ) -> WorkspaceResult<Task> {
        if !policy.allow_mutation || !policy.allow_repository_execution {
            return Err(invalid(
                "explicit worktree checkout and repository-execution consent is required",
            ));
        }
        if cancel.is_cancelled() {
            return Err(invalid("worktree creation was cancelled before checkout"));
        }
        if title.trim().is_empty()
            || title.len() > 400
            || title.contains('\0')
            || draft.len() > 1024 * 1024
        {
            return Err(invalid("invalid fork title or draft size"));
        }
        let _lifecycle = self.lock_worktree_lifecycle().await;
        let source = self.task(plan.source.id).await?;
        if !unchanged(&plan.source, &source) {
            return Err(invalid(
                "the source task changed; review a new worktree plan",
            ));
        }
        if !matches!(
            self.workspace_for_task(&source).await?.location,
            WorkspaceLocation::Local { .. }
        ) {
            return Err(invalid(
                "the reviewed source is no longer a local workspace",
            ));
        }
        let parent = plan
            .destination
            .parent()
            .ok_or_else(|| invalid("invalid worktree destination"))?;
        validate_parent(parent, &plan.repository)?;
        if std::fs::symlink_metadata(&plan.destination).is_ok() {
            return Err(invalid(
                "the reviewed worktree destination already exists; nothing was overwritten",
            ));
        }
        let git = GitOperations::new(source.working_directory.clone());
        let output = git
            .execute(
                GitOperation::Worktrees,
                GitOperationOptions::default(),
                cancel.clone(),
                None,
            )
            .await?;
        let entries = parse_git_worktrees(&output.stdout)?;
        if !entries.iter().any(|entry| {
            entry.path == plan.repository
                && entry.head.as_ref() == Some(&plan.head)
                && !entry.bare
                && !entry.locked
                && !entry.prunable
        }) {
            return Err(invalid(
                "the source Git HEAD or worktree changed; review a new plan",
            ));
        }
        // Policy carries only the two reviewed local grants. Never inherit hooks,
        // credentials, signing or networking from unrelated Git reviews.
        let options = GitOperationOptions {
            policy: GitOperationPolicy {
                allow_mutation: true,
                allow_repository_execution: true,
                ..Default::default()
            },
            ..Default::default()
        };
        git.execute(GitOperation::AddNewWorktree { path: plan.destination.clone(), branch: plan.branch.clone(), head: plan.head.clone() },
            options, cancel, None).await.map_err(|error| invalid(format!(
                "Worktree checkout failed: {error}. Inspect {} and branch {} before retrying; no task or prompt was created.", plan.destination.display(), plan.branch)))?;
        let directory = plan.destination.join(&plan.relative_project);
        let saved = async {
            self.validate_task_directory(
                source.project_id,
                directory.clone(),
                plan.destination.clone(),
            )
            .await?;
            let entries = self.project_worktrees(source.project_id).await?;
            if !entries.iter().any(|entry| {
                entry.repository_path == plan.destination
                    && entry.path == directory
                    && entry.branch.as_deref() == Some(plan.branch.as_str())
                    && entry.assigned_task.is_none()
                    && !entry.locked
                    && !entry.prunable
            }) {
                return Err(invalid(
                    "the new worktree is no longer available for this task",
                ));
            }
            self.create_scoped_task_at(
                source.project_id,
                title,
                source.agent_id,
                source.scope,
                draft,
                Some((directory, plan.destination.clone())),
            )
            .await
        }
        .await;
        saved.map_err(|error| invalid(format!(
            "Worktree {} on branch {} was created, but its task could not be saved: {error}. Keep it for recovery or remove it explicitly in Git. Nothing was sent.", plan.destination.display(), plan.branch)))
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{git, repository};
    use super::*;
    #[tokio::test]
    async fn new_worktree_fork_is_pinned_isolated_durable_and_unsent() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repository");
        repository(&repo);
        std::fs::write(repo.join("file.txt"), "committed").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-qm", "file"]);
        let db = dir.path().join("workspace.db");
        let service = WorkspaceService::open(db.clone()).await.unwrap();
        let project = service.add_local_workspace(repo.clone()).await.unwrap();
        let source = service
            .create_task(
                project.id,
                "source".into(),
                service.profiles().await.unwrap()[0].id.clone(),
            )
            .await
            .unwrap();
        let plan = service
            .prepare_new_worktree_fork(source.id, dir.path().canonicalize().unwrap())
            .await
            .unwrap();
        std::fs::write(repo.join("file.txt"), "dirty source").unwrap();
        let destination = plan.destination.clone();
        let branch = plan.branch.clone();
        assert!(
            service
                .create_new_worktree_fork(
                    plan.clone(),
                    "fork".into(),
                    "review".into(),
                    GitOperationPolicy::default(),
                    CancellationToken::new()
                )
                .await
                .is_err()
        );
        assert!(!destination.exists());
        let policy = GitOperationPolicy {
            allow_mutation: true,
            allow_repository_execution: true,
            ..Default::default()
        };
        let task = service
            .create_new_worktree_fork(
                plan.clone(),
                "fork".into(),
                "Review 日本語".into(),
                policy,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(destination.join("file.txt")).unwrap(),
            "committed"
        );
        assert_eq!(
            std::fs::read_to_string(repo.join("file.txt")).unwrap(),
            "dirty source"
        );
        assert_eq!(task.working_directory, destination);
        assert!(
            service
                .create_new_worktree_fork(
                    plan,
                    "fork".into(),
                    "review".into(),
                    policy,
                    CancellationToken::new()
                )
                .await
                .is_err()
        );
        drop(service);
        let service = WorkspaceService::open(db).await.unwrap();
        assert_eq!(service.task_draft(task.id).await.unwrap(), "Review 日本語");
        assert!(
            service
                .thread(task.thread_id)
                .await
                .unwrap()
                .turns
                .is_empty()
        );
        let entry = service
            .project_worktrees(project.id)
            .await
            .unwrap()
            .into_iter()
            .find(|e| e.path == destination)
            .unwrap();
        assert_eq!(entry.assigned_task, Some(task.id));
        assert_eq!(entry.branch.as_deref(), Some(branch.as_str()));
    }
    #[tokio::test]
    async fn stale_head_and_cancelled_plans_create_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        repository(&repo);
        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(repo.clone()).await.unwrap();
        let task = service
            .create_task(
                project.id,
                "source".into(),
                service.profiles().await.unwrap()[0].id.clone(),
            )
            .await
            .unwrap();
        let plan = service
            .prepare_new_worktree_fork(task.id, dir.path().canonicalize().unwrap())
            .await
            .unwrap();
        let policy = GitOperationPolicy {
            allow_mutation: true,
            allow_repository_execution: true,
            ..Default::default()
        };
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(
            service
                .create_new_worktree_fork(
                    plan.clone(),
                    "fork".into(),
                    String::new(),
                    policy,
                    cancel
                )
                .await
                .is_err()
        );
        git(&repo, &["commit", "--allow-empty", "-qm", "advance"]);
        assert!(
            service
                .create_new_worktree_fork(
                    plan.clone(),
                    "fork".into(),
                    String::new(),
                    policy,
                    CancellationToken::new()
                )
                .await
                .is_err()
        );
        assert!(!plan.destination.exists());
        assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
    }
    #[tokio::test]
    async fn failed_nested_project_keeps_exact_recoverable_worktree_without_partial_task() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        repository(&repo);
        // The project exists locally, but was never committed. Checkout must not
        // invent it or silently fall back to the repository root for execution.
        let nested = repo.join("uncommitted-project");
        std::fs::create_dir(&nested).unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(nested).await.unwrap();
        let task = service
            .create_task(
                project.id,
                "source".into(),
                service.profiles().await.unwrap()[0].id.clone(),
            )
            .await
            .unwrap();
        let plan = service
            .prepare_new_worktree_fork(task.id, dir.path().canonicalize().unwrap())
            .await
            .unwrap();
        let policy = GitOperationPolicy {
            allow_mutation: true,
            allow_repository_execution: true,
            ..Default::default()
        };
        let error = service
            .create_new_worktree_fork(
                plan.clone(),
                "fork".into(),
                "draft".into(),
                policy,
                CancellationToken::new(),
            )
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&plan.destination.display().to_string()) && error.contains(&plan.branch)
        );
        assert!(plan.destination.is_dir());
        assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
    }
    #[cfg(unix)]
    #[test]
    fn worktree_parent_refuses_symlinks_and_source_descendants() {
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().canonicalize().unwrap();
        let repo = parent.join("repo");
        std::fs::create_dir(&repo).unwrap();
        let linked = parent.join("linked");
        std::os::unix::fs::symlink(&parent, &linked).unwrap();
        assert!(validate_parent(&linked, &repo).is_err());
        assert!(validate_parent(&repo, &repo).is_err());
    }
}
