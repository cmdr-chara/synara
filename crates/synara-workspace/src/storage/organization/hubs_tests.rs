use crate::WorkspaceService;
use synara_core::TaskScope;

#[tokio::test]
async fn legacy_studio_is_a_read_only_hub_and_normal_chats_stay_separate() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let project = service.add_local_workspace(dir.path().into()).await.unwrap();
    let agent = service.profiles().await.unwrap()[0].id.clone();
    let studio = service.create_scoped_task_with_draft(project.id, "Design".into(), agent.clone(), TaskScope::Studio, "Keep 日本語".into()).await.unwrap();
    let chat = service.create_scoped_task_with_draft(project.id, "Normal".into(), agent, TaskScope::Chat, "Standalone".into()).await.unwrap();
    let hubs = service.hubs().await.unwrap();
    assert_eq!(hubs.len(), 1);
    assert!(hubs[0].imported);
    assert_eq!(hubs[0].profile.main_task, studio.id);
    assert_eq!(hubs[0].threads, 1);
    assert_eq!(service.task_draft(studio.id).await.unwrap(), "Keep 日本語");
    assert_eq!(service.task_draft(chat.id).await.unwrap(), "Standalone");
    service.access(move |store| {
        assert!(store.preference_raw(&format!("hub:{}",project.id))?.is_none());
        Ok(())
    }).await.unwrap();
    assert!(service.session(studio.thread_id).await.unwrap().is_none());
}

#[tokio::test]
async fn hub_creation_context_conflicts_and_archive_preserve_data_on_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("work");
    std::fs::create_dir(&root).unwrap();
    let db = dir.path().join("state.db");
    let service = WorkspaceService::open(db.clone()).await.unwrap();
    let agent = service.profiles().await.unwrap()[0].id.clone();
    let (mut hub, main) = service.create_hub(root.clone(), "Research".into(), agent.clone()).await.unwrap();
    assert!(service.create_hub(root, "Duplicate".into(), agent.clone()).await.is_err());
    hub.instructions = "Use primary sources".into(); hub.memory = "Keep 日本語 references".into();
    let stale = hub.clone();
    hub = service.save_hub(0, hub).await.unwrap();
    assert!(service.save_hub(0, stale).await.is_err());
    let thread = service.create_hub_thread(hub.project, "New task".into(), agent.clone()).await.unwrap();
    assert_eq!(service.task_draft(thread.id).await.unwrap(), hub.context_draft());
    assert!(service.task_draft(main.id).await.unwrap().is_empty());
    assert!(service.thread(thread.thread_id).await.unwrap().messages.is_empty());
    hub.archived = true;
    let hub = service.save_hub(hub.revision, hub).await.unwrap();
    assert!(service.create_hub_thread(hub.project, "No".into(), agent).await.is_err());
    let backup = dir.path().join("backup.db");
    service.backup_to(backup.clone(), crate::RecoveryOptions::default()).await.unwrap();
    let restored = dir.path().join("restored.db");
    WorkspaceService::restore_to(backup, restored.clone(), crate::RecoveryOptions::default()).await.unwrap();
    drop(service);
    let service = WorkspaceService::open(db).await.unwrap();
    let restored = WorkspaceService::open(restored).await.unwrap();
    assert_eq!(restored.hubs().await.unwrap()[0].profile, hub);
    assert_eq!(service.hubs().await.unwrap()[0].profile, hub);
    assert_eq!(service.task_draft(thread.id).await.unwrap(), hub.context_draft());
    assert!(service.session(thread.thread_id).await.unwrap().is_none());
}

#[tokio::test]
async fn malformed_hub_metadata_is_not_replaced_by_legacy_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let agent = service.profiles().await.unwrap()[0].id.clone();
    let (hub, _) = service.create_hub(dir.path().into(), "Keep".into(), agent).await.unwrap();
    let project = hub.project;
    service.access(move |store| {
        store.connection.execute("UPDATE preferences SET data=?1 WHERE key=?2", rusqlite::params!["{\"version\":99}",format!("hub:{project}")]).map_err(crate::StorageError::from)?;
        Ok(())
    }).await.unwrap();
    assert!(service.hubs().await.is_err());
    assert!(service.save_hub(0, hub).await.is_err());
    service.access(move |store| {
        assert_eq!(store.preference_raw(&format!("hub:{project}"))?.as_deref(), Some("{\"version\":99}"));
        Ok(())
    }).await.unwrap();
}
