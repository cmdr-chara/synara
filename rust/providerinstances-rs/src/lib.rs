//! JS bindings for `packages/shared/src/providerInstances.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `unresolvedAutomationInstanceId`.
#[napi(js_name = "unresolvedAutomationInstanceId")]
pub fn unresolved_automation_instance_id(provider: String) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::unresolved_automation_instance_id(provider)?;
        crate::js::to_json(&out)
    })
}

/// `isUnresolvedAutomationInstanceId`.
#[napi(js_name = "isUnresolvedAutomationInstanceId")]
pub fn is_unresolved_automation_instance_id(instance_id: Option<String>) -> napi::Result<bool> {
    run(move || {
        let out = port::is_unresolved_automation_instance_id(instance_id)?;
        Ok(out)
    })
}

/// `isProviderKind`.
#[napi(js_name = "isProviderKind")]
pub fn is_provider_kind(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_provider_kind(value)?;
        Ok(out)
    })
}

/// `defaultInstanceIdForProvider`.
#[napi(js_name = "defaultInstanceIdForProvider")]
pub fn default_instance_id_for_provider(provider: String) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::default_instance_id_for_provider(provider)?;
        crate::js::to_json(&out)
    })
}

/// `providerProfileDirectoryEnvironment`.
#[napi(js_name = "providerProfileDirectoryEnvironment")]
pub fn provider_profile_directory_environment(
    provider: String,
    profile_dir: String,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::provider_profile_directory_environment(provider, profile_dir)?;
        crate::js::to_json(&out)
    })
}

/// `providerImportedDirectoryConfig`.
#[napi(js_name = "providerImportedDirectoryConfig")]
pub fn provider_imported_directory_config(
    provider: String,
    directory: String,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::provider_imported_directory_config(provider, directory)?;
        crate::js::to_json(&out)
    })
}

/// `inferLegacyProviderKindFromInstanceId`.
#[napi(js_name = "inferLegacyProviderKindFromInstanceId")]
pub fn infer_legacy_provider_kind_from_instance_id(
    instance_id: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::infer_legacy_provider_kind_from_instance_id(instance_id)?;
        crate::js::to_json(&out)
    })
}

/// `inferLegacyProviderKindFromModel`.
#[napi(js_name = "inferLegacyProviderKindFromModel")]
pub fn infer_legacy_provider_kind_from_model(model: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::infer_legacy_provider_kind_from_model(model)?;
        crate::js::to_json(&out)
    })
}

/// `inferLegacyProviderKindFromModelSelection`.
#[napi(js_name = "inferLegacyProviderKindFromModelSelection")]
pub fn infer_legacy_provider_kind_from_model_selection(selection: String) -> napi::Result<String> {
    run(move || {
        let selection = crate::js::from_json(&selection)?;
        let out = port::infer_legacy_provider_kind_from_model_selection(selection)?;
        crate::js::to_json(&out)
    })
}

/// `resolveModelSelectionInstanceId`.
#[napi(js_name = "resolveModelSelectionInstanceId")]
pub fn resolve_model_selection_instance_id(selection: String) -> napi::Result<String> {
    run(move || {
        let selection = crate::js::from_json(&selection)?;
        let out = port::resolve_model_selection_instance_id(selection)?;
        crate::js::to_json(&out)
    })
}

/// `resolveProviderStatusInstanceId`.
#[napi(js_name = "resolveProviderStatusInstanceId")]
pub fn resolve_provider_status_instance_id(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_provider_status_instance_id(input)?;
        crate::js::to_json(&out)
    })
}

/// `codexAccountInstanceId`.
#[napi(js_name = "codexAccountInstanceId")]
pub fn codex_account_instance_id(account_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::codex_account_instance_id(account_id)?;
        crate::js::to_json(&out)
    })
}

/// `deriveProviderInstanceConfigMap`.
#[napi(js_name = "deriveProviderInstanceConfigMap")]
pub fn derive_provider_instance_config_map(settings: String) -> napi::Result<String> {
    run(move || {
        let settings = crate::js::from_json(&settings)?;
        let out = port::derive_provider_instance_config_map(settings)?;
        crate::js::to_json(&out)
    })
}

/// `deriveProviderInstances`.
#[napi(js_name = "deriveProviderInstances")]
pub fn derive_provider_instances(settings: String) -> napi::Result<String> {
    run(move || {
        let settings = crate::js::from_json(&settings)?;
        let out = port::derive_provider_instances(settings)?;
        crate::js::to_json(&out)
    })
}

