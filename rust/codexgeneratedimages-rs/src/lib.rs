//! JS bindings for `apps/server/src/codexGeneratedImages.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `normalizeNonEmptyString`.
#[napi(js_name = "normalizeNonEmptyString")]
pub fn normalize_non_empty_string(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_non_empty_string(value)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeCodexGeneratedImageItemType`.
#[napi(js_name = "normalizeCodexGeneratedImageItemType")]
pub fn normalize_codex_generated_image_item_type(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::normalize_codex_generated_image_item_type(raw)?;
        Ok(out)
    })
}

/// `isCodexGeneratedImageItemType`.
#[napi(js_name = "isCodexGeneratedImageItemType")]
pub fn is_codex_generated_image_item_type(raw: String) -> napi::Result<bool> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::is_codex_generated_image_item_type(raw)?;
        Ok(out)
    })
}

/// `resolveCodexHomePath`.
#[napi(js_name = "resolveCodexHomePath")]
pub fn resolve_codex_home_path(codex_home: String) -> napi::Result<String> {
    run(move || {
        let codex_home = crate::js::from_json(&codex_home)?;
        let out = port::resolve_codex_home_path(codex_home)?;
        Ok(out)
    })
}

/// `resolveCodexGeneratedImagesRoot`.
#[napi(js_name = "resolveCodexGeneratedImagesRoot")]
pub fn resolve_codex_generated_images_root(codex_home: String) -> napi::Result<String> {
    run(move || {
        let codex_home = crate::js::from_json(&codex_home)?;
        let out = port::resolve_codex_generated_images_root(codex_home)?;
        Ok(out)
    })
}

/// `codexConfiguredHomePathsFromSettings`.
#[napi(js_name = "codexConfiguredHomePathsFromSettings")]
pub fn codex_configured_home_paths_from_settings(settings: String) -> napi::Result<String> {
    run(move || {
        let settings = crate::js::from_json(&settings)?;
        let out = port::codex_configured_home_paths_from_settings(settings)?;
        crate::js::to_json(&out)
    })
}

/// `enabledCodexProviderInstanceIdsFromSettings`.
#[napi(js_name = "enabledCodexProviderInstanceIdsFromSettings")]
pub fn enabled_codex_provider_instance_ids_from_settings(settings: String) -> napi::Result<String> {
    run(move || {
        let settings = crate::js::from_json(&settings)?;
        let out = port::enabled_codex_provider_instance_ids_from_settings(settings)?;
        crate::js::to_json(&out)
    })
}

/// `resolveCodexGeneratedImagesRoots`.
#[napi(js_name = "resolveCodexGeneratedImagesRoots")]
pub fn resolve_codex_generated_images_roots(home_path: String) -> napi::Result<Vec<String>> {
    run(move || {
        let home_path = crate::js::from_json(&home_path)?;
        let out = port::resolve_codex_generated_images_roots(home_path)?;
        Ok(out)
    })
}

/// `resolveCodexGeneratedImageHomes`.
#[napi(js_name = "resolveCodexGeneratedImageHomes")]
pub fn resolve_codex_generated_image_homes(home_path: String) -> napi::Result<Vec<String>> {
    run(move || {
        let home_path = crate::js::from_json(&home_path)?;
        let out = port::resolve_codex_generated_image_homes(home_path)?;
        Ok(out)
    })
}

/// `firstStringValue`.
#[napi(js_name = "firstStringValue")]
pub fn first_string_value(record: String, keys: Vec<String>) -> napi::Result<String> {
    run(move || {
        let record = crate::js::from_json(&record)?;
        let out = port::first_string_value(record, keys)?;
        crate::js::to_json(&out)
    })
}

/// `extractCodexGeneratedImagePath`.
#[napi(js_name = "extractCodexGeneratedImagePath")]
pub fn extract_codex_generated_image_path(record: String) -> napi::Result<String> {
    run(move || {
        let record = crate::js::from_json(&record)?;
        let out = port::extract_codex_generated_image_path(record)?;
        crate::js::to_json(&out)
    })
}

/// `extractCodexGeneratedImageCallId`.
#[napi(js_name = "extractCodexGeneratedImageCallId")]
pub fn extract_codex_generated_image_call_id(record: String) -> napi::Result<String> {
    run(move || {
        let record = crate::js::from_json(&record)?;
        let out = port::extract_codex_generated_image_call_id(record)?;
        crate::js::to_json(&out)
    })
}

/// `predictedCodexGeneratedImagePath`.
#[napi(js_name = "predictedCodexGeneratedImagePath")]
pub fn predicted_codex_generated_image_path(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::predicted_codex_generated_image_path(input)?;
        crate::js::to_json(&out)
    })
}

/// `annotateCodexGeneratedImagePayload`.
#[napi(js_name = "annotateCodexGeneratedImagePayload")]
pub fn annotate_codex_generated_image_payload(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::annotate_codex_generated_image_payload(input)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizeNestedCodexGeneratedImagePayloads`.
#[napi(js_name = "sanitizeNestedCodexGeneratedImagePayloads")]
pub fn sanitize_nested_codex_generated_image_payloads(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::sanitize_nested_codex_generated_image_payloads(input)?;
        crate::js::to_json(&out)
    })
}

