use super::*;
async fn seed(service: &WorkspaceService, root: &Path) -> Task {
    let project = service.add_local_workspace(root.into()).await.unwrap();
    service
        .create_task(
            project.id,
            "Snapshot task".into(),
            crate::default_profiles()[0].id.clone(),
        )
        .await
        .unwrap()
}
async fn state(service: &WorkspaceService, task: TaskId, draft: &str, notes: &str) -> TaskContext {
    service.save_task_draft(task, draft.into()).await.unwrap();
    let mut context = service.task_context(task).await.unwrap();
    context.notes = notes.into();
    context.checklist = vec![ChecklistItem::new(format!("Review {notes}"))];
    service
        .save_task_context(task, context.revision, context)
        .await
        .unwrap()
}
#[tokio::test]
async fn checkpoint_restart_atomic_restore_preserves_files_events_sessions_and_other_tasks() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("data.db");
    let service = WorkspaceService::open(path.clone()).await.unwrap();
    let task = seed(&service, root.path()).await;
    let other = seed(&service, root.path()).await;
    let file = root.path().join("work.txt");
    std::fs::write(&file, b"unrelated workspace changes\r\n").unwrap();
    service
        .record(
            task.thread_id,
            ThreadEvent::TextDelta {
                message_id: Some("message".into()),
                role: Role::User,
                text: "Keep transcript".into(),
            },
        )
        .await
        .unwrap();
    service
        .save_session(
            task.thread_id,
            SessionReference {
                agent_id: task.agent_id.clone(),
                remote_id: "unchanged-session".into(),
                working_directory: task.working_directory.clone(),
                title: None,
            },
        )
        .await
        .unwrap();
    let before = state(
        &service,
        task.id,
        "Original 日本語\n  draft",
        "Original notes",
    )
    .await;
    let history = service
        .capture_task_checkpoint(task.id, "Original 日本語\n  draft".into())
        .await
        .unwrap();
    let changed = state(&service, task.id, "Later unsent draft", "Later notes").await;
    state(&service, other.id, "Other draft", "Other notes").await;
    drop(service);
    let service = WorkspaceService::open(path).await.unwrap();
    let loaded = service.task_checkpoints(task.id).await.unwrap();
    assert_eq!(loaded.items[0].id, history.items[0].id);
    let review = service
        .review_task_checkpoint(task.id, history.items[0].id.clone())
        .await
        .unwrap();
    let result = service
        .restore_task_checkpoint(review.clone())
        .await
        .unwrap();
    assert_eq!(result.draft, "Original 日本語\n  draft");
    assert_eq!(result.context.notes, before.notes);
    assert_eq!(result.context.checklist, before.checklist);
    assert_eq!(result.context.revision, changed.revision + 1);
    assert!(
        service.restore_task_checkpoint(review).await.is_err(),
        "never replay a consumed review"
    );
    assert_eq!(service.task_draft(other.id).await.unwrap(), "Other draft");
    assert_eq!(
        std::fs::read(file).unwrap(),
        b"unrelated workspace changes\r\n"
    );
    assert_eq!(
        service
            .session(task.thread_id)
            .await
            .unwrap()
            .unwrap()
            .remote_id,
        "unchanged-session"
    );
    let thread = service.thread(task.thread_id).await.unwrap();
    assert_eq!(thread.last_sequence, 1);
    assert_eq!(thread.messages.len(), 1);
    let recovery = result.history.items.last().unwrap();
    assert_eq!(recovery.label, "Before revert (recovery)");
    let review = service
        .review_task_checkpoint(task.id, recovery.id.clone())
        .await
        .unwrap();
    let recovered = service.restore_task_checkpoint(review).await.unwrap();
    assert_eq!(recovered.draft, "Later unsent draft");
    assert_eq!(recovered.context.notes, "Later notes");
}
#[tokio::test]
async fn stale_draft_notes_history_owner_and_expiry_never_overwrite() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("data.db");
    let a = WorkspaceService::open(path.clone()).await.unwrap();
    let b = WorkspaceService::open(path).await.unwrap();
    let task = seed(&a, root.path()).await;
    state(&a, task.id, "original", "original").await;
    let history = a
        .capture_task_checkpoint(task.id, "original".into())
        .await
        .unwrap();
    let id = history.items[0].id.clone();
    let review = a.review_task_checkpoint(task.id, id.clone()).await.unwrap();
    b.save_task_draft(task.id, "concurrent draft".into())
        .await
        .unwrap();
    assert!(a.restore_task_checkpoint(review).await.is_err());
    assert_eq!(a.task_draft(task.id).await.unwrap(), "concurrent draft");
    let review = a.review_task_checkpoint(task.id, id.clone()).await.unwrap();
    let mut notes = b.task_context(task.id).await.unwrap();
    notes.notes = "concurrent notes".into();
    b.save_task_context(task.id, notes.revision, notes)
        .await
        .unwrap();
    assert!(a.restore_task_checkpoint(review).await.is_err());
    assert_eq!(
        a.task_context(task.id).await.unwrap().notes,
        "concurrent notes"
    );
    let review = a.review_task_checkpoint(task.id, id.clone()).await.unwrap();
    a.capture_task_checkpoint(task.id, "concurrent draft".into())
        .await
        .unwrap();
    assert!(a.restore_task_checkpoint(review).await.is_err());
    let mut review = a.review_task_checkpoint(task.id, id.clone()).await.unwrap();
    review.reviewed_at = Instant::now() - Duration::from_secs(301);
    assert!(a.restore_task_checkpoint(review).await.is_err());
    let other = seed(&a, root.path()).await;
    assert!(
        a.review_task_checkpoint(other.id, id.clone())
            .await
            .is_err()
    );
    let review = a.review_task_checkpoint(task.id, id).await.unwrap();
    a.archive_task(task.id).await.unwrap();
    assert!(a.restore_task_checkpoint(review).await.is_err());
}
#[tokio::test]
async fn partial_storage_failure_rolls_back_draft_notes_and_recovery_together() {
    let root = tempfile::tempdir().unwrap();
    let s = WorkspaceService::memory().unwrap();
    let t = seed(&s, root.path()).await;
    state(&s, t.id, "first", "first").await;
    let h = s
        .capture_task_checkpoint(t.id, "first".into())
        .await
        .unwrap();
    state(&s, t.id, "second", "second").await;
    let before = s.task_context(t.id).await.unwrap();
    let review = s
        .review_task_checkpoint(t.id, h.items[0].id.clone())
        .await
        .unwrap();
    s.access(|store| {store.connection.execute_batch("CREATE TEMP TRIGGER checkpoint_fault BEFORE UPDATE ON preferences WHEN NEW.key LIKE 'task-context:%' BEGIN SELECT RAISE(ABORT,'injected storage failure'); END;").map_err(sql)?;Ok(())}).await.unwrap();
    assert!(s.restore_task_checkpoint(review).await.is_err());
    assert_eq!(s.task_draft(t.id).await.unwrap(), "second");
    assert_eq!(s.task_context(t.id).await.unwrap(), before);
    assert_eq!(s.task_checkpoints(t.id).await.unwrap().revision, h.revision);
}
#[tokio::test]
async fn bounded_history_and_oversize_refusal_preserve_current_work() {
    let root = tempfile::tempdir().unwrap();
    let s = WorkspaceService::memory().unwrap();
    let t = seed(&s, root.path()).await;
    for n in 0..12 {
        let text = format!("{n}:{}", "a".repeat(60 * 1024));
        s.save_task_draft(t.id, text.clone()).await.unwrap();
        s.capture_task_checkpoint(t.id, text).await.unwrap();
    }
    let h = s.task_checkpoints(t.id).await.unwrap();
    assert!(h.items.len() <= 8);
    assert!(encode(&h).unwrap().len() <= MAX_HISTORY);
    assert_eq!(h.revision, 12);
    let latest = h.items.last().unwrap().id.clone();
    let big = "x".repeat(129 * 1024);
    s.save_task_draft(t.id, big.clone()).await.unwrap();
    assert!(s.capture_task_checkpoint(t.id, big.clone()).await.is_err());
    assert!(s.review_task_checkpoint(t.id, latest).await.is_err());
    assert_eq!(s.task_draft(t.id).await.unwrap(), big);
    assert_eq!(s.task_checkpoints(t.id).await.unwrap().revision, 12);
    assert!(
        s.capture_task_checkpoint(t.id, "not the current draft".into())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn malformed_future_and_wrong_owner_history_are_not_repaired_silently() {
    let root = tempfile::tempdir().unwrap();
    let s = WorkspaceService::memory().unwrap();
    let t = seed(&s, root.path()).await;
    for value in [
        serde_json::json!({"version":99}),
        serde_json::json!({"version":1,"revision":0,"task":TaskId::new(),"items":[]}),
    ] {
        let inserted = value.clone();
        s.access(move |store| {
            store.set_preference(&key(t.id), &inserted)?;
            Ok(())
        })
        .await
        .unwrap();
        assert!(s.task_checkpoints(t.id).await.is_err());
        assert!(
            s.capture_task_checkpoint(t.id, String::new())
                .await
                .is_err()
        );
        s.access(move |store| {
            assert_eq!(
                store.preference::<serde_json::Value>(&key(t.id))?.unwrap(),
                value
            );
            Ok(())
        })
        .await
        .unwrap();
    }
}
#[tokio::test]
async fn checkpoint_key_is_valid_and_permanent_task_deletion_removes_history() {
    let root = tempfile::tempdir().unwrap();
    let s = WorkspaceService::memory().unwrap();
    let t = seed(&s, root.path()).await;
    assert!(valid_preference_key(&key(t.id)));
    assert!(!valid_preference_key("task-checkpoints:malformed"));
    s.capture_task_checkpoint(t.id, String::new())
        .await
        .unwrap();
    s.archive_task(t.id).await.unwrap();
    s.delete_task(t.id).await.unwrap();
    assert!(s.task_checkpoints(t.id).await.is_err());
    s.access(move |store| {
        assert!(store.preference_raw(&key(t.id))?.is_none());
        Ok(())
    })
    .await
    .unwrap();
}
