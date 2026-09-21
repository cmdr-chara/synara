use super::*;
fn repo() -> GithubRepository { GithubRepository::parse("https://github.com/owner/project.git").unwrap() }
#[test] fn repository_identity_is_strict_and_provider_is_not_guessed() {
    assert_eq!(repo(), GithubRepository::parse("git@github.com:owner/project.git").unwrap());
    assert_eq!(repo(), GithubRepository::parse("ssh://git@github.com/owner/project.git").unwrap());
    for value in ["https://github.com.evil/owner/project", "https://u:p@github.com/owner/project", "https://gitlab.com/owner/project", "file:///tmp/repo", "https://github.com/owner/repo/extra", "https://github.com/owner/%2f"] { assert!(GithubRepository::parse(value).is_err(), "{value}"); }
}
#[test] fn provider_paths_cannot_escape_editor_root() {
    for filename in ["../private", "/etc/passwd", "a/../b", "C:/private", "a\\b", "a//b", "a\n"] {
        let file = PrFile { filename: filename.into(), status: "modified".into(), additions: 1, deletions: 0, patch: None };
        assert!(file.editor_path().is_err());
    }
}
#[test] fn merge_requires_pinned_head_and_explicit_method() {
    let action = PrAction::Merge { number: 9, sha: "a".repeat(40), method: MergeMethod::Squash };
    let (method, path, body) = action.plan(&repo()).unwrap();
    assert_eq!(method, "PUT"); assert_eq!(path, "repos/owner/project/pulls/9/merge");
    assert_eq!(body["sha"], "a".repeat(40)); assert_eq!(body["merge_method"], "squash");
    assert!(PrAction::Merge { number: 9, sha: "branch".into(), method: MergeMethod::Merge }.plan(&repo()).is_err());
}
#[test] fn all_write_shapes_are_explicit_and_bounded() {
    for action in [PrAction::Comment { number: 1, body: "text".into() }, PrAction::State { number: 1, open: true },
        PrAction::Draft { node_id: "PR_test".into(), draft: true }, PrAction::Draft { node_id: "PR_test".into(), draft: false },
        PrAction::Review { number: 1, sha: "b".repeat(40), body: "Review".into(), kind: ReviewKind::RequestChanges },
        PrAction::Create { title: "$(echo safe)".into(), body: "Body".into(), base: "base".into(), head: "head".into(), draft: true }] {
        let (method, path, body) = action.plan(&repo()).unwrap(); let (launch, stdin) = request_plan(method, &path, Some(body)).unwrap();
        assert_eq!(launch.command, PathBuf::from("gh")); assert_eq!(launch.args[0], "api");
        assert!(!launch.args.iter().any(|s| s.contains("$(echo"))); assert!(stdin.is_some());
    }
    assert!(PrAction::Comment { number: 1, body: "x".repeat(65537) }.plan(&repo()).is_err());
    assert!(PrAction::Comment { number: 0, body: "ok".into() }.plan(&repo()).is_err());
}
#[test] fn incomplete_provider_state_is_not_invented() {
    let pr: PullRequest = serde_json::from_value(json!({"number":1,"title":"x","state":"open","user":{"login":"owner"}})).unwrap();
    assert!(pr.draft.is_none()); assert!(pr.merged.is_none()); assert!(pr.head.is_null());
    assert!(decode_list::<Value>(json!([1,2]), 1).is_err());
}
#[tokio::test] async fn confirmation_and_cancellation_gate_before_spawning() {
    let service = PullRequests::new(PathBuf::from("/not-a-directory")); let action = PrAction::State { number: 1, open: false };
    assert!(service.perform(&repo(), action.clone(), false, Default::default()).await.unwrap_err().contains("confirmation"));
    let cancel = PullRequestCancellation::new(); cancel.cancel();
    assert_eq!(service.perform(&repo(), action, true, cancel).await.unwrap_err(), "Cancelled");
}
#[cfg(unix)]
#[tokio::test] async fn discovery_does_not_change_worktree_or_index() {
    let dir = tempfile::tempdir().unwrap(); let run = |args: &[&str]| { assert!(std::process::Command::new("git").args(args).current_dir(dir.path()).status().unwrap().success()); };
    run(&["init", "-q"]); run(&["remote", "add", "origin", "https://github.com/owner/project.git"]);
    std::fs::write(dir.path().join("user.txt"), "staged").unwrap(); run(&["add", "user.txt"]);
    std::fs::write(dir.path().join("user.txt"), "unstaged").unwrap();
    let before = std::fs::read(dir.path().join(".git/index")).unwrap();
    let found = PullRequests::new(dir.path().into()).discover(Default::default()).await.unwrap();
    assert_eq!(found, vec![("origin".into(), repo())]);
    assert_eq!(before, std::fs::read(dir.path().join(".git/index")).unwrap());
    assert_eq!(std::fs::read_to_string(dir.path().join("user.txt")).unwrap(), "unstaged");
}
