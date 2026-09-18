use super::*;

fn seed(store: &Store, root: &Path) -> Task {
    let workspace = Workspace {
        id: WorkspaceId::new(),
        name: "Recovery æ 日本語".into(),
        location: WorkspaceLocation::Local { root: root.into() },
    };
    let project = Project {
        id: ProjectId::new(),
        workspace_id: workspace.id,
        name: "Project".into(),
        relative_directory: PathBuf::new(),
    };
    store.save_workspace(&workspace).unwrap();
    store.save_project(&project).unwrap();
    let task = Task {
        id: TaskId::new(),
        project_id: project.id,
        title: "Task".into(),
        state: TaskState::Ready,
        thread_id: ThreadId::new(),
        agent_id: "never-launch-this-profile".into(),
        working_directory: root.into(),
        updated_at_ms: 0,
    };
    store.save_task(&task).unwrap();
    task
}
fn event(task: &Task, sequence: u64) -> EventEnvelope {
    EventEnvelope {
        id: EventId::new(),
        thread_id: task.thread_id,
        sequence,
        timestamp_ms: sequence as i64,
        event: ThreadEvent::TextDelta {
            message_id: None,
            role: Role::Assistant,
            text: "Persisted æ 🦀\n".into(),
        },
    }
}
fn digest(path: &Path) -> String {
    hex::encode(Sha256::digest(std::fs::read(path).unwrap()))
}

#[test]
fn wal_snapshot_preserves_catalog_sessions_preferences_and_event_identity() {
    let root = tempfile::tempdir().unwrap();
    let original = root.path().join("original.sqlite3");
    let backup = root.path().join("snapshot.sqlite3");
    let restored = root.path().join("recovered.sqlite3");
    let mut store = Store::open(&original).unwrap();
    let task = seed(&store, root.path());
    let first = event(&task, 1);
    store.append(&first).unwrap();
    let session = SessionReference {
        agent_id: task.agent_id.clone(),
        remote_id: "reference-only".into(),
        working_directory: root.path().into(),
        title: Some("Saved session".into()),
    };
    store.save_session(task.thread_id, &session).unwrap();
    let preference = serde_json::json!({"theme":"dark","font_size":14});
    store.set_preference("appearance", &preference).unwrap();
    // This is a live WAL database, not a closed main-file copy.
    assert!(original.with_extension("sqlite3-wal").exists());
    let receipt = store
        .backup_to(&backup, &RecoveryOptions::default())
        .unwrap();
    assert_eq!(receipt.sha256, digest(&backup));
    assert_eq!(receipt.bytes, backup.metadata().unwrap().len());
    assert!(!backup.with_extension("sqlite3-wal").exists());
    store.append(&event(&task, 2)).unwrap();
    let restored_receipt =
        Store::restore_to(&backup, &restored, &RecoveryOptions::default()).unwrap();
    assert_eq!(restored_receipt.sha256, digest(&restored));
    assert_eq!(digest(&backup), receipt.sha256);
    let mut recovery = Store::open(&restored).unwrap();
    assert_eq!(
        recovery.events(task.thread_id, 0, 10).unwrap(),
        vec![first.clone()]
    );
    assert!(!recovery.append(&first).unwrap());
    assert_eq!(recovery.session(task.thread_id).unwrap(), Some(session));
    assert_eq!(
        recovery
            .preference::<serde_json::Value>("appearance")
            .unwrap(),
        Some(preference)
    );
    assert_eq!(store.last_sequence(task.thread_id).unwrap(), 2);
    assert_eq!(recovery.last_sequence(task.thread_id).unwrap(), 1);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            backup.metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn restore_never_replaces_existing_database_or_unrelated_file() {
    let root = tempfile::tempdir().unwrap();
    let backup = root.path().join("backup.sqlite3");
    Store::memory()
        .unwrap()
        .backup_to(&backup, &RecoveryOptions::default())
        .unwrap();
    let destination = root.path().join("keep.txt");
    std::fs::write(&destination, b"irreplaceable user content").unwrap();
    for destination in [&destination, &backup] {
        let before = digest(destination);
        assert!(matches!(
            Store::restore_to(&backup, destination, &RecoveryOptions::default()),
            Err(StorageError::RecoveryDestination)
        ));
        assert_eq!(digest(destination), before);
    }
    assert!(matches!(
        Store::memory()
            .unwrap()
            .backup_to(Path::new("relative.sqlite3"), &RecoveryOptions::default()),
        Err(StorageError::RecoveryDestination)
    ));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
}

#[test]
fn rejects_corrupt_foreign_newer_and_hostile_schema_without_modifying_input() {
    let root = tempfile::tempdir().unwrap();
    for kind in [
        "corrupt", "foreign", "newer", "trigger", "view", "bad-json", "bad-id", "bad-head",
    ] {
        let source = root.path().join(format!("{kind}.sqlite3"));
        let destination = root.path().join(format!("{kind}-restored.sqlite3"));
        if kind == "corrupt" {
            std::fs::write(&source, b"not sqlite").unwrap();
        } else if kind == "foreign" {
            Connection::open(&source)
                .unwrap()
                .execute_batch("CREATE TABLE other(value TEXT); PRAGMA user_version=3")
                .unwrap();
        } else {
            let mut store = Store::open(&source).unwrap();
            let task = seed(&store, root.path());
            store.append(&event(&task, 1)).unwrap();
            let sql = match kind {
                "newer" => "PRAGMA user_version=999",
                "trigger" => {
                    "CREATE TRIGGER poison AFTER INSERT ON preferences BEGIN DELETE FROM events; END"
                }
                "view" => "CREATE VIEW poison AS SELECT load_extension('/untrusted')",
                "bad-json" => "UPDATE events SET data='{'",
                "bad-id" => "UPDATE workspaces SET data='{}'",
                "bad-head" => "UPDATE event_heads SET sequence=70",
                _ => unreachable!(),
            };
            store.connection.execute_batch(sql).unwrap();
        }
        let before = digest(&source);
        assert!(
            Store::restore_to(&source, &destination, &RecoveryOptions::default()).is_err(),
            "{kind} accepted"
        );
        assert!(!destination.exists());
        assert_eq!(digest(&source), before, "{kind} mutated");
    }
    assert!(!std::fs::read_dir(root.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".synara-recovery")
    }));
}

