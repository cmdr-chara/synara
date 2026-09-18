//! The custom-profile path is exercised in an isolated child, without global env mutation.
use std::{path::Path, sync::Arc, time::Duration};
use synara_acp::AcpBackend;
use synara_agent::DenyInteractions;
use synara_core::Role;
use synara_workspace::{Controller, WorkspaceService, parse_profiles};

const CANARY: &str = "synthetic-value-not-a-credential-🦀";
const STAGES: &[&str] = &[
    "start",
    "copy-agent",
    "open-workspace",
    "parse-profile",
    "reload-profile",
    "create-task",
    "first-prompt",
    "second-prompt",
    "identity",
    "transcript",
    "arguments",
    "environment",
    "working-directory",
    "trace",
    "shutdown",
    "complete",
];

fn checkpoint(stage: &str) {
    assert!(STAGES.contains(&stage));
    eprintln!("profile-checkpoint:{stage}");
}

#[tokio::test]
async fn custom_command_profile_preserves_arguments_and_only_inherits_named_variables() {
    let root = tempfile::tempdir().unwrap();
    let mut child = tokio::process::Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "--ignored",
            "--exact",
            "custom_profile_child_entry",
            "--nocapture",
        ])
        .env_clear()
        .env("SYNARA_BCD_CHILD_ROOT", root.path())
        .env("SYNARA_BCD_CANARY", CANARY)
        .env("SYNARA_BCD_UNLISTED", CANARY)
        .env("HOME", root.path())
        .env("XDG_CONFIG_HOME", root.path().join("config"))
        .env("XDG_DATA_HOME", root.path().join("data"))
        .env("XDG_CACHE_HOME", root.path().join("cache"))
        .kill_on_drop(true);
    if let Some(value) = std::env::var_os("SystemRoot") {
        child.env("SystemRoot", value);
    }
    let result = tokio::time::timeout(Duration::from_secs(30), child.output())
        .await
        .expect("isolated custom-profile proof exceeded its deadline")
        .unwrap();
    // Only fixed, allowlisted checkpoint names may leave the isolated test.
    // Never print arbitrary panic text, process output, paths or environment.
    let stderr = String::from_utf8_lossy(&result.stderr);
    let last_stage = stderr
        .lines()
        .filter_map(|line| line.strip_prefix("profile-checkpoint:"))
        .rfind(|stage| STAGES.contains(stage))
        .unwrap_or("no-checkpoint");
    assert!(
        result.status.success(),
        "isolated profile proof failed after {last_stage} (exit {:?})",
        result.status.code()
    );
    assert!(!contains(&result.stdout, CANARY.as_bytes()));
    assert!(!contains(&result.stderr, CANARY.as_bytes()));
    assert_no_persisted_canary(root.path());
}

fn contains(bytes: &[u8], needle: &[u8]) -> bool {
    bytes.windows(needle.len()).any(|window| window == needle)
}

fn assert_no_persisted_canary(root: &Path) {
    for entry in std::fs::read_dir(root).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            assert_no_persisted_canary(&entry.path());
        } else if entry
            .path()
            .extension()
            .is_some_and(|ext| ext == "sqlite3" || ext == "json")
            || entry.file_name().to_string_lossy().contains("sqlite3-")
        {
            assert!(
                !contains(&std::fs::read(entry.path()).unwrap(), CANARY.as_bytes()),
                "a synthetic inherited value reached persistent storage"
            );
        }
    }
}

