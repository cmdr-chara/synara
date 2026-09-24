use super::*;

fn text(name: &str, body: &str) -> AttachmentInput {
    AttachmentInput::Bytes {
        name: name.into(),
        bytes: body.as_bytes().to_vec(),
    }
}
fn webp_2x2() -> Vec<u8> {
    let pixels = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let mut bytes = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut bytes)
        .encode(&pixels, 2, 2, image::ExtendedColorType::Rgba8)
        .unwrap();
    bytes
}
fn animated_webp_from_still(still: &[u8]) -> Vec<u8> {
    fn chunk(out: &mut Vec<u8>, name: &[u8; 4], payload: &[u8]) {
        out.extend_from_slice(name);
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(payload);
        if payload.len() % 2 == 1 {
            out.push(0);
        }
    }

    let mut webp = b"WEBP".to_vec();
    // VP8X marks an animation canvas of 2 x 2 pixels.
    chunk(&mut webp, b"VP8X", &[0x02, 0, 0, 0, 1, 0, 0, 1, 0, 0]);
    chunk(&mut webp, b"ANIM", &[0; 6]);
    let mut frame = vec![0; 16];
    frame[6..9].copy_from_slice(&[1, 0, 0]);
    frame[9..12].copy_from_slice(&[1, 0, 0]);
    frame.extend_from_slice(&still[12..]);
    chunk(&mut webp, b"ANMF", &frame);

    let mut riff = b"RIFF".to_vec();
    riff.extend_from_slice(&(webp.len() as u32).to_le_bytes());
    riff.extend_from_slice(&webp);
    riff
}
async fn task(service: &WorkspaceService, root: PathBuf) -> Task {
    let project = service.add_local_workspace(root).await.unwrap();
    let agent = service.profiles().await.unwrap()[0].id.clone();
    service
        .create_task(project.id, "Attachment test".into(), agent)
        .await
        .unwrap()
}
#[test]
fn base64_uses_standard_padding_and_inspection_rejects_hostile_files() {
    for (source, encoded) in [
        ("", ""),
        ("f", "Zg=="),
        ("fo", "Zm8="),
        ("foo", "Zm9v"),
        ("foob", "Zm9vYg=="),
    ] {
        assert_eq!(intake::base64(source.as_bytes()), encoded);
    }
    assert_eq!(intake::base64(&[0xfb, 0xff]), "+/8=");
    for (name, bytes) in [
        ("../escape.txt", &b"hello"[..]),
        ("not-an-image.png", &b"fake"[..]),
        ("data.bin", &b"\0bad"[..]),
    ] {
        assert!(intake::inspect(name.into(), bytes).is_err());
    }
    let info = intake::inspect("notes.md".into(), "日本語\nkeep  ".as_bytes()).unwrap();
    assert_eq!(info.kind, AttachmentKind::Text);
}
#[tokio::test]
async fn snapshots_restore_atomic_imports_reject_stale_edits_and_never_start_agents() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("state.db");
    let service = WorkspaceService::open(database.clone()).await.unwrap();
    let task = task(&service, dir.path().into()).await;
    let source = dir.path().join("source.md");
    std::fs::write(&source, "original 日本語").unwrap();
    let first = service
        .add_attachments(task.id, 0, vec![AttachmentInput::File(source.clone())])
        .await
        .unwrap();
    std::fs::write(&source, "changed on disk").unwrap();
    assert_eq!(
        service
            .attachment_preview(task.id, first.pending[0].id.clone())
            .await
            .unwrap()
            .bytes,
        "original 日本語".as_bytes()
    );
    assert!(
        service
            .add_attachments(task.id, 0, vec![text("stale.txt", "no")])
            .await
            .is_err()
    );
    assert!(
        service
            .add_attachments(
                task.id,
                first.revision,
                vec![text("valid.txt", "yes"), text("invalid.pdf", "no")]
            )
            .await
            .is_err()
    );
    assert_eq!(
        service
            .attachment_draft(task.id)
            .await
            .unwrap()
            .pending
            .len(),
        1
    );
    let prompt = service
        .attached_prompt(task.id, "Inspect".into(), first.revision)
        .await
        .unwrap();
    assert!(matches!(&prompt.parts[2], PromptPart::Context {text,..} if text == "original 日本語"));
    assert!(first.unsupported(&AgentCapabilities::default()).is_some());
    assert!(service.session(task.thread_id).await.unwrap().is_none());
    assert!(
        service
            .thread(task.thread_id)
            .await
            .unwrap()
            .messages
            .is_empty()
    );
    service
        .edit_followups(
            task.id,
            0,
            crate::FollowupEdit::Add("Review the next revision".into()),
        )
        .await
        .unwrap();
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::new(2, 2))
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    let bytes = encoded.into_inner();
    let image_task = service
        .create_task(task.project_id, "Image".into(), task.agent_id.clone())
        .await
        .unwrap();
    let image = service
        .add_attachments(
            image_task.id,
            0,
            vec![AttachmentInput::Bytes {
                name: "sample.png".into(),
                bytes: bytes.clone(),
            }],
        )
        .await
        .unwrap();
    assert_eq!(image.pending[0].dimensions, Some((2, 2)));
    let image_prompt = service
        .attached_prompt(image_task.id, "Look".into(), image.revision)
        .await
        .unwrap();
    assert!(
        matches!(&image_prompt.parts[2], PromptPart::MediaImage(image) if image.base64 == intake::base64(&bytes) && image.mime_type == "image/png" && image.source == synara_core::ImageSource::Uploaded)
    );
    drop(service);
    let reopened = WorkspaceService::open(database).await.unwrap();
    assert_eq!(
        reopened.attachment_draft(task.id).await.unwrap().pending,
        first.pending
    );
    assert_eq!(
        reopened.followup_queue(task.id).await.unwrap().items[0].text,
        "Review the next revision"
    );
    let backup = dir.path().join("backup.db");
    reopened
        .backup_to(backup, crate::RecoveryOptions::default())
        .await
        .unwrap();
}

