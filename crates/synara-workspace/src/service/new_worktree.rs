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
    remote: bool,
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
    pub fn remote(&self) -> bool {
        self.remote
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
fn remote_worktree_destination(repository: &Path, token: uuid::Uuid) -> WorkspaceResult<PathBuf> {
    let repository = normalized_absolute(repository)
        .ok_or_else(|| invalid("Git returned an invalid remote repository root"))?;
    let parent = repository
        .parent()
        .and_then(normalized_absolute)
        .ok_or_else(|| invalid("the remote repository has no safe sibling directory"))?;
    if parent.parent().is_none() {
        return Err(invalid(
            "managed SSH worktrees require a repository below the remote filesystem root",
        ));
    }
    Ok(parent.join(format!("worktree-{token}")))
}

async fn git_for_workspace(
    service: &WorkspaceService,
    workspace: &Workspace,
    root: PathBuf,
) -> WorkspaceResult<GitOperations> {
    match &workspace.location {
        WorkspaceLocation::Local { .. } => Ok(GitOperations::new(root)),
        WorkspaceLocation::Ssh { .. } => {
            let profile = service
                .ssh_profile(workspace.id)
                .await?
                .ok_or_else(|| invalid("SSH profile is missing"))?;
            Ok(GitOperations::with_host(
                root,
                std::sync::Arc::new(profile.host(workspace)?),
            ))
        }
    }
}

fn reviewed_remote_destination(
    repository: &Path,
    branch: &str,
    destination: &Path,
) -> WorkspaceResult<()> {
    let token = branch
        .strip_prefix("synara/")
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
        .filter(|id| format!("synara/{id}") == branch)
        .ok_or_else(|| invalid("the reviewed remote worktree branch is invalid"))?;
    let expected = remote_worktree_destination(repository, token)?;
    if destination != expected {
        return Err(invalid("the reviewed remote worktree destination changed"));
    }
    Ok(())
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
        let remote = matches!(workspace.location, WorkspaceLocation::Ssh { .. });
        let git = git_for_workspace(self, &workspace, source.working_directory.clone()).await?;
        let output = git
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
        let token = uuid::Uuid::new_v4();
        let destination = if remote {
            remote_worktree_destination(&entry.path, token)?
        } else {
            validate_parent(&parent, &entry.path)?.join(format!("worktree-{token}"))
        };
        let relative_project = source
            .working_directory
            .strip_prefix(&entry.path)
            .map_err(|_| invalid("invalid source project directory"))?
            .to_path_buf();
        Ok(NewWorktreePlan {
            source,
            repository: entry.path.clone(),
            relative_project,
            destination,
            branch: format!("synara/{token}"),
            head,
            remote,
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
        let workspace = self.workspace_for_task(&source).await?;
        let remote = matches!(workspace.location, WorkspaceLocation::Ssh { .. });
        if remote != plan.remote {
            return Err(invalid(
                "the reviewed workspace location changed; review a new worktree plan",
            ));
        }
        if remote {
            reviewed_remote_destination(&plan.repository, &plan.branch, &plan.destination)?;
        } else {
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
        }
        let git = git_for_workspace(self, &workspace, source.working_directory.clone()).await?;
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
        let workspace = self.workspace_for_task(&source).await?;

        let _lifecycle = self.lock_worktree_lifecycle().await;
        let current_source = self.task(source_id).await?;
        if !unchanged(&source, &current_source) {
            return Err(invalid(
                "the source task changed; review the fork before recovering its worktree",
            ));
        }
        let current_workspace = self.workspace_for_task(&current_source).await?;
        if current_workspace.id != workspace.id {
            return Err(invalid(
                "the reviewed workspace changed; review the fork again",
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

        let git = git_for_workspace(
            self,
            &current_workspace,
            current_source.working_directory.clone(),
        )
        .await?;
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

    /// Remove a fixed, previously reviewed set of recoverable Synara worktrees.
    /// Each exact path/branch pair is revalidated by the single-worktree owner.
    /// Failures retain that checkout and are reported; nothing is forced.
    pub async fn cleanup_recoverable_worktrees(
        &self,
        source_id: TaskId,
        scratch_parent: PathBuf,
        reviewed: Vec<(PathBuf, String)>,
        policy: GitOperationPolicy,
        cancel: CancellationToken,
    ) -> WorkspaceResult<(usize, Vec<String>)> {
        if !policy.allow_mutation || !policy.allow_repository_execution {
            return Err(invalid(
                "explicit worktree cleanup and repository-execution consent is required",
            ));
        }
        if reviewed.is_empty() || reviewed.len() > 64 {
            return Err(invalid(
                "review between 1 and 64 managed worktrees before bulk cleanup",
            ));
        }
        let mut identities = std::collections::HashSet::new();
        for (path, branch) in &reviewed {
            let Some(path) = normalized_absolute(path) else {
                return Err(invalid("reviewed worktree path is invalid"));
            };
            if !identities.insert((path, branch.clone())) {
                return Err(invalid(
                    "the reviewed cleanup contains a duplicate worktree",
                ));
            }
        }

        let mut removed = 0usize;
        let mut retained = Vec::new();
        for (path, branch) in reviewed {
            if cancel.is_cancelled() {
                retained.push(format!(
                    "{branch} · {} · cleanup cancelled before removal",
                    path.display()
                ));
                continue;
            }
            let exact_policy = GitOperationPolicy {
                allow_mutation: true,
                allow_repository_execution: true,
                ..Default::default()
            };
            match self
                .cleanup_recoverable_worktree(
                    source_id,
                    path.clone(),
                    scratch_parent.clone(),
                    branch.clone(),
                    exact_policy,
                    cancel.clone(),
                )
                .await
            {
                Ok(()) => removed = removed.saturating_add(1),
                Err(error) => retained.push(
                    format!("{branch} · {} · {error}", path.display())
                        .chars()
                        .take(1024)
                        .collect(),
                ),
            }
        }
        Ok((removed, retained))
    }

    /// Remove only an unassigned Synara-managed scratch checkout after an
    /// explicit review. Git's normal non-force worktree removal remains the
    /// final dirty/locked-worktree guard; the generated branch is retained.
    pub async fn cleanup_recoverable_worktree(
        &self,
        source_id: TaskId,
        worktree_directory: PathBuf,
        scratch_parent: PathBuf,
        expected_branch: String,
        policy: GitOperationPolicy,
        cancel: CancellationToken,
    ) -> WorkspaceResult<()> {
        if !policy.allow_mutation || !policy.allow_repository_execution {
            return Err(invalid(
                "explicit worktree cleanup and repository-execution consent is required",
            ));
        }
        if cancel.is_cancelled() {
            return Err(invalid("worktree cleanup was cancelled before removal"));
        }

        let source = self.task(source_id).await?;
        if !quiet(&source) {
            return Err(invalid(
                "stop or restore the source task before cleaning up an isolated fork",
            ));
        }
        if !matches!(
            self.workspace_for_task(&source).await?.location,
            WorkspaceLocation::Local { .. }
        ) {
            return Err(invalid(
                "managed worktree cleanup currently requires a local project",
            ));
        }

        let _lifecycle = self.lock_worktree_lifecycle().await;
        let current_source = self.task(source_id).await?;
        if !unchanged(&source, &current_source) {
            return Err(invalid(
                "the source task changed; review the cleanup choice again",
            ));
        }
        let selected_path = normalized_absolute(&worktree_directory)
            .ok_or_else(|| invalid("select a valid linked worktree directory"))?;
        let worktree = self
            .project_worktrees(current_source.project_id)
            .await?
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
            return Err(invalid(
                "the selected worktree now belongs to a task; nothing was removed",
            ));
        }
        if worktree.branch.as_deref() != Some(expected_branch.as_str()) {
            return Err(invalid(
                "the selected worktree branch changed; review cleanup again",
            ));
        }
        let token = expected_branch
            .strip_prefix("synara/")
            .filter(|token| uuid::Uuid::parse_str(token).is_ok_and(|id| id.to_string() == *token))
            .ok_or_else(|| invalid("the selected worktree is not a Synara recovery branch"))?;
        let expected_directory = format!("worktree-{token}");
        if worktree
            .repository_path
            .file_name()
            .and_then(|name| name.to_str())
            != Some(expected_directory.as_str())
        {
            return Err(invalid(
                "the selected worktree branch and directory no longer identify the same Synara fork",
            ));
        }

        let git = GitOperations::new(current_source.working_directory.clone());
        let listed = git
            .execute(
                GitOperation::Worktrees,
                GitOperationOptions::default(),
                CancellationToken::new(),
                None,
            )
            .await?;
        let entries = parse_git_worktrees(&listed.stdout)?;
        let source_repository = entries
            .iter()
            .filter(|entry| current_source.working_directory.starts_with(&entry.path))
            .max_by_key(|entry| entry.path.components().count())
            .filter(|entry| !entry.bare && !entry.locked && !entry.prunable)
            .ok_or_else(|| invalid("the source worktree is unavailable or locked"))?;
        let scratch = validate_parent(&scratch_parent, &source_repository.path)?;
        if worktree.repository_path.parent() != Some(scratch.as_path()) {
            return Err(invalid(
                "the selected worktree is outside Synara's managed scratch directory",
            ));
        }
        let target = entries
            .iter()
            .find(|entry| entry.path == worktree.repository_path)
            .filter(|entry| {
                !entry.bare
                    && !entry.locked
                    && !entry.prunable
                    && entry.branch.as_deref() == Some(expected_branch.as_str())
            })
            .ok_or_else(|| invalid("the selected Synara worktree changed; review cleanup again"))?;
        if source_repository.path == target.path
            || source_repository.path.starts_with(&target.path)
            || target.path.starts_with(&source_repository.path)
        {
            return Err(invalid(
                "the selected worktree overlaps the source repository",
            ));
        }

        let options = GitOperationOptions {
            policy: GitOperationPolicy {
                allow_mutation: true,
                allow_repository_execution: true,
                ..Default::default()
            },
            ..Default::default()
        };
        git.execute(
            GitOperation::RemoveWorktree {
                path: worktree.repository_path.clone(),
            },
            options,
            cancel,
            None,
        )
        .await
        .map_err(|error| {
            invalid(format!(
                "Managed worktree cleanup failed: {error}. Dirty or locked files were not forced away."
            ))
        })?;

        let listed = git
            .execute(
                GitOperation::Worktrees,
                GitOperationOptions::default(),
                CancellationToken::new(),
                None,
            )
            .await?;
        if parse_git_worktrees(&listed.stdout)?
            .iter()
            .any(|entry| entry.path == worktree.repository_path)
        {
            return Err(invalid(
                "Git still reports the managed worktree after cleanup",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{git, repository};
    use super::*;
    #[test]
    fn remote_worktree_destination_is_uuid_sibling_and_never_root_level() {
        let token = uuid::Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();
        assert_eq!(
            remote_worktree_destination(Path::new("/srv/repo"), token).unwrap(),
            Path::new("/srv/worktree-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee")
        );
        assert!(remote_worktree_destination(Path::new("/repo"), token).is_err());
        assert!(
            reviewed_remote_destination(
                Path::new("/srv/repo"),
                "synara/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                Path::new("/srv/worktree-aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"),
            )
            .is_ok()
        );
        assert!(
            reviewed_remote_destination(
                Path::new("/srv/repo"),
                "synara/aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
                Path::new("/srv/other"),
            )
            .is_err()
        );
    }

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
    async fn recoverable_worktree_cleanup_is_exact_non_force_and_keeps_dirty_checkouts() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let scratch = dir.path().join("scratch");
        std::fs::create_dir(&scratch).unwrap();
        repository(&repo);
        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(repo.clone()).await.unwrap();
        let source = service
            .create_task(
                project.id,
                "source".into(),
                service.profiles().await.unwrap()[0].id.clone(),
            )
            .await
            .unwrap();
        let policy = GitOperationPolicy {
            allow_mutation: true,
            allow_repository_execution: true,
            ..Default::default()
        };

        let token = uuid::Uuid::new_v4();
        let branch = format!("synara/{token}");
        let destination = scratch.join(format!("worktree-{token}"));
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                branch.as_str(),
                destination.to_str().unwrap(),
                "HEAD",
            ],
        );
        assert!(
            service
                .cleanup_recoverable_worktree(
                    source.id,
                    destination.clone(),
                    scratch.clone(),
                    branch.clone(),
                    GitOperationPolicy::default(),
                    CancellationToken::new(),
                )
                .await
                .is_err()
        );
        assert!(destination.exists());
        service
            .cleanup_recoverable_worktree(
                source.id,
                destination.clone(),
                scratch.clone(),
                branch,
                policy,
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert!(!destination.exists());

        let token = uuid::Uuid::new_v4();
        let dirty_branch = format!("synara/{token}");
        let dirty = scratch.join(format!("worktree-{token}"));
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                dirty_branch.as_str(),
                dirty.to_str().unwrap(),
                "HEAD",
            ],
        );
        std::fs::write(dirty.join("untracked.txt"), "keep me").unwrap();
        assert!(
            service
                .cleanup_recoverable_worktree(
                    source.id,
                    dirty.clone(),
                    scratch,
                    dirty_branch,
                    policy,
                    CancellationToken::new(),
                )
                .await
                .is_err()
        );
        assert!(dirty.exists());
        assert_eq!(
            std::fs::read_to_string(dirty.join("untracked.txt")).unwrap(),
            "keep me"
        );
    }

    #[tokio::test]
    async fn bulk_cleanup_removes_only_exact_clean_unassigned_synara_worktrees() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let scratch = dir.path().join("scratch");
        std::fs::create_dir(&scratch).unwrap();
        repository(&repo);
        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(repo.clone()).await.unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let source = service
            .create_task(project.id, "source".into(), agent.clone())
            .await
            .unwrap();

        let make = |token: uuid::Uuid| {
            (
                format!("synara/{token}"),
                scratch.join(format!("worktree-{token}")),
            )
        };
        let (clean_branch, clean) = make(uuid::Uuid::new_v4());
        let (dirty_branch, dirty) = make(uuid::Uuid::new_v4());
        let (assigned_branch, assigned) = make(uuid::Uuid::new_v4());
        for (branch, path) in [
            (&clean_branch, &clean),
            (&dirty_branch, &dirty),
            (&assigned_branch, &assigned),
        ] {
            git(
                &repo,
                &[
                    "worktree",
                    "add",
                    "-b",
                    branch.as_str(),
                    path.to_str().unwrap(),
                    "HEAD",
                ],
            );
        }
        std::fs::write(dirty.join("untracked.txt"), "keep me").unwrap();
        let assigned_task = service
            .create_scoped_task_in_worktree(
                project.id,
                "assigned".into(),
                agent,
                TaskScope::Project,
                String::new(),
                assigned.clone(),
            )
            .await
            .unwrap();
        assert_eq!(assigned_task.working_directory, assigned);

        let reviewed = vec![
            (clean.clone(), clean_branch.clone()),
            (dirty.clone(), dirty_branch.clone()),
            (assigned.clone(), assigned_branch.clone()),
        ];
        let policy = GitOperationPolicy {
            allow_mutation: true,
            allow_repository_execution: true,
            ..Default::default()
        };
        let (removed, retained) = service
            .cleanup_recoverable_worktrees(
                source.id,
                scratch.clone(),
                reviewed,
                policy,
                CancellationToken::new(),
            )
            .await
            .unwrap();

        assert_eq!(removed, 1);
        assert_eq!(retained.len(), 2);
        assert!(!clean.exists());
        assert!(dirty.exists());
        assert_eq!(
            std::fs::read_to_string(dirty.join("untracked.txt")).unwrap(),
            "keep me"
        );
        assert!(assigned.exists());
        let worktrees = service.project_worktrees(project.id).await.unwrap();
        assert!(
            worktrees.iter().any(|item| {
                item.path == assigned && item.assigned_task == Some(assigned_task.id)
            })
        );
        assert!(
            retained
                .iter()
                .any(|reason| reason.contains(&dirty_branch) && reason.contains("not forced"))
        );
        assert!(retained.iter().any(
            |reason| reason.contains(&assigned_branch) && reason.contains("belongs to a task")
        ));

        let branch_list = git(&repo, &["branch", "--format=%(refname:short)"]);
        assert!(branch_list.contains(&clean_branch));
        assert!(branch_list.contains(&dirty_branch));
        assert!(branch_list.contains(&assigned_branch));
    }

    #[tokio::test]
    async fn bulk_cleanup_requires_reviewed_consent_before_any_removal() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().join("repo");
        let scratch = dir.path().join("scratch");
        std::fs::create_dir(&scratch).unwrap();
        repository(&repo);
        let service = WorkspaceService::memory().unwrap();
        let project = service.add_local_workspace(repo.clone()).await.unwrap();
        let source = service
            .create_task(
                project.id,
                "source".into(),
                service.profiles().await.unwrap()[0].id.clone(),
            )
            .await
            .unwrap();
        let token = uuid::Uuid::new_v4();
        let branch = format!("synara/{token}");
        let path = scratch.join(format!("worktree-{token}"));
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-b",
                branch.as_str(),
                path.to_str().unwrap(),
                "HEAD",
            ],
        );

        assert!(
            service
                .cleanup_recoverable_worktrees(
                    source.id,
                    scratch,
                    vec![(path.clone(), branch)],
                    GitOperationPolicy::default(),
                    CancellationToken::new(),
                )
                .await
                .is_err()
        );
        assert!(path.exists());
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
