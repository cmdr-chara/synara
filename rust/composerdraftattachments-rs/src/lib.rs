//! JS bindings for `apps/web/src/composerDraftAttachments.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;
pub mod utf16;

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[allow(dead_code)]
fn run<T>(f: impl FnOnce() -> Result<T, String>) -> napi::Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `pendingComposerAttachmentSyncGenerationCount`.
#[napi(js_name = "pendingComposerAttachmentSyncGenerationCount")]
pub fn pending_composer_attachment_sync_generation_count() -> napi::Result<f64> {
    run(move || {
        let out = port::pending_composer_attachment_sync_generation_count()?;
        Ok(out)
    })
}

/// `mergeComposerImages`.
#[napi(js_name = "mergeComposerImages")]
pub fn merge_composer_images(
    existing_images: String,
    incoming_images: String,
) -> napi::Result<String> {
    run(move || {
        let existing_images = crate::js::from_json(&existing_images)?;
        let incoming_images = crate::js::from_json(&incoming_images)?;
        let out = port::merge_composer_images(existing_images, incoming_images)?;
        crate::js::to_json(&out)
    })
}

/// `composerFileDedupKey`.
#[napi(js_name = "composerFileDedupKey")]
pub fn composer_file_dedup_key(file: String) -> napi::Result<String> {
    run(move || {
        let file = crate::js::from_json(&file)?;
        let out = port::composer_file_dedup_key(file)?;
        Ok(out)
    })
}

/// `revokeObjectPreviewUrl`.
#[napi(js_name = "revokeObjectPreviewUrl")]
pub fn revoke_object_preview_url(preview_url: String) -> napi::Result<()> {
    run(move || {
        port::revoke_object_preview_url(preview_url)?;
        Ok(())
    })
}

/// `revokeQueuedTurnPreviewUrls`.
#[napi(js_name = "revokeQueuedTurnPreviewUrls")]
pub fn revoke_queued_turn_preview_urls(queued_turn: String) -> napi::Result<()> {
    run(move || {
        let queued_turn = crate::js::from_json(&queued_turn)?;
        port::revoke_queued_turn_preview_urls(queued_turn)?;
        Ok(())
    })
}

/// `revokePromptHistorySavedDraftPreviewUrls`.
#[napi(js_name = "revokePromptHistorySavedDraftPreviewUrls")]
pub fn revoke_prompt_history_saved_draft_preview_urls(saved_draft: String) -> napi::Result<()> {
    run(move || {
        let saved_draft = crate::js::from_json(&saved_draft)?;
        port::revoke_prompt_history_saved_draft_preview_urls(saved_draft)?;
        Ok(())
    })
}

/// `revokeDraftPreviewUrls`.
#[napi(js_name = "revokeDraftPreviewUrls")]
pub fn revoke_draft_preview_urls(draft: String) -> napi::Result<()> {
    run(move || {
        let draft = crate::js::from_json(&draft)?;
        port::revoke_draft_preview_urls(draft)?;
        Ok(())
    })
}

/// `revokeDraftComposerImagePreviewUrls`.
#[napi(js_name = "revokeDraftComposerImagePreviewUrls")]
pub fn revoke_draft_composer_image_preview_urls(draft: String) -> napi::Result<()> {
    run(move || {
        let draft = crate::js::from_json(&draft)?;
        port::revoke_draft_composer_image_preview_urls(draft)?;
        Ok(())
    })
}

/// `isComposerImageBlobReferenced`.
#[napi(js_name = "isComposerImageBlobReferenced")]
pub fn is_composer_image_blob_referenced(
    drafts_by_thread_id: String,
    blob_key: String,
) -> napi::Result<bool> {
    run(move || {
        let drafts_by_thread_id = crate::js::from_json(&drafts_by_thread_id)?;
        let out = port::is_composer_image_blob_referenced(drafts_by_thread_id, blob_key)?;
        Ok(out)
    })
}

