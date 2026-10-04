use super::*;

async fn seed(service: &WorkspaceService, root: PathBuf) -> Task {
    let project = service.add_local_workspace(root).await.unwrap();
    let agent = service.profiles().await.unwrap()[0].id.clone();
    let task = service
        .create_scoped_task_with_draft(
            project.id,
            "Original".into(),
            agent,
            TaskScope::Chat,
            "Original unsent draft".into(),
        )
        .await
        .unwrap();
    for (id, role, text) in [
        ("u", Role::User, "Visible request"),
        ("r", Role::Reasoning, "private thoughts"),
        ("a", Role::Assistant, "Visible answer"),
    ] {
        service
            .record(
                task.thread_id,
                ThreadEvent::TextDelta {
                    message_id: Some(id.into()),
                    role,
                    text: text.into(),
                },
            )
            .await
            .unwrap();
    }
    task
}
#[tokio::test]
async fn handoff_is_unsent_atomic_related_and_restart_inert() {
    let root = tempfile::tempdir().unwrap();
    let db = root.path().join("state.db");
    let service = WorkspaceService::open(db.clone()).await.unwrap();
    let parent = seed(&service, root.path().into()).await;
    service
        .save_session(
            parent.thread_id,
            SessionReference {
                agent_id: parent.agent_id.clone(),
                remote_id: "not-transferable".into(),
                working_directory: parent.working_directory.clone(),
                title: None,
            },
        )
        .await
        .unwrap();
    let before = service.thread(parent.thread_id).await.unwrap();
    let review = service
        .review_handoff(parent.id, HandoffTarget::Agent(parent.agent_id.clone()))
        .await
        .unwrap();
    assert_eq!(review.included_messages(), 2);
    assert_eq!(review.omitted_messages(), 0);
    assert!(!review.context().contains("private thoughts"));
    let draft = format!("{}Explain the result", review.context());
    let child = service
        .create_handoff(review.clone(), draft.clone())
        .await
        .unwrap();
    assert!(
        service
            .create_handoff(review, "Never overwrite".into())
            .await
            .is_err()
    );
    assert_eq!(
        service.thread(parent.thread_id).await.unwrap().messages,
        before.messages
    );
    assert_eq!(
        service.task_draft(parent.id).await.unwrap(),
        "Original unsent draft"
    );
    assert_eq!(child.working_directory, parent.working_directory);
    assert_eq!(child.project_id, parent.project_id);
    assert_eq!(child.scope, parent.scope);
    assert_ne!(child.thread_id, parent.thread_id);
    drop(service);
    let service = WorkspaceService::open(db).await.unwrap();
    assert_eq!(service.task_draft(child.id).await.unwrap(), draft);
    let thread = service.thread(child.thread_id).await.unwrap();
    assert!(thread.messages.is_empty() && thread.tools.is_empty() && thread.permissions.is_empty());
    assert!(service.session(child.thread_id).await.unwrap().is_none());
    assert_eq!(
        service
            .session(parent.thread_id)
            .await
            .unwrap()
            .unwrap()
            .remote_id,
        "not-transferable"
    );
    assert_eq!(
        service.thread_origin(child.id).await.unwrap().unwrap().kind,
        RelatedThreadKind::Handoff
    );
    assert!(
        service
            .side_threads(parent.id)
            .await
            .unwrap()
            .threads
            .is_empty()
    );
}
#[tokio::test]
async fn same_task_handoff_is_atomic_route_safe_and_preserves_user_owned_state() {
    let root = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = seed(&service, root.path().into()).await;
    let before = service.thread(task.thread_id).await.unwrap();
    let other_agent = service.profiles().await.unwrap()[1].id.clone();
    assert_ne!(other_agent, task.agent_id);

    // The existing user draft belongs to the source composer and is never overwritten.
    let review = service
        .review_handoff(task.id, HandoffTarget::Agent(other_agent.clone()))
        .await
        .unwrap();
    assert!(
        service
            .continue_handoff_in_place(review, "reviewed continuation".into())
            .await
            .is_err()
    );
    assert_eq!(
        service.task_draft(task.id).await.unwrap(),
        "Original unsent draft"
    );

    service
        .save_task_draft(task.id, String::new())
        .await
        .unwrap();
    let attachments = service.attachment_draft(task.id).await.unwrap();
    let attachments = service
        .add_attachments(
            task.id,
            attachments.revision,
            vec![crate::AttachmentInput::Bytes {
                name: "note.txt".into(),
                bytes: b"private pending attachment".to_vec(),
            }],
        )
        .await
        .unwrap();
    let review = service
        .review_handoff(task.id, HandoffTarget::Agent(other_agent.clone()))
        .await
        .unwrap();
    assert!(
        service
            .continue_handoff_in_place(review, "reviewed continuation".into())
            .await
            .is_err()
    );
    assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
    service
        .edit_attachments(
            task.id,
            attachments.revision,
            crate::AttachmentEdit::ClearPending,
        )
        .await
        .unwrap();

    service
        .save_session(
            task.thread_id,
            SessionReference {
                agent_id: task.agent_id.clone(),
                remote_id: "old-provider-session".into(),
                working_directory: task.working_directory.clone(),
                title: None,
            },
        )
        .await
        .unwrap();

    let review = service
        .review_handoff(task.id, HandoffTarget::Agent(other_agent.clone()))
        .await
        .unwrap();
    let switched = service
        .continue_handoff_in_place(review, "reviewed continuation".into())
        .await
        .unwrap();
    assert_eq!(switched.id, task.id);
    assert_eq!(switched.thread_id, task.thread_id);
    assert_eq!(switched.working_directory, task.working_directory);
    assert_eq!(switched.scope, task.scope);
    assert_eq!(switched.agent_id, other_agent);
    assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
    assert_eq!(
        service.task_draft(task.id).await.unwrap(),
        "reviewed continuation"
    );
    assert!(service.session(task.thread_id).await.unwrap().is_none());
    assert_eq!(
        service.thread(task.thread_id).await.unwrap().messages,
        before.messages
    );

    // Switching back to the original agent keeps the same task and thread.
    service
        .save_task_draft(task.id, String::new())
        .await
        .unwrap();
    let review = service
        .review_handoff(task.id, HandoffTarget::Agent(task.agent_id.clone()))
        .await
        .unwrap();
    let switched = service
        .continue_handoff_in_place(review, "reviewed ACP continuation".into())
        .await
        .unwrap();
    assert_eq!(switched.id, task.id);
    assert_eq!(switched.agent_id, task.agent_id);
}

