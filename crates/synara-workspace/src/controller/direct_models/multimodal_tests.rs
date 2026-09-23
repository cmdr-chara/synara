use super::{
    tests::{configure, server, setup},
    *,
};
use crate::{AttachmentInput, AttachmentKind};
use std::io::Cursor;

fn png() -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::new_rgba8(2, 2)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}
async fn select_updated(
    controller: &Controller,
    task: &Task,
    settings: ProviderSettings,
    history_turns: Option<u16>,
) -> ProviderSettings {
    let settings = controller
        .save_direct_model_settings(settings)
        .await
        .unwrap();
    let mut selection = controller
        .workspace
        .direct_model_binding(task.id)
        .await
        .unwrap()
        .unwrap()
        .selection;
    selection.history_turns = history_turns;
    let sequence = controller
        .workspace
        .thread(task.thread_id)
        .await
        .unwrap()
        .last_sequence;
    controller
        .select_direct_model(task.id, Some(selection), settings.revision, sequence)
        .await
        .unwrap();
    settings
}

#[tokio::test]
async fn direct_images_are_sent_persisted_and_replayed_by_the_existing_owner() {
    let (root, workspace, controller, task) = setup().await;
    let (endpoint, first_server) = server(false).await;
    let mut settings = configure(&controller, &task, endpoint).await;
    settings.providers[0].models[0].capabilities.images = synara_model::Support::Supported;
    let mut settings = select_updated(&controller, &task, settings, None).await;
    let draft = workspace
        .add_attachments(
            task.id,
            0,
            vec![AttachmentInput::Capture {
                name: "window.png".into(),
                bytes: png(),
                source: ImageSource::AppSnap,
            }],
        )
        .await
        .unwrap();
    assert_eq!(draft.pending[0].kind, AttachmentKind::Png);
    controller
        .submit_with_attachments(task.id, "Describe this image".into(), draft.revision)
        .await
        .unwrap();
    let body = first_server.await.unwrap();
    let data = body["messages"][0]["content"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["type"] == "image_url")
        .unwrap()["image_url"]["url"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(data.starts_with("data:image/png;base64,"));
    let reopened = WorkspaceService::open(root.path().join("data.sqlite3"))
        .await
        .unwrap();
    let restored = reopened.thread(task.thread_id).await.unwrap();
    assert_eq!(restored.images.len(), 1);
    assert_eq!(restored.images[0].image.source, ImageSource::AppSnap);
    assert_eq!(
        format!("data:image/png;base64,{}", restored.images[0].image.base64),
        data
    );
    assert_eq!(
        restored.messages[0].text,
        draft.transcript_text("Describe this image")
    );
    workspace
        .acknowledge_attachments(
            task.id,
            draft.pending.iter().map(|a| a.id.clone()).collect(),
        )
        .await
        .unwrap();
    let (endpoint, second_server) = server(false).await;
    settings.providers[0].endpoint = endpoint;
    select_updated(&controller, &task, settings, None).await;
    controller
        .submit(task.id, "Continue using the image".into())
        .await
        .unwrap();
    let second = second_server.await.unwrap();
    assert!(second.to_string().contains(&data));
    assert_eq!(second["messages"].as_array().unwrap().len(), 3);
    assert!(workspace.session(task.thread_id).await.unwrap().is_none());
    assert_eq!(
        workspace.thread(task.thread_id).await.unwrap().images.len(),
        1
    );
}

#[tokio::test]
async fn direct_text_files_stay_visible_and_survive_later_turns() {
    let (_root, workspace, controller, task) = setup().await;
    let (endpoint, first_server) = server(false).await;
    let mut settings = configure(&controller, &task, endpoint).await;
    let draft = workspace
        .add_attachments(
            task.id,
            0,
            vec![AttachmentInput::Bytes {
                name: "notes.txt".into(),
                bytes: "Visible attachment sentinel\nSecond line"
                    .as_bytes()
                    .to_vec(),
            }],
        )
        .await
        .unwrap();
    controller
        .submit_with_attachments(task.id, "Read the selected file".into(), draft.revision)
        .await
        .unwrap();
    let first = first_server.await.unwrap();
    assert!(first.to_string().contains("Visible attachment sentinel"));
    let thread = workspace.thread(task.thread_id).await.unwrap();
    assert!(
        thread.messages[0]
            .text
            .starts_with(&draft.transcript_text("Read the selected file"))
    );
    assert!(
        thread.messages[0]
            .text
            .contains("Visible attachment sentinel\nSecond line")
    );
    let (endpoint, second_server) = server(false).await;
    settings.providers[0].endpoint = endpoint;
    select_updated(&controller, &task, settings, None).await;
    controller
        .submit(task.id, "What did the file say?".into())
        .await
        .unwrap();
    assert!(
        second_server
            .await
            .unwrap()
            .to_string()
            .contains("Visible attachment sentinel")
    );
}

#[tokio::test]
async fn unsupported_images_and_stale_attachment_reviews_do_not_record_or_consume() {
    let (_root, workspace, controller, task) = setup().await;
    configure(&controller, &task, "http://127.0.0.1:1/v1".into()).await;
    let draft = workspace
        .add_attachments(
            task.id,
            0,
            vec![AttachmentInput::Bytes {
                name: "image.png".into(),
                bytes: png(),
            }],
        )
        .await
        .unwrap();
    let error = controller
        .submit_with_attachments(task.id, "Do not send".into(), draft.revision)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("images"));
    assert_eq!(
        workspace
            .thread(task.thread_id)
            .await
            .unwrap()
            .last_sequence,
        0
    );
    assert_eq!(
        workspace
            .attachment_draft(task.id)
            .await
            .unwrap()
            .pending
            .len(),
        1
    );
    assert!(
        controller
            .submit_with_attachments(task.id, "stale".into(), 0)
            .await
            .is_err()
    );
    assert_eq!(
        workspace
            .thread(task.thread_id)
            .await
            .unwrap()
            .last_sequence,
        0
    );
    assert_eq!(
        workspace.attachment_draft(task.id).await.unwrap().revision,
        draft.revision
    );
}