/// `findSupersededComposerImageBlobAttachments`.
#[napi(js_name = "findSupersededComposerImageBlobAttachments")]
pub fn find_superseded_composer_image_blob_attachments(
    previous_attachments: String,
    next_attachments: String,
) -> napi::Result<String> {
    run(move || {
        let previous_attachments = crate::js::from_json(&previous_attachments)?;
        let next_attachments = crate::js::from_json(&next_attachments)?;
        let out = port::find_superseded_composer_image_blob_attachments(
            previous_attachments,
            next_attachments,
        )?;
        crate::js::to_json(&out)
    })
}

/// `normalizePersistedAttachment`.
#[napi(js_name = "normalizePersistedAttachment")]
pub fn normalize_persisted_attachment(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_persisted_attachment(value)?;
        crate::js::to_json(&out)
    })
}

/// `toStorageSafePersistedAttachment`.
#[napi(js_name = "toStorageSafePersistedAttachment")]
pub fn to_storage_safe_persisted_attachment(attachment: String) -> napi::Result<String> {
    run(move || {
        let attachment = crate::js::from_json(&attachment)?;
        let out = port::to_storage_safe_persisted_attachment(attachment)?;
        crate::js::to_json(&out)
    })
}

/// `persistQueuedComposerImages`.
#[napi(js_name = "persistQueuedComposerImages")]
pub fn persist_queued_composer_images(images: String) -> napi::Result<String> {
    run(move || {
        let images = crate::js::from_json(&images)?;
        let out = port::persist_queued_composer_images(images)?;
        crate::js::to_json(&out)
    })
}

/// `hydrateImagesFromPersisted`.
#[napi(js_name = "hydrateImagesFromPersisted")]
pub fn hydrate_images_from_persisted(attachments: String) -> napi::Result<String> {
    run(move || {
        let attachments = crate::js::from_json(&attachments)?;
        let out = port::hydrate_images_from_persisted(attachments)?;
        crate::js::to_json(&out)
    })
}

// ---- the binary wire ------------------------------------------------------------------------

type WireBuffers = (
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint32Array,
);

thread_local! {
    static WIRE: std::cell::RefCell<Option<WireBuffers>> = const { std::cell::RefCell::new(None) };
    static PENDING: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static SCRATCH: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers the argument, result and argument-length buffers `index.cjs` uses.
#[napi(js_name = "rustAnythingWireInit")]
pub fn rust_anything_wire_init(
    input: napi::bindgen_prelude::Uint8Array,
    output: napi::bindgen_prelude::Uint8Array,
    len: napi::bindgen_prelude::Uint32Array,
) {
    WIRE.with(|w| *w.borrow_mut() = Some((input, output, len)));
}

/// Copies a result that did not fit into the (now larger) result buffer.
#[napi(js_name = "rustAnythingWireFlush")]
pub fn rust_anything_wire_flush() -> f64 {
    let bytes = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    WIRE.with(|w| {
        let mut w = w.borrow_mut();
        let Some((_, output, _)) = w.as_mut() else {
            return -1.0;
        };
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = bytes.len().min(out.len());
        out[..n].copy_from_slice(&bytes[..n]);
        n as f64
    })
}

/// Reads the argument bytes (`wLen[0]` of them), runs `f`, and writes its result; a negative
/// return is the size of a result that did not fit, kept for `rustAnythingWireFlush`.
fn wire_io(
    f: impl FnOnce(&mut wire::Reader, &mut wire::Writer) -> Result<(), String>,
) -> napi::Result<f64> {
    WIRE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some((input, output, len)) = guard.as_mut() else {
            return Err(napi::Error::from_reason(
                "rust-anything: the wire buffers are not registered",
            ));
        };
        let len = (len.first().copied().unwrap_or(0) as usize).min(input.len());
        let mut w = wire::Writer(SCRATCH.with(|s| std::mem::take(&mut *s.borrow_mut())));
        w.0.clear();
        let result = run(|| f(&mut wire::Reader::new(&input[..len]), &mut w));
        let keep = |w: wire::Writer| SCRATCH.with(|s| *s.borrow_mut() = w.0);
        match result {
            // A result of another shape than the TS type: JS runs the call through JSON.
            Err(e) if e.reason.starts_with(wire::MISMATCH) => {
                keep(w);
                return Ok(f64::NAN);
            }
            Err(e) => {
                keep(w);
                let mut m = wire::Writer::default();
                m.string16(&e.reason);
                // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
                let out = unsafe { output.as_mut() };
                if m.0.len() > out.len() {
                    return Err(e);
                }
                out[..m.0.len()].copy_from_slice(&m.0);
                return Ok(f64::INFINITY);
            }
            Ok(()) => {}
        }
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = w.0.len();
        if n > out.len() {
            PENDING.with(|p| *p.borrow_mut() = w.0);
            return Ok(-(n as f64));
        }
        out[..n].copy_from_slice(&w.0);
        keep(w);
        Ok(n as f64)
    })
}

