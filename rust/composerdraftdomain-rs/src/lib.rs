//! JS bindings for `apps/web/src/composerDraftDomain.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `projectDraftThreadMappingKey`.
#[napi(js_name = "projectDraftThreadMappingKey")]
pub fn project_draft_thread_mapping_key(
    project_id: String,
    entry_point: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let project_id = crate::js::from_json(&project_id)?;
        let out = port::project_draft_thread_mapping_key(project_id, entry_point)?;
        crate::js::to_json(&out)
    })
}

/// `projectDraftThreadEntryPointFromKey`.
#[napi(js_name = "projectDraftThreadEntryPointFromKey")]
pub fn project_draft_thread_entry_point_from_key(key: String) -> napi::Result<String> {
    run(move || {
        let out = port::project_draft_thread_entry_point_from_key(key)?;
        Ok(out)
    })
}

/// `projectIdFromDraftThreadMappingKey`.
#[napi(js_name = "projectIdFromDraftThreadMappingKey")]
pub fn project_id_from_draft_thread_mapping_key(key: String) -> napi::Result<String> {
    run(move || {
        let out = port::project_id_from_draft_thread_mapping_key(key)?;
        crate::js::to_json(&out)
    })
}

/// `buildDraftThreadState`.
#[napi(js_name = "buildDraftThreadState")]
pub fn build_draft_thread_state(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_draft_thread_state(input)?;
        crate::js::to_json(&out)
    })
}

/// `draftThreadStatesEqual`.
#[napi(js_name = "draftThreadStatesEqual")]
pub fn draft_thread_states_equal(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::draft_thread_states_equal(left, right)?;
        Ok(out)
    })
}

/// `removeProjectDraftMappingsForThread`.
#[napi(js_name = "removeProjectDraftMappingsForThread")]
pub fn remove_project_draft_mappings_for_thread(
    project_draft_thread_id_by_project_id: String,
    thread_id: String,
) -> napi::Result<String> {
    run(move || {
        let project_draft_thread_id_by_project_id =
            crate::js::from_json(&project_draft_thread_id_by_project_id)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::remove_project_draft_mappings_for_thread(
            project_draft_thread_id_by_project_id,
            thread_id,
        )?;
        crate::js::to_json(&out)
    })
}

/// `createEmptyThreadDraft`.
#[napi(js_name = "createEmptyThreadDraft")]
pub fn create_empty_thread_draft() -> napi::Result<String> {
    run(move || {
        let out = port::create_empty_thread_draft()?;
        crate::js::to_json(&out)
    })
}

/// `terminalContextDedupKey`.
#[napi(js_name = "terminalContextDedupKey")]
pub fn terminal_context_dedup_key(context: String) -> napi::Result<String> {
    run(move || {
        let context = crate::js::from_json(&context)?;
        let out = port::terminal_context_dedup_key(context)?;
        Ok(out)
    })
}

/// `assistantSelectionDedupKey`.
#[napi(js_name = "assistantSelectionDedupKey")]
pub fn assistant_selection_dedup_key(selection: String) -> napi::Result<String> {
    run(move || {
        let selection = crate::js::from_json(&selection)?;
        let out = port::assistant_selection_dedup_key(selection)?;
        Ok(out)
    })
}

