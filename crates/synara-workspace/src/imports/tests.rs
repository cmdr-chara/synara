use super::*;
use serde_json::json;
use std::fs;
use synara_core::{TaskScope, TaskState};

const SESSION: &str = "11111111-1111-4111-8111-111111111111";
const USER: &str = "22222222-2222-4222-8222-222222222222";
const ANSWER: &str = "33333333-3333-4333-8333-333333333333";
const BRANCH: &str = "44444444-4444-4444-8444-444444444444";
fn codex_rows() -> Vec<Value> {
    vec![
        json!({"type":"session_meta","payload":{"id":SESSION,"cwd":"/untrusted/provider/cwd"}}),
        json!({"type":"event_msg","timestamp":"2025-01-02T03:04:05Z","payload":{"type":"user_message","message":"Reviewed source question"}}),
        json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Reviewed source question"}]}}),
        json!({"type":"response_item","payload":{"type":"reasoning","encrypted_content":"DO_NOT_IMPORT"}}),
        json!({"type":"event_msg","timestamp":"2025-01-02T03:04:06+00:00","payload":{"type":"agent_message","message":"Reviewed source answer"}}),
        json!({"type":"response_item","payload":{"type":"message","role":"assistant","content":[{"type":"output_text","text":"Reviewed source answer"}]}}),
        json!({"type":"turn_context","payload":{"approval_policy":"never","secrets":"DO_NOT_IMPORT"}}),
    ]
}
fn claude(id: &str, parent: Option<&str>, kind: &str, text: &str) -> Value {
    json!({"type":kind,"uuid":id,"parentUuid":parent,"sessionId":SESSION,"cwd":"/not-authority","message":{"role":kind,"content":[{"type":"text","text":text}]}})
}
fn write_rows(root: &Path, name: &str, rows: &[Value]) -> PathBuf {
    let path = root.join(name);
    fs::write(
        &path,
        rows.iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();
    path
}
async fn fixture() -> (
    tempfile::TempDir,
    WorkspaceService,
    Project,
    HistoryPreview,
    Vec<u8>,
) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    fs::create_dir(root.join("source")).unwrap();
    fs::create_dir(root.join("destination")).unwrap();
    let path = write_rows(&root.join("source"), "rollout.jsonl", &codex_rows());
    let original = fs::read(path).unwrap();
    let scan = HistorySource::discover(
        root.join("source"),
        HistoryProvider::Codex,
        Default::default(),
    )
    .await
    .unwrap();
    let preview = scan
        .source
        .preview(scan.files[0].clone(), None, Default::default())
        .await
        .unwrap();
    let service = WorkspaceService::open(root.join("store.sqlite3"))
        .await
        .unwrap();
    let project = service
        .add_local_workspace(root.join("destination"))
        .await
        .unwrap();
    (dir, service, project, preview, original)
}
#[test]
fn codex_visible_events_are_authoritative_and_not_duplicated() {
    let parsed = parse_codex(&codex_rows()).unwrap();
    assert_eq!(parsed.messages.len(), 2);
    assert_eq!(parsed.messages[0].role, Role::User);
    assert_eq!(parsed.messages[1].role, Role::Assistant);
    assert_eq!(parsed.messages[0].timestamp_ms, Some(1735787045000));
    assert!(
        !parsed
            .messages
            .iter()
            .any(|m| m.text.contains("DO_NOT_IMPORT"))
    );
}
#[test]
fn codex_legacy_response_items_are_explicit_and_only_supported_role_text() {
    let rows: Vec<_> = codex_rows()
        .into_iter()
        .filter(|v| v["type"] != "event_msg")
        .collect();
    let parsed = parse_codex(&rows).unwrap();
    assert_eq!(parsed.messages.len(), 2);
    assert!(parsed.warnings.iter().any(|w| w.contains("older rollout")));
    let mut mixed = rows;
    mixed.push(json!({"type":"session_meta","payload":{"id":USER}}));
    assert!(parse_codex(&mixed).is_err());
}
#[test]
fn claude_imports_one_explicit_branch_not_merged_siblings() {
    let rows = vec![
        claude(USER, None, "user", "Shared question"),
        claude(ANSWER, Some(USER), "assistant", "Earlier branch"),
        claude(BRANCH, Some(USER), "assistant", "Later branch"),
    ];
    let latest = parse_claude(&rows, None).unwrap();
    assert_eq!(latest.leaves.len(), 2);
    assert_eq!(
        latest
            .messages
            .iter()
            .map(|m| m.text.as_str())
            .collect::<Vec<_>>(),
        vec!["Shared question", "Later branch"]
    );
    let older = parse_claude(&rows, Some(ANSWER)).unwrap();
    assert_eq!(older.messages[1].text, "Earlier branch");
    assert!(parse_claude(&rows, Some(USER)).is_err());
}
#[test]
fn claude_excludes_reasoning_tools_sidechains_and_meta_but_retains_chain() {
    let mut user = claude(USER, None, "user", "Hello");
    user["message"]["content"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"tool_result","content":"DO_NOT_IMPORT"}));
    let mut answer = claude(ANSWER, Some(USER), "assistant", "Visible answer");
    answer["message"]["content"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"thinking","thinking":"DO_NOT_IMPORT"}));
    let mut side = claude(BRANCH, Some(ANSWER), "assistant", "DO_NOT_IMPORT");
    side["isSidechain"] = json!(true);
    let parsed = parse_claude(&[user, answer, side], None).unwrap();
    assert_eq!(parsed.messages.len(), 2);
    assert!(
        !parsed
            .messages
            .iter()
            .any(|m| m.text.contains("DO_NOT_IMPORT"))
    );
}
#[test]
fn claude_rejects_cycles_missing_parents_conflicting_ids_and_mixed_sessions() {
    assert!(
        parse_claude(
            &[
                claude(USER, Some(ANSWER), "user", "cycle"),
                claude(ANSWER, Some(USER), "assistant", "cycle")
            ],
            None
        )
        .is_err()
    );
    assert!(parse_claude(&[claude(USER, Some(ANSWER), "user", "missing")], None).is_err());
    assert!(
        parse_claude(
            &[
                claude(USER, None, "user", "one"),
                claude(USER, None, "user", "two")
            ],
            None
        )
        .is_err()
    );
    let mut other = claude(ANSWER, Some(USER), "assistant", "other");
    other["sessionId"] = json!(BRANCH);
    assert!(parse_claude(&[claude(USER, None, "user", "one"), other], None).is_err());
}
#[test]
fn malformed_oversized_text_and_out_of_range_timestamps_are_bounded() {
    assert!(bounded_text(&"x".repeat(MAX_MESSAGE + 1)).is_err());
    assert!(bounded_text("a\0b").is_err());
    assert_eq!(timestamp(&json!("not-a-date")), None);
    assert_eq!(timestamp(&json!("9999-12-31T23:59:59Z")), None);
    assert_eq!(safe_label(" \nhello\t world", 7), "hello w");
}
#[tokio::test]
async fn history_import_is_atomic_inert_scoped_and_duplicate_safe_across_restart() {
    let (dir, service, project, preview, original) = fixture().await;
    let review = service
        .review_history_import(
            preview.clone(),
            project.id,
            crate::default_profiles()[0].id.clone(),
        )
        .await
        .unwrap();
    assert_eq!(review.preview().messages().len(), 2);
    assert!(service.catalog().await.unwrap().tasks.is_empty());
    let HistoryImportOutcome::Imported(task) = service
        .import_history(review.clone(), Default::default())
        .await
        .unwrap()
    else {
        panic!("new import")
    };
    assert_eq!(task.scope, TaskScope::Chat);
    assert_eq!(task.state, TaskState::Ready);
    assert_eq!(
        task.working_directory,
        dir.path().canonicalize().unwrap().join("destination")
    );
    let thread = service.thread(task.thread_id).await.unwrap();
    assert_eq!(thread.messages.len(), 2);
    assert!(
        thread.tools.is_empty()
            && thread.permissions.is_empty()
            && thread.inputs.is_empty()
            && thread.turns.is_empty()
    );
    assert_eq!(thread.state, TaskState::Ready);
    assert!(service.session(task.thread_id).await.unwrap().is_none());
    assert_eq!(service.task_draft(task.id).await.unwrap(), "");
    assert_eq!(fs::read(preview.source_path()).unwrap(), original);
    drop(service);
    let reopened = WorkspaceService::open(dir.path().join("store.sqlite3"))
        .await
        .unwrap();
    let outcome = reopened
        .import_history(review, Default::default())
        .await
        .unwrap();
    assert!(
        matches!(outcome,HistoryImportOutcome::AlreadyImported {task:Some(existing),source_changed:false} if existing.id==task.id)
    );
    assert_eq!(reopened.catalog().await.unwrap().tasks.len(), 1);
    assert!(reopened.session(task.thread_id).await.unwrap().is_none());
}
#[tokio::test]
async fn changed_source_and_changed_destination_refuse_unreviewed_import() {
    let (_dir, service, project, preview, original) = fixture().await;
    let review = service
        .review_history_import(
            preview.clone(),
            project.id,
            crate::default_profiles()[0].id.clone(),
        )
        .await
        .unwrap();
    fs::write(
        preview.source_path(),
        [original.clone(), b"\n".to_vec()].concat(),
    )
    .unwrap();
    assert!(
        service
            .import_history(review.clone(), Default::default())
            .await
            .is_err()
    );
    assert!(service.catalog().await.unwrap().tasks.is_empty());
    fs::write(preview.source_path(), original).unwrap();
    service
        .access(move |store| {
            let mut changed = project;
            changed.name = "Changed destination".into();
            store.save_project(&changed)?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        service
            .import_history(review, Default::default())
            .await
            .is_err()
    );
    assert!(service.catalog().await.unwrap().tasks.is_empty());
}
#[tokio::test]
async fn cancellation_and_transaction_failure_leave_no_partial_task_or_receipt() {
    let (_dir, service, project, preview, _) = fixture().await;
    let review = service
        .review_history_import(preview, project.id, crate::default_profiles()[0].id.clone())
        .await
        .unwrap();
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(
        service
            .import_history(review.clone(), cancel)
            .await
            .is_err()
    );
    service
        .access(|store| {
            store.history_import_fault(true)?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        service
            .import_history(review.clone(), Default::default())
            .await
            .is_err()
    );
    assert!(service.catalog().await.unwrap().tasks.is_empty());
    service
        .access(|store| {
            assert!(
                store
                    .preference::<HistoryImportLedger>(IMPORT_LEDGER)?
                    .is_none()
            );
            store.history_import_fault(false)?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(matches!(
        service
            .import_history(review, Default::default())
            .await
            .unwrap(),
        HistoryImportOutcome::Imported(_)
    ));
}
#[tokio::test]
async fn malformed_tail_is_retryable_and_discovery_does_not_read_credentials() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_rows(dir.path(), "session.jsonl", &codex_rows());
    fs::write(dir.path().join("auth.json"), b"DO_NOT_IMPORT").unwrap();
    fs::write(dir.path().join("bad.jsonl"), b"{\"type\":").unwrap();
    let discovery = HistorySource::discover(
        dir.path().into(),
        HistoryProvider::Codex,
        Default::default(),
    )
    .await
    .unwrap();
    assert_eq!(discovery.files.len(), 2);
    assert!(
        discovery
            .source
            .preview(
                HistoryFile {
                    relative_path: "auth.json".into(),
                    bytes: 0
                },
                None,
                Default::default()
            )
            .await
            .is_err()
    );
    assert!(
        discovery
            .source
            .preview(
                discovery.files[0].clone(),
                Some(SESSION.into()),
                Default::default()
            )
            .await
            .is_err()
    );
    let bad = discovery
        .files
        .iter()
        .find(|f| f.relative_path == Path::new("bad.jsonl"))
        .unwrap();
    assert!(
        discovery
            .source
            .preview(bad.clone(), None, Default::default())
            .await
            .is_err()
    );
    fs::copy(path, dir.path().join("bad.jsonl")).unwrap();
    assert!(
        discovery
            .source
            .preview(bad.clone(), None, Default::default())
            .await
            .is_ok()
    );
}
#[cfg(unix)]
#[tokio::test]
async fn symlinks_special_files_and_traversal_never_become_history_sources() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    write_rows(outside.path(), "outside.jsonl", &codex_rows());
    symlink(outside.path(), dir.path().join("linked")).unwrap();
    symlink(
        outside.path().join("outside.jsonl"),
        dir.path().join("linked.jsonl"),
    )
    .unwrap();
    let fifo = dir.path().join("pipe.jsonl");
    assert!(
        std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    let discovery = HistorySource::discover(
        dir.path().into(),
        HistoryProvider::Codex,
        Default::default(),
    )
    .await
    .unwrap();
    assert!(discovery.files.is_empty());
    assert!(discovery.skipped >= 3);
    for path in [
        PathBuf::from("../outside.jsonl"),
        PathBuf::from("linked.jsonl"),
        PathBuf::from("pipe.jsonl"),
    ] {
        assert!(
            discovery
                .source
                .preview(
                    HistoryFile {
                        relative_path: path,
                        bytes: 1
                    },
                    None,
                    Default::default()
                )
                .await
                .is_err()
        );
    }
}
#[tokio::test]
async fn ledger_is_included_in_validated_backup_and_restore_without_provider_sessions() {
    let (dir, service, project, preview, _) = fixture().await;
    let review = service
        .review_history_import(preview, project.id, crate::default_profiles()[0].id.clone())
        .await
        .unwrap();
    let HistoryImportOutcome::Imported(task) = service
        .import_history(review.clone(), Default::default())
        .await
        .unwrap()
    else {
        panic!("new")
    };
    let backup = dir.path().join("backup.sqlite3");
    service
        .backup_to(backup.clone(), Default::default())
        .await
        .unwrap();
    let restored = dir.path().join("restored.sqlite3");
    WorkspaceService::restore_to(backup, restored.clone(), Default::default())
        .await
        .unwrap();
    let restored = WorkspaceService::open(restored).await.unwrap();
    assert!(
        matches!(restored.import_history(review,Default::default()).await.unwrap(),HistoryImportOutcome::AlreadyImported {task:Some(existing),..} if existing.id==task.id)
    );
    assert!(restored.session(task.thread_id).await.unwrap().is_none());
}