/// `pendingComposerAttachmentSyncGenerationCount` over the binary wire.
#[napi(js_name = "pendingComposerAttachmentSyncGenerationCountWire")]
pub fn pending_composer_attachment_sync_generation_count_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::pending_composer_attachment_sync_generation_count()?;
        w.f64(*out);
        Ok(())
    })
}

static S_MERGE_COMPOSER_IMAGES_0: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("previewUrl", wire::Schema::Str16),
    ("file", wire::Schema::Dyn),
    ("source", wire::Schema::Dyn),
]));
static S_MERGE_COMPOSER_IMAGES_1: wire::Schema = wire::Schema::Dyn;
static S_MERGE_COMPOSER_IMAGES_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Obj(&[
        ("previewUrl", wire::Schema::Str16),
        ("file", wire::Schema::Dyn),
        ("source", wire::Schema::Dyn),
    ])));

/// `mergeComposerImages` over the binary wire.
#[napi(js_name = "mergeComposerImagesWire")]
pub fn merge_composer_images_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let existing_images: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_MERGE_COMPOSER_IMAGES_0.inner()))?;
        let incoming_images: serde_json::Value = wire::read_value(r, &S_MERGE_COMPOSER_IMAGES_1)?;
        let out = &port::merge_composer_images(existing_images, incoming_images)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.tag(1);
                w.count(x0.len());
                for x1 in x0.iter() {
                    wire::write_value(w, x1, S_MERGE_COMPOSER_IMAGES_OUT.inner().inner())?;
                }
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_COMPOSER_FILE_DEDUP_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `composerFileDedupKey` over the binary wire.
#[napi(js_name = "composerFileDedupKeyWire")]
pub fn composer_file_dedup_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file: serde_json::Value = wire::read_value(r, &S_COMPOSER_FILE_DEDUP_KEY_0)?;
        let out = &port::composer_file_dedup_key(file)?;
        w.string16(out);
        Ok(())
    })
}

/// `revokeObjectPreviewUrl` over the binary wire.
#[napi(js_name = "revokeObjectPreviewUrlWire")]
pub fn revoke_object_preview_url_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let preview_url: String = r.string16()?;
        let () = port::revoke_object_preview_url(preview_url)?;
        Ok(())
    })
}

static S_REVOKE_QUEUED_TURN_PREVIEW_URLS_0: wire::Schema = wire::Schema::Dyn;

/// `revokeQueuedTurnPreviewUrls` over the binary wire.
#[napi(js_name = "revokeQueuedTurnPreviewUrlsWire")]
pub fn revoke_queued_turn_preview_urls_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let queued_turn: serde_json::Value =
            wire::read_value(r, &S_REVOKE_QUEUED_TURN_PREVIEW_URLS_0)?;
        let () = port::revoke_queued_turn_preview_urls(queued_turn)?;
        Ok(())
    })
}

/// `revokePromptHistorySavedDraftPreviewUrls` over the binary wire.
#[napi(js_name = "revokePromptHistorySavedDraftPreviewUrlsWire")]
pub fn revoke_prompt_history_saved_draft_preview_urls_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let saved_draft: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let () = port::revoke_prompt_history_saved_draft_preview_urls(saved_draft)?;
        Ok(())
    })
}

/// `revokeDraftPreviewUrls` over the binary wire.
#[napi(js_name = "revokeDraftPreviewUrlsWire")]
pub fn revoke_draft_preview_urls_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let draft: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let () = port::revoke_draft_preview_urls(draft)?;
        Ok(())
    })
}