#[tokio::test]
#[ignore = "invoked only by the parent test in a clean child environment"]
async fn custom_profile_child_entry() {
    let Some(root) = std::env::var_os("SYNARA_BCD_CHILD_ROOT") else {
        return;
    };
    checkpoint("start");
    let root = std::path::PathBuf::from(root);
    let executable = root
        .join("agent binaries with spaces 🦀")
        .join(format!("custom agent æ{}", std::env::consts::EXE_SUFFIX));
    checkpoint("copy-agent");
    std::fs::create_dir_all(executable.parent().unwrap()).unwrap();
    std::fs::copy(env!("CARGO_BIN_EXE_synara-acp-fixture"), &executable).unwrap();
    let cwd = root.join("project with spaces 日本語");
    std::fs::create_dir(&cwd).unwrap();
    let db = root.join("synara.sqlite3");
    checkpoint("open-workspace");
    let workspace = WorkspaceService::open(db.clone()).await.unwrap();
    let project = workspace.add_local_workspace(cwd.clone()).await.unwrap();
    let extra = vec![
        "literal argument with spaces",
        "æ 日本語 🦀",
        "$(touch SHOULD_NOT_EXIST)",
        "; touch ALSO_MUST_NOT_EXIST",
    ];
    for (id, inherited) in [("custom-with-env", true), ("custom-without-env", false)] {
        let args = [vec!["--integration-fixture", "custom"], extra.clone()].concat();
        let json = serde_json::json!([{
            "id": id,
            "name": "User supplied profile æ",
            "command": executable,
            "args": args,
            "inherit_env": if inherited { vec!["SYNARA_BCD_CANARY"] } else { vec![] },
        }])
        .to_string();
        assert!(!json.contains(CANARY));
        checkpoint("parse-profile");
        let profiles = parse_profiles(&json).unwrap();
        workspace.save_profiles(profiles.clone()).await.unwrap();
        checkpoint("reload-profile");
        let reloaded = WorkspaceService::open(db.clone()).await.unwrap();
        assert_eq!(reloaded.profiles().await.unwrap(), profiles);
        checkpoint("create-task");
        let task = reloaded
            .create_task(project.id, id.into(), id.into())
            .await
            .unwrap();
        let controller = Controller::new(
            reloaded.clone(),
            Arc::new(AcpBackend::default()),
            Arc::new(DenyInteractions),
        );
        checkpoint("first-prompt");
        controller
            .submit(task.id, "launch-proof".into())
            .await
            .unwrap();
        let first = controller.details(task.id).await.unwrap().unwrap();
        checkpoint("second-prompt");
        controller
            .submit(task.id, "launch-proof".into())
            .await
            .unwrap();
        let second = controller.details(task.id).await.unwrap().unwrap();
        checkpoint("identity");
        assert_eq!(first.connection.id, second.connection.id);
        assert_eq!(first.session_id, second.session_id);
        assert_eq!(
            first.connection.identity.as_ref().unwrap().version.as_str(),
            "1.0.0"
        );
        checkpoint("transcript");
        let thread = reloaded.thread(task.thread_id).await.unwrap();
        let replies = thread
            .messages
            .iter()
            .filter(|message| message.role == Role::Assistant)
            .collect::<Vec<_>>();
        assert_eq!(replies.len(), 2);
        for message in replies {
            let proof: serde_json::Value = serde_json::from_str(&message.text).unwrap();
            checkpoint("arguments");
            assert_eq!(proof["args"], serde_json::json!(extra));
            checkpoint("environment");
            assert_eq!(proof["canaryPresent"], inherited);
            assert_eq!(proof["canaryCorrect"], inherited);
            assert_eq!(proof["unlistedPresent"], false);
            checkpoint("working-directory");
            // macOS /var and Windows extended-length paths have canonical aliases.
            // Prove the actual directory identity, not one textual spelling of it.
            let actual_cwd = Path::new(proof["cwd"].as_str().unwrap());
            assert_eq!(
                std::fs::canonicalize(actual_cwd).unwrap(),
                std::fs::canonicalize(&cwd).unwrap()
            );
        }
        checkpoint("trace");
        for trace in controller.trace(task.id, false).await.unwrap() {
            assert!(!format!("{trace:?}").contains(CANARY));
        }
        checkpoint("shutdown");
        controller.shutdown().await.unwrap();
        assert!(!cwd.join("SHOULD_NOT_EXIST").exists());
        assert!(!cwd.join("ALSO_MUST_NOT_EXIST").exists());
    }
    checkpoint("complete");
}
