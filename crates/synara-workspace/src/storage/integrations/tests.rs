use super::*;
use std::path::PathBuf;

async fn task(service: &WorkspaceService, root: PathBuf) -> Task {
    let project = service.add_local_workspace(root).await.unwrap();
    let agent = service.profiles().await.unwrap()[0].id.clone();
    service
        .create_task(project.id, "Integration fixture".into(), agent)
        .await
        .unwrap()
}
fn config(task: &Task) -> ManagedMcp {
    let mut config = ManagedMcp::new(task.id, task.agent_id.clone());
    config.name = "Fixture tools".into();
    config.endpoint = "https://example.com/mcp".into();
    config
}
#[tokio::test]
async fn integrations_skill_review_install_update_remove_preserves_origin_and_never_enables() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let path = root.join("SKILL.md");
    let service = WorkspaceService::memory().unwrap();
    std::fs::write(&path,"---\nname: Review changes\ndescription: Check the requested diff\nversion: '1.0'\n---\n# Review\nKeep the scope small.\n").unwrap();
    let review = service.review_skill(path.clone()).await.unwrap();
    assert_eq!(review.document().origin.version.as_deref(), Some("1.0"));
    assert_eq!(review.document().title, "Review changes");
    let first = service.install_skill(0, review, None).await.unwrap();
    assert_eq!(first.revision, 1);
    assert!(!first.skills[0].enabled);
    let id = first.skills[0].id.clone();
    let enabled = service
        .edit_skill(
            1,
            SkillEdit::SetEnabled {
                id: id.clone(),
                enabled: true,
            },
        )
        .await
        .unwrap();
    assert!(enabled.skills[0].enabled);
    assert!(
        service
            .edit_skill(1, SkillEdit::Remove(id.clone()))
            .await
            .is_err()
    );
    let stale = service.review_skill(path.clone()).await.unwrap();
    std::fs::write(&path, "# Revised review\nInspect the exact candidate.\n").unwrap();
    assert!(
        service
            .install_skill(2, stale, Some(id.clone()))
            .await
            .is_err()
    );
    assert_eq!(service.integrations().await.unwrap(), enabled);
    let review = service.review_skill(path.clone()).await.unwrap();
    let updated = service
        .install_skill(2, review, Some(id.clone()))
        .await
        .unwrap();
    assert!(!updated.skills[0].enabled);
    assert_eq!(updated.skills[0].id, id);
    assert_eq!(
        updated.skills[0].previous_origins,
        vec![first.skills[0].origin.clone()]
    );
    assert_ne!(
        updated.skills[0].origin.sha256,
        first.skills[0].origin.sha256
    );
    let removed = service.edit_skill(3, SkillEdit::Remove(id)).await.unwrap();
    assert!(removed.skills.is_empty());
    assert!(path.is_file());
}
#[tokio::test]
async fn integrations_duplicate_corrupt_future_and_checksum_failures_preserve_raw_settings() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().canonicalize().unwrap().join("SKILL.md");
    std::fs::write(&path, "# Skill\nLiteral text, not a program.").unwrap();
    let service = WorkspaceService::memory().unwrap();
    let first = service
        .install_skill(0, service.review_skill(path.clone()).await.unwrap(), None)
        .await
        .unwrap();
    assert!(
        service
            .install_skill(1, service.review_skill(path.clone()).await.unwrap(), None)
            .await
            .is_err()
    );
    let mut tampered = first.clone();
    tampered.skills[0].markdown.push_str("Changed");
    assert!(tampered.validate().is_err());
    for raw in [
        "{",
        "{\"version\":99}",
        "{\"version\":1,\"revision\":0,\"skills\":[],\"mcp\":[],\"token\":\"forbidden\"}",
    ] {
        let copy = raw.to_string();
        service
            .access(move |store| {
                store
                    .connection
                    .execute(
                        "UPDATE preferences SET data=?1 WHERE key=?2",
                        params![copy, INTEGRATIONS_KEY],
                    )
                    .map_err(StorageError::from)?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(service.integrations().await.is_err());
        assert!(
            service
                .edit_skill(1, SkillEdit::Remove(first.skills[0].id.clone()))
                .await
                .is_err()
        );
        let raw = raw.to_owned();
        service
            .access(move |store| {
                assert_eq!(
                    store.preference_raw(INTEGRATIONS_KEY)?.as_deref(),
                    Some(raw.as_str())
                );
                Ok(())
            })
            .await
            .unwrap();
    }
}
#[tokio::test]
async fn integrations_mcp_scope_refs_cas_session_invalidation_and_backup_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let service = WorkspaceService::open(db.clone()).await.unwrap();
    let task = task(&service, dir.path().into()).await;
    let mut config = config(&task);
    config.enabled = true; // Save cannot sneak in enablement.
    config.bearer =
        Some(synara_runtime::SecretReference::new("dev.synara", "fixture-account").unwrap());
    let id = config.id.clone();
    let first = service
        .edit_mcp(0, McpEdit::Save(config.clone()))
        .await
        .unwrap();
    assert!(!first.mcp[0].enabled);
    let second = service
        .edit_mcp(
            1,
            McpEdit::SetEnabled {
                id: id.clone(),
                enabled: true,
            },
        )
        .await
        .unwrap();
    assert!(second.mcp[0].applies_to(task.id, &task.agent_id));
    assert!(!second.mcp[0].applies_to(TaskId::new(), &task.agent_id));
    assert!(!second.mcp[0].applies_to(task.id, "a-different-profile"));
    let json = serde_json::to_string(&second).unwrap();
    assert!(json.contains("fixture-account"));
    assert!(!json.contains("Authorization"));
    assert!(
        service
            .edit_mcp(1, McpEdit::Remove(id.clone()))
            .await
            .is_err()
    );
    let mut hijack = config.clone();
    hijack.task = TaskId::new();
    assert!(service.edit_mcp(2, McpEdit::Save(hijack)).await.is_err());
    service
        .save_session(
            task.thread_id,
            SessionReference {
                agent_id: task.agent_id.clone(),
                remote_id: "old-session".into(),
                working_directory: task.working_directory.clone(),
                title: None,
            },
        )
        .await
        .unwrap();
    let third = service
        .edit_mcp(
            2,
            McpEdit::SetEnabled {
                id: id.clone(),
                enabled: false,
            },
        )
        .await
        .unwrap();
    assert!(service.session(task.thread_id).await.unwrap().is_none());
    let backup = dir.path().join("backup.db");
    service
        .backup_to(backup.clone(), crate::RecoveryOptions::default())
        .await
        .unwrap();
    let restored = dir.path().join("restored.db");
    WorkspaceService::restore_to(backup, restored.clone(), crate::RecoveryOptions::default())
        .await
        .unwrap();
    let reopened = WorkspaceService::open(restored).await.unwrap();
    assert_eq!(reopened.integrations().await.unwrap(), third);
    assert!(
        reopened
            .edit_mcp(3, McpEdit::Remove(id))
            .await
            .unwrap()
            .mcp
            .is_empty()
    );
}
#[test]
fn integrations_endpoint_validation_is_fail_closed() {
    for value in [
        "http://example.com/mcp",
        "http://localhost/mcp",
        "https://name:secret@example.com/mcp",
        "https://example.com/mcp?token=secret",
        "https://example.com/mcp#secret",
        "file:///etc/passwd",
        "https://example.com/mcp\r\nAuthorization: secret",
        "https://example.com/a b",
    ] {
        assert!(
            crate::integrations::endpoint(value).is_err(),
            "accepted {value}"
        );
    }
    for value in [
        "https://example.com/mcp",
        "http://127.0.0.1:1234/mcp",
        "http://[::1]/mcp",
    ] {
        assert!(
            crate::integrations::endpoint(value).is_ok(),
            "rejected {value}"
        );
    }
}
#[test]
fn integrations_skill_intake_rejects_nontext_oversize_and_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let path = root.join("SKILL.md");
    for bytes in [
        vec![0xff],
        vec![b'a'; MAX_SKILL_BYTES + 1],
        b"secret\0tail".to_vec(),
        vec![],
    ] {
        std::fs::write(&path, bytes).unwrap();
        assert!(SkillReview::inspect(path.clone()).is_err());
    }
    assert!(SkillReview::inspect(root.join("install.sh")).is_err());
    std::fs::write(
        &path,
        "# Plain document\n```sh\ntouch /tmp/do-not-execute\n```\n",
    )
    .unwrap();
    assert!(SkillReview::inspect(path.clone()).is_ok());
    #[cfg(unix)]
    {
        let link = root.join("link.md");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(SkillReview::inspect(link).is_err());
    }
}
#[tokio::test]
async fn integrations_unknown_profile_and_duplicate_scope_cannot_expand_consent() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = task(&service, dir.path().into()).await;
    let item = config(&task);
    let first = service
        .edit_mcp(0, McpEdit::Save(item.clone()))
        .await
        .unwrap();
    let mut duplicate = item.clone();
    duplicate.id = uuid::Uuid::new_v4().to_string();
    assert!(service.edit_mcp(1, McpEdit::Save(duplicate)).await.is_err());
    let mut other = item;
    other.agent_id = "not-the-task-agent".into();
    assert!(service.edit_mcp(1, McpEdit::Save(other)).await.is_err());
    assert_eq!(service.integrations().await.unwrap(), first);
}