#[test]
fn old_schema_migrates_only_the_staged_copy() {
    let root = tempfile::tempdir().unwrap();
    for version in [1, 2] {
        let source = root.path().join(format!("v{version}.sqlite3"));
        let destination = root.path().join(format!("v{version}-restored.sqlite3"));
        let mut store = Store::open(&source).unwrap();
        let task = seed(&store, root.path());
        store.append(&event(&task, 1)).unwrap();
        store
            .connection
            .execute_batch("DROP TABLE thread_activity;")
            .unwrap();
        if version == 1 {
            store
                .connection
                .execute_batch("DROP TABLE event_heads;")
                .unwrap();
        }
        store
            .connection
            .pragma_update(None, "user_version", version)
            .unwrap();
        drop(store);
        let before = digest(&source);
        Store::restore_to(&source, &destination, &RecoveryOptions::default()).unwrap();
        let restored = Store::open(&destination).unwrap();
        assert_eq!(restored.last_sequence(task.thread_id).unwrap(), 1);
        assert_eq!(restored.replay(task.thread_id).unwrap().last_sequence, 1);
        assert_eq!(digest(&source), before);
        let source =
            Connection::open_with_flags(&source, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(
            source
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            version
        );
    }
}

#[test]
fn cancellation_deadline_and_size_fail_without_publishing_partial_data() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("cancelled.sqlite3");
    let store = Store::memory().unwrap();
    let options = RecoveryOptions::default();
    options.cancellation.cancel();
    assert!(matches!(
        store.backup_to(&destination, &options),
        Err(StorageError::RecoveryCancelled)
    ));
    let options = RecoveryOptions {
        timeout: Duration::ZERO,
        ..RecoveryOptions::default()
    };
    assert!(matches!(
        store.backup_to(&destination, &options),
        Err(StorageError::RecoveryTimeout)
    ));
    let options = RecoveryOptions {
        max_bytes: 4096,
        ..RecoveryOptions::default()
    };
    assert!(matches!(
        store.backup_to(&destination, &options),
        Err(StorageError::Limit)
    ));
    assert!(!destination.exists());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn long_validation_queries_observe_cancellation() {
    let options = RecoveryOptions::default();
    let budget = Budget::new(&options).unwrap();
    let connection = Connection::open_in_memory().unwrap();
    budget.guard(&connection).unwrap();
    let cancel = options.cancellation.clone();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        cancel.cancel();
    });
    let result = connection.query_row("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x < 1000000000) SELECT sum(x) FROM n", [], |r| r.get::<_, i64>(0)).map_err(StorageError::from);
    assert!(matches!(
        budget.result(result),
        Err(StorageError::RecoveryCancelled)
    ));
    thread.join().unwrap();
}