/// `revokeDraftComposerImagePreviewUrls` over the binary wire.
#[napi(js_name = "revokeDraftComposerImagePreviewUrlsWire")]
pub fn revoke_draft_composer_image_preview_urls_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let draft: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let () = port::revoke_draft_composer_image_preview_urls(draft)?;
        Ok(())
    })
}

static S_IS_COMPOSER_IMAGE_BLOB_REFERENCED_0: wire::Schema = wire::Schema::Dyn;

/// `isComposerImageBlobReferenced` over the binary wire.
#[napi(js_name = "isComposerImageBlobReferencedWire")]
pub fn is_composer_image_blob_referenced_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let drafts_by_thread_id: serde_json::Value =
            wire::read_value(r, &S_IS_COMPOSER_IMAGE_BLOB_REFERENCED_0)?;
        let blob_key: String = r.string16()?;
        let out = &port::is_composer_image_blob_referenced(drafts_by_thread_id, blob_key)?;
        w.bool(*out);
        Ok(())
    })
}

/// `findSupersededComposerImageBlobAttachments` over the binary wire.
#[napi(js_name = "findSupersededComposerImageBlobAttachmentsWire")]
pub fn find_superseded_composer_image_blob_attachments_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let previous_attachments: Vec<serde_json::Value> =
            serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!("TypeError: {e}"))?;
        let next_attachments: Vec<serde_json::Value> =
            serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!("TypeError: {e}"))?;
        let out = &port::find_superseded_composer_image_blob_attachments(
            previous_attachments,
            next_attachments,
        )?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_NORMALIZE_PERSISTED_ATTACHMENT_0: wire::Schema = wire::Schema::Dyn;

/// `normalizePersistedAttachment` over the binary wire.
#[napi(js_name = "normalizePersistedAttachmentWire")]
pub fn normalize_persisted_attachment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_NORMALIZE_PERSISTED_ATTACHMENT_0)?;
        let out = &port::normalize_persisted_attachment(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_TO_STORAGE_SAFE_PERSISTED_ATTACHMENT_0: wire::Schema = wire::Schema::Dyn;
static S_TO_STORAGE_SAFE_PERSISTED_ATTACHMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `toStorageSafePersistedAttachment` over the binary wire.
#[napi(js_name = "toStorageSafePersistedAttachmentWire")]
pub fn to_storage_safe_persisted_attachment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let attachment: serde_json::Value =
            wire::read_value(r, &S_TO_STORAGE_SAFE_PERSISTED_ATTACHMENT_0)?;
        let out = &port::to_storage_safe_persisted_attachment(attachment)?;
        wire::write_value(w, out, &S_TO_STORAGE_SAFE_PERSISTED_ATTACHMENT_OUT)?;
        Ok(())
    })
}

static S_PERSIST_QUEUED_COMPOSER_IMAGES_0: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("previewUrl", wire::Schema::Str16),
        ("file", wire::Schema::Dyn),
        ("source", wire::Schema::Dyn),
    ]));

/// `persistQueuedComposerImages` over the binary wire.
#[napi(js_name = "persistQueuedComposerImagesWire")]
pub fn persist_queued_composer_images_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let images: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_PERSIST_QUEUED_COMPOSER_IMAGES_0.inner()))?;
        let out = &port::persist_queued_composer_images(images)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_HYDRATE_IMAGES_FROM_PERSISTED_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("previewUrl", wire::Schema::Str16),
        ("file", wire::Schema::Dyn),
        ("source", wire::Schema::Dyn),
    ]));

/// `hydrateImagesFromPersisted` over the binary wire.
#[napi(js_name = "hydrateImagesFromPersistedWire")]
pub fn hydrate_images_from_persisted_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let attachments: Vec<serde_json::Value> =
            serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!("TypeError: {e}"))?;
        let out = &port::hydrate_images_from_persisted(attachments)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_HYDRATE_IMAGES_FROM_PERSISTED_OUT.inner())?;
        }
        Ok(())
    })
}