/// `deriveUnsupportedProviderInstances`.
#[napi(js_name = "deriveUnsupportedProviderInstances")]
pub fn derive_unsupported_provider_instances(settings: String) -> napi::Result<String> {
    run(move || {
        let settings = crate::js::from_json(&settings)?;
        let out = port::derive_unsupported_provider_instances(settings)?;
        crate::js::to_json(&out)
    })
}

/// `resolveProviderInstance`.
#[napi(js_name = "resolveProviderInstance")]
pub fn resolve_provider_instance(settings: String, input: String) -> napi::Result<String> {
    run(move || {
        let settings = crate::js::from_json(&settings)?;
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_provider_instance(settings, input)?;
        crate::js::to_json(&out)
    })
}

/// `providerStartOptionsFromInstance`.
#[napi(js_name = "providerStartOptionsFromInstance")]
pub fn provider_start_options_from_instance(instance: String) -> napi::Result<String> {
    run(move || {
        let instance = crate::js::from_json(&instance)?;
        let out = port::provider_start_options_from_instance(instance)?;
        crate::js::to_json(&out)
    })
}

/// `mergeProviderStartOptions`.
#[napi(js_name = "mergeProviderStartOptions")]
pub fn merge_provider_start_options(base: String, overlay: String) -> napi::Result<String> {
    run(move || {
        let base = crate::js::from_json(&base)?;
        let overlay = crate::js::from_json(&overlay)?;
        let out = port::merge_provider_start_options(base, overlay)?;
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

static S_UNRESOLVED_AUTOMATION_INSTANCE_ID_0: wire::Schema = wire::Schema::Dyn;
static S_UNRESOLVED_AUTOMATION_INSTANCE_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `unresolvedAutomationInstanceId` over the binary wire.
#[napi(js_name = "unresolvedAutomationInstanceIdWire")]
pub fn unresolved_automation_instance_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_UNRESOLVED_AUTOMATION_INSTANCE_ID_0)?;
        let out = &port::unresolved_automation_instance_id(provider)?;
        wire::write_value(w, out, &S_UNRESOLVED_AUTOMATION_INSTANCE_ID_OUT)?;
        Ok(())
    })
}

/// `isUnresolvedAutomationInstanceId` over the binary wire.
#[napi(js_name = "isUnresolvedAutomationInstanceIdWire")]
pub fn is_unresolved_automation_instance_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let instance_id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_unresolved_automation_instance_id(instance_id)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_PROVIDER_KIND_0: wire::Schema = wire::Schema::Dyn;

/// `isProviderKind` over the binary wire.
#[napi(js_name = "isProviderKindWire")]
pub fn is_provider_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value = wire::read_value(r, &S_IS_PROVIDER_KIND_0)?;
        let out = &port::is_provider_kind(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_DEFAULT_INSTANCE_ID_FOR_PROVIDER_0: wire::Schema = wire::Schema::Dyn;
static S_DEFAULT_INSTANCE_ID_FOR_PROVIDER_OUT: wire::Schema = wire::Schema::Dyn;

/// `defaultInstanceIdForProvider` over the binary wire.
#[napi(js_name = "defaultInstanceIdForProviderWire")]
pub fn default_instance_id_for_provider_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_DEFAULT_INSTANCE_ID_FOR_PROVIDER_0)?;
        let out = &port::default_instance_id_for_provider(provider)?;
        wire::write_value(w, out, &S_DEFAULT_INSTANCE_ID_FOR_PROVIDER_OUT)?;
        Ok(())
    })
}

static S_PROVIDER_PROFILE_DIRECTORY_ENVIRONMENT_0: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_PROFILE_DIRECTORY_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `providerProfileDirectoryEnvironment` over the binary wire.
#[napi(js_name = "providerProfileDirectoryEnvironmentWire")]
pub fn provider_profile_directory_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_PROVIDER_PROFILE_DIRECTORY_ENVIRONMENT_0)?;
        let profile_dir: String = r.string16()?;
        let out = &port::provider_profile_directory_environment(provider, profile_dir)?;
        wire::write_value(w, out, &S_PROVIDER_PROFILE_DIRECTORY_ENVIRONMENT_OUT)?;
        Ok(())
    })
}

static S_PROVIDER_IMPORTED_DIRECTORY_CONFIG_0: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_IMPORTED_DIRECTORY_CONFIG_OUT: wire::Schema = wire::Schema::Dyn;