#[tokio::test]
async fn cancelled_attachment_intake_never_becomes_a_delayed_send() {
    let (_root, workspace, controller, task) = setup().await;
    configure(&controller, &task, "http://127.0.0.1:1/v1".into()).await;
    let draft = workspace
        .add_attachments(
            task.id,
            0,
            vec![AttachmentInput::Bytes {
                name: "notes.txt".into(),
                bytes: b"owned context".to_vec(),
            }],
        )
        .await
        .unwrap();
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert!(
        controller
            .submit_prompt_owned(
                task.id,
                "not sent".into(),
                Some(draft.revision),
                cancellation
            )
            .await
            .is_err()
    );
    assert_eq!(
        workspace
            .thread(task.thread_id)
            .await
            .unwrap()
            .last_sequence,
        0
    );
    assert_eq!(
        workspace
            .attachment_draft(task.id)
            .await
            .unwrap()
            .pending
            .len(),
        1
    );
    assert!(workspace.session(task.thread_id).await.unwrap().is_none());
}

#[tokio::test]
async fn current_only_context_is_reviewed_persisted_and_non_destructive() {
    let (root, workspace, controller, task) = setup().await;
    let (endpoint, first_server) = server(false).await;
    let mut settings = configure(&controller, &task, endpoint).await;
    controller
        .submit(task.id, "Previous private context".into())
        .await
        .unwrap();
    first_server.await.unwrap();
    let previous = workspace.thread(task.thread_id).await.unwrap();
    let (endpoint, second_server) = server(false).await;
    settings.providers[0].endpoint = endpoint;
    select_updated(&controller, &task, settings, Some(0)).await;
    controller
        .submit(task.id, "Current message".into())
        .await
        .unwrap();
    let body = second_server.await.unwrap();
    assert_eq!(body["messages"].as_array().unwrap().len(), 1);
    assert!(!body.to_string().contains("Previous private context"));
    let restored = WorkspaceService::open(root.path().join("data.sqlite3"))
        .await
        .unwrap();
    assert_eq!(
        restored
            .direct_model_binding(task.id)
            .await
            .unwrap()
            .unwrap()
            .selection
            .history_turns,
        Some(0)
    );
    let thread = restored.thread(task.thread_id).await.unwrap();
    assert!(
        thread
            .messages
            .iter()
            .any(|m| m.text == "Previous private context")
    );
    assert!(thread.last_sequence > previous.last_sequence);
}

#[tokio::test]
async fn over_budget_history_fails_before_any_prompt_event() {
    let (_root, workspace, controller, task) = setup().await;
    configure(&controller, &task, "http://127.0.0.1:1/v1".into()).await;
    workspace
        .record(
            task.thread_id,
            ThreadEvent::TextDelta {
                message_id: Some("prior".into()),
                role: Role::User,
                text: "a".repeat(3 * 1024 * 1024),
            },
        )
        .await
        .unwrap();
    let sequence = workspace
        .thread(task.thread_id)
        .await
        .unwrap()
        .last_sequence;
    let error = controller
        .submit(task.id, "b".repeat(1024 * 1024))
        .await
        .unwrap_err();
    assert!(error.to_string().contains("limit"));
    assert_eq!(
        workspace
            .thread(task.thread_id)
            .await
            .unwrap()
            .last_sequence,
        sequence
    );
    assert!(workspace.session(task.thread_id).await.unwrap().is_none());
}
