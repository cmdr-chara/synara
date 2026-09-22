use super::*;
use synara_agent::EventSink;
use synara_core::*;
fn image(source: ImageSource) -> TranscriptImage {
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        12,
        8,
        image::Rgba([29, 112, 203, 255]),
    ))
    .write_to(&mut encoded, image::ImageFormat::Png)
    .unwrap();
    TranscriptImage {
        source,
        mime_type: "image/png".into(),
        base64: intake::base64(encoded.get_ref()),
    }
}
async fn task(service: &WorkspaceService, root: PathBuf) -> Task {
    let project = service.add_local_workspace(root).await.unwrap();
    let agent = service.profiles().await.unwrap()[0].id.clone();
    service
        .create_task(project.id, "Media test".into(), agent)
        .await
        .unwrap()
}
#[test]
fn canonical_base64_padding_and_declared_format_are_checked() {
    let original = image(ImageSource::Uploaded);
    assert!(checked(&original).is_ok());
    for data in ["AA=A", "A===", "====", "AB==", "AA==AAAA", "AA!A"] {
        let mut changed = original.clone();
        changed.base64 = data.into();
        assert!(decode_image(&changed).is_err(), "{data}");
    }
    let mut changed = original.clone();
    changed.mime_type = "image/jpeg".into();
    assert!(checked(&changed).is_err());
    changed.base64 = "AA==".into();
    assert!(checked(&changed).is_err());
    changed.mime_type = "application/pdf".into();
    assert!(!changed.bounded());
    changed = original;
    changed.base64 = "A".repeat(4 * 1024 * 1024);
    assert!(!changed.bounded());
}
#[test]
fn corrupt_truncated_animated_and_oversized_images_are_not_rendered() {
    let original = image(ImageSource::Uploaded);
    let bytes = decode_image(&original).unwrap();
    for altered in [bytes[..20].to_vec(), vec![0; 100]] {
        let mut invalid = original.clone();
        invalid.base64 = intake::base64(&altered);
        assert!(checked(&invalid).is_err());
    }
    let mut oversized = bytes.clone();
    oversized[16..20].copy_from_slice(&9000u32.to_be_bytes());
    let mut invalid = original.clone();
    invalid.base64 = intake::base64(&oversized);
    assert!(checked(&invalid).is_err());
    let mut animated = bytes;
    animated.splice(
        8..8,
        [
            0, 0, 0, 8, b'a', b'c', b'T', b'L', 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        ],
    );
    invalid.base64 = intake::base64(&animated);
    assert!(checked(&invalid).is_err());
}
#[tokio::test]
async fn durable_originals_scope_restart_export_and_recent_pruning() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.db");
    let service = WorkspaceService::open(database.clone()).await.unwrap();
    let a = task(&service, dir.path().into()).await;
    let b = service
        .create_task(a.project_id, "Other".into(), a.agent_id.clone())
        .await
        .unwrap();
    let original = image(ImageSource::Uploaded);
    let bytes = decode_image(&original).unwrap();
    let pending = service
        .add_attachments(
            a.id,
            0,
            vec![AttachmentInput::Bytes {
                name: "upload.png".into(),
                bytes: bytes.clone(),
            }],
        )
        .await
        .unwrap();
    let envelope = service
        .record(
            a.thread_id,
            ThreadEvent::ImageMessage {
                message_id: Some("uploaded".into()),
                role: Role::User,
                image: original.clone(),
            },
        )
        .await
        .unwrap();
    let acknowledged = service
        .acknowledge_attachments(a.id, vec![pending.pending[0].id.clone()])
        .await
        .unwrap();
    service
        .edit_attachments(a.id, acknowledged.revision, AttachmentEdit::ForgetRecent)
        .await
        .unwrap();
    assert!(
        service
            .transcript_image_preview(b.id, envelope.id, false)
            .await
            .is_err()
    );
    assert!(
        service
            .transcript_image_preview(a.id, EventId::new(), false)
            .await
            .is_err()
    );
    assert!(service.session(a.thread_id).await.unwrap().is_none());
    drop(service);
    let reopened = WorkspaceService::open(database).await.unwrap();
    let thread = reopened.thread(a.thread_id).await.unwrap();
    assert_eq!(thread.images.len(), 1);
    assert_eq!(thread.images[0].image, original);
    assert_eq!(thread.images[0].message_id, "uploaded");
    let preview = reopened
        .transcript_image_preview(a.id, envelope.id, true)
        .await
        .unwrap();
    assert_eq!(preview.info.source, ImageSource::Uploaded);
    assert!(preview.info.dimensions.unwrap().0 <= 1600);
    let destination = dir.path().join("saved.png");
    reopened
        .export_transcript_image(a.id, envelope.id, destination.clone())
        .await
        .unwrap();
    assert_eq!(std::fs::read(&destination).unwrap(), bytes);
    assert!(
        reopened
            .export_transcript_image(a.id, envelope.id, destination.clone())
            .await
            .is_err()
    );
    assert_eq!(std::fs::read(destination).unwrap(), bytes);
    assert!(
        reopened
            .export_transcript_image(a.id, envelope.id, PathBuf::from("relative.png"))
            .await
            .is_err()
    );
    #[cfg(unix)]
    {
        let target = dir.path().join("unrelated.txt");
        std::fs::write(&target, "keep").unwrap();
        let link = dir.path().join("link.png");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(
            reopened
                .export_transcript_image(a.id, envelope.id, link)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_to_string(target).unwrap(), "keep");
    }
    reopened
        .backup_to(
            dir.path().join("backup.db"),
            crate::RecoveryOptions::default(),
        )
        .await
        .unwrap();
}
#[tokio::test]
async fn capture_provenance_survives_reuse_without_guessing_from_names() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = task(&service, dir.path().into()).await;
    let bytes = decode_image(&image(ImageSource::Uploaded)).unwrap();
    let captured = service
        .add_attachments(
            task.id,
            0,
            vec![AttachmentInput::Capture {
                name: "arbitrary.png".into(),
                bytes: bytes.clone(),
                source: ImageSource::AppSnap,
            }],
        )
        .await
        .unwrap();
    let prompt = service
        .attached_prompt(task.id, "Look".into(), captured.revision)
        .await
        .unwrap();
    assert!(
        matches!(&prompt.parts[2],PromptPart::MediaImage(image) if image.source==ImageSource::AppSnap)
    );
    let state = service
        .acknowledge_attachments(task.id, vec![captured.pending[0].id.clone()])
        .await
        .unwrap();
    let state = service
        .edit_attachments(
            task.id,
            state.revision,
            AttachmentEdit::Reuse(state.recent[0].id.clone()),
        )
        .await
        .unwrap();
    assert_eq!(state.pending[0].source, ImageSource::AppSnap);
    assert!(
        service
            .add_attachments(
                task.id,
                state.revision,
                vec![AttachmentInput::Capture {
                    name: "fake.png".into(),
                    bytes,
                    source: ImageSource::AgentReturned
                }]
            )
            .await
            .is_err()
    );
}
#[tokio::test]
async fn malformed_provider_media_becomes_notice_and_history_continues() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = task(&service, dir.path().into()).await;
    let mut bad = image(ImageSource::AgentReturned);
    bad.base64 = "AA==".into();
    service
        .emit(
            task.thread_id,
            ThreadEvent::ImageMessage {
                message_id: None,
                role: Role::Assistant,
                image: bad,
            },
        )
        .await
        .unwrap();
    service
        .emit(
            task.thread_id,
            ThreadEvent::TextDelta {
                message_id: None,
                role: Role::Assistant,
                text: "Still usable".into(),
            },
        )
        .await
        .unwrap();
    let thread = service.thread(task.thread_id).await.unwrap();
    assert!(thread.images.is_empty());
    assert_eq!(thread.messages[0].text, "Still usable");
    assert!(thread.timeline.iter().any(
        |row| matches!(row,TranscriptItem::Notice{text,..} if text.contains("Image unavailable"))
    ));
}
#[tokio::test]
async fn image_count_boundary_preserves_previous_history_and_does_not_break_sink() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = task(&service, dir.path().into()).await;
    for _ in 0..256 {
        service
            .record(
                task.thread_id,
                ThreadEvent::ImageMessage {
                    message_id: None,
                    role: Role::Assistant,
                    image: image(ImageSource::AgentReturned),
                },
            )
            .await
            .unwrap();
    }
    service
        .emit(
            task.thread_id,
            ThreadEvent::ImageMessage {
                message_id: None,
                role: Role::Assistant,
                image: image(ImageSource::AgentReturned),
            },
        )
        .await
        .unwrap();
    let thread = service.thread(task.thread_id).await.unwrap();
    assert_eq!(thread.images.len(), 256);
    assert!(
        matches!(thread.timeline.last(),Some(TranscriptItem::Notice{text,..}) if text.contains("not retained"))
    );
}