#[test]
fn sqlite_full_rolls_back_event_head_projection_and_recency() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("full.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let task = seed(&store, root.path());
    let first = event(&task, 1);
    store.append(&first).unwrap();
    let before = store.task(task.id).unwrap().unwrap();
    let pages: i64 = store
        .connection
        .query_row("PRAGMA page_count", [], |r| r.get(0))
        .unwrap();
    store
        .connection
        .pragma_update(None, "max_page_count", pages)
        .unwrap();
    let mut too_big = event(&task, 2);
    too_big.event = ThreadEvent::TextDelta {
        message_id: None,
        role: Role::Assistant,
        text: "x".repeat(1024 * 1024),
    };
    let error = store.append(&too_big).unwrap_err();
    assert!(
        matches!(error, StorageError::Database(rusqlite::Error::SqliteFailure(error, _)) if error.code == rusqlite::ErrorCode::DiskFull)
    );
    assert_eq!(store.last_sequence(task.thread_id).unwrap(), 1);
    assert_eq!(
        serde_json::to_value(store.task(task.id).unwrap().unwrap()).unwrap(),
        serde_json::to_value(before).unwrap()
    );
    assert_eq!(store.events(task.thread_id, 0, 20).unwrap(), vec![first]);
    drop(store);
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.replay(task.thread_id).unwrap().last_sequence, 1);
}

#[test]
fn concurrent_writer_backup_is_one_consistent_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("live.sqlite3");
    let mut reader = Store::open(&path).unwrap();
    let task = seed(&reader, root.path());
    reader.append(&event(&task, 1)).unwrap();
    let task_copy = task.clone();
    let writer = std::thread::spawn(move || {
        let mut writer = Store::open(&path).unwrap();
        for sequence in 2..=60 {
            writer.append(&event(&task_copy, sequence)).unwrap();
        }
    });
    let destination = root.path().join("snapshot.sqlite3");
    reader
        .backup_to(&destination, &RecoveryOptions::default())
        .unwrap();
    writer.join().unwrap();
    let snapshot = Store::open(&destination).unwrap();
    let end = snapshot.last_sequence(task.thread_id).unwrap();
    assert!((1..=60).contains(&end));
    assert_eq!(snapshot.replay(task.thread_id).unwrap().last_sequence, end);
    assert_eq!(reader.last_sequence(task.thread_id).unwrap(), 60);
}

#[cfg(unix)]
#[test]
fn rejects_source_and_destination_symlinks_without_following_them() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.sqlite3");
    Store::memory()
        .unwrap()
        .backup_to(&source, &RecoveryOptions::default())
        .unwrap();
    let source_link = root.path().join("source-link");
    symlink(&source, &source_link).unwrap();
    let destination = root.path().join("destination.sqlite3");
    assert!(Store::restore_to(&source_link, &destination, &RecoveryOptions::default()).is_err());
    let destination_link = root.path().join("dest-link");
    symlink(&destination, &destination_link).unwrap();
    assert!(Store::restore_to(&source, &destination_link, &RecoveryOptions::default()).is_err());
    assert!(!destination.exists());
}

#[tokio::test]
async fn service_restore_returns_an_artifact_not_an_executing_workspace() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.sqlite3");
    let service = crate::WorkspaceService::memory().unwrap();
    service
        .backup_to(source.clone(), RecoveryOptions::default())
        .await
        .unwrap();
    let destination = root.path().join("recovered.sqlite3");
    let receipt = crate::WorkspaceService::restore_to(
        source,
        destination.clone(),
        RecoveryOptions::default(),
    )
    .await
    .unwrap();
    assert_eq!(receipt.path, destination.canonicalize().unwrap());
    assert!(service.catalog().await.unwrap().tasks.is_empty());
}

#[cfg(unix)]
#[test]
fn system_directory_alias_is_resolved_but_database_leaf_is_never_followed() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let real = root.path().join("real");
    std::fs::create_dir(&real).unwrap();
    let alias = root.path().join("system-alias");
    symlink(&real, &alias).unwrap();
    let path = alias.join("db.sqlite3");
    let store = Store::open(&path).unwrap();
    assert!(real.join("db.sqlite3").exists());
    let backup = alias.join("backup.sqlite3");
    store
        .backup_to(&backup, &RecoveryOptions::default())
        .unwrap();
    Store::restore_to(
        &backup,
        &alias.join("recovered.sqlite3"),
        &RecoveryOptions::default(),
    )
    .unwrap();
    let linked = alias.join("leaf-link.sqlite3");
    symlink(real.join("db.sqlite3"), &linked).unwrap();
    assert!(Store::open(&linked).is_err());
}

#[test]
fn inaccessible_parent_and_corrupt_database_are_not_replaced() {
    let root = tempfile::tempdir().unwrap();
    let corrupt = root.path().join("corrupt.sqlite3");
    std::fs::write(&corrupt, b"preserve these corrupt bytes for recovery").unwrap();
    let before = digest(&corrupt);
    assert!(Store::open(&corrupt).is_err());
    assert_eq!(digest(&corrupt), before);
    let path = root.path().join("missing-parent/db.sqlite3");
    assert!(Store::open(&path).is_err());
    assert!(!path.exists());
}
