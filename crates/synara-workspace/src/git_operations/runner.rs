use super::{
    GitCredentialPolicy, GitHookPolicy, GitNetworkPolicy, GitOperationError, GitOperationErrorKind,
    GitOperationOptions, GitOperationOutput, GitOperationPhase, GitOperationProgress,
    GitOperations, GitSigningPolicy, plan::Plan,
};
use std::time::Duration;
use synara_runtime::{LaunchSpec, ProcessHandle, ProcessReader, RuntimeError};
use tokio::{io::AsyncReadExt, sync::watch, time::Instant};
use tokio_util::sync::CancellationToken;

type Result<T> = std::result::Result<T, GitOperationError>;

struct StopOnDrop(ProcessHandle);
impl Drop for StopOnDrop {
    fn drop(&mut self) {
        // This remains effective when a caller drops or aborts execute().
        self.0.request_stop();
    }
}

fn early(kind: GitOperationErrorKind) -> GitOperationError {
    GitOperationError::before_spawn(kind)
}

pub(super) fn launch(
    options: &GitOperationOptions,
    arguments: Vec<String>,
    local: bool,
) -> LaunchSpec {
    // These typed operations accept refs and explicit worktree destinations, not
    // pathspecs. A global --literal-pathspecs leaks into stash's internal clean
    // command and prevents its generated pathspecs from removing saved files.
    // Literal stage/unstage paths remain protected by the separate GitService.
    let mut command = vec!["--no-pager".into()];
    let mut config = |value: &str| {
        command.extend(["-c".into(), value.into()]);
    };
    for value in [
        "color.ui=false",
        "core.fsmonitor=false",
        "credential.interactive=false",
        "submodule.recurse=false",
        "maintenance.auto=false",
        "gc.auto=0",
        "protocol.allow=never",
        "protocol.ext.allow=never",
    ] {
        config(value);
    }
    if options.policy.hooks == GitHookPolicy::Disabled {
        config("core.hooksPath=/dev/null");
    }
    if options.policy.credentials == GitCredentialPolicy::Disabled {
        config("credential.helper=");
        config("core.askPass=");
    }
    if options.policy.signing == GitSigningPolicy::Unsigned {
        config("commit.gpgSign=false");
        config("tag.gpgSign=false");
        config("push.gpgSign=false");
    }
    match options.policy.network {
        GitNetworkPolicy::Disabled => {}
        GitNetworkPolicy::Https | GitNetworkPolicy::HttpsAndSsh => {
            config("protocol.https.allow=always");
            config("http.sslVerify=true");
            config("http.followRedirects=false");
            if options.policy.network == GitNetworkPolicy::HttpsAndSsh {
                config("protocol.ssh.allow=always");
                config(
                    "core.sshCommand=ssh -oBatchMode=yes -oStrictHostKeyChecking=yes -oForwardAgent=no -oForwardX11=no -oClearAllForwardings=yes -oPermitLocalCommand=no -oControlMaster=no -oControlPath=none -oControlPersist=no",
                );
            }
        }
        GitNetworkPolicy::LocalFilesystem => config("protocol.file.allow=always"),
    }
    command.extend(arguments);
    let mut spec = LaunchSpec::new(if local { "git" } else { "env" });
    if local {
        for (key, value) in [
            ("GIT_TERMINAL_PROMPT", "0"),
            ("GCM_INTERACTIVE", "never"),
            ("GIT_OPTIONAL_LOCKS", "0"),
            ("LC_ALL", "C"),
        ] {
            spec.env.insert(key.into(), value.into());
        }
    } else {
        // Existing SSH hosts prohibit arbitrary environment values in launch.env.
        // These fixed non-secret settings cross the POSIX boundary as structured
        // arguments. Never substitute a LocalHost when remote spawning fails.
        spec.args = vec![
            "GIT_TERMINAL_PROMPT=0".into(),
            "GCM_INTERACTIVE=never".into(),
            "GIT_OPTIONAL_LOCKS=0".into(),
            "LC_ALL=C".into(),
            "git".into(),
        ];
    }
    spec.args.extend(command);
    spec
}

pub(super) fn classify(stderr: &[u8]) -> GitOperationErrorKind {
    use GitOperationErrorKind::*;
    // Inspect transient output for a category, but never put it into Display,
    // Debug, progress, or persisted diagnostics: it can contain credentials.
    let text = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    if text.contains("index.lock") || text.contains("another git process") {
        IndexLocked
    } else if text.contains("non-fast-forward")
        || text.contains("not possible to fast-forward")
        || text.contains("fetch first")
    {
        NonFastForward
    } else if text.contains("authentication failed")
        || text.contains("could not read username")
        || text.contains("terminal prompts disabled")
        || text.contains("permission denied (publickey)")
    {
        Authentication
    } else if text.contains("would be overwritten")
        || text.contains("modified or untracked")
        || text.contains("not a clean working tree")
    {
        DirtyWorktree
    } else if text.contains("not fully merged") {
        UnmergedBranch
    } else if text.contains("conflict") || text.contains("unmerged") {
        Conflict
    } else if text.contains("could not resolve host")
        || text.contains("failed to connect")
        || text.contains("unable to access")
        || text.contains("host key verification failed")
        || text.contains("ssl certificate")
        || text.contains("transport")
    {
        Network
    } else if text.contains("git: not found") || text.contains("git: command not found") {
        MissingGit
    } else {
        Failed
    }
}