/// `extractCodexGeneratedImageReference`.
#[napi(js_name = "extractCodexGeneratedImageReference")]
pub fn extract_codex_generated_image_reference(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::extract_codex_generated_image_reference(input)?;
        crate::js::to_json(&out)
    })
}

/// `codexGeneratedImageArtifact`.
#[napi(js_name = "codexGeneratedImageArtifact")]
pub fn codex_generated_image_artifact(reference: String) -> napi::Result<String> {
    run(move || {
        let reference = crate::js::from_json(&reference)?;
        let out = port::codex_generated_image_artifact(reference)?;
        crate::js::to_json(&out)
    })
}

/// `isCodexGeneratedImageArtifact`.
#[napi(js_name = "isCodexGeneratedImageArtifact")]
pub fn is_codex_generated_image_artifact(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_codex_generated_image_artifact(value)?;
        Ok(out)
    })
}

/// `markdownImagePath`.
#[napi(js_name = "markdownImagePath")]
pub fn markdown_image_path(file_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::markdown_image_path(file_path)?;
        Ok(out)
    })
}

/// `generatedImageMarkdown`.
#[napi(js_name = "generatedImageMarkdown")]
pub fn generated_image_markdown(file_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::generated_image_markdown(file_path)?;
        Ok(out)
    })
}

/// `generatedImagePathFromRuntimeEvent`.
#[napi(js_name = "generatedImagePathFromRuntimeEvent")]
pub fn generated_image_path_from_runtime_event(event: String) -> napi::Result<String> {
    run(move || {
        let event = crate::js::from_json(&event)?;
        let out = port::generated_image_path_from_runtime_event(event)?;
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

static S_NORMALIZE_NON_EMPTY_STRING_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_NON_EMPTY_STRING_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `normalizeNonEmptyString` over the binary wire.
#[napi(js_name = "normalizeNonEmptyStringWire")]
pub fn normalize_non_empty_string_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NORMALIZE_NON_EMPTY_STRING_0)?;
        let out = &port::normalize_non_empty_string(value)?;
        wire::write_value(w, out, &S_NORMALIZE_NON_EMPTY_STRING_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_CODEX_GENERATED_IMAGE_ITEM_TYPE_0: wire::Schema = wire::Schema::Dyn;

/// `normalizeCodexGeneratedImageItemType` over the binary wire.
#[napi(js_name = "normalizeCodexGeneratedImageItemTypeWire")]
pub fn normalize_codex_generated_image_item_type_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: Value = wire::read_value(r, &S_NORMALIZE_CODEX_GENERATED_IMAGE_ITEM_TYPE_0)?;
        let out = &port::normalize_codex_generated_image_item_type(raw)?;
        w.string16(out);
        Ok(())
    })
}

static S_IS_CODEX_GENERATED_IMAGE_ITEM_TYPE_0: wire::Schema = wire::Schema::Dyn;

/// `isCodexGeneratedImageItemType` over the binary wire.
#[napi(js_name = "isCodexGeneratedImageItemTypeWire")]
pub fn is_codex_generated_image_item_type_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: Value = wire::read_value(r, &S_IS_CODEX_GENERATED_IMAGE_ITEM_TYPE_0)?;
        let out = &port::is_codex_generated_image_item_type(raw)?;
        w.bool(*out);
        Ok(())
    })
}

/// `resolveCodexHomePath` over the binary wire.
#[napi(js_name = "resolveCodexHomePathWire")]
pub fn resolve_codex_home_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let codex_home: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_codex_home_path(codex_home)?;
        w.string16(out);
        Ok(())
    })
}

/// `resolveCodexGeneratedImagesRoot` over the binary wire.
#[napi(js_name = "resolveCodexGeneratedImagesRootWire")]
pub fn resolve_codex_generated_images_root_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let codex_home: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_codex_generated_images_root(codex_home)?;
        w.string16(out);
        Ok(())
    })
}