#[tokio::test]
async fn integrations_orphan_scope_can_be_disabled_and_removed_but_not_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = task(&service, dir.path().into()).await;
    let config = config(&task);
    let id = config.id.clone();
    service.edit_mcp(0, McpEdit::Save(config)).await.unwrap();
    service
        .access(move |store| {
            store
                .connection
                .execute("DELETE FROM tasks WHERE id=?1", [task.id.to_string()])
                .map_err(StorageError::from)?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        service
            .edit_mcp(
                1,
                McpEdit::SetEnabled {
                    id: id.clone(),
                    enabled: true
                }
            )
            .await
            .is_err()
    );
    service
        .edit_mcp(
            1,
            McpEdit::SetEnabled {
                id: id.clone(),
                enabled: false,
            },
        )
        .await
        .unwrap();
    assert!(
        service
            .edit_mcp(2, McpEdit::Remove(id))
            .await
            .unwrap()
            .mcp
            .is_empty()
    );
}

#[tokio::test]
async fn integrations_old_profile_grants_never_block_independent_revocation() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = task(&service, dir.path().into()).await;
    let first = config(&task);
    let first_id = first.id.clone();
    service.edit_mcp(0, McpEdit::Save(first)).await.unwrap();
    service
        .edit_mcp(
            1,
            McpEdit::SetEnabled {
                id: first_id.clone(),
                enabled: true,
            },
        )
        .await
        .unwrap();
    let mut second = config(&task);
    second.name = "Second independent grant".into();
    let second_id = second.id.clone();
    service.edit_mcp(2, McpEdit::Save(second)).await.unwrap();
    service
        .edit_mcp(
            3,
            McpEdit::SetEnabled {
                id: second_id.clone(),
                enabled: true,
            },
        )
        .await
        .unwrap();
    let next_agent = service
        .profiles()
        .await
        .unwrap()
        .into_iter()
        .find(|p| p.id != task.agent_id)
        .unwrap()
        .id;
    service.set_task_agent(task.id, next_agent).await.unwrap();
    let value = service
        .edit_mcp(
            4,
            McpEdit::SetEnabled {
                id: first_id.clone(),
                enabled: false,
            },
        )
        .await
        .unwrap();
    assert!(
        value.mcp[1].enabled,
        "revoking one record does not rewrite a sibling grant"
    );
    assert!(
        service
            .edit_mcp(
                5,
                McpEdit::SetEnabled {
                    id: first_id.clone(),
                    enabled: true
                }
            )
            .await
            .is_err()
    );
    service
        .edit_mcp(5, McpEdit::Remove(second_id))
        .await
        .unwrap();
    assert!(
        service
            .edit_mcp(6, McpEdit::Remove(first_id))
            .await
            .unwrap()
            .mcp
            .is_empty()
    );
}