#[tokio::test]
async fn webp_is_previewable_and_prompt_conversion_is_bounded_png() {
    let dir = tempfile::tempdir().unwrap();
    let database = dir.path().join("attachments.db");
    let service = WorkspaceService::open(database.clone()).await.unwrap();
    let task = task(&service, dir.path().into()).await;
    let webp = webp_2x2();
    let draft = service
        .add_attachments(
            task.id,
            0,
            vec![AttachmentInput::Bytes {
                name: "sample.webp".into(),
                bytes: webp.clone(),
            }],
        )
        .await
        .unwrap();
    assert_eq!(draft.pending[0].kind, AttachmentKind::Webp);
    assert_eq!(draft.pending[0].kind.mime_type(), "image/webp");
    assert_eq!(draft.pending[0].dimensions, Some((2, 2)));

    drop(service);
    let service = WorkspaceService::open(database).await.unwrap();
    assert_eq!(
        service.attachment_draft(task.id).await.unwrap().pending,
        draft.pending
    );

    let preview = service
        .attachment_preview(task.id, draft.pending[0].id.clone())
        .await
        .unwrap();
    assert_eq!(preview.bytes, webp);
    assert_eq!(preview.info.kind, AttachmentKind::Webp);

    let png = intake::webp_to_png(&preview.bytes).unwrap();
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(png.len() <= MAX_ATTACHMENT_BATCH_BYTES);
    let decoded = image::load_from_memory_with_format(&png, image::ImageFormat::Png).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (2, 2));

    let prompt = service
        .attached_prompt(task.id, "Inspect this image".into(), draft.revision)
        .await
        .unwrap();
    assert!(matches!(
        &prompt.parts[2],
        PromptPart::MediaImage(image)
            if image.mime_type == "image/png"
                && image.base64 == intake::base64(&png)
                && image.bounded()
                && image.source == synara_core::ImageSource::Uploaded
    ));
    assert!(intake::bounded_png(vec![0; MAX_ATTACHMENT_BATCH_BYTES + 1]).is_err());
}

#[test]
fn animated_webp_is_rejected_before_preview_or_prompt_projection() {
    let still = webp_2x2();
    let animated = animated_webp_from_still(&still);
    assert!(
        intake::still_webp_preview(&animated)
            .unwrap_err()
            .to_string()
            .contains("Animated WebP")
    );
    let error = intake::inspect("moving.webp".into(), &animated).unwrap_err();
    assert!(error.to_string().contains("Animated WebP is unsupported"));
}