/// Push --porcelain reports per-ref rejection status on stdout, not stderr.
/// Inspect only its tab-delimited status field. Ref names, remote banners, and
/// hook messages must not be mistaken for a non-fast-forward rejection.
pub(super) fn classify_push(stdout: &[u8], stderr: &[u8]) -> GitOperationErrorKind {
    for line in stdout.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let mut fields = line.splitn(3, |byte| *byte == b'\t');
        if fields.next() != Some(b"!") {
            continue;
        }
        let Some(refspec) = fields.next() else {
            continue;
        };
        if !refspec.starts_with(b"refs/heads/") || !refspec.contains(&b':') {
            continue;
        }
        if matches!(
            fields.next(),
            Some(b"[rejected] (non-fast-forward)" | b"[rejected] (fetch first)")
        ) {
            return GitOperationErrorKind::NonFastForward;
        }
    }
    classify(stderr)
}

async fn read(
    mut stream: ProcessReader,
    limit: usize,
    stderr: bool,
    progress: Option<watch::Sender<GitOperationProgress>>,
) -> std::result::Result<Vec<u8>, GitOperationErrorKind> {
    let mut bytes = Vec::with_capacity(limit);
    let mut chunk = [0u8; 8192];
    loop {
        let count = stream
            .read(&mut chunk)
            .await
            .map_err(|_| GitOperationErrorKind::Failed)?;
        if count == 0 {
            return Ok(bytes);
        }
        if let Some(progress) = &progress {
            progress.send_modify(|state| {
                if stderr {
                    state.stderr_bytes = state.stderr_bytes.saturating_add(count as u64);
                } else {
                    state.stdout_bytes = state.stdout_bytes.saturating_add(count as u64);
                }
            });
        }
        if count > limit.saturating_sub(bytes.len()) {
            return Err(GitOperationErrorKind::OutputLimit);
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
}

impl GitOperations {
    pub(super) async fn execute_plan(
        &self,
        plan: Plan,
        options: GitOperationOptions,
        cancel: CancellationToken,
        progress: Option<watch::Sender<GitOperationProgress>>,
    ) -> Result<GitOperationOutput> {
        if let Some(progress) = &progress {
            progress.send_replace(GitOperationProgress::default());
        }
        let result = self.run_plan(plan, options, cancel, progress.clone()).await;
        if let Some(progress) = &progress {
            progress.send_modify(|state| {
                state.phase = match &result {
                    Ok(_) => GitOperationPhase::Completed,
                    Err(error) if error.kind == GitOperationErrorKind::Cancelled => {
                        GitOperationPhase::Cancelled
                    }
                    Err(_) => GitOperationPhase::Failed,
                };
            });
        }
        result
    }

    async fn run_plan(
        &self,
        plan: Plan,
        options: GitOperationOptions,
        cancel: CancellationToken,
        progress: Option<watch::Sender<GitOperationProgress>>,
    ) -> Result<GitOperationOutput> {
        use GitOperationErrorKind::*;
        if options.timeout.is_zero()
            || options.timeout > Duration::from_secs(300)
            || !(1..=8 * 1024 * 1024).contains(&options.max_stdout_bytes)
            || !(1..=256 * 1024).contains(&options.max_stderr_bytes)
        {
            return Err(early(InvalidInput));
        }
        let _queue = self
            .queue
            .clone()
            .try_acquire_owned()
            .map_err(|_| early(QueueFull))?;
        let deadline = Instant::now() + options.timeout;
        let _serial = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(early(Cancelled)),
            () = tokio::time::sleep_until(deadline) => return Err(early(Timeout)),
            guard = self.serial.lock() => guard,
        };
        let spec = launch(&options, plan.args, self.host.is_local());
        let process = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(early(Cancelled)),
            () = tokio::time::sleep_until(deadline) => return Err(early(Timeout)),
            process = self.host.spawn(&spec, &self.root) => process.map_err(|error| {
                early(match error {
                    RuntimeError::Io(error) if error.kind() == std::io::ErrorKind::NotFound => MissingGit,
                    _ => Failed,
                })
            })?,
        };
        let owner = StopOnDrop(process.handle.clone());
        drop(process.stdin);
        if let Some(progress) = &progress {
            progress.send_modify(|state| state.phase = GitOperationPhase::Running);
        }
        let result = {
            let operation = async {
                let (stdout, stderr, exit) = tokio::try_join!(
                    read(
                        process.stdout,
                        options.max_stdout_bytes,
                        false,
                        progress.clone()
                    ),
                    read(
                        process.stderr,
                        options.max_stderr_bytes,
                        true,
                        progress.clone()
                    ),
                    async { process.handle.wait().await.map_err(|_| Failed) }
                )?;
                if !exit.success() {
                    return Err(if plan.push_porcelain {
                        classify_push(&stdout, &stderr)
                    } else {
                        classify(&stderr)
                    });
                }
                Ok(GitOperationOutput {
                    stdout,
                    stderr_bytes: stderr.len(),
                })
            };
            tokio::select! {
                biased;
                () = cancel.cancelled() => Err(Cancelled),
                () = tokio::time::sleep_until(deadline) => Err(Timeout),
                result = operation => result,
            }
        };
        match result {
            Ok(output) => Ok(output),
            Err(kind) => {
                owner.0.request_stop();
                let cleanup_confirmed = matches!(
                    tokio::time::timeout(Duration::from_secs(8), owner.0.wait()).await,
                    Ok(Ok(exit)) if exit.error.is_none()
                );
                Err(GitOperationError {
                    kind,
                    may_have_mutated: plan.mutation,
                    cleanup_confirmed,
                })
            }
        }
    }
}
