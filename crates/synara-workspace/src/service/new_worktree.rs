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
            "Worktree {} on branch {} was created, but its task could not be saved: {error}. From the source message, choose the existing unassigned Synara worktree to recover the unsent fork, or remove it explicitly in Git. Nothing was sent.", plan.destination.display(), plan.branch)))
    }

    /// Recover an unsent fork after checkout succeeded but task persistence
    /// failed. This only assigns the exact unassigned Synara worktree selected
    /// by the caller; it never runs a checkout or other mutating Git command.
    pub async fn recover_new_worktree_fork(
        &self,
        source_id: TaskId,
        worktree_directory: PathBuf,
        title: String,
        draft: String,
    ) -> WorkspaceResult<Task> {
        if title.trim().is_empty()
            || title.len() > 400
            || title.contains('\0')
            || draft.len() > 1024 * 1024
        {
            return Err(invalid("invalid fork title or draft size"));
        }

        let source = self.task(source_id).await?;
        if !quiet(&source) {
            return Err(invalid(
                "stop or restore the source task before recovering an isolated fork",
            ));
        }
        if !matches!(
            self.workspace_for_task(&source).await?.location,
            WorkspaceLocation::Local { .. }
        ) {
            return Err(invalid(
                "worktree recovery currently requires a local project",
            ));
        }

        let _lifecycle = self.lock_worktree_lifecycle().await;
        let current_source = self.task(source_id).await?;
        if !unchanged(&source, &current_source) {
            return Err(invalid(
                "the source task changed; review the fork before recovering its worktree",
            ));
        }
        if !matches!(
            self.workspace_for_task(&current_source).await?.location,
            WorkspaceLocation::Local { .. }
        ) {
            return Err(invalid(
                "the reviewed source is no longer a local workspace",
            ));
        }

        let selected_path = normalized_absolute(&worktree_directory)
            .ok_or_else(|| invalid("select a valid linked worktree directory"))?;
        let worktrees = self.project_worktrees(current_source.project_id).await?;
        let worktree = worktrees
            .into_iter()
            .find(|item| item.path == selected_path && !item.project_root)
            .ok_or_else(|| {
                invalid("the selected directory is no longer a linked project worktree")
            })?;
        if worktree.bare || worktree.prunable || worktree.locked {
            return Err(invalid(
                "the selected linked worktree is locked or unavailable",
            ));
        }
        if worktree.assigned_task.is_some() {
            return Err(invalid("this linked worktree already belongs to a task"));
        }

        let branch_token = worktree
            .branch
            .as_deref()
            .and_then(|branch| branch.strip_prefix("synara/"))
            .filter(|token| uuid::Uuid::parse_str(token).is_ok_and(|id| id.to_string() == *token))
            .ok_or_else(|| {
                invalid("the selected worktree does not have a Synara recovery branch")
            })?;
        let expected_directory = format!("worktree-{branch_token}");
        if worktree
            .repository_path
            .file_name()
            .and_then(|name| name.to_str())
            != Some(expected_directory.as_str())
        {
            return Err(invalid(
                "the selected worktree branch and directory do not identify the same Synara fork",
            ));
        }

        let git = GitOperations::new(current_source.working_directory.clone());
        let output = git
            .execute(
                GitOperation::Worktrees,
                GitOperationOptions::default(),
                CancellationToken::new(),
                None,
            )
            .await?;
        let entries = parse_git_worktrees(&output.stdout)?;
        let source_repository = entries
            .iter()
            .filter(|entry| current_source.working_directory.starts_with(&entry.path))
            .max_by_key(|entry| entry.path.components().count())
            .filter(|entry| !entry.bare && !entry.locked && !entry.prunable)
            .ok_or_else(|| invalid("the source worktree is unavailable or locked"))?;
        let recovered_entry = entries
            .iter()
            .find(|entry| entry.path == worktree.repository_path)
            .filter(|entry| {
                !entry.bare
                    && !entry.locked
                    && !entry.prunable
                    && entry.branch.as_deref() == worktree.branch.as_deref()
            })
            .ok_or_else(|| {
                invalid("the selected Synara worktree changed; review the recovery choice")
            })?;
        if source_repository.path == recovered_entry.path
            || source_repository.path.starts_with(&recovered_entry.path)
            || recovered_entry.path.starts_with(&source_repository.path)
        {
            return Err(invalid(
                "the selected worktree overlaps the source repository",
            ));
        }

        self.validate_task_directory(
            current_source.project_id,
            worktree.path.clone(),
            worktree.repository_path.clone(),
        )
        .await?;
        self.create_scoped_task_at(
            current_source.project_id,
            title,
            current_source.agent_id,
            current_source.scope,
            draft,
            Some((worktree.path, worktree.repository_path)),
        )
        .await
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

        // The nested project was not present in the checkout because it had
        // never been committed. Once restored by the user, recovery attaches
        // this same worktree and saves the original unsent draft without a
        // second checkout.
        let recovery_directory = plan.destination.join(&plan.relative_project);
        std::fs::create_dir_all(&recovery_directory).unwrap();
        let count_before = service.project_worktrees(project.id).await.unwrap().len();
        let recovered = service
            .recover_new_worktree_fork(
                task.id,
                recovery_directory.clone(),
                "Recovered fork".into(),
                "unsent review".into(),
            )
            .await
            .unwrap();
        assert_eq!(recovered.working_directory, recovery_directory);
        assert_eq!(
            service.task_draft(recovered.id).await.unwrap(),
            "unsent review"
        );
        assert!(
            service
                .thread(recovered.thread_id)
                .await
                .unwrap()
                .turns
                .is_empty()
        );
        let worktrees_after = service.project_worktrees(project.id).await.unwrap();
        assert_eq!(worktrees_after.len(), count_before);
        let recovered_entry = worktrees_after
            .iter()
            .find(|entry| entry.path == recovery_directory)
            .unwrap();
        assert_eq!(
            recovered_entry.branch.as_deref(),
            Some(plan.branch.as_str())
        );
        assert_eq!(recovered_entry.assigned_task, Some(recovered.id));
    }

    #[tokio::test]
    async fn recovery_rejects_a_regular_unassigned_worktree() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let linked = dir.path().join("ordinary-worktree");
        repository(&repo);
        git(
            &repo,
            &[
                "worktree",
                "add",
                "--quiet",
                "-b",
                "feature/ordinary",
                linked.to_str().unwrap(),
                "HEAD",
            ],
        );
        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(repo).await.unwrap();
        let source = service
            .create_task(
                project.id,
                "source".into(),
                service.profiles().await.unwrap()[0].id.clone(),
            )
            .await
            .unwrap();

        assert!(
            service
                .recover_new_worktree_fork(
                    source.id,
                    linked,
                    "Recovered fork".into(),
                    "draft".into(),
                )
                .await
                .is_err()
        );
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