#[test]
fn projected_image_bytes_keep_the_combined_prompt_media_bound() {
    assert_eq!(
        bounded_projected_media(0, MAX_ATTACHMENT_BATCH_BYTES).unwrap(),
        MAX_ATTACHMENT_BATCH_BYTES
    );
    assert!(bounded_projected_media(MAX_ATTACHMENT_BATCH_BYTES, 1).is_err());
    assert!(bounded_projected_media(usize::MAX, 1).is_err());
}

#[tokio::test]
async fn folder_attachment_is_a_bounded_names_only_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = task(&service, dir.path().into()).await;
    let folder = dir.path().join("selected-folder");
    let nested = folder.join("nested");
    std::fs::create_dir_all(&nested).unwrap();
    for index in 0..258 {
        let name = format!("item-{index:03}.txt");
        let body = if index == 0 {
            "private source contents"
        } else {
            "ordinary file contents"
        };
        std::fs::write(folder.join(name), body).unwrap();
    }
    std::fs::write(nested.join("nested-secret.txt"), "nested contents").unwrap();

    let draft = service
        .add_attachments(task.id, 0, vec![AttachmentInput::Folder(folder.clone())])
        .await
        .unwrap();
    let info = &draft.pending[0];
    assert!(info.is_folder_snapshot());
    assert_eq!(info.kind, AttachmentKind::Text);
    assert!(draft.unsupported(&AgentCapabilities::default()).is_some());
    let preview = service
        .attachment_preview(task.id, info.id.clone())
        .await
        .unwrap();
    let snapshot = String::from_utf8(preview.bytes).unwrap();
    assert!(snapshot.contains("item-000.txt"));
    assert!(snapshot.contains("FOLDER (contents not scanned)") && snapshot.contains("nested"));
    assert!(snapshot.contains("3 additional entries omitted"));
    assert!(!snapshot.contains("private source contents"));
    assert!(!snapshot.contains("nested-secret.txt"));
    assert!(!snapshot.contains(folder.to_str().unwrap()));

    let prompt = service
        .attached_prompt(task.id, "Review this folder".into(), draft.revision)
        .await
        .unwrap();
    assert!(matches!(
        &prompt.parts[2],
        PromptPart::Context { text, .. } if text == &snapshot
    ));
}

#[tokio::test]
async fn acknowledgement_preserves_newer_files_and_future_metadata_is_never_replaced() {
    let dir = tempfile::tempdir().unwrap();
    let service = WorkspaceService::memory().unwrap();
    let task = task(&service, dir.path().into()).await;
    let first = service
        .add_attachments(task.id, 0, vec![text("a.txt", "first")])
        .await
        .unwrap();
    let second = service
        .add_attachments(task.id, first.revision, vec![text("b.txt", "next")])
        .await
        .unwrap();
    let state = service
        .acknowledge_attachments(task.id, vec![first.pending[0].id.clone()])
        .await
        .unwrap();
    assert_eq!(state.pending.len(), 1);
    assert_eq!(state.pending[0], second.pending[1]);
    assert_eq!(state.recent, first.pending);
    let reused = service
        .edit_attachments(
            task.id,
            state.revision,
            AttachmentEdit::Reuse(state.recent[0].id.clone()),
        )
        .await
        .unwrap();
    assert_eq!(reused.pending.len(), 2);
    assert_ne!(reused.pending[1].id, state.recent[0].id);
    let late = service
        .acknowledge_attachments(task.id, vec![state.recent[0].id.clone()])
        .await
        .unwrap();
    assert_eq!(late.pending, reused.pending);
    let id = task.id;
    service
        .access(move |store| {
            store
                .connection
                .execute(
                    "UPDATE preferences SET data=?1 WHERE key=?2",
                    params!["{\"version\":99}", key(id)],
                )
                .map_err(StorageError::from)?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(service.attachment_draft(id).await.is_err());
    assert!(
        service
            .edit_attachments(id, reused.revision, AttachmentEdit::ClearPending)
            .await
            .is_err()
    );
    service
        .access(move |store| {
            assert_eq!(
                store.preference_raw(&key(id))?.as_deref(),
                Some("{\"version\":99}")
            );
            Ok(())
        })
        .await
        .unwrap();
}
