//! User-directed Git operations on the selected execution host.
//!
//! Construct policy from a trusted user action, never from agent/tool output.
//! Repository filters and configured helpers can execute code, so checkout and
//! network operations require a separate repository-execution approval. This is
//! not an operating-system sandbox. No operation offers force-push, hard reset,
//! forced branch/worktree deletion, stash pop, or stash clear.
mod plan;
mod runner;

// Shared fixed-category diagnostics for the legacy status/diff service.
pub(crate) fn classify_failure(stderr: &[u8]) -> GitOperationErrorKind {
    runner::classify(stderr)
}

use std::{path::PathBuf, sync::Arc, time::Duration};
use synara_runtime::{ExecutionHost, LocalHost};
use tokio::sync::{Mutex, Semaphore, watch};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GitHookPolicy {
    #[default]
    Disabled,
    Configured,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GitSigningPolicy {
    #[default]
    Unsigned,
    Configured,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GitCredentialPolicy {
    #[default]
    Disabled,
    ConfiguredNoninteractive,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GitNetworkPolicy {
    #[default]
    Disabled,
    Https,
    HttpsAndSsh,
    /// Filesystem transport on the selected host, not an implicit local fallback.
    LocalFilesystem,
}

/// Deliberately not deserializable. The UI/controller owns the consent boundary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GitOperationPolicy {
    pub allow_mutation: bool,
    pub allow_repository_execution: bool,
    pub hooks: GitHookPolicy,
    pub signing: GitSigningPolicy,
    pub credentials: GitCredentialPolicy,
    pub network: GitNetworkPolicy,
}

#[derive(Clone, Debug)]
pub struct GitOperationOptions {
    pub policy: GitOperationPolicy,
    pub timeout: Duration,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
}
impl Default for GitOperationOptions {
    fn default() -> Self {
        Self {
            policy: GitOperationPolicy::default(),
            timeout: Duration::from_secs(30),
            max_stdout_bytes: 4 * 1024 * 1024,
            max_stderr_bytes: 64 * 1024,
        }
    }
}

/// No arbitrary arguments, shell commands, force flags, or revision expressions.
#[derive(Clone)]
pub enum GitOperation {
    Branches,
    RemoteNames,
    RemoteUrl {
        name: String,
        push: bool,
    },
    Worktrees,
    Stashes,
    CreateBranch {
        name: String,
        start: String,
    },
    RenameBranch {
        old: String,
        new: String,
    },
    DeleteBranch {
        name: String,
    },
    SwitchBranch {
        name: String,
    },
    AddRemote {
        name: String,
        url: String,
    },
    SetRemoteUrl {
        name: String,
        url: String,
    },
    RemoveRemote {
        name: String,
    },
    Fetch {
        remote: String,
        branch: String,
    },
    PullFastForward {
        remote: String,
        branch: String,
    },
    Push {
        remote: String,
        local_branch: String,
        remote_branch: String,
    },
    AddWorktree {
        path: PathBuf,
        branch: String,
    },
    AddNewWorktree {
        path: PathBuf,
        branch: String,
        head: String,
    },
    RemoveWorktree {
        path: PathBuf,
    },
    /// Drops administrative `.git/worktrees` entries whose directories vanished
    /// out of band. Runs after a confirmed removal so stale metadata cannot pin
    /// branches or confuse later listings.
    PruneWorktrees,
    SaveStash {
        message: String,
        include_untracked: bool,
    },
    /// Applying by object identity retains the stash, even on conflict.
    ApplyStash {
        object_id: String,
    },
    Commit {
        message: String,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GitOperationPhase {
    #[default]
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Progress contains only counters and state, never URLs, arguments or stderr.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GitOperationProgress {
    pub phase: GitOperationPhase,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
}

pub struct GitOperationOutput {
    /// Bounded raw output. Treat its contents as untrusted, potentially private data.
    pub stdout: Vec<u8>,
    pub stderr_bytes: usize,
}
impl std::fmt::Debug for GitOperationOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitOperationOutput")
            .field("stdout_bytes", &self.stdout.len())
            .field("stderr_bytes", &self.stderr_bytes)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum GitOperationErrorKind {
    #[error("invalid Git operation input")]
    InvalidInput,
    #[error("explicit user consent is required for this Git operation")]
    ConsentRequired,
    #[error("the Git operation queue is full")]
    QueueFull,
    #[error("Git operation cancelled")]
    Cancelled,
    #[error("Git operation timed out")]
    Timeout,
    #[error("Git output exceeded the configured limit")]
    OutputLimit,
    #[error("Git is not available on the selected host")]
    MissingGit,
    #[error("another Git process holds the repository index lock")]
    IndexLocked,
    #[error("Git reported a conflict; existing index and working files were retained")]
    Conflict,
    #[error("Git authentication failed; configure credentials on the selected host")]
    Authentication,
    #[error("Git network or transport verification failed")]
    Network,
    #[error("Git refused a non-fast-forward operation")]
    NonFastForward,
    #[error("Git refused to overwrite local changes")]
    DirtyWorktree,
    #[error("Git refused deletion of an unmerged branch")]
    UnmergedBranch,
    #[error("Git operation failed")]
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{kind} (mutation possible: {may_have_mutated}, cleanup confirmed: {cleanup_confirmed})")]
pub struct GitOperationError {
    pub kind: GitOperationErrorKind,
    /// Cancellation/failure is never presented as an automatic transaction rollback.
    pub may_have_mutated: bool,
    /// Process exit, not proof that every remotely detached descendant was killed.
    pub cleanup_confirmed: bool,
}
impl GitOperationError {
    fn before_spawn(kind: GitOperationErrorKind) -> Self {
        Self {
            kind,
            may_have_mutated: false,
            cleanup_confirmed: true,
        }
    }
}

#[derive(Clone)]
pub struct GitOperations {
    root: PathBuf,
    host: Arc<dyn ExecutionHost>,
    serial: Arc<Mutex<()>>,
    queue: Arc<Semaphore>,
}
impl GitOperations {
    pub fn new(root: PathBuf) -> Self {
        Self::with_host(root, Arc::new(LocalHost))
    }

    pub fn with_host(root: PathBuf, host: Arc<dyn ExecutionHost>) -> Self {
        Self {
            root,
            host,
            serial: Arc::new(Mutex::new(())),
            queue: Arc::new(Semaphore::new(16)),
        }
    }

    pub async fn execute(
        &self,
        operation: GitOperation,
        options: GitOperationOptions,
        cancel: CancellationToken,
        progress: Option<watch::Sender<GitOperationProgress>>,
    ) -> Result<GitOperationOutput, GitOperationError> {
        let plan = plan::build(operation, &options.policy, self.host.is_local())
            .map_err(GitOperationError::before_spawn)?;
        self.execute_plan(plan, options, cancel, progress).await
    }
}

#[cfg(test)]
mod tests;