static S_CODEX_CONFIGURED_HOME_PATHS_FROM_SETTINGS_0: wire::Schema = wire::Schema::Dyn;
static S_CODEX_CONFIGURED_HOME_PATHS_FROM_SETTINGS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Dyn);

/// `codexConfiguredHomePathsFromSettings` over the binary wire.
#[napi(js_name = "codexConfiguredHomePathsFromSettingsWire")]
pub fn codex_configured_home_paths_from_settings_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let settings: Value = wire::read_value(r, &S_CODEX_CONFIGURED_HOME_PATHS_FROM_SETTINGS_0)?;
        let out = &port::codex_configured_home_paths_from_settings(settings)?;
        wire::write_value(w, out, &S_CODEX_CONFIGURED_HOME_PATHS_FROM_SETTINGS_OUT)?;
        Ok(())
    })
}

static S_ENABLED_CODEX_PROVIDER_INSTANCE_IDS_FROM_SETTINGS_0: wire::Schema = wire::Schema::Dyn;
static S_ENABLED_CODEX_PROVIDER_INSTANCE_IDS_FROM_SETTINGS_OUT: wire::Schema = wire::Schema::Dyn;

/// `enabledCodexProviderInstanceIdsFromSettings` over the binary wire.
#[napi(js_name = "enabledCodexProviderInstanceIdsFromSettingsWire")]
pub fn enabled_codex_provider_instance_ids_from_settings_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let settings: Value =
            wire::read_value(r, &S_ENABLED_CODEX_PROVIDER_INSTANCE_IDS_FROM_SETTINGS_0)?;
        let out = &port::enabled_codex_provider_instance_ids_from_settings(settings)?;
        wire::write_value(
            w,
            out,
            &S_ENABLED_CODEX_PROVIDER_INSTANCE_IDS_FROM_SETTINGS_OUT,
        )?;
        Ok(())
    })
}

/// `resolveCodexGeneratedImagesRoots` over the binary wire.
#[napi(js_name = "resolveCodexGeneratedImagesRootsWire")]
pub fn resolve_codex_generated_images_roots_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let home_path: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_codex_generated_images_roots(home_path)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

/// `resolveCodexGeneratedImageHomes` over the binary wire.
#[napi(js_name = "resolveCodexGeneratedImageHomesWire")]
pub fn resolve_codex_generated_image_homes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let home_path: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_codex_generated_image_homes(home_path)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_FIRST_STRING_VALUE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `firstStringValue` over the binary wire.
#[napi(js_name = "firstStringValueWire")]
pub fn first_string_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let record: Option<Value> = wire::read_dyn_opt(r)?;
        let keys: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::first_string_value(record, keys)?;
        wire::write_value(w, out, &S_FIRST_STRING_VALUE_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_CODEX_GENERATED_IMAGE_PATH_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `extractCodexGeneratedImagePath` over the binary wire.
#[napi(js_name = "extractCodexGeneratedImagePathWire")]
pub fn extract_codex_generated_image_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let record: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::extract_codex_generated_image_path(record)?;
        wire::write_value(w, out, &S_EXTRACT_CODEX_GENERATED_IMAGE_PATH_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_CODEX_GENERATED_IMAGE_CALL_ID_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `extractCodexGeneratedImageCallId` over the binary wire.
#[napi(js_name = "extractCodexGeneratedImageCallIdWire")]
pub fn extract_codex_generated_image_call_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let record: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::extract_codex_generated_image_call_id(record)?;
        wire::write_value(w, out, &S_EXTRACT_CODEX_GENERATED_IMAGE_CALL_ID_OUT)?;
        Ok(())
    })
}

static S_PREDICTED_CODEX_GENERATED_IMAGE_PATH_0: wire::Schema = wire::Schema::Dyn;
static S_PREDICTED_CODEX_GENERATED_IMAGE_PATH_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `predictedCodexGeneratedImagePath` over the binary wire.
#[napi(js_name = "predictedCodexGeneratedImagePathWire")]
pub fn predicted_codex_generated_image_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_PREDICTED_CODEX_GENERATED_IMAGE_PATH_0)?;
        let out = &port::predicted_codex_generated_image_path(input)?;
        wire::write_value(w, out, &S_PREDICTED_CODEX_GENERATED_IMAGE_PATH_OUT)?;
        Ok(())
    })
}

static S_ANNOTATE_CODEX_GENERATED_IMAGE_PAYLOAD_0: wire::Schema = wire::Schema::Dyn;
static S_ANNOTATE_CODEX_GENERATED_IMAGE_PAYLOAD_OUT: wire::Schema = wire::Schema::Dyn;