/// `normalizeAssistantSelection`.
#[napi(js_name = "normalizeAssistantSelection")]
pub fn normalize_assistant_selection(selection: String) -> napi::Result<String> {
    run(move || {
        let selection = crate::js::from_json(&selection)?;
        let out = port::normalize_assistant_selection(selection)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeAssistantSelections`.
#[napi(js_name = "normalizeAssistantSelections")]
pub fn normalize_assistant_selections(selections: String) -> napi::Result<String> {
    run(move || {
        let selections = crate::js::from_json(&selections)?;
        let out = port::normalize_assistant_selections(selections)?;
        crate::js::to_json(&out)
    })
}

/// `fileCommentDedupKey`.
#[napi(js_name = "fileCommentDedupKey")]
pub fn file_comment_dedup_key(comment: String) -> napi::Result<String> {
    run(move || {
        let comment = crate::js::from_json(&comment)?;
        let out = port::file_comment_dedup_key(comment)?;
        Ok(out)
    })
}

/// `normalizeFileComment`.
#[napi(js_name = "normalizeFileComment")]
pub fn normalize_file_comment(comment: String) -> napi::Result<String> {
    run(move || {
        let comment = crate::js::from_json(&comment)?;
        let out = port::normalize_file_comment(comment)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeFileComments`.
#[napi(js_name = "normalizeFileComments")]
pub fn normalize_file_comments(comments: String) -> napi::Result<String> {
    run(move || {
        let comments = crate::js::from_json(&comments)?;
        let out = port::normalize_file_comments(comments)?;
        crate::js::to_json(&out)
    })
}

/// `normalizePastedTexts`.
#[napi(js_name = "normalizePastedTexts")]
pub fn normalize_pasted_texts(pasted_texts: String) -> napi::Result<String> {
    run(move || {
        let pasted_texts = crate::js::from_json(&pasted_texts)?;
        let out = port::normalize_pasted_texts(pasted_texts)?;
        crate::js::to_json(&out)
    })
}

/// `hydratePastedTextsFromPersisted`.
#[napi(js_name = "hydratePastedTextsFromPersisted")]
pub fn hydrate_pasted_texts_from_persisted(persisted: String) -> napi::Result<String> {
    run(move || {
        let persisted = crate::js::from_json(&persisted)?;
        let out = port::hydrate_pasted_texts_from_persisted(persisted)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeTerminalContextForThread`.
#[napi(js_name = "normalizeTerminalContextForThread")]
pub fn normalize_terminal_context_for_thread(
    thread_id: String,
    context: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let context = crate::js::from_json(&context)?;
        let out = port::normalize_terminal_context_for_thread(thread_id, context)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeTerminalContextsForThread`.
#[napi(js_name = "normalizeTerminalContextsForThread")]
pub fn normalize_terminal_contexts_for_thread(
    thread_id: String,
    contexts: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let contexts = crate::js::from_json(&contexts)?;
        let out = port::normalize_terminal_contexts_for_thread(thread_id, contexts)?;
        crate::js::to_json(&out)
    })
}

/// `captureComposerPromptHistorySavedDraft`.
#[napi(js_name = "captureComposerPromptHistorySavedDraft")]
pub fn capture_composer_prompt_history_saved_draft(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::capture_composer_prompt_history_saved_draft(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildTransferredComposerDraft`.
#[napi(js_name = "buildTransferredComposerDraft")]
pub fn build_transferred_composer_draft(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_transferred_composer_draft(input)?;
        crate::js::to_json(&out)
    })
}

/// `cloneComposerImageAttachment`.
#[napi(js_name = "cloneComposerImageAttachment")]
pub fn clone_composer_image_attachment(image: String) -> napi::Result<String> {
    run(move || {
        let image = crate::js::from_json(&image)?;
        let out = port::clone_composer_image_attachment(image)?;
        crate::js::to_json(&out)
    })
}

/// `shouldRemoveDraft`.
#[napi(js_name = "shouldRemoveDraft")]
pub fn should_remove_draft(draft: String) -> napi::Result<bool> {
    run(move || {
        let draft = crate::js::from_json(&draft)?;
        let out = port::should_remove_draft(draft)?;
        Ok(out)
    })
}

/// `composerDraftsMatchForCleanup`.
#[napi(js_name = "composerDraftsMatchForCleanup")]
pub fn composer_drafts_match_for_cleanup(current: String, captured: String) -> napi::Result<bool> {
    run(move || {
        let current = crate::js::from_json(&current)?;
        let captured = crate::js::from_json(&captured)?;
        let out = port::composer_drafts_match_for_cleanup(current, captured)?;
        Ok(out)
    })
}

/// `composerDraftHasAttachments`.
#[napi(js_name = "composerDraftHasAttachments")]
pub fn composer_draft_has_attachments(draft: String) -> napi::Result<bool> {
    run(move || {
        let draft = crate::js::from_json(&draft)?;
        let out = port::composer_draft_has_attachments(draft)?;
        Ok(out)
    })
}

/// `composerDraftHasUnsentContent`.
#[napi(js_name = "composerDraftHasUnsentContent")]
pub fn composer_draft_has_unsent_content(draft: String) -> napi::Result<bool> {
    run(move || {
        let draft = crate::js::from_json(&draft)?;
        let out = port::composer_draft_has_unsent_content(draft)?;
        Ok(out)
    })
}

/// `composerThreadDraftIsPending`.
#[napi(js_name = "composerThreadDraftIsPending")]
pub fn composer_thread_draft_is_pending(draft: String) -> napi::Result<bool> {
    run(move || {
        let draft = crate::js::from_json(&draft)?;
        let out = port::composer_thread_draft_is_pending(draft)?;
        Ok(out)
    })
}

/// `composerThreadDraftPreviewText`.
#[napi(js_name = "composerThreadDraftPreviewText")]
pub fn composer_thread_draft_preview_text(
    draft: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let draft = crate::js::from_json(&draft)?;
        let out = port::composer_thread_draft_preview_text(draft)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `normalizeDraftThreadEntryPoint`.
#[napi(js_name = "normalizeDraftThreadEntryPoint")]
pub fn normalize_draft_thread_entry_point(
    value: String,
    fallback: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_draft_thread_entry_point(value, fallback)?;
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

static S_PROJECT_DRAFT_THREAD_MAPPING_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_PROJECT_DRAFT_THREAD_MAPPING_KEY_OUT: wire::Schema = wire::Schema::Str16;

/// `projectDraftThreadMappingKey` over the binary wire.
#[napi(js_name = "projectDraftThreadMappingKeyWire")]
pub fn project_draft_thread_mapping_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let project_id: serde_json::Value =
            wire::read_value(r, &S_PROJECT_DRAFT_THREAD_MAPPING_KEY_0)?;
        let entry_point: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::project_draft_thread_mapping_key(project_id, entry_point)?;
        wire::write_value(w, out, &S_PROJECT_DRAFT_THREAD_MAPPING_KEY_OUT)?;
        Ok(())
    })
}

/// `projectDraftThreadEntryPointFromKey` over the binary wire.
#[napi(js_name = "projectDraftThreadEntryPointFromKeyWire")]
pub fn project_draft_thread_entry_point_from_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let key: String = r.string16()?;
        let out = &port::project_draft_thread_entry_point_from_key(key)?;
        w.string16(out);
        Ok(())
    })
}

static S_PROJECT_ID_FROM_DRAFT_THREAD_MAPPING_KEY_OUT: wire::Schema = wire::Schema::Dyn;

/// `projectIdFromDraftThreadMappingKey` over the binary wire.
#[napi(js_name = "projectIdFromDraftThreadMappingKeyWire")]
pub fn project_id_from_draft_thread_mapping_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let key: String = r.string16()?;
        let out = &port::project_id_from_draft_thread_mapping_key(key)?;
        wire::write_value(w, out, &S_PROJECT_ID_FROM_DRAFT_THREAD_MAPPING_KEY_OUT)?;
        Ok(())
    })
}

static S_BUILD_DRAFT_THREAD_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_DRAFT_THREAD_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("projectId", wire::Schema::Dyn),
    ("createdAt", wire::Schema::Str16),
    ("runtimeMode", wire::Schema::Dyn),
    ("interactionMode", wire::Schema::Dyn),
    ("entryPoint", wire::Schema::Str16),
    ("branch", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("worktreePath", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "workingDirectory",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("lastKnownPr", wire::Schema::Dyn),
    ("envMode", wire::Schema::Str16),
    ("goal", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("isTemporary", wire::Schema::OptJs(&wire::Schema::Bool)),
    ("promotedTo", wire::Schema::Dyn),
]);

/// `buildDraftThreadState` over the binary wire.
#[napi(js_name = "buildDraftThreadStateWire")]
pub fn build_draft_thread_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_DRAFT_THREAD_STATE_0)?;
        let out = &port::build_draft_thread_state(input)?;
        wire::write_value(w, out, &S_BUILD_DRAFT_THREAD_STATE_OUT)?;
        Ok(())
    })
}

static S_DRAFT_THREAD_STATES_EQUAL_1: wire::Schema = wire::Schema::Dyn;

/// `draftThreadStatesEqual` over the binary wire.
#[napi(js_name = "draftThreadStatesEqualWire")]
pub fn draft_thread_states_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let right: serde_json::Value = wire::read_value(r, &S_DRAFT_THREAD_STATES_EQUAL_1)?;
        let out = &port::draft_thread_states_equal(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

static S_REMOVE_PROJECT_DRAFT_MAPPINGS_FOR_THREAD_0: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_PROJECT_DRAFT_MAPPINGS_FOR_THREAD_1: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_PROJECT_DRAFT_MAPPINGS_FOR_THREAD_OUT: wire::Schema = wire::Schema::Dyn;

/// `removeProjectDraftMappingsForThread` over the binary wire.
#[napi(js_name = "removeProjectDraftMappingsForThreadWire")]
pub fn remove_project_draft_mappings_for_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let project_draft_thread_id_by_project_id: serde_json::Value =
            wire::read_value(r, &S_REMOVE_PROJECT_DRAFT_MAPPINGS_FOR_THREAD_0)?;
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_REMOVE_PROJECT_DRAFT_MAPPINGS_FOR_THREAD_1)?;
        let out = &port::remove_project_draft_mappings_for_thread(
            project_draft_thread_id_by_project_id,
            thread_id,
        )?;
        wire::write_value(w, out, &S_REMOVE_PROJECT_DRAFT_MAPPINGS_FOR_THREAD_OUT)?;
        Ok(())
    })
}

static S_CREATE_EMPTY_THREAD_DRAFT_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "pendingUserInputDrafts",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Obj(&[
            ("request", wire::Schema::Dyn),
            ("answers", wire::Schema::Dyn),
        ]))),
    ),
    ("prompt", wire::Schema::Str16),
    (
        "promptHistorySavedDraft",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("prompt", wire::Schema::Str16),
            ("images", wire::Schema::List(&wire::Schema::Dyn)),
            ("files", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "nonPersistedImageIds",
                wire::Schema::List(&wire::Schema::Str16),
            ),
            (
                "persistedAttachments",
                wire::Schema::List(&wire::Schema::Dyn),
            ),
            (
                "assistantSelections",
                wire::Schema::List(&wire::Schema::Dyn),
            ),
            ("browserAnnotations", wire::Schema::List(&wire::Schema::Dyn)),
            ("terminalContexts", wire::Schema::List(&wire::Schema::Dyn)),
            ("fileComments", wire::Schema::List(&wire::Schema::Dyn)),
            ("pastedTexts", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "pullRequestContexts",
                wire::Schema::List(&wire::Schema::Dyn),
            ),
            ("skills", wire::Schema::List(&wire::Schema::Dyn)),
            ("mentions", wire::Schema::List(&wire::Schema::Dyn)),
        ])),
    ),
    (
        "images",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("previewUrl", wire::Schema::Str16),
            ("file", wire::Schema::Dyn),
            ("source", wire::Schema::Dyn),
        ])),
    ),
    (
        "files",
        wire::Schema::List(&wire::Schema::Obj(&[("file", wire::Schema::Dyn)])),
    ),
    (
        "nonPersistedImageIds",
        wire::Schema::List(&wire::Schema::Str16),
    ),
    (
        "persistedAttachments",
        wire::Schema::List(&wire::Schema::Dyn),
    ),
    (
        "assistantSelections",
        wire::Schema::List(&wire::Schema::Dyn),
    ),
    (
        "browserAnnotations",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("ordinal", wire::Schema::Num),
            ("tabId", wire::Schema::Str16),
            ("documentKey", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "terminalContexts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("threadId", wire::Schema::Dyn),
            ("createdAt", wire::Schema::Str16),
        ])),
    ),
    (
        "fileComments",
        wire::Schema::List(&wire::Schema::Obj(&[("id", wire::Schema::Str16)])),
    ),
    (
        "pastedTexts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("createdAt", wire::Schema::Str16),
            ("text", wire::Schema::Str16),
            ("lineCount", wire::Schema::Num),
            ("charCount", wire::Schema::Num),
        ])),
    ),
    (
        "pullRequestContexts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("createdAt", wire::Schema::Str16),
            ("scope", wire::Schema::Dyn),
            ("itemKind", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("prNumber", wire::Schema::Num),
            ("prUrl", wire::Schema::Str16),
            ("title", wire::Schema::Str16),
            ("subtitle", wire::Schema::Str16),
            ("text", wire::Schema::Str16),
        ])),
    ),
    ("skills", wire::Schema::List(&wire::Schema::Dyn)),
    ("mentions", wire::Schema::List(&wire::Schema::Dyn)),
    ("queuedTurns", wire::Schema::List(&wire::Schema::Dyn)),
    (
        "queueStoppedTurnId",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "queueResumedTurnId",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "restoredSourceProposedPlan",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("threadId", wire::Schema::Dyn),
            ("restoredPrompt", wire::Schema::Str16),
            ("sourceProposedPlan", wire::Schema::Dyn),
        ])),
    ),
    ("modelSelectionByProvider", wire::Schema::Dyn),
    ("activeProvider", wire::Schema::Dyn),
    ("providerOptionsForDispatch", wire::Schema::Dyn),
    ("runtimeMode", wire::Schema::Dyn),
    ("interactionMode", wire::Schema::Dyn),
    (
        "enableComputerControl",
        wire::Schema::OptJs(&wire::Schema::Bool),
    ),
    (
        "computerControlMode",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "computerControlGeneration",
        wire::Schema::OptJs(&wire::Schema::Num),
    ),
]);

/// `createEmptyThreadDraft` over the binary wire.
#[napi(js_name = "createEmptyThreadDraftWire")]
pub fn create_empty_thread_draft_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::create_empty_thread_draft()?;
        wire::write_value(w, out, &S_CREATE_EMPTY_THREAD_DRAFT_OUT)?;
        Ok(())
    })
}

static S_TERMINAL_CONTEXT_DEDUP_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `terminalContextDedupKey` over the binary wire.
#[napi(js_name = "terminalContextDedupKeyWire")]
pub fn terminal_context_dedup_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let context: serde_json::Value = wire::read_value(r, &S_TERMINAL_CONTEXT_DEDUP_KEY_0)?;
        let out = &port::terminal_context_dedup_key(context)?;
        w.string16(out);
        Ok(())
    })
}

static S_ASSISTANT_SELECTION_DEDUP_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `assistantSelectionDedupKey` over the binary wire.
#[napi(js_name = "assistantSelectionDedupKeyWire")]
pub fn assistant_selection_dedup_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selection: serde_json::Value = wire::read_value(r, &S_ASSISTANT_SELECTION_DEDUP_KEY_0)?;
        let out = &port::assistant_selection_dedup_key(selection)?;
        w.string16(out);
        Ok(())
    })
}

static S_NORMALIZE_ASSISTANT_SELECTION_0: wire::Schema = wire::Schema::Dyn;

/// `normalizeAssistantSelection` over the binary wire.
#[napi(js_name = "normalizeAssistantSelectionWire")]
pub fn normalize_assistant_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selection: serde_json::Value = wire::read_value(r, &S_NORMALIZE_ASSISTANT_SELECTION_0)?;
        let out = &port::normalize_assistant_selection(selection)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `normalizeAssistantSelections` over the binary wire.
#[napi(js_name = "normalizeAssistantSelectionsWire")]
pub fn normalize_assistant_selections_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selections: Vec<serde_json::Value> =
            serde_json::from_value(wire::read_dyn(r)?).map_err(|e| format!("TypeError: {e}"))?;
        let out = &port::normalize_assistant_selections(selections)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_FILE_COMMENT_DEDUP_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `fileCommentDedupKey` over the binary wire.
#[napi(js_name = "fileCommentDedupKeyWire")]
pub fn file_comment_dedup_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let comment: serde_json::Value = wire::read_value(r, &S_FILE_COMMENT_DEDUP_KEY_0)?;
        let out = &port::file_comment_dedup_key(comment)?;
        w.string16(out);
        Ok(())
    })
}

static S_NORMALIZE_FILE_COMMENT_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_FILE_COMMENT_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[("id", wire::Schema::Str16)]));

/// `normalizeFileComment` over the binary wire.
#[napi(js_name = "normalizeFileCommentWire")]
pub fn normalize_file_comment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let comment: serde_json::Value = wire::read_value(r, &S_NORMALIZE_FILE_COMMENT_0)?;
        let out = &port::normalize_file_comment(comment)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_NORMALIZE_FILE_COMMENT_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_NORMALIZE_FILE_COMMENTS_0: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[("id", wire::Schema::Str16)]));
static S_NORMALIZE_FILE_COMMENTS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[("id", wire::Schema::Str16)]));

/// `normalizeFileComments` over the binary wire.
#[napi(js_name = "normalizeFileCommentsWire")]
pub fn normalize_file_comments_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let comments: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_NORMALIZE_FILE_COMMENTS_0.inner()))?;
        let out = &port::normalize_file_comments(comments)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_NORMALIZE_FILE_COMMENTS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_NORMALIZE_PASTED_TEXTS_0: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("createdAt", wire::Schema::Str16),
    ("text", wire::Schema::Str16),
    ("lineCount", wire::Schema::Num),
    ("charCount", wire::Schema::Num),
]));
static S_NORMALIZE_PASTED_TEXTS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("createdAt", wire::Schema::Str16),
    ("text", wire::Schema::Str16),
    ("lineCount", wire::Schema::Num),
    ("charCount", wire::Schema::Num),
]));

/// `normalizePastedTexts` over the binary wire.
#[napi(js_name = "normalizePastedTextsWire")]
pub fn normalize_pasted_texts_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pasted_texts: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_NORMALIZE_PASTED_TEXTS_0.inner()))?;
        let out = &port::normalize_pasted_texts(pasted_texts)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_NORMALIZE_PASTED_TEXTS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_HYDRATE_PASTED_TEXTS_FROM_PERSISTED_0: wire::Schema = wire::Schema::Dyn;
static S_HYDRATE_PASTED_TEXTS_FROM_PERSISTED_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("id", wire::Schema::Str16),
        ("createdAt", wire::Schema::Str16),
        ("text", wire::Schema::Str16),
        ("lineCount", wire::Schema::Num),
        ("charCount", wire::Schema::Num),
    ]));

/// `hydratePastedTextsFromPersisted` over the binary wire.
#[napi(js_name = "hydratePastedTextsFromPersistedWire")]
pub fn hydrate_pasted_texts_from_persisted_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let persisted: serde_json::Value =
            wire::read_value(r, &S_HYDRATE_PASTED_TEXTS_FROM_PERSISTED_0)?;
        let out = &port::hydrate_pasted_texts_from_persisted(persisted)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_HYDRATE_PASTED_TEXTS_FROM_PERSISTED_OUT.inner())?;
        }
        Ok(())
    })
}

static S_NORMALIZE_TERMINAL_CONTEXT_FOR_THREAD_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_TERMINAL_CONTEXT_FOR_THREAD_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_TERMINAL_CONTEXT_FOR_THREAD_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("id", wire::Schema::Str16),
        ("threadId", wire::Schema::Dyn),
        ("createdAt", wire::Schema::Str16),
    ]));

/// `normalizeTerminalContextForThread` over the binary wire.
#[napi(js_name = "normalizeTerminalContextForThreadWire")]
pub fn normalize_terminal_context_for_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_TERMINAL_CONTEXT_FOR_THREAD_0)?;
        let context: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_TERMINAL_CONTEXT_FOR_THREAD_1)?;
        let out = &port::normalize_terminal_context_for_thread(thread_id, context)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_NORMALIZE_TERMINAL_CONTEXT_FOR_THREAD_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_NORMALIZE_TERMINAL_CONTEXTS_FOR_THREAD_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_TERMINAL_CONTEXTS_FOR_THREAD_1: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("id", wire::Schema::Str16),
        ("threadId", wire::Schema::Dyn),
        ("createdAt", wire::Schema::Str16),
    ]));
static S_NORMALIZE_TERMINAL_CONTEXTS_FOR_THREAD_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("id", wire::Schema::Str16),
        ("threadId", wire::Schema::Dyn),
        ("createdAt", wire::Schema::Str16),
    ]));

/// `normalizeTerminalContextsForThread` over the binary wire.
#[napi(js_name = "normalizeTerminalContextsForThreadWire")]
pub fn normalize_terminal_contexts_for_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_TERMINAL_CONTEXTS_FOR_THREAD_0)?;
        let contexts: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_NORMALIZE_TERMINAL_CONTEXTS_FOR_THREAD_1.inner()))?;
        let out = &port::normalize_terminal_contexts_for_thread(thread_id, contexts)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_NORMALIZE_TERMINAL_CONTEXTS_FOR_THREAD_OUT.inner())?;
        }
        Ok(())
    })
}

static S_CAPTURE_COMPOSER_PROMPT_HISTORY_SAVED_DRAFT_0: wire::Schema = wire::Schema::Dyn;
static S_CAPTURE_COMPOSER_PROMPT_HISTORY_SAVED_DRAFT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("prompt", wire::Schema::Str16),
    (
        "images",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("previewUrl", wire::Schema::Str16),
            ("file", wire::Schema::Dyn),
            ("source", wire::Schema::Dyn),
        ])),
    ),
    (
        "files",
        wire::Schema::List(&wire::Schema::Obj(&[("file", wire::Schema::Dyn)])),
    ),
    (
        "nonPersistedImageIds",
        wire::Schema::List(&wire::Schema::Str16),
    ),
    (
        "persistedAttachments",
        wire::Schema::List(&wire::Schema::Dyn),
    ),
    (
        "assistantSelections",
        wire::Schema::List(&wire::Schema::Dyn),
    ),
    (
        "browserAnnotations",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("ordinal", wire::Schema::Num),
            ("tabId", wire::Schema::Str16),
            ("documentKey", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "terminalContexts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("threadId", wire::Schema::Dyn),
            ("createdAt", wire::Schema::Str16),
        ])),
    ),
    (
        "fileComments",
        wire::Schema::List(&wire::Schema::Obj(&[("id", wire::Schema::Str16)])),
    ),
    (
        "pastedTexts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("createdAt", wire::Schema::Str16),
            ("text", wire::Schema::Str16),
            ("lineCount", wire::Schema::Num),
            ("charCount", wire::Schema::Num),
        ])),
    ),
    (
        "pullRequestContexts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("createdAt", wire::Schema::Str16),
            ("scope", wire::Schema::Dyn),
            ("itemKind", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("prNumber", wire::Schema::Num),
            ("prUrl", wire::Schema::Str16),
            ("title", wire::Schema::Str16),
            ("subtitle", wire::Schema::Str16),
            ("text", wire::Schema::Str16),
        ])),
    ),
    ("skills", wire::Schema::List(&wire::Schema::Dyn)),
    ("mentions", wire::Schema::List(&wire::Schema::Dyn)),
]);

/// `captureComposerPromptHistorySavedDraft` over the binary wire.
#[napi(js_name = "captureComposerPromptHistorySavedDraftWire")]
pub fn capture_composer_prompt_history_saved_draft_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_CAPTURE_COMPOSER_PROMPT_HISTORY_SAVED_DRAFT_0)?;
        let out = &port::capture_composer_prompt_history_saved_draft(input)?;
        wire::write_value(w, out, &S_CAPTURE_COMPOSER_PROMPT_HISTORY_SAVED_DRAFT_OUT)?;
        Ok(())
    })
}

static S_BUILD_TRANSFERRED_COMPOSER_DRAFT_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_TRANSFERRED_COMPOSER_DRAFT_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "pendingUserInputDrafts",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Obj(&[
            ("request", wire::Schema::Dyn),
            ("answers", wire::Schema::Dyn),
        ]))),
    ),
    ("prompt", wire::Schema::Str16),
    (
        "promptHistorySavedDraft",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("prompt", wire::Schema::Str16),
            ("images", wire::Schema::List(&wire::Schema::Dyn)),
            ("files", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "nonPersistedImageIds",
                wire::Schema::List(&wire::Schema::Str16),
            ),
            (
                "persistedAttachments",
                wire::Schema::List(&wire::Schema::Dyn),
            ),
            (
                "assistantSelections",
                wire::Schema::List(&wire::Schema::Dyn),
            ),
            ("browserAnnotations", wire::Schema::List(&wire::Schema::Dyn)),
            ("terminalContexts", wire::Schema::List(&wire::Schema::Dyn)),
            ("fileComments", wire::Schema::List(&wire::Schema::Dyn)),
            ("pastedTexts", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "pullRequestContexts",
                wire::Schema::List(&wire::Schema::Dyn),
            ),
            ("skills", wire::Schema::List(&wire::Schema::Dyn)),
            ("mentions", wire::Schema::List(&wire::Schema::Dyn)),
        ])),
    ),
    (
        "images",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("previewUrl", wire::Schema::Str16),
            ("file", wire::Schema::Dyn),
            ("source", wire::Schema::Dyn),
        ])),
    ),
    (
        "files",
        wire::Schema::List(&wire::Schema::Obj(&[("file", wire::Schema::Dyn)])),
    ),
    (
        "nonPersistedImageIds",
        wire::Schema::List(&wire::Schema::Str16),
    ),
    (
        "persistedAttachments",
        wire::Schema::List(&wire::Schema::Dyn),
    ),
    (
        "assistantSelections",
        wire::Schema::List(&wire::Schema::Dyn),
    ),
    (
        "browserAnnotations",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("ordinal", wire::Schema::Num),
            ("tabId", wire::Schema::Str16),
            ("documentKey", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    (
        "terminalContexts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("threadId", wire::Schema::Dyn),
            ("createdAt", wire::Schema::Str16),
        ])),
    ),
    (
        "fileComments",
        wire::Schema::List(&wire::Schema::Obj(&[("id", wire::Schema::Str16)])),
    ),
    (
        "pastedTexts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("createdAt", wire::Schema::Str16),
            ("text", wire::Schema::Str16),
            ("lineCount", wire::Schema::Num),
            ("charCount", wire::Schema::Num),
        ])),
    ),
    (
        "pullRequestContexts",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Str16),
            ("createdAt", wire::Schema::Str16),
            ("scope", wire::Schema::Dyn),
            ("itemKind", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("prNumber", wire::Schema::Num),
            ("prUrl", wire::Schema::Str16),
            ("title", wire::Schema::Str16),
            ("subtitle", wire::Schema::Str16),
            ("text", wire::Schema::Str16),
        ])),
    ),
    ("skills", wire::Schema::List(&wire::Schema::Dyn)),
    ("mentions", wire::Schema::List(&wire::Schema::Dyn)),
    ("queuedTurns", wire::Schema::List(&wire::Schema::Dyn)),
    (
        "queueStoppedTurnId",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "queueResumedTurnId",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "restoredSourceProposedPlan",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("threadId", wire::Schema::Dyn),
            ("restoredPrompt", wire::Schema::Str16),
            ("sourceProposedPlan", wire::Schema::Dyn),
        ])),
    ),
    ("modelSelectionByProvider", wire::Schema::Dyn),
    ("activeProvider", wire::Schema::Dyn),
    ("providerOptionsForDispatch", wire::Schema::Dyn),
    ("runtimeMode", wire::Schema::Dyn),
    ("interactionMode", wire::Schema::Dyn),
    (
        "enableComputerControl",
        wire::Schema::OptJs(&wire::Schema::Bool),
    ),
    (
        "computerControlMode",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "computerControlGeneration",
        wire::Schema::OptJs(&wire::Schema::Num),
    ),
]);

/// `buildTransferredComposerDraft` over the binary wire.
#[napi(js_name = "buildTransferredComposerDraftWire")]
pub fn build_transferred_composer_draft_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_TRANSFERRED_COMPOSER_DRAFT_0)?;
        let out = &port::build_transferred_composer_draft(input)?;
        wire::write_value(w, out, &S_BUILD_TRANSFERRED_COMPOSER_DRAFT_OUT)?;
        Ok(())
    })
}

static S_CLONE_COMPOSER_IMAGE_ATTACHMENT_0: wire::Schema = wire::Schema::Dyn;
static S_CLONE_COMPOSER_IMAGE_ATTACHMENT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("previewUrl", wire::Schema::Str16),
    ("file", wire::Schema::Dyn),
    ("source", wire::Schema::Dyn),
]);

/// `cloneComposerImageAttachment` over the binary wire.
#[napi(js_name = "cloneComposerImageAttachmentWire")]
pub fn clone_composer_image_attachment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let image: serde_json::Value = wire::read_value(r, &S_CLONE_COMPOSER_IMAGE_ATTACHMENT_0)?;
        let out = &port::clone_composer_image_attachment(image)?;
        wire::write_value(w, out, &S_CLONE_COMPOSER_IMAGE_ATTACHMENT_OUT)?;
        Ok(())
    })
}

static S_SHOULD_REMOVE_DRAFT_0: wire::Schema = wire::Schema::Dyn;

/// `shouldRemoveDraft` over the binary wire.
#[napi(js_name = "shouldRemoveDraftWire")]
pub fn should_remove_draft_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let draft: serde_json::Value = wire::read_value(r, &S_SHOULD_REMOVE_DRAFT_0)?;
        let out = &port::should_remove_draft(draft)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPOSER_DRAFTS_MATCH_FOR_CLEANUP_0: wire::Schema = wire::Schema::Dyn;
static S_COMPOSER_DRAFTS_MATCH_FOR_CLEANUP_1: wire::Schema = wire::Schema::Dyn;

/// `composerDraftsMatchForCleanup` over the binary wire.
#[napi(js_name = "composerDraftsMatchForCleanupWire")]
pub fn composer_drafts_match_for_cleanup_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let current: serde_json::Value =
            wire::read_value(r, &S_COMPOSER_DRAFTS_MATCH_FOR_CLEANUP_0)?;
        let captured: serde_json::Value =
            wire::read_value(r, &S_COMPOSER_DRAFTS_MATCH_FOR_CLEANUP_1)?;
        let out = &port::composer_drafts_match_for_cleanup(current, captured)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPOSER_DRAFT_HAS_ATTACHMENTS_0: wire::Schema = wire::Schema::Dyn;

/// `composerDraftHasAttachments` over the binary wire.
#[napi(js_name = "composerDraftHasAttachmentsWire")]
pub fn composer_draft_has_attachments_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let draft: serde_json::Value = wire::read_value(r, &S_COMPOSER_DRAFT_HAS_ATTACHMENTS_0)?;
        let out = &port::composer_draft_has_attachments(draft)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPOSER_DRAFT_HAS_UNSENT_CONTENT_0: wire::Schema = wire::Schema::Dyn;

/// `composerDraftHasUnsentContent` over the binary wire.
#[napi(js_name = "composerDraftHasUnsentContentWire")]
pub fn composer_draft_has_unsent_content_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let draft: serde_json::Value = wire::read_value(r, &S_COMPOSER_DRAFT_HAS_UNSENT_CONTENT_0)?;
        let out = &port::composer_draft_has_unsent_content(draft)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPOSER_THREAD_DRAFT_IS_PENDING_0: wire::Schema = wire::Schema::Dyn;

/// `composerThreadDraftIsPending` over the binary wire.
#[napi(js_name = "composerThreadDraftIsPendingWire")]
pub fn composer_thread_draft_is_pending_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let draft: serde_json::Value = wire::read_value(r, &S_COMPOSER_THREAD_DRAFT_IS_PENDING_0)?;
        let out = &port::composer_thread_draft_is_pending(draft)?;
        w.bool(*out);
        Ok(())
    })
}

/// `composerThreadDraftPreviewText` over the binary wire.
#[napi(js_name = "composerThreadDraftPreviewTextWire")]
pub fn composer_thread_draft_preview_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let draft: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::composer_thread_draft_preview_text(draft)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_NORMALIZE_DRAFT_THREAD_ENTRY_POINT_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_DRAFT_THREAD_ENTRY_POINT_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeDraftThreadEntryPoint` over the binary wire.
#[napi(js_name = "normalizeDraftThreadEntryPointWire")]
pub fn normalize_draft_thread_entry_point_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_DRAFT_THREAD_ENTRY_POINT_0)?;
        let fallback: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::normalize_draft_thread_entry_point(value, fallback)?;
        wire::write_value(w, out, &S_NORMALIZE_DRAFT_THREAD_ENTRY_POINT_OUT)?;
        Ok(())
    })
}