/// `providerImportedDirectoryConfig` over the binary wire.
#[napi(js_name = "providerImportedDirectoryConfigWire")]
pub fn provider_imported_directory_config_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_PROVIDER_IMPORTED_DIRECTORY_CONFIG_0)?;
        let directory: String = r.string16()?;
        let out = &port::provider_imported_directory_config(provider, directory)?;
        wire::write_value(w, out, &S_PROVIDER_IMPORTED_DIRECTORY_CONFIG_OUT)?;
        Ok(())
    })
}

static S_INFER_LEGACY_PROVIDER_KIND_FROM_INSTANCE_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `inferLegacyProviderKindFromInstanceId` over the binary wire.
#[napi(js_name = "inferLegacyProviderKindFromInstanceIdWire")]
pub fn infer_legacy_provider_kind_from_instance_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let instance_id: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::infer_legacy_provider_kind_from_instance_id(instance_id)?;
        wire::write_value(w, out, &S_INFER_LEGACY_PROVIDER_KIND_FROM_INSTANCE_ID_OUT)?;
        Ok(())
    })
}

static S_INFER_LEGACY_PROVIDER_KIND_FROM_MODEL_OUT: wire::Schema = wire::Schema::Dyn;

/// `inferLegacyProviderKindFromModel` over the binary wire.
#[napi(js_name = "inferLegacyProviderKindFromModelWire")]
pub fn infer_legacy_provider_kind_from_model_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::infer_legacy_provider_kind_from_model(model)?;
        wire::write_value(w, out, &S_INFER_LEGACY_PROVIDER_KIND_FROM_MODEL_OUT)?;
        Ok(())
    })
}

static S_INFER_LEGACY_PROVIDER_KIND_FROM_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `inferLegacyProviderKindFromModelSelection` over the binary wire.
#[napi(js_name = "inferLegacyProviderKindFromModelSelectionWire")]
pub fn infer_legacy_provider_kind_from_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selection: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::infer_legacy_provider_kind_from_model_selection(selection)?;
        wire::write_value(
            w,
            out,
            &S_INFER_LEGACY_PROVIDER_KIND_FROM_MODEL_SELECTION_OUT,
        )?;
        Ok(())
    })
}

static S_RESOLVE_MODEL_SELECTION_INSTANCE_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveModelSelectionInstanceId` over the binary wire.
#[napi(js_name = "resolveModelSelectionInstanceIdWire")]
pub fn resolve_model_selection_instance_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selection: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::resolve_model_selection_instance_id(selection)?;
        wire::write_value(w, out, &S_RESOLVE_MODEL_SELECTION_INSTANCE_ID_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_PROVIDER_STATUS_INSTANCE_ID_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_PROVIDER_STATUS_INSTANCE_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveProviderStatusInstanceId` over the binary wire.
#[napi(js_name = "resolveProviderStatusInstanceIdWire")]
pub fn resolve_provider_status_instance_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_PROVIDER_STATUS_INSTANCE_ID_0)?;
        let out = &port::resolve_provider_status_instance_id(input)?;
        wire::write_value(w, out, &S_RESOLVE_PROVIDER_STATUS_INSTANCE_ID_OUT)?;
        Ok(())
    })
}

static S_CODEX_ACCOUNT_INSTANCE_ID_OUT: wire::Schema = wire::Schema::Dyn;

/// `codexAccountInstanceId` over the binary wire.
#[napi(js_name = "codexAccountInstanceIdWire")]
pub fn codex_account_instance_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let account_id: String = r.string16()?;
        let out = &port::codex_account_instance_id(account_id)?;
        wire::write_value(w, out, &S_CODEX_ACCOUNT_INSTANCE_ID_OUT)?;
        Ok(())
    })
}

static S_DERIVE_PROVIDER_INSTANCE_CONFIG_MAP_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_PROVIDER_INSTANCE_CONFIG_MAP_OUT: wire::Schema = wire::Schema::Dyn;

/// `deriveProviderInstanceConfigMap` over the binary wire.
#[napi(js_name = "deriveProviderInstanceConfigMapWire")]
pub fn derive_provider_instance_config_map_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let settings: serde_json::Value =
            wire::read_value(r, &S_DERIVE_PROVIDER_INSTANCE_CONFIG_MAP_0)?;
        let out = &port::derive_provider_instance_config_map(settings)?;
        wire::write_value(w, out, &S_DERIVE_PROVIDER_INSTANCE_CONFIG_MAP_OUT)?;
        Ok(())
    })
}

static S_DERIVE_PROVIDER_INSTANCES_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_PROVIDER_INSTANCES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("instanceId", wire::Schema::Dyn),
    ("driver", wire::Schema::Dyn),
    ("displayName", wire::Schema::Str16),
    ("enabled", wire::Schema::Bool),
    ("isDefault", wire::Schema::Bool),
    ("config", wire::Schema::Map16(&wire::Schema::Dyn)),
    ("environment", wire::Schema::Dyn),
    ("raw", wire::Schema::Dyn),
]));

