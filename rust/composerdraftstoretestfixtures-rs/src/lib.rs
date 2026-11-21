//! JS bindings for `apps/web/src/composerDraftStoreTestFixtures.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `stubComposerDraftBrowserStorage`.
#[napi(js_name = "stubComposerDraftBrowserStorage")]
pub fn stub_composer_draft_browser_storage() -> napi::Result<()> {
    run(move || {
        port::stub_composer_draft_browser_storage()?;
        Ok(())
    })
}

/// `makeBrowserAnnotation`.
#[napi(js_name = "makeBrowserAnnotation")]
pub fn make_browser_annotation(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::make_browser_annotation(input)?;
        crate::js::to_json(&out)
    })
}

/// `makeImage`.
#[napi(js_name = "makeImage")]
pub fn make_image(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::make_image(input)?;
        crate::js::to_json(&out)
    })
}

/// `makeFile`.
#[napi(js_name = "makeFile")]
pub fn make_file(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::make_file(input)?;
        crate::js::to_json(&out)
    })
}

/// `makeTerminalContext`.
#[napi(js_name = "makeTerminalContext")]
pub fn make_terminal_context(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::make_terminal_context(input)?;
        crate::js::to_json(&out)
    })
}

/// `makeQueuedTurn`.
#[napi(js_name = "makeQueuedTurn")]
pub fn make_queued_turn(id: String) -> napi::Result<String> {
    run(move || {
        let out = port::make_queued_turn(id)?;
        crate::js::to_json(&out)
    })
}

/// `makeQueuedChatTurn`.
#[napi(js_name = "makeQueuedChatTurn")]
pub fn make_queued_chat_turn(id: String, image: String) -> napi::Result<String> {
    run(move || {
        let image = crate::js::from_json(&image)?;
        let out = port::make_queued_chat_turn(id, image)?;
        crate::js::to_json(&out)
    })
}

/// `resetComposerDraftStore`.
#[napi(js_name = "resetComposerDraftStore")]
pub fn reset_composer_draft_store() -> napi::Result<String> {
    run(move || {
        let out = port::reset_composer_draft_store()?;
        crate::js::to_json(&out)
    })
}

/// `modelSelection`.
#[napi(js_name = "modelSelection")]
pub fn model_selection(
    provider: String,
    model: String,
    options: String,
    instance_id: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let options = crate::js::from_json(&options)?;
        let out = port::model_selection(provider, model, options, instance_id)?;
        crate::js::to_json(&out)
    })
}

/// `providerModelOptions`.
#[napi(js_name = "providerModelOptions")]
pub fn provider_model_options(options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::provider_model_options(options)?;
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

/// `stubComposerDraftBrowserStorage` over the binary wire.
#[napi(js_name = "stubComposerDraftBrowserStorageWire")]
pub fn stub_composer_draft_browser_storage_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::stub_composer_draft_browser_storage()?;
        Ok(())
    })
}

static S_MAKE_BROWSER_ANNOTATION_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_BROWSER_ANNOTATION_OUT: wire::Schema = wire::Schema::Obj(&[
    ("ordinal", wire::Schema::Num),
    ("tabId", wire::Schema::Str16),
    ("documentKey", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `makeBrowserAnnotation` over the binary wire.
#[napi(js_name = "makeBrowserAnnotationWire")]
pub fn make_browser_annotation_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_MAKE_BROWSER_ANNOTATION_0)?;
        let out = &port::make_browser_annotation(input)?;
        wire::write_value(w, out, &S_MAKE_BROWSER_ANNOTATION_OUT)?;
        Ok(())
    })
}

static S_MAKE_IMAGE_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_IMAGE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("previewUrl", wire::Schema::Str16),
    ("file", wire::Schema::Dyn),
    ("source", wire::Schema::Dyn),
]);

/// `makeImage` over the binary wire.
#[napi(js_name = "makeImageWire")]
pub fn make_image_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_MAKE_IMAGE_0)?;
        let out = &port::make_image(input)?;
        wire::write_value(w, out, &S_MAKE_IMAGE_OUT)?;
        Ok(())
    })
}

static S_MAKE_FILE_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_FILE_OUT: wire::Schema = wire::Schema::Obj(&[("file", wire::Schema::Dyn)]);

/// `makeFile` over the binary wire.
#[napi(js_name = "makeFileWire")]
pub fn make_file_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_MAKE_FILE_0)?;
        let out = &port::make_file(input)?;
        wire::write_value(w, out, &S_MAKE_FILE_OUT)?;
        Ok(())
    })
}

static S_MAKE_TERMINAL_CONTEXT_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_TERMINAL_CONTEXT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("threadId", wire::Schema::Dyn),
    ("createdAt", wire::Schema::Str16),
]);

/// `makeTerminalContext` over the binary wire.
#[napi(js_name = "makeTerminalContextWire")]
pub fn make_terminal_context_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_MAKE_TERMINAL_CONTEXT_0)?;
        let out = &port::make_terminal_context(input)?;
        wire::write_value(w, out, &S_MAKE_TERMINAL_CONTEXT_OUT)?;
        Ok(())
    })
}

static S_MAKE_QUEUED_TURN_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeQueuedTurn` over the binary wire.
#[napi(js_name = "makeQueuedTurnWire")]
pub fn make_queued_turn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: String = r.string16()?;
        let out = &port::make_queued_turn(id)?;
        wire::write_value(w, out, &S_MAKE_QUEUED_TURN_OUT)?;
        Ok(())
    })
}

static S_MAKE_QUEUED_CHAT_TURN_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeQueuedChatTurn` over the binary wire.
#[napi(js_name = "makeQueuedChatTurnWire")]
pub fn make_queued_chat_turn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let id: String = r.string16()?;
        let image: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::make_queued_chat_turn(id, image)?;
        wire::write_value(w, out, &S_MAKE_QUEUED_CHAT_TURN_OUT)?;
        Ok(())
    })
}

static S_RESET_COMPOSER_DRAFT_STORE_OUT: wire::Schema = wire::Schema::Dyn;

/// `resetComposerDraftStore` over the binary wire.
#[napi(js_name = "resetComposerDraftStoreWire")]
pub fn reset_composer_draft_store_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::reset_composer_draft_store()?;
        wire::write_value(w, out, &S_RESET_COMPOSER_DRAFT_STORE_OUT)?;
        Ok(())
    })
}

static S_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `modelSelection` over the binary wire.
#[napi(js_name = "modelSelectionWire")]
pub fn model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_value(r, &S_MODEL_SELECTION_0)?;
        let model: String = r.string16()?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let instance_id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::model_selection(provider, model, options, instance_id)?;
        wire::write_value(w, out, &S_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}

static S_PROVIDER_MODEL_OPTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_MODEL_OPTIONS_OUT: wire::Schema = wire::Schema::Dyn;

/// `providerModelOptions` over the binary wire.
#[napi(js_name = "providerModelOptionsWire")]
pub fn provider_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let options: serde_json::Value = wire::read_value(r, &S_PROVIDER_MODEL_OPTIONS_0)?;
        let out = &port::provider_model_options(options)?;
        wire::write_value(w, out, &S_PROVIDER_MODEL_OPTIONS_OUT)?;
        Ok(())
    })
}
