//! Live remote Git verification against scripts/ssh_smoke.py.
#![cfg(unix)]

use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use synara_runtime::{ApprovedPortForward, PinnedSshHost, SshTarget};
use synara_workspace::{
    GitNetworkPolicy, GitOperation, GitOperationErrorKind, GitOperationOptions, GitOperationOutput,
    GitOperationPolicy, GitOperations, GitService,
};
use tokio_util::sync::CancellationToken;

fn fixture() -> (PathBuf, SshTarget) {
    let root = PathBuf::from(std::env::var_os("SYNARA_SSH_SMOKE_ROOT").expect("use ssh_smoke.py"));
    assert!(root.is_absolute());
    assert_eq!(
        std::fs::read_to_string(root.join("fixture.marker")).unwrap(),
        "Synara isolated SSH fixture v1\n"
    );
    let target = SshTarget {
        host: "127.0.0.1".into(),
        port: std::env::var("SYNARA_SSH_SMOKE_PORT")
            .unwrap()
            .parse()
            .unwrap(),
        user: Some(std::env::var("SYNARA_SSH_SMOKE_USER").unwrap()),
    };
    (root, target)
}

fn git(args: &[&str], cwd: &std::path::Path) -> String {
    let output = std::process::Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgSign=false",
        ])
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn explicit_forwarding_uses_pinned_ssh_loopback_and_cleans_up() {
    let (root, target) = fixture();
    let remote_listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let remote_port = remote_listener.local_addr().unwrap().port();
    let server = tokio::task::spawn_blocking(move || {
        for _ in 0..4 {
            let (mut stream, _) = remote_listener.accept().unwrap();
            let mut request = [0_u8; 4];
            if stream.read_exact(&mut request).is_err() {
                continue;
            }
            assert_eq!(&request, b"ping");
            stream.write_all(b"pong").unwrap();
            return;
        }
        panic!("forward never delivered the test payload");
    });

    let local_probe = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let local_port = local_probe.local_addr().unwrap().port();
    drop(local_probe);
    let host = PinnedSshHost::new(target, root.join("known hosts"), root.join("identity")).unwrap();
    let forward = host
        .open_forward(
            ApprovedPortForward::agent_approved(local_port, remote_port, "ssh-live-forward")
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(forward.alive());
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, local_port));
    let response = tokio::task::spawn_blocking(move || {
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
        stream.write_all(b"ping").unwrap();
        let mut response = [0_u8; 4];
        stream.read_exact(&mut response).unwrap();
        response
    })
    .await
    .unwrap();
    assert_eq!(&response, b"pong");
    server.await.unwrap();
    forward.close().await.unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        match TcpListener::bind((Ipv4Addr::LOCALHOST, local_port)) {
            Ok(listener) => {
                drop(listener);
                break;
            }
            Err(_) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(error) => panic!("forward listener leaked after shutdown: {error}"),
        }
    }

    let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, local_port)).unwrap();
    assert!(
        host.open_forward(ApprovedPortForward::manual(local_port, remote_port).unwrap())
            .await
            .is_err()
    );
    drop(occupied);
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn remote_git_uses_pinned_transport_for_status_stage_diff_and_commit() {
    let (root, target) = fixture();
    let project = root.join("remote-git-project");
    std::fs::create_dir(&project).unwrap();
    git(&["init", "-q"], &project);
    git(&["config", "user.name", "Synara SSH fixture"], &project);
    git(
        &["config", "user.email", "fixture@example.invalid"],
        &project,
    );
    std::fs::write(project.join("tracked.txt"), "base\n").unwrap();
    git(&["add", "--", "tracked.txt"], &project);
    git(&["commit", "-q", "-m", "base"], &project);
    std::fs::write(project.join("tracked.txt"), "changed\n").unwrap();

    let host = PinnedSshHost::new(target, root.join("known hosts"), root.join("identity")).unwrap();
    let service = GitService::with_host(project.clone(), Arc::new(host));
    let status = service.status().await.unwrap();
    assert!(
        status
            .entries
            .iter()
            .any(|entry| entry.path == std::path::Path::new("tracked.txt"))
    );
    assert!(
        service
            .diff(false, None)
            .await
            .unwrap()
            .contains("+changed")
    );
    service.stage(PathBuf::from("tracked.txt")).await.unwrap();
    assert!(service.diff(true, None).await.unwrap().contains("+changed"));
    service
        .commit("remote fixture commit".into())
        .await
        .unwrap();
    assert!(service.status().await.unwrap().entries.is_empty());
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn remote_git_does_not_fall_back_to_local_repository_on_trust_failure() {
    let (root, target) = fixture();
    let project = root.join("remote-git-no-fallback");
    std::fs::create_dir(&project).unwrap();
    git(&["init", "-q"], &project);

    let host =
        PinnedSshHost::new(target, root.join("changed hosts"), root.join("identity")).unwrap();
    let service = GitService::with_host(project, Arc::new(host));
    assert!(service.status().await.is_err());
}

fn typed_options() -> GitOperationOptions {
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

fn seeded_project(root: &std::path::Path, name: &str, seed: bool) -> PathBuf {
    let project = root.join(name);
    std::fs::create_dir(&project).unwrap();
    git(&["init", "-q", "--initial-branch=main"], &project);
    git(&["config", "user.name", "Synara SSH fixture"], &project);
    git(
        &["config", "user.email", "fixture@example.invalid"],
        &project,
    );
    git(&["config", "core.autocrlf", "false"], &project);
    if seed {
        std::fs::write(project.join("tracked.txt"), "base\n").unwrap();
        git(&["add", "--", "tracked.txt"], &project);
        git(&["commit", "-qm", "base"], &project);
    }
    project
}

async fn typed(service: &GitOperations, operation: GitOperation) -> GitOperationOutput {
    service
        .execute(operation, typed_options(), CancellationToken::new(), None)
        .await
        .unwrap()
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn remote_typed_branches_stash_and_worktrees_preserve_recovery_data() {
    let (root, target) = fixture();
    let project = seeded_project(&root, "typed git naïve project", true);
    let host = Arc::new(
        PinnedSshHost::new(target, root.join("known hosts"), root.join("identity")).unwrap(),
    );
    let service = GitOperations::with_host(project.clone(), host);
    typed(
        &service,
        GitOperation::CreateBranch {
            name: "topic/naïve".into(),
            start: "HEAD".into(),
        },
    )
    .await;
    typed(
        &service,
        GitOperation::RenameBranch {
            old: "topic/naïve".into(),
            new: "topic".into(),
        },
    )
    .await;
    typed(
        &service,
        GitOperation::SwitchBranch {
            name: "topic".into(),
        },
    )
    .await;
    assert_eq!(git(&["branch", "--show-current"], &project).trim(), "topic");
    std::fs::write(project.join("tracked.txt"), "changed\n").unwrap();
    std::fs::write(project.join("[untracked naïve].txt"), "recover me\n").unwrap();
    typed(
        &service,
        GitOperation::SaveStash {
            message: "literal ; $(no-shell)".into(),
            include_untracked: true,
        },
    )
    .await;
    assert!(!project.join("[untracked naïve].txt").exists());
    assert_eq!(
        std::fs::read(project.join("tracked.txt")).unwrap(),
        b"base\n"
    );
    let stashes = typed(&service, GitOperation::Stashes).await.stdout;
    let identity =
        String::from_utf8(stashes.split(|byte| *byte == 0).next().unwrap().to_vec()).unwrap();
    typed(
        &service,
        GitOperation::ApplyStash {
            object_id: identity.clone(),
        },
    )
    .await;
    assert_eq!(
        std::fs::read(project.join("[untracked naïve].txt")).unwrap(),
        b"recover me\n"
    );
    assert!(
        typed(&service, GitOperation::Stashes)
            .await
            .stdout
            .starts_with(identity.as_bytes())
    );
    git(
        &["add", "--", "tracked.txt", "[untracked naïve].txt"],
        &project,
    );
    typed(
        &service,
        GitOperation::Commit {
            message: "remote topic".into(),
        },
    )
    .await;
    typed(
        &service,
        GitOperation::SwitchBranch {
            name: "main".into(),
        },
    )
    .await;
    let error = service
        .execute(
            GitOperation::DeleteBranch {
                name: "topic".into(),
            },
            typed_options(),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind, GitOperationErrorKind::UnmergedBranch);
    let worktree = root.join("typed worktree space");
    typed(
        &service,
        GitOperation::AddWorktree {
            path: worktree.clone(),
            branch: "topic".into(),
        },
    )
    .await;
    std::fs::write(worktree.join("tracked.txt"), "precious dirty work\n").unwrap();
    let error = service
        .execute(
            GitOperation::RemoveWorktree {
                path: worktree.clone(),
            },
            typed_options(),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind, GitOperationErrorKind::DirtyWorktree);
    assert_eq!(
        std::fs::read(worktree.join("tracked.txt")).unwrap(),
        b"precious dirty work\n"
    );
    std::fs::write(worktree.join("tracked.txt"), "changed\n").unwrap();
    typed(
        &service,
        GitOperation::RemoveWorktree {
            path: worktree.clone(),
        },
    )
    .await;
    assert!(!worktree.exists());
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn remote_typed_fetch_pull_push_keep_divergent_head_and_files() {
    let (root, target) = fixture();
    let first = seeded_project(&root, "typed network first", true);
    let second = seeded_project(&root, "typed network second", false);
    let bare = root.join("typed network bare.git");
    std::fs::create_dir(&bare).unwrap();
    git(&["init", "--bare", "-q", "--initial-branch=main"], &bare);
    let host = Arc::new(
        PinnedSshHost::new(target, root.join("known hosts"), root.join("identity")).unwrap(),
    );
    let a = GitOperations::with_host(first.clone(), host.clone());
    let b = GitOperations::with_host(second.clone(), host);
    for service in [&a, &b] {
        typed(
            service,
            GitOperation::AddRemote {
                name: "origin".into(),
                url: bare.to_str().unwrap().into(),
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
    typed(&a, push()).await;
    typed(&b, pull()).await;
    std::fs::write(first.join("tracked.txt"), "remote update\n").unwrap();
    git(&["add", "--", "tracked.txt"], &first);
    typed(
        &a,
        GitOperation::Commit {
            message: "remote update".into(),
        },
    )
    .await;
    typed(&a, push()).await;
    typed(
        &b,
        GitOperation::Fetch {
            remote: "origin".into(),
            branch: "main".into(),
        },
    )
    .await;
    assert_eq!(
        std::fs::read(second.join("tracked.txt")).unwrap(),
        b"base\n"
    );
    typed(&b, pull()).await;
    assert_eq!(
        std::fs::read(second.join("tracked.txt")).unwrap(),
        b"remote update\n"
    );
    for (project, service, content) in [
        (&first, &a, "first divergence\n"),
        (&second, &b, "second divergence\n"),
    ] {
        std::fs::write(project.join("tracked.txt"), content).unwrap();
        git(&["add", "--", "tracked.txt"], project);
        typed(
            service,
            GitOperation::Commit {
                message: "divergence".into(),
            },
        )
        .await;
    }
    typed(&a, push()).await;
    let before = git(&["rev-parse", "HEAD"], &second);
    for operation in [push(), pull()] {
        let error = b
            .execute(operation, typed_options(), CancellationToken::new(), None)
            .await
            .unwrap_err();
        assert_eq!(error.kind, GitOperationErrorKind::NonFastForward);
    }
    assert_eq!(git(&["rev-parse", "HEAD"], &second), before);
    assert_eq!(
        std::fs::read(second.join("tracked.txt")).unwrap(),
        b"second divergence\n"
    );
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn remote_typed_mutation_refuses_changed_host_without_local_fallback() {
    let (root, target) = fixture();
    let project = seeded_project(&root, "typed no fallback", true);
    let host = Arc::new(
        PinnedSshHost::new(target, root.join("changed hosts"), root.join("identity")).unwrap(),
    );
    let service = GitOperations::with_host(project.clone(), host);
    let before = git(&["rev-parse", "HEAD"], &project);
    let error = service
        .execute(
            GitOperation::CreateBranch {
                name: "must-not-exist".into(),
                start: "HEAD".into(),
            },
            typed_options(),
            CancellationToken::new(),
            None,
        )
        .await
        .unwrap_err();
    assert_eq!(error.kind, GitOperationErrorKind::Network);
    assert!(git(&["branch", "--list", "must-not-exist"], &project).is_empty());
    assert_eq!(git(&["rev-parse", "HEAD"], &project), before);
    assert_eq!(
        std::fs::read(project.join("tracked.txt")).unwrap(),
        b"base\n"
    );
}
