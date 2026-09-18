use super::*;
use std::{path::Path, process::Command, sync::Mutex as StdMutex};
use synara_runtime::{LaunchSpec, RuntimeError, SpawnedProcess};

fn options() -> GitOperationOptions {
    GitOperationOptions {
        policy: GitOperationPolicy {
            allow_mutation: true,
            allow_repository_execution: true,
            network: GitNetworkPolicy::LocalFilesystem,
            ..GitOperationPolicy::default()
        },
        ..GitOperationOptions::default()
    }
}

fn git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgSign=false",
        ])
        .args(arguments)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn fixture(seed: bool) -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &["init", "-q", "--initial-branch=main"]);
    git(root.path(), &["config", "user.name", "Fixture"]);
    git(
        root.path(),
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(root.path(), &["config", "core.autocrlf", "false"]);
    if seed {
        std::fs::write(root.path().join("base.txt"), "original\n").unwrap();
        git(root.path(), &["add", "--", "base.txt"]);
        git(root.path(), &["commit", "-qm", "initial"]);
    }
    root
}

async fn run(service: &GitOperations, operation: GitOperation) -> GitOperationOutput {
    service
        .execute(operation, options(), CancellationToken::new(), None)
        .await
        .unwrap()
}

async fn fail(service: &GitOperations, operation: GitOperation) -> GitOperationError {
    service
        .execute(operation, options(), CancellationToken::new(), None)
        .await
        .unwrap_err()
}

#[test]
fn ref_validation_rejects_options_and_revision_expressions() {
    for name in [
        "",
        "-f",
        "../escape",
        "a..b",
        "a@{1}",
        "a b",
        "a\nb",
        "a:b",
        "a.lock",
        ".hidden",
        "a//b",
        "a/",
        "a\\b",
    ] {
        assert!(plan::branch(name).is_err(), "{name:?}");
    }
    for name in ["main", "feature/naïve", "a-b_c.d"] {
        plan::branch(name).unwrap();
    }
    for start in ["--force", "HEAD~1", "@{-1}", "main"] {
        assert!(
            plan::build(
                GitOperation::CreateBranch {
                    name: "safe".into(),
                    start: start.into()
                },
                &options().policy,
                true
            )
            .is_err()
        );
    }
}

#[test]
fn set_remote_url_is_not_remote_add() {
    let plan = plan::build(
        GitOperation::SetRemoteUrl {
            name: "origin".into(),
            url: "https://example.invalid/new.git".into(),
        },
        &options().policy,
        true,
    )
    .unwrap();
    assert_eq!(&plan.args[..2], &["remote", "set-url"]);
}

#[test]
fn mutation_execution_and_network_consents_are_independent() {
    let readonly = GitOperationPolicy::default();
    assert!(plan::build(GitOperation::Branches, &readonly, true).is_ok());
    assert_eq!(
        plan::build(
            GitOperation::DeleteBranch {
                name: "topic".into()
            },
            &readonly,
            true
        )
        .err(),
        Some(GitOperationErrorKind::ConsentRequired)
    );
    let mutation = GitOperationPolicy {
        allow_mutation: true,
        ..readonly
    };
    assert!(
        plan::build(
            GitOperation::CreateBranch {
                name: "topic".into(),
                start: "HEAD".into()
            },
            &mutation,
            true
        )
        .is_ok()
    );
    assert_eq!(
        plan::build(
            GitOperation::SwitchBranch {
                name: "topic".into()
            },
            &mutation,
            true
        )
        .err(),
        Some(GitOperationErrorKind::ConsentRequired)
    );
    let execution = GitOperationPolicy {
        allow_repository_execution: true,
        ..mutation
    };
    assert_eq!(
        plan::build(
            GitOperation::Fetch {
                remote: "origin".into(),
                branch: "main".into()
            },
            &execution,
            true
        )
        .err(),
        Some(GitOperationErrorKind::ConsentRequired)
    );
    let hooks = GitOperationPolicy {
        hooks: GitHookPolicy::Configured,
        ..readonly
    };
    assert_eq!(
        plan::build(GitOperation::Branches, &hooks, true).err(),
        Some(GitOperationErrorKind::ConsentRequired)
    );
}

