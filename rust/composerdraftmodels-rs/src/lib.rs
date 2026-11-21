//! JS bindings for `apps/web/src/composerDraftModels.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `normalizeProviderKind`.
#[napi(js_name = "normalizeProviderKind")]
pub fn normalize_provider_kind(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_provider_kind(value)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeProviderInstanceId`.
#[napi(js_name = "normalizeProviderInstanceId")]
pub fn normalize_provider_instance_id(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_provider_instance_id(value)?;
        crate::js::to_json(&out)
    })
}

/// `providerInstanceModelSelectionKey`.
#[napi(js_name = "providerInstanceModelSelectionKey")]
pub fn provider_instance_model_selection_key(
    provider: String,
    instance_id: String,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let instance_id = crate::js::from_json(&instance_id)?;
        let out = port::provider_instance_model_selection_key(provider, instance_id)?;
        crate::js::to_json(&out)
    })
}

/// `modelSelectionStorageKey`.
#[napi(js_name = "modelSelectionStorageKey")]
pub fn model_selection_storage_key(selection: String) -> napi::Result<String> {
    run(move || {
        let selection = crate::js::from_json(&selection)?;
        let out = port::model_selection_storage_key(selection)?;
        crate::js::to_json(&out)
    })
}

/// `readModelSelectionForProviderInstance`.
#[napi(js_name = "readModelSelectionForProviderInstance")]
pub fn read_model_selection_for_provider_instance(
    selections: String,
    provider: String,
    instance_id: String,
) -> napi::Result<String> {
    run(move || {
        let selections = crate::js::from_json(&selections)?;
        let provider = crate::js::from_json(&provider)?;
        let instance_id = crate::js::from_json(&instance_id)?;
        let out =
            port::read_model_selection_for_provider_instance(selections, provider, instance_id)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeModelSelectionMapByInstance`.
#[napi(js_name = "normalizeModelSelectionMapByInstance")]
pub fn normalize_model_selection_map_by_instance(selections: String) -> napi::Result<String> {
    run(move || {
        let selections = crate::js::from_json(&selections)?;
        let out = port::normalize_model_selection_map_by_instance(selections)?;
        crate::js::to_json(&out)
    })
}

/// `makeModelSelection`.
#[napi(js_name = "makeModelSelection")]
pub fn make_model_selection(
    provider: String,
    model: String,
    options: String,
    supports_auto_mode: String,
    instance_id: String,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let model = crate::js::from_json(&model)?;
        let options = crate::js::from_json(&options)?;
        let supports_auto_mode = crate::js::from_json(&supports_auto_mode)?;
        let instance_id = crate::js::from_json(&instance_id)?;
        let out =
            port::make_model_selection(provider, model, options, supports_auto_mode, instance_id)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeProviderModelOptions`.
#[napi(js_name = "normalizeProviderModelOptions")]
pub fn normalize_provider_model_options(
    value: String,
    provider: String,
    legacy: String,
) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let provider = crate::js::from_json(&provider)?;
        let legacy = crate::js::from_json(&legacy)?;
        let out = port::normalize_provider_model_options(value, provider, legacy)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeModelSelection`.
#[napi(js_name = "normalizeModelSelection")]
pub fn normalize_model_selection(value: String, legacy: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let legacy = crate::js::from_json(&legacy)?;
        let out = port::normalize_model_selection(value, legacy)?;
        crate::js::to_json(&out)
    })
}

/// `reconcileProviderScopedModelSelection`.
#[napi(js_name = "reconcileProviderScopedModelSelection")]
pub fn reconcile_provider_scoped_model_selection(
    requested: String,
    current: String,
) -> napi::Result<String> {
    run(move || {
        let requested = crate::js::from_json(&requested)?;
        let current = crate::js::from_json(&current)?;
        let out = port::reconcile_provider_scoped_model_selection(requested, current)?;
        crate::js::to_json(&out)
    })
}