/// `annotateCodexGeneratedImagePayload` over the binary wire.
#[napi(js_name = "annotateCodexGeneratedImagePayloadWire")]
pub fn annotate_codex_generated_image_payload_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_ANNOTATE_CODEX_GENERATED_IMAGE_PAYLOAD_0)?;
        let out = &port::annotate_codex_generated_image_payload(input)?;
        wire::write_value(w, out, &S_ANNOTATE_CODEX_GENERATED_IMAGE_PAYLOAD_OUT)?;
        Ok(())
    })
}

static S_SANITIZE_NESTED_CODEX_GENERATED_IMAGE_PAYLOADS_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_NESTED_CODEX_GENERATED_IMAGE_PAYLOADS_OUT: wire::Schema = wire::Schema::Dyn;

/// `sanitizeNestedCodexGeneratedImagePayloads` over the binary wire.
#[napi(js_name = "sanitizeNestedCodexGeneratedImagePayloadsWire")]
pub fn sanitize_nested_codex_generated_image_payloads_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value =
            wire::read_value(r, &S_SANITIZE_NESTED_CODEX_GENERATED_IMAGE_PAYLOADS_0)?;
        let out = &port::sanitize_nested_codex_generated_image_payloads(input)?;
        wire::write_value(
            w,
            out,
            &S_SANITIZE_NESTED_CODEX_GENERATED_IMAGE_PAYLOADS_OUT,
        )?;
        Ok(())
    })
}

static S_EXTRACT_CODEX_GENERATED_IMAGE_REFERENCE_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_CODEX_GENERATED_IMAGE_REFERENCE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("path", wire::Schema::Str16),
        ("callId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `extractCodexGeneratedImageReference` over the binary wire.
#[napi(js_name = "extractCodexGeneratedImageReferenceWire")]
pub fn extract_codex_generated_image_reference_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_EXTRACT_CODEX_GENERATED_IMAGE_REFERENCE_0)?;
        let out = &port::extract_codex_generated_image_reference(input)?;
        wire::write_value(w, out, &S_EXTRACT_CODEX_GENERATED_IMAGE_REFERENCE_OUT)?;
        Ok(())
    })
}

static S_CODEX_GENERATED_IMAGE_ARTIFACT_0: wire::Schema = wire::Schema::Dyn;
static S_CODEX_GENERATED_IMAGE_ARTIFACT_OUT: wire::Schema = wire::Schema::Dyn;

/// `codexGeneratedImageArtifact` over the binary wire.
#[napi(js_name = "codexGeneratedImageArtifactWire")]
pub fn codex_generated_image_artifact_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let reference: Value = wire::read_value(r, &S_CODEX_GENERATED_IMAGE_ARTIFACT_0)?;
        let out = &port::codex_generated_image_artifact(reference)?;
        wire::write_value(w, out, &S_CODEX_GENERATED_IMAGE_ARTIFACT_OUT)?;
        Ok(())
    })
}

static S_IS_CODEX_GENERATED_IMAGE_ARTIFACT_0: wire::Schema = wire::Schema::Dyn;

/// `isCodexGeneratedImageArtifact` over the binary wire.
#[napi(js_name = "isCodexGeneratedImageArtifactWire")]
pub fn is_codex_generated_image_artifact_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_IS_CODEX_GENERATED_IMAGE_ARTIFACT_0)?;
        let out = &port::is_codex_generated_image_artifact(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `markdownImagePath` over the binary wire.
#[napi(js_name = "markdownImagePathWire")]
pub fn markdown_image_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let out = &port::markdown_image_path(file_path)?;
        w.string16(out);
        Ok(())
    })
}

/// `generatedImageMarkdown` over the binary wire.
#[napi(js_name = "generatedImageMarkdownWire")]
pub fn generated_image_markdown_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let out = &port::generated_image_markdown(file_path)?;
        w.string16(out);
        Ok(())
    })
}

static S_GENERATED_IMAGE_PATH_FROM_RUNTIME_EVENT_0: wire::Schema = wire::Schema::Dyn;
static S_GENERATED_IMAGE_PATH_FROM_RUNTIME_EVENT_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `generatedImagePathFromRuntimeEvent` over the binary wire.
#[napi(js_name = "generatedImagePathFromRuntimeEventWire")]
pub fn generated_image_path_from_runtime_event_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: Value = wire::read_value(r, &S_GENERATED_IMAGE_PATH_FROM_RUNTIME_EVENT_0)?;
        let out = &port::generated_image_path_from_runtime_event(event)?;
        wire::write_value(w, out, &S_GENERATED_IMAGE_PATH_FROM_RUNTIME_EVENT_OUT)?;
        Ok(())
    })
}