#[test]
fn remote_urls_reject_helpers_credentials_and_options() {
    for url in [
        "ext::sh malicious",
        "-oProxyCommand=evil",
        "http://example.invalid/a",
        "https://user:SECRET@example.invalid/a",
        "https://example.invalid/a?token=SECRET",
        "ssh://user;touch@host/a",
        "ssh://host$(evil)/a",
    ] {
        assert!(
            plan::build(
                GitOperation::AddRemote {
                    name: "origin".into(),
                    url: url.into()
                },
                &options().policy,
                true
            )
            .is_err()
        );
    }
    for url in [
        "https://example.invalid/a.git",
        "ssh://git@example.invalid/a.git",
        "/tmp/space repository.git",
        "C:/space repository.git",
    ] {
        assert!(
            plan::build(
                GitOperation::AddRemote {
                    name: "origin".into(),
                    url: url.into()
                },
                &options().policy,
                true
            )
            .is_ok()
        );
    }
}

#[test]
fn launch_policy_is_explicit_and_remote_environment_is_fixed() {
    let local = runner::launch(&GitOperationOptions::default(), vec!["status".into()], true);
    for argument in [
        "protocol.allow=never",
        "protocol.ext.allow=never",
        "core.fsmonitor=false",
        "core.hooksPath=/dev/null",
        "credential.helper=",
        "commit.gpgSign=false",
    ] {
        assert!(local.args.iter().any(|value| value == argument));
    }
    assert_eq!(
        local.env.get("GIT_TERMINAL_PROMPT").map(String::as_str),
        Some("0")
    );
    let remote = runner::launch(&options(), vec!["status".into()], false);
    assert_eq!(remote.command, Path::new("env"));
    assert!(remote.env.is_empty());
    assert_eq!(remote.args[0], "GIT_TERMINAL_PROMPT=0");
    assert!(remote.args.iter().any(|value| value == "git"));
    let mut approved = options();
    approved.policy.hooks = GitHookPolicy::Configured;
    approved.policy.signing = GitSigningPolicy::Configured;
    approved.policy.credentials = GitCredentialPolicy::ConfiguredNoninteractive;
    approved.policy.network = GitNetworkPolicy::HttpsAndSsh;
    let spec = runner::launch(&approved, vec!["status".into()], true);
    assert!(
        !spec
            .args
            .iter()
            .any(|value| value == "core.hooksPath=/dev/null")
    );
    assert!(
        !spec
            .args
            .iter()
            .any(|value| value == "commit.gpgSign=false")
    );
    assert!(!spec.args.iter().any(|value| value == "credential.helper="));
    assert!(spec.args.iter().any(|value| value == "http.sslVerify=true"));
    assert!(
        spec.args
            .iter()
            .any(|value| value == "http.followRedirects=false")
    );
}

#[test]
fn diagnostics_and_debug_do_not_copy_untrusted_stderr() {
    let canary = "Authentication failed for https://user:SECRET_CANARY@example.invalid/";
    let kind = runner::classify(canary.as_bytes());
    assert_eq!(kind, GitOperationErrorKind::Authentication);
    let error = GitOperationError {
        kind,
        may_have_mutated: true,
        cleanup_confirmed: true,
    };
    assert!(!format!("{error} {error:?}").contains("SECRET_CANARY"));
    let output = GitOperationOutput {
        stdout: canary.as_bytes().to_vec(),
        stderr_bytes: 100,
    };
    assert!(!format!("{output:?}").contains("SECRET_CANARY"));
}

#[derive(Default)]
struct RejectHost {
    calls: StdMutex<Vec<(LaunchSpec, PathBuf)>>,
}
#[async_trait::async_trait]
impl ExecutionHost for RejectHost {
    fn label(&self) -> String {
        "fixture remote".into()
    }
    fn is_local(&self) -> bool {
        false
    }
    async fn spawn(&self, launch: &LaunchSpec, cwd: &Path) -> Result<SpawnedProcess, RuntimeError> {
        self.calls
            .lock()
            .unwrap()
            .push((launch.clone(), cwd.into()));
        Err(RuntimeError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "SECRET_CANARY",
        )))
    }
}