#[tokio::test]
async fn handoff_refuses_stale_source_and_agent_configuration() {
    let root = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = seed(&service, root.path().into()).await;
    let choice = HandoffTarget::Agent(task.agent_id.clone());
    let review = service
        .review_handoff(task.id, choice.clone())
        .await
        .unwrap();
    service
        .record(
            task.thread_id,
            ThreadEvent::Notice {
                message: "new event".into(),
            },
        )
        .await
        .unwrap();
    assert!(
        service
            .create_handoff(review, "retained".into())
            .await
            .is_err()
    );
    let review = service.review_handoff(task.id, choice).await.unwrap();
    service
        .access(|store| {
            let mut profiles = default_profiles();
            profiles[0].name.push_str(" changed");
            store.set_preference("agent_profiles", &profiles)?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(
        service
            .create_handoff(review, "retained".into())
            .await
            .is_err()
    );
    assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
}
#[tokio::test]
async fn handoff_recap_uses_the_source_agent() {
    let root = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = seed(&service, root.path().into()).await;
    let review = service.review_recap(task.id).await.unwrap();
    assert_eq!(review.included_messages(), 2);
    let child = service
        .create_handoff(review, "recap request".into())
        .await
        .unwrap();
    assert_eq!(child.agent_id, task.agent_id);
    assert_eq!(
        service.thread_origin(child.id).await.unwrap().unwrap().kind,
        RelatedThreadKind::Recap
    );
}
#[tokio::test]
async fn handoff_rollback_preserves_source_and_review_can_be_retried_explicitly() {
    let root = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = seed(&service, root.path().into()).await;
    let review = service
        .review_handoff(task.id, HandoffTarget::Agent(task.agent_id.clone()))
        .await
        .unwrap();
    service.access(|store| {store.connection.execute_batch("CREATE TEMP TRIGGER fail_handoff BEFORE INSERT ON preferences WHEN NEW.key LIKE 'thread-origin:%' BEGIN SELECT RAISE(ABORT,'injected'); END;")?;Ok(())}).await.unwrap();
    assert!(
        service
            .create_handoff(review.clone(), "draft".into())
            .await
            .is_err()
    );
    assert_eq!(service.catalog().await.unwrap().tasks.len(), 1);
    service
        .access(|store| {
            store
                .connection
                .execute_batch("DROP TRIGGER fail_handoff;")?;
            Ok(())
        })
        .await
        .unwrap();
    service
        .create_handoff(review, "explicit retry".into())
        .await
        .unwrap();
    assert_eq!(service.catalog().await.unwrap().tasks.len(), 2);
}
#[tokio::test]
async fn handoff_rechecks_workspace_and_rejects_running_or_oversized_draft() {
    let root = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = seed(&service, root.path().into()).await;
    let choice = HandoffTarget::Agent(task.agent_id.clone());
    let review = service
        .review_handoff(task.id, choice.clone())
        .await
        .unwrap();
    assert!(
        service
            .create_handoff(review.clone(), "x".repeat(MAX_DRAFT + 1))
            .await
            .is_err()
    );
    service
        .rename_project(task.project_id, "Changed authority".into())
        .await
        .unwrap();
    assert!(
        service
            .create_handoff(review, "draft".into())
            .await
            .is_err()
    );
    service
        .record(
            task.thread_id,
            ThreadEvent::PromptStarted {
                turn: "active".into(),
            },
        )
        .await
        .unwrap();
    assert!(service.review_handoff(task.id, choice).await.is_err());
}
#[tokio::test]
async fn handoff_bounds_recent_context_and_exposes_omissions() {
    let root = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = seed(&service, root.path().into()).await;
    for n in 0..140 {
        service
            .record(
                task.thread_id,
                ThreadEvent::TextDelta {
                    message_id: Some(format!("later-{n}")),
                    role: Role::Assistant,
                    text: format!("answer {n}"),
                },
            )
            .await
            .unwrap();
    }
    let review = service
        .review_handoff(task.id, HandoffTarget::Agent(task.agent_id))
        .await
        .unwrap();
    assert_eq!(review.included_messages(), 128);
    assert_eq!(review.omitted_messages(), 14);
    assert!(review.context().contains("answer 139"));
    assert!(!review.context().contains("Visible request"));
}
