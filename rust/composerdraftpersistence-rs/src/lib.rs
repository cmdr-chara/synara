//! JS bindings for `apps/web/src/composerDraftPersistence.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

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

/// `migratePersistedComposerDraftStoreState`.
#[napi(js_name = "migratePersistedComposerDraftStoreState")]
pub fn migrate_persisted_composer_draft_store_state(
    persisted_state: String,
) -> napi::Result<String> {
    run(move || {
        let persisted_state = crate::js::from_json(&persisted_state)?;
        let out = port::migrate_persisted_composer_draft_store_state(persisted_state)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeCurrentPersistedComposerDraftStoreState`.
#[napi(js_name = "normalizeCurrentPersistedComposerDraftStoreState")]
pub fn normalize_current_persisted_composer_draft_store_state(
    persisted_state: String,
) -> napi::Result<String> {
    run(move || {
        let persisted_state = crate::js::from_json(&persisted_state)?;
        let out = port::normalize_current_persisted_composer_draft_store_state(persisted_state)?;
        crate::js::to_json(&out)
    })
}

/// `toHydratedThreadDraft`.
#[napi(js_name = "toHydratedThreadDraft")]
pub fn to_hydrated_thread_draft(
    thread_id: String,
    persisted_draft: String,
) -> napi::Result<String> {
    run(move || {
        let thread_id = crate::js::from_json(&thread_id)?;
        let persisted_draft = crate::js::from_json(&persisted_draft)?;
        let out = port::to_hydrated_thread_draft(thread_id, persisted_draft)?;
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

static S_MIGRATE_PERSISTED_COMPOSER_DRAFT_STORE_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_MIGRATE_PERSISTED_COMPOSER_DRAFT_STORE_STATE_OUT: wire::Schema = wire::Schema::Dyn;

/// `migratePersistedComposerDraftStoreState` over the binary wire.
#[napi(js_name = "migratePersistedComposerDraftStoreStateWire")]
pub fn migrate_persisted_composer_draft_store_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let persisted_state: serde_json::Value =
            wire::read_value(r, &S_MIGRATE_PERSISTED_COMPOSER_DRAFT_STORE_STATE_0)?;
        let out = &port::migrate_persisted_composer_draft_store_state(persisted_state)?;
        wire::write_value(w, out, &S_MIGRATE_PERSISTED_COMPOSER_DRAFT_STORE_STATE_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_CURRENT_PERSISTED_COMPOSER_DRAFT_STORE_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_CURRENT_PERSISTED_COMPOSER_DRAFT_STORE_STATE_OUT: wire::Schema =
    wire::Schema::Dyn;

/// `normalizeCurrentPersistedComposerDraftStoreState` over the binary wire.
#[napi(js_name = "normalizeCurrentPersistedComposerDraftStoreStateWire")]
pub fn normalize_current_persisted_composer_draft_store_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let persisted_state: serde_json::Value = wire::read_value(
            r,
            &S_NORMALIZE_CURRENT_PERSISTED_COMPOSER_DRAFT_STORE_STATE_0,
        )?;
        let out = &port::normalize_current_persisted_composer_draft_store_state(persisted_state)?;
        wire::write_value(
            w,
            out,
            &S_NORMALIZE_CURRENT_PERSISTED_COMPOSER_DRAFT_STORE_STATE_OUT,
        )?;
        Ok(())
    })
}

static S_TO_HYDRATED_THREAD_DRAFT_0: wire::Schema = wire::Schema::Dyn;
static S_TO_HYDRATED_THREAD_DRAFT_1: wire::Schema = wire::Schema::Dyn;
static S_TO_HYDRATED_THREAD_DRAFT_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `toHydratedThreadDraft` over the binary wire.
#[napi(js_name = "toHydratedThreadDraftWire")]
pub fn to_hydrated_thread_draft_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread_id: serde_json::Value = wire::read_value(r, &S_TO_HYDRATED_THREAD_DRAFT_0)?;
        let persisted_draft: serde_json::Value =
            wire::read_value(r, &S_TO_HYDRATED_THREAD_DRAFT_1)?;
        let out = &port::to_hydrated_thread_draft(thread_id, persisted_draft)?;
        wire::write_value(w, out, &S_TO_HYDRATED_THREAD_DRAFT_OUT)?;
        Ok(())
    })
}
