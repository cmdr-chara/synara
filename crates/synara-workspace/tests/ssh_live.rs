//! Live remote Git verification against scripts/ssh_smoke.py.
#![cfg(unix)]

use std::{path::PathBuf, sync::Arc};
use synara_runtime::{PinnedSshHost, SshTarget};
use synara_workspace::GitService;

fn fixture() -> (PathBuf, SshTarget) {
    let root = PathBuf::from(std::env::var_os("SYNARA_SSH_SMOKE_ROOT").expect("use ssh_smoke.py"));
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

fn git(args: &[&str], cwd: &std::path::Path) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap();
    assert!(status.success());
}

#[tokio::test]
#[ignore = "requires the isolated server from scripts/ssh_smoke.py"]
async fn remote_git_uses_pinned_transport_for_status_stage_diff_and_commit() {
    let (root, target) = fixture();
    let project = root.join("remote-git-project");
    std::fs::create_dir(&project).unwrap();
    git(&["init", "-q"], &project);
    git(&["config", "user.name", "Synara SSH fixture"], &project);
    git(&["config", "user.email", "fixture@example.invalid"], &project);
    std::fs::write(project.join("tracked.txt"), "base\n").unwrap();
    git(&["add", "--", "tracked.txt"], &project);
    git(&["commit", "-q", "-m", "base"], &project);
    std::fs::write(project.join("tracked.txt"), "changed\n").unwrap();

    let host =
        PinnedSshHost::new(target, root.join("known hosts"), root.join("identity")).unwrap();
    let service = GitService::with_host(project.clone(), Arc::new(host));
    let status = service.status().await.unwrap();
    assert!(status.entries.iter().any(|entry| entry.path == PathBuf::from("tracked.txt")));
    assert!(service.diff(false, None).await.unwrap().contains("+changed"));
    service.stage(PathBuf::from("tracked.txt")).await.unwrap();
    assert!(service.diff(true, None).await.unwrap().contains("+changed"));
    service.commit("remote fixture commit".into()).await.unwrap();
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