/// `stripNonStickyModelOptions`.
#[napi(js_name = "stripNonStickyModelOptions")]
pub fn strip_non_sticky_model_options(selection: String) -> napi::Result<String> {
    run(move || {
        let selection = crate::js::from_json(&selection)?;
        let out = port::strip_non_sticky_model_options(selection)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizeStickyModelSelectionMap`.
#[napi(js_name = "sanitizeStickyModelSelectionMap")]
pub fn sanitize_sticky_model_selection_map(map: String) -> napi::Result<String> {
    run(move || {
        let map = crate::js::from_json(&map)?;
        let out = port::sanitize_sticky_model_selection_map(map)?;
        crate::js::to_json(&out)
    })
}

/// `legacySyncModelSelectionOptions`.
#[napi(js_name = "legacySyncModelSelectionOptions")]
pub fn legacy_sync_model_selection_options(
    model_selection: String,
    model_options: String,
) -> napi::Result<String> {
    run(move || {
        let model_selection = crate::js::from_json(&model_selection)?;
        let model_options = crate::js::from_json(&model_options)?;
        let out = port::legacy_sync_model_selection_options(model_selection, model_options)?;
        crate::js::to_json(&out)
    })
}

/// `legacyMergeModelSelectionIntoProviderModelOptions`.
#[napi(js_name = "legacyMergeModelSelectionIntoProviderModelOptions")]
pub fn legacy_merge_model_selection_into_provider_model_options(
    model_selection: String,
    current_model_options: String,
) -> napi::Result<String> {
    run(move || {
        let model_selection = crate::js::from_json(&model_selection)?;
        let current_model_options = crate::js::from_json(&current_model_options)?;
        let out = port::legacy_merge_model_selection_into_provider_model_options(
            model_selection,
            current_model_options,
        )?;
        crate::js::to_json(&out)
    })
}

/// `legacyToModelSelectionByProvider`.
#[napi(js_name = "legacyToModelSelectionByProvider")]
pub fn legacy_to_model_selection_by_provider(
    model_selection: String,
    model_options: String,
) -> napi::Result<String> {
    run(move || {
        let model_selection = crate::js::from_json(&model_selection)?;
        let model_options = crate::js::from_json(&model_options)?;
        let out = port::legacy_to_model_selection_by_provider(model_selection, model_options)?;
        crate::js::to_json(&out)
    })
}

/// `deriveEffectiveComposerModelState`.
#[napi(js_name = "deriveEffectiveComposerModelState")]
pub fn derive_effective_composer_model_state(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_effective_composer_model_state(input)?;
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

static S_NORMALIZE_PROVIDER_KIND_0: wire::Schema = wire::Schema::Dyn;

/// `normalizeProviderKind` over the binary wire.
#[napi(js_name = "normalizeProviderKindWire")]
pub fn normalize_provider_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_NORMALIZE_PROVIDER_KIND_0)?;
        let out = &port::normalize_provider_kind(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_NORMALIZE_PROVIDER_INSTANCE_ID_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PROVIDER_INSTANCE_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeProviderInstanceId` over the binary wire.
#[napi(js_name = "normalizeProviderInstanceIdWire")]
pub fn normalize_provider_instance_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_NORMALIZE_PROVIDER_INSTANCE_ID_0)?;
        let out = &port::normalize_provider_instance_id(value)?;
        wire::write_value(w, out, &S_NORMALIZE_PROVIDER_INSTANCE_ID_OUT)?;
        Ok(())
    })
}

static S_PROVIDER_INSTANCE_MODEL_SELECTION_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_INSTANCE_MODEL_SELECTION_KEY_OUT: wire::Schema = wire::Schema::Dyn;

/// `providerInstanceModelSelectionKey` over the binary wire.
#[napi(js_name = "providerInstanceModelSelectionKeyWire")]
pub fn provider_instance_model_selection_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_PROVIDER_INSTANCE_MODEL_SELECTION_KEY_0)?;
        let instance_id: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::provider_instance_model_selection_key(provider, instance_id)?;
        wire::write_value(w, out, &S_PROVIDER_INSTANCE_MODEL_SELECTION_KEY_OUT)?;
        Ok(())
    })
}