/// `deriveProviderInstances` over the binary wire.
#[napi(js_name = "deriveProviderInstancesWire")]
pub fn derive_provider_instances_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let settings: serde_json::Value = wire::read_value(r, &S_DERIVE_PROVIDER_INSTANCES_0)?;
        let out = &port::derive_provider_instances(settings)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_DERIVE_PROVIDER_INSTANCES_OUT.inner())?;
        }
        Ok(())
    })
}

static S_DERIVE_UNSUPPORTED_PROVIDER_INSTANCES_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_UNSUPPORTED_PROVIDER_INSTANCES_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("instanceId", wire::Schema::Dyn),
        ("driver", wire::Schema::Dyn),
        ("displayName", wire::Schema::Str16),
        ("enabled", wire::Schema::Bool),
        ("config", wire::Schema::Map16(&wire::Schema::Dyn)),
        ("environment", wire::Schema::Dyn),
        ("raw", wire::Schema::Dyn),
    ]));

/// `deriveUnsupportedProviderInstances` over the binary wire.
#[napi(js_name = "deriveUnsupportedProviderInstancesWire")]
pub fn derive_unsupported_provider_instances_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let settings: serde_json::Value =
            wire::read_value(r, &S_DERIVE_UNSUPPORTED_PROVIDER_INSTANCES_0)?;
        let out = &port::derive_unsupported_provider_instances(settings)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_DERIVE_UNSUPPORTED_PROVIDER_INSTANCES_OUT.inner())?;
        }
        Ok(())
    })
}

static S_RESOLVE_PROVIDER_INSTANCE_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_PROVIDER_INSTANCE_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_PROVIDER_INSTANCE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("instanceId", wire::Schema::Dyn),
    ("driver", wire::Schema::Dyn),
    ("displayName", wire::Schema::Str16),
    ("enabled", wire::Schema::Bool),
    ("isDefault", wire::Schema::Bool),
    ("config", wire::Schema::Map16(&wire::Schema::Dyn)),
    ("environment", wire::Schema::Dyn),
    ("raw", wire::Schema::Dyn),
]));

/// `resolveProviderInstance` over the binary wire.
#[napi(js_name = "resolveProviderInstanceWire")]
pub fn resolve_provider_instance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let settings: serde_json::Value = wire::read_value(r, &S_RESOLVE_PROVIDER_INSTANCE_0)?;
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_PROVIDER_INSTANCE_1)?;
        let out = &port::resolve_provider_instance(settings, input)?;
        wire::write_value(w, out, &S_RESOLVE_PROVIDER_INSTANCE_OUT)?;
        Ok(())
    })
}

static S_PROVIDER_START_OPTIONS_FROM_INSTANCE_0: wire::Schema = wire::Schema::Dyn;
static S_PROVIDER_START_OPTIONS_FROM_INSTANCE_OUT: wire::Schema = wire::Schema::Dyn;

/// `providerStartOptionsFromInstance` over the binary wire.
#[napi(js_name = "providerStartOptionsFromInstanceWire")]
pub fn provider_start_options_from_instance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let instance: serde_json::Value =
            wire::read_value(r, &S_PROVIDER_START_OPTIONS_FROM_INSTANCE_0)?;
        let out = &port::provider_start_options_from_instance(instance)?;
        wire::write_value(w, out, &S_PROVIDER_START_OPTIONS_FROM_INSTANCE_OUT)?;
        Ok(())
    })
}

static S_MERGE_PROVIDER_START_OPTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_MERGE_PROVIDER_START_OPTIONS_1: wire::Schema = wire::Schema::Dyn;
static S_MERGE_PROVIDER_START_OPTIONS_OUT: wire::Schema = wire::Schema::Dyn;

/// `mergeProviderStartOptions` over the binary wire.
#[napi(js_name = "mergeProviderStartOptionsWire")]
pub fn merge_provider_start_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base: serde_json::Value = wire::read_value(r, &S_MERGE_PROVIDER_START_OPTIONS_0)?;
        let overlay: serde_json::Value = wire::read_value(r, &S_MERGE_PROVIDER_START_OPTIONS_1)?;
        let out = &port::merge_provider_start_options(base, overlay)?;
        wire::write_value(w, out, &S_MERGE_PROVIDER_START_OPTIONS_OUT)?;
        Ok(())
    })
}