#[tokio::test]
async fn remote_failures_never_fall_back_to_local_git() {
    let host = Arc::new(RejectHost::default());
    let service = GitOperations::with_host("/remote/space project".into(), host.clone());
    let error = service
        .execute(
            GitOperation::Branches,
            GitOperationOptions::default(),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind, GitOperationErrorKind::MissingGit);
    assert!(!format!("{error:?}").contains("SECRET_CANARY"));
    let calls = host.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].1, Path::new("/remote/space project"));
    assert_eq!(calls[0].0.command, Path::new("env"));
}

#[tokio::test]
async fn queue_and_queued_cancellation_are_bounded_without_spawning() {
    let host = Arc::new(RejectHost::default());
    let service = GitOperations::with_host("/remote/project".into(), host.clone());
    let slots = service.queue.clone().acquire_many_owned(16).await.unwrap();
    assert_eq!(
        fail(&service, GitOperation::Branches).await.kind,
        GitOperationErrorKind::QueueFull
    );
    drop(slots);
    let _lock = service.serial.lock().await;
    let cancel = CancellationToken::new();
    cancel.cancel();
    let (tx, rx) = watch::channel(GitOperationProgress::default());
    let error = service
        .execute(GitOperation::Branches, options(), cancel, Some(tx))
        .await
        .unwrap_err();
    assert_eq!(error.kind, GitOperationErrorKind::Cancelled);
    assert!(!error.may_have_mutated);
    assert_eq!(rx.borrow().phase, GitOperationPhase::Cancelled);
    let limited = GitOperationOptions {
        timeout: Duration::from_millis(20),
        ..options()
    };
    let error = service
        .execute(
            GitOperation::Branches,
            limited,
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind, GitOperationErrorKind::Timeout);
    assert!(host.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn branch_lifecycle_preserves_head_and_refuses_unmerged_deletion() {
    let root = fixture(true);
    let service = GitOperations::new(root.path().into());
    run(
        &service,
        GitOperation::CreateBranch {
            name: "feature/naïve".into(),
            start: "HEAD".into(),
        },
    )
    .await;
    run(
        &service,
        GitOperation::RenameBranch {
            old: "feature/naïve".into(),
            new: "topic".into(),
        },
    )
    .await;
    run(
        &service,
        GitOperation::SwitchBranch {
            name: "topic".into(),
        },
    )
    .await;
    std::fs::write(root.path().join("base.txt"), "topic\n").unwrap();
    git(root.path(), &["add", "--", "base.txt"]);
    run(
        &service,
        GitOperation::Commit {
            message: "topic commit".into(),
        },
    )
    .await;
    run(
        &service,
        GitOperation::SwitchBranch {
            name: "main".into(),
        },
    )
    .await;
    let before = git(root.path(), &["rev-parse", "HEAD"]);
    assert_eq!(
        fail(
            &service,
            GitOperation::DeleteBranch {
                name: "topic".into()
            }
        )
        .await
        .kind,
        GitOperationErrorKind::UnmergedBranch
    );
    assert_eq!(git(root.path(), &["rev-parse", "HEAD"]), before);
    assert_eq!(
        std::fs::read(root.path().join("base.txt")).unwrap(),
        b"original\n"
    );
    let branches = run(&service, GitOperation::Branches).await;
    assert!(
        String::from_utf8(branches.stdout)
            .unwrap()
            .contains("refs/heads/topic")
    );
    run(
        &service,
        GitOperation::CreateBranch {
            name: "disposable".into(),
            start: "HEAD".into(),
        },
    )
    .await;
    run(
        &service,
        GitOperation::DeleteBranch {
            name: "disposable".into(),
        },
    )
    .await;
}

#[tokio::test]
async fn dirty_switch_and_index_lock_preserve_user_work() {
    let root = fixture(true);
    let service = GitOperations::new(root.path().into());
    run(
        &service,
        GitOperation::CreateBranch {
            name: "topic".into(),
            start: "HEAD".into(),
        },
    )
    .await;
    run(
        &service,
        GitOperation::SwitchBranch {
            name: "topic".into(),
        },
    )
    .await;
    std::fs::write(root.path().join("base.txt"), "topic\n").unwrap();
    git(root.path(), &["add", "--", "base.txt"]);
    run(
        &service,
        GitOperation::Commit {
            message: "topic".into(),
        },
    )
    .await;
    run(
        &service,
        GitOperation::SwitchBranch {
            name: "main".into(),
        },
    )
    .await;
    std::fs::write(root.path().join("base.txt"), "uncommitted user work\n").unwrap();
    assert_eq!(
        fail(
            &service,
            GitOperation::SwitchBranch {
                name: "topic".into()
            }
        )
        .await
        .kind,
        GitOperationErrorKind::DirtyWorktree
    );
    assert_eq!(
        std::fs::read(root.path().join("base.txt")).unwrap(),
        b"uncommitted user work\n"
    );
    let lock = root.path().join(".git/index.lock");
    std::fs::write(&lock, "other process owns this").unwrap();
    assert_eq!(
        fail(
            &service,
            GitOperation::Commit {
                message: "blocked".into()
            }
        )
        .await
        .kind,
        GitOperationErrorKind::IndexLocked
    );
    assert_eq!(
        std::fs::read_to_string(lock).unwrap(),
        "other process owns this"
    );
}

#[tokio::test]
async fn worktree_lifecycle_does_not_force_remove_dirty_files() {
    let root = fixture(true);
    let sibling = tempfile::tempdir().unwrap();
    let destination = sibling.path().join("space naïve worktree");
    let service = GitOperations::new(root.path().into());
    run(
        &service,
        GitOperation::CreateBranch {
            name: "worktree-topic".into(),
            start: "HEAD".into(),
        },
    )
    .await;
    run(
        &service,
        GitOperation::AddWorktree {
            path: destination.clone(),
            branch: "worktree-topic".into(),
        },
    )
    .await;
    assert!(destination.join("base.txt").is_file());
    assert!(
        run(&service, GitOperation::Worktrees)
            .await
            .stdout
            .contains(&0)
    );
    std::fs::write(destination.join("base.txt"), "precious work\n").unwrap();
    assert_eq!(
        fail(
            &service,
            GitOperation::RemoveWorktree {
                path: destination.clone()
            }
        )
        .await
        .kind,
        GitOperationErrorKind::DirtyWorktree
    );
    assert_eq!(
        std::fs::read(destination.join("base.txt")).unwrap(),
        b"precious work\n"
    );
    std::fs::write(destination.join("base.txt"), "original\n").unwrap();
    run(
        &service,
        GitOperation::RemoveWorktree {
            path: destination.clone(),
        },
    )
    .await;
    assert!(!destination.exists());
}

#[tokio::test]
async fn stash_roundtrip_uses_object_identity_and_retains_recovery_copy() {
    let root = fixture(true);
    let service = GitOperations::new(root.path().into());
    std::fs::write(root.path().join("base.txt"), "saved work\n").unwrap();
    std::fs::write(root.path().join("untracked naïve.txt"), "untracked\n").unwrap();
    run(
        &service,
        GitOperation::SaveStash {
            message: "literal ; $(not-a-command)".into(),
            include_untracked: true,
        },
    )
    .await;
    let list = run(&service, GitOperation::Stashes).await.stdout;
    let identity = String::from_utf8(list.split(|b| *b == 0).next().unwrap().to_vec()).unwrap();
    assert_eq!(identity.len(), 40);
    assert_eq!(
        std::fs::read(root.path().join("base.txt")).unwrap(),
        b"original\n"
    );
    assert!(!root.path().join("untracked naïve.txt").exists());
    run(
        &service,
        GitOperation::ApplyStash {
            object_id: identity.clone(),
        },
    )
    .await;
    assert_eq!(
        std::fs::read(root.path().join("base.txt")).unwrap(),
        b"saved work\n"
    );
    assert_eq!(
        std::fs::read(root.path().join("untracked naïve.txt")).unwrap(),
        b"untracked\n"
    );
    assert_eq!(run(&service, GitOperation::Stashes).await.stdout, list);
    std::fs::write(root.path().join("base.txt"), "new unsaved work\n").unwrap();
    let error = fail(
        &service,
        GitOperation::ApplyStash {
            object_id: identity,
        },
    )
    .await;
    assert!(matches!(
        error.kind,
        GitOperationErrorKind::DirtyWorktree | GitOperationErrorKind::Conflict
    ));
    assert_eq!(
        std::fs::read(root.path().join("base.txt")).unwrap(),
        b"new unsaved work\n"
    );
    assert_eq!(run(&service, GitOperation::Stashes).await.stdout, list);
}

#[tokio::test]
async fn remote_configuration_has_no_implicit_fetch_and_supports_update() {
    let root = fixture(true);
    let service = GitOperations::new(root.path().into());
    run(
        &service,
        GitOperation::AddRemote {
            name: "origin".into(),
            url: "https://example.invalid/old.git".into(),
        },
    )
    .await;
    assert_eq!(
        run(&service, GitOperation::RemoteNames).await.stdout,
        b"origin\n"
    );
    run(
        &service,
        GitOperation::SetRemoteUrl {
            name: "origin".into(),
            url: "https://example.invalid/new.git".into(),
        },
    )
    .await;
    let endpoint = run(
        &service,
        GitOperation::RemoteUrl {
            name: "origin".into(),
            push: false,
        },
    )
    .await;
    assert_eq!(endpoint.stdout, b"https://example.invalid/new.git\n");
    assert!(!root.path().join(".git/FETCH_HEAD").exists());
    run(
        &service,
        GitOperation::RemoveRemote {
            name: "origin".into(),
        },
    )
    .await;
    assert!(
        run(&service, GitOperation::RemoteNames)
            .await
            .stdout
            .is_empty()
    );
}

#[tokio::test]
async fn local_remote_fetch_pull_push_and_divergence_are_safe() {
    let bare = tempfile::tempdir().unwrap();
    git(bare.path(), &["init", "--bare", "-q"]);
    let first = fixture(true);
    let second = fixture(false);
    let a = GitOperations::new(first.path().into());
    let b = GitOperations::new(second.path().into());
    for service in [&a, &b] {
        run(
            service,
            GitOperation::AddRemote {
                name: "origin".into(),
                url: bare.path().to_str().unwrap().into(),
            },
        )
        .await;
    }
    let push = || GitOperation::Push {
        remote: "origin".into(),
        local_branch: "main".into(),
        remote_branch: "main".into(),
    };
    let pull = || GitOperation::PullFastForward {
        remote: "origin".into(),
        branch: "main".into(),
    };
    run(&a, push()).await;
    run(&b, pull()).await;
    std::fs::write(first.path().join("base.txt"), "remote update\n").unwrap();
    git(first.path(), &["add", "--", "base.txt"]);
    run(
        &a,
        GitOperation::Commit {
            message: "update".into(),
        },
    )
    .await;
    run(&a, push()).await;
    run(
        &b,
        GitOperation::Fetch {
            remote: "origin".into(),
            branch: "main".into(),
        },
    )
    .await;
    assert_eq!(
        std::fs::read(second.path().join("base.txt")).unwrap(),
        b"original\n"
    );
    run(&b, pull()).await;
    assert_eq!(
        std::fs::read(second.path().join("base.txt")).unwrap(),
        b"remote update\n"
    );
    for (root, service, content) in [
        (&first, &a, "first divergence\n"),
        (&second, &b, "second divergence\n"),
    ] {
        std::fs::write(root.path().join("base.txt"), content).unwrap();
        git(root.path(), &["add", "--", "base.txt"]);
        run(
            service,
            GitOperation::Commit {
                message: content.into(),
            },
        )
        .await;
    }
    run(&a, push()).await;
    let before = git(second.path(), &["rev-parse", "HEAD"]);
    assert_eq!(
        fail(&b, push()).await.kind,
        GitOperationErrorKind::NonFastForward
    );
    assert_eq!(
        fail(&b, pull()).await.kind,
        GitOperationErrorKind::NonFastForward
    );
    assert_eq!(git(second.path(), &["rev-parse", "HEAD"]), before);
    assert_eq!(
        std::fs::read(second.path().join("base.txt")).unwrap(),
        b"second divergence\n"
    );
}

#[cfg(unix)]
mod ownership {
    use super::*;
    use synara_runtime::ProcessHandle;

    struct ShellHost {
        script: &'static str,
        handles: StdMutex<Vec<ProcessHandle>>,
    }
    #[async_trait::async_trait]
    impl ExecutionHost for ShellHost {
        fn label(&self) -> String {
            "isolated shell fixture".into()
        }
        fn is_local(&self) -> bool {
            true
        }
        async fn spawn(&self, _: &LaunchSpec, cwd: &Path) -> Result<SpawnedProcess, RuntimeError> {
            let mut spec = LaunchSpec::new("sh");
            spec.args = vec!["-c".into(), self.script.into()];
            let process = LocalHost.spawn(&spec, cwd).await?;
            self.handles.lock().unwrap().push(process.handle.clone());
            Ok(process)
        }
    }

    #[tokio::test]
    async fn cancellation_and_future_drop_stop_the_owned_process_even_with_other_handles() {
        for abort in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let host = Arc::new(ShellHost {
                script: "printf ready; exec sleep 60",
                handles: StdMutex::new(vec![]),
            });
            let service = GitOperations::with_host(root.path().into(), host.clone());
            let cancel = CancellationToken::new();
            let signal = cancel.clone();
            let (tx, mut rx) = watch::channel(GitOperationProgress::default());
            let task = tokio::spawn(async move {
                service
                    .execute(GitOperation::Branches, options(), signal, Some(tx))
                    .await
            });
            tokio::time::timeout(
                Duration::from_secs(10),
                rx.wait_for(|state| state.phase == GitOperationPhase::Running),
            )
            .await
            .unwrap()
            .unwrap();
            let handle = host.handles.lock().unwrap()[0].clone();
            if abort {
                task.abort();
                assert!(task.await.unwrap_err().is_cancelled());
            } else {
                cancel.cancel();
                let error = tokio::time::timeout(Duration::from_secs(15), task)
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap_err();
                assert_eq!(error.kind, GitOperationErrorKind::Cancelled);
                assert!(error.cleanup_confirmed);
            }
            let exit = tokio::time::timeout(Duration::from_secs(15), handle.wait())
                .await
                .unwrap()
                .unwrap();
            assert!(!exit.success());
        }
    }

    #[tokio::test]
    async fn stdout_stderr_and_deadline_limits_stop_the_process() {
        for (script, expected) in [
            ("printf abcdef", GitOperationErrorKind::OutputLimit),
            ("printf abcdef >&2", GitOperationErrorKind::OutputLimit),
            ("exec sleep 60", GitOperationErrorKind::Timeout),
        ] {
            let root = tempfile::tempdir().unwrap();
            let host = Arc::new(ShellHost {
                script,
                handles: StdMutex::new(vec![]),
            });
            let service = GitOperations::with_host(root.path().into(), host.clone());
            let limits = GitOperationOptions {
                max_stdout_bytes: 5,
                max_stderr_bytes: 5,
                timeout: Duration::from_millis(100),
                ..options()
            };
            let error = service
                .execute(
                    GitOperation::Branches,
                    limits,
                    CancellationToken::new(),
                    None,
                )
                .await
                .unwrap_err();
            assert_eq!(error.kind, expected);
            assert!(error.cleanup_confirmed);
        }
    }

    #[tokio::test]
    async fn configured_hooks_only_run_after_explicit_approval_and_config_is_unchanged() {
        use std::os::unix::fs::PermissionsExt;
        let root = fixture(true);
        let hooks = root.path().join("hooks");
        std::fs::create_dir(&hooks).unwrap();
        let script = hooks.join("pre-commit");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf marker > hook-ran\nprintf SECRET_CANARY >&2\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
        git(
            root.path(),
            &["config", "core.hooksPath", hooks.to_str().unwrap()],
        );
        git(root.path(), &["config", "commit.gpgSign", "true"]);
        std::fs::write(root.path().join("base.txt"), "default policy\n").unwrap();
        git(root.path(), &["add", "--", "base.txt"]);
        let service = GitOperations::new(root.path().into());
        run(
            &service,
            GitOperation::Commit {
                message: "unsigned without hooks".into(),
            },
        )
        .await;
        assert!(!root.path().join("hook-ran").exists());
        let config = std::fs::read(root.path().join(".git/config")).unwrap();
        let mut approved = options();
        approved.policy.hooks = GitHookPolicy::Configured;
        let error = service
            .execute(
                GitOperation::Commit {
                    message: "approved hook".into(),
                },
                approved,
                CancellationToken::new(),
                None,
            )
            .await
            .unwrap_err();
        assert!(root.path().join("hook-ran").is_file());
        assert!(!format!("{error} {error:?}").contains("SECRET_CANARY"));
        assert_eq!(
            std::fs::read(root.path().join(".git/config")).unwrap(),
            config
        );
    }
}