static S_MODEL_SELECTION_STORAGE_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_MODEL_SELECTION_STORAGE_KEY_OUT: wire::Schema = wire::Schema::Dyn;

/// `modelSelectionStorageKey` over the binary wire.
#[napi(js_name = "modelSelectionStorageKeyWire")]
pub fn model_selection_storage_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selection: serde_json::Value = wire::read_value(r, &S_MODEL_SELECTION_STORAGE_KEY_0)?;
        let out = &port::model_selection_storage_key(selection)?;
        wire::write_value(w, out, &S_MODEL_SELECTION_STORAGE_KEY_OUT)?;
        Ok(())
    })
}

static S_READ_MODEL_SELECTION_FOR_PROVIDER_INSTANCE_1: wire::Schema = wire::Schema::Dyn;
static S_READ_MODEL_SELECTION_FOR_PROVIDER_INSTANCE_OUT: wire::Schema = wire::Schema::Dyn;

/// `readModelSelectionForProviderInstance` over the binary wire.
#[napi(js_name = "readModelSelectionForProviderInstanceWire")]
pub fn read_model_selection_for_provider_instance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selections: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let provider: serde_json::Value =
            wire::read_value(r, &S_READ_MODEL_SELECTION_FOR_PROVIDER_INSTANCE_1)?;
        let instance_id: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out =
            &port::read_model_selection_for_provider_instance(selections, provider, instance_id)?;
        wire::write_value(w, out, &S_READ_MODEL_SELECTION_FOR_PROVIDER_INSTANCE_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_MODEL_SELECTION_MAP_BY_INSTANCE_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_MODEL_SELECTION_MAP_BY_INSTANCE_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeModelSelectionMapByInstance` over the binary wire.
#[napi(js_name = "normalizeModelSelectionMapByInstanceWire")]
pub fn normalize_model_selection_map_by_instance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selections: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_MODEL_SELECTION_MAP_BY_INSTANCE_0)?;
        let out = &port::normalize_model_selection_map_by_instance(selections)?;
        wire::write_value(w, out, &S_NORMALIZE_MODEL_SELECTION_MAP_BY_INSTANCE_OUT)?;
        Ok(())
    })
}

static S_MAKE_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_MODEL_SELECTION_1: wire::Schema = wire::Schema::Dyn;
static S_MAKE_MODEL_SELECTION_3: wire::Schema = wire::Schema::Dyn;
static S_MAKE_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeModelSelection` over the binary wire.
#[napi(js_name = "makeModelSelectionWire")]
pub fn make_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_value(r, &S_MAKE_MODEL_SELECTION_0)?;
        let model: serde_json::Value = wire::read_value(r, &S_MAKE_MODEL_SELECTION_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let supports_auto_mode: serde_json::Value = wire::read_value(r, &S_MAKE_MODEL_SELECTION_3)?;
        let instance_id: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out =
            &port::make_model_selection(provider, model, options, supports_auto_mode, instance_id)?;
        wire::write_value(w, out, &S_MAKE_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_PROVIDER_MODEL_OPTIONS_0: wire::Schema = wire::Schema::Dyn;

/// `normalizeProviderModelOptions` over the binary wire.
#[napi(js_name = "normalizeProviderModelOptionsWire")]
pub fn normalize_provider_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_NORMALIZE_PROVIDER_MODEL_OPTIONS_0)?;
        let provider: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let legacy: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::normalize_provider_model_options(value, provider, legacy)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_NORMALIZE_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;

/// `normalizeModelSelection` over the binary wire.
#[napi(js_name = "normalizeModelSelectionWire")]
pub fn normalize_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_NORMALIZE_MODEL_SELECTION_0)?;
        let legacy: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::normalize_model_selection(value, legacy)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RECONCILE_PROVIDER_SCOPED_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_RECONCILE_PROVIDER_SCOPED_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `reconcileProviderScopedModelSelection` over the binary wire.
#[napi(js_name = "reconcileProviderScopedModelSelectionWire")]
pub fn reconcile_provider_scoped_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let requested: serde_json::Value =
            wire::read_value(r, &S_RECONCILE_PROVIDER_SCOPED_MODEL_SELECTION_0)?;
        let current: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::reconcile_provider_scoped_model_selection(requested, current)?;
        wire::write_value(w, out, &S_RECONCILE_PROVIDER_SCOPED_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}

static S_STRIP_NON_STICKY_MODEL_OPTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_STRIP_NON_STICKY_MODEL_OPTIONS_OUT: wire::Schema = wire::Schema::Dyn;

/// `stripNonStickyModelOptions` over the binary wire.
#[napi(js_name = "stripNonStickyModelOptionsWire")]
pub fn strip_non_sticky_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selection: serde_json::Value =
            wire::read_value(r, &S_STRIP_NON_STICKY_MODEL_OPTIONS_0)?;
        let out = &port::strip_non_sticky_model_options(selection)?;
        wire::write_value(w, out, &S_STRIP_NON_STICKY_MODEL_OPTIONS_OUT)?;
        Ok(())
    })
}

static S_SANITIZE_STICKY_MODEL_SELECTION_MAP_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_STICKY_MODEL_SELECTION_MAP_OUT: wire::Schema = wire::Schema::Dyn;

/// `sanitizeStickyModelSelectionMap` over the binary wire.
#[napi(js_name = "sanitizeStickyModelSelectionMapWire")]
pub fn sanitize_sticky_model_selection_map_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let map: serde_json::Value = wire::read_value(r, &S_SANITIZE_STICKY_MODEL_SELECTION_MAP_0)?;
        let out = &port::sanitize_sticky_model_selection_map(map)?;
        wire::write_value(w, out, &S_SANITIZE_STICKY_MODEL_SELECTION_MAP_OUT)?;
        Ok(())
    })
}

static S_LEGACY_SYNC_MODEL_SELECTION_OPTIONS_0: wire::Schema = wire::Schema::Dyn;

/// `legacySyncModelSelectionOptions` over the binary wire.
#[napi(js_name = "legacySyncModelSelectionOptionsWire")]
pub fn legacy_sync_model_selection_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_selection: serde_json::Value =
            wire::read_value(r, &S_LEGACY_SYNC_MODEL_SELECTION_OPTIONS_0)?;
        let model_options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::legacy_sync_model_selection_options(model_selection, model_options)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `legacyMergeModelSelectionIntoProviderModelOptions` over the binary wire.
#[napi(js_name = "legacyMergeModelSelectionIntoProviderModelOptionsWire")]
pub fn legacy_merge_model_selection_into_provider_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_selection: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let current_model_options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::legacy_merge_model_selection_into_provider_model_options(
            model_selection,
            current_model_options,
        )?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_LEGACY_TO_MODEL_SELECTION_BY_PROVIDER_OUT: wire::Schema = wire::Schema::Dyn;

/// `legacyToModelSelectionByProvider` over the binary wire.
#[napi(js_name = "legacyToModelSelectionByProviderWire")]
pub fn legacy_to_model_selection_by_provider_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_selection: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let model_options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::legacy_to_model_selection_by_provider(model_selection, model_options)?;
        wire::write_value(w, out, &S_LEGACY_TO_MODEL_SELECTION_BY_PROVIDER_OUT)?;
        Ok(())
    })
}

static S_DERIVE_EFFECTIVE_COMPOSER_MODEL_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_EFFECTIVE_COMPOSER_MODEL_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("selectedModel", wire::Schema::Dyn),
    ("modelOptions", wire::Schema::Dyn),
]);

/// `deriveEffectiveComposerModelState` over the binary wire.
#[napi(js_name = "deriveEffectiveComposerModelStateWire")]
pub fn derive_effective_composer_model_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_DERIVE_EFFECTIVE_COMPOSER_MODEL_STATE_0)?;
        let out = &port::derive_effective_composer_model_state(input)?;
        wire::write_value(w, out, &S_DERIVE_EFFECTIVE_COMPOSER_MODEL_STATE_OUT)?;
        Ok(())
    })
}
