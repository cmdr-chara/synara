//! JS bindings for `apps/web/src/providerModelOptions.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `getOmpModelSelectionIssue`.
#[napi(js_name = "getOmpModelSelectionIssue")]
pub fn get_omp_model_selection_issue(model: String, options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::get_omp_model_selection_issue(model, options)?;
        crate::js::to_json(&out)
    })
}

/// `providerModelOptionProvenanceLabel`.
#[napi(js_name = "providerModelOptionProvenanceLabel")]
pub fn provider_model_option_provenance_label(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::provider_model_option_provenance_label(input)?;
        Ok(out)
    })
}

/// `formatProviderModelOptionName`.
#[napi(js_name = "formatProviderModelOptionName")]
pub fn format_provider_model_option_name(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::format_provider_model_option_name(input)?;
        Ok(out)
    })
}

/// `normalizeClaudeModelOptionSlug`.
#[napi(js_name = "normalizeClaudeModelOptionSlug")]
pub fn normalize_claude_model_option_slug(model: String) -> napi::Result<String> {
    run(move || {
        let model = crate::js::from_json(&model)?;
        let out = port::normalize_claude_model_option_slug(model)?;
        Ok(out)
    })
}

/// `mergeDynamicModelOptions`.
#[napi(js_name = "mergeDynamicModelOptions")]
pub fn merge_dynamic_model_options(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::merge_dynamic_model_options(input)?;
        crate::js::to_json(&out)
    })
}

/// `providerModelCostMultiplierLabel`.
#[napi(js_name = "providerModelCostMultiplierLabel")]
pub fn provider_model_cost_multiplier_label(
    description: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::provider_model_cost_multiplier_label(description)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `groupProviderModelOptions`.
#[napi(js_name = "groupProviderModelOptions")]
pub fn group_provider_model_options(options: String) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::group_provider_model_options(options)?;
        crate::js::to_json(&out)
    })
}

/// `groupProviderModelOptionsWithFavorites`.
#[napi(js_name = "groupProviderModelOptionsWithFavorites")]
pub fn group_provider_model_options_with_favorites(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::group_provider_model_options_with_favorites(input)?;
        crate::js::to_json(&out)
    })
}

/// `shouldUseCollapsibleModelGroups`.
#[napi(js_name = "shouldUseCollapsibleModelGroups")]
pub fn should_use_collapsible_model_groups(
    group_count: f64,
    is_searching: bool,
) -> napi::Result<bool> {
    run(move || {
        let out = port::should_use_collapsible_model_groups(group_count, is_searching)?;
        Ok(out)
    })
}

/// `resolveModelGroupDefaultOpen`.
#[napi(js_name = "resolveModelGroupDefaultOpen")]
pub fn resolve_model_group_default_open(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_model_group_default_open(input)?;
        Ok(out)
    })
}

/// `buildNextProviderOptions`.
#[napi(js_name = "buildNextProviderOptions")]
pub fn build_next_provider_options(
    provider: String,
    model_options: String,
    patch: String,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let model_options = crate::js::from_json(&model_options)?;
        let patch = crate::js::from_json(&patch)?;
        let out = port::build_next_provider_options(provider, model_options, patch)?;
        crate::js::to_json(&out)
    })
}

/// `buildProviderOptionPatch`.
#[napi(js_name = "buildProviderOptionPatch")]
pub fn build_provider_option_patch(
    provider: String,
    option_id: String,
    value: String,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let value = crate::js::from_json(&value)?;
        let out = port::build_provider_option_patch(provider, option_id, value)?;
        crate::js::to_json(&out)
    })
}

/// `buildModelSelection`.
#[napi(js_name = "buildModelSelection")]
pub fn build_model_selection(
    provider: String,
    model: String,
    options: String,
    supports_auto_mode_or_metadata: String,
    explicit_metadata: String,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let options = crate::js::from_json(&options)?;
        let supports_auto_mode_or_metadata = crate::js::from_json(&supports_auto_mode_or_metadata)?;
        let explicit_metadata = crate::js::from_json(&explicit_metadata)?;
        let out = port::build_model_selection(
            provider,
            model,
            options,
            supports_auto_mode_or_metadata,
            explicit_metadata,
        )?;
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

static S_GET_OMP_MODEL_SELECTION_ISSUE_1: wire::Schema = wire::Schema::Dyn;
static S_GET_OMP_MODEL_SELECTION_ISSUE_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `getOmpModelSelectionIssue` over the binary wire.
#[napi(js_name = "getOmpModelSelectionIssueWire")]
pub fn get_omp_model_selection_issue_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: String = r.string16()?;
        let options: Value = wire::read_value(r, &S_GET_OMP_MODEL_SELECTION_ISSUE_1)?;
        let out = &port::get_omp_model_selection_issue(model, options)?;
        wire::write_value(w, out, &S_GET_OMP_MODEL_SELECTION_ISSUE_OUT)?;
        Ok(())
    })
}

static S_PROVIDER_MODEL_OPTION_PROVENANCE_LABEL_0: wire::Schema = wire::Schema::Dyn;

/// `providerModelOptionProvenanceLabel` over the binary wire.
#[napi(js_name = "providerModelOptionProvenanceLabelWire")]
pub fn provider_model_option_provenance_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_PROVIDER_MODEL_OPTION_PROVENANCE_LABEL_0)?;
        let out = &port::provider_model_option_provenance_label(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_FORMAT_PROVIDER_MODEL_OPTION_NAME_0: wire::Schema = wire::Schema::Dyn;

/// `formatProviderModelOptionName` over the binary wire.
#[napi(js_name = "formatProviderModelOptionNameWire")]
pub fn format_provider_model_option_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_FORMAT_PROVIDER_MODEL_OPTION_NAME_0)?;
        let out = &port::format_provider_model_option_name(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_NORMALIZE_CLAUDE_MODEL_OPTION_SLUG_0: wire::Schema = wire::Schema::Dyn;

/// `normalizeClaudeModelOptionSlug` over the binary wire.
#[napi(js_name = "normalizeClaudeModelOptionSlugWire")]
pub fn normalize_claude_model_option_slug_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Value = wire::read_value(r, &S_NORMALIZE_CLAUDE_MODEL_OPTION_SLUG_0)?;
        let out = &port::normalize_claude_model_option_slug(model)?;
        w.string16(out);
        Ok(())
    })
}

static S_MERGE_DYNAMIC_MODEL_OPTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_MERGE_DYNAMIC_MODEL_OPTIONS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("slug", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "upstreamProviderId",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "upstreamProviderName",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
]));

/// `mergeDynamicModelOptions` over the binary wire.
#[napi(js_name = "mergeDynamicModelOptionsWire")]
pub fn merge_dynamic_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_MERGE_DYNAMIC_MODEL_OPTIONS_0)?;
        let out = &port::merge_dynamic_model_options(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_MERGE_DYNAMIC_MODEL_OPTIONS_OUT.inner())?;
        }
        Ok(())
    })
}

/// `providerModelCostMultiplierLabel` over the binary wire.
#[napi(js_name = "providerModelCostMultiplierLabelWire")]
pub fn provider_model_cost_multiplier_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let description: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::provider_model_cost_multiplier_label(description)?;
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

static S_GROUP_PROVIDER_MODEL_OPTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_GROUP_PROVIDER_MODEL_OPTIONS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("key", wire::Schema::Str16),
        ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "options",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("slug", wire::Schema::Str16),
                ("name", wire::Schema::Str16),
                ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
                (
                    "upstreamProviderId",
                    wire::Schema::OptJs(&wire::Schema::Str16),
                ),
                (
                    "upstreamProviderName",
                    wire::Schema::OptJs(&wire::Schema::Str16),
                ),
            ])),
        ),
    ]));

/// `groupProviderModelOptions` over the binary wire.
#[napi(js_name = "groupProviderModelOptionsWire")]
pub fn group_provider_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let options: Value = wire::read_value(r, &S_GROUP_PROVIDER_MODEL_OPTIONS_0)?;
        let out = &port::group_provider_model_options(options)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_GROUP_PROVIDER_MODEL_OPTIONS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_GROUP_PROVIDER_MODEL_OPTIONS_WITH_FAVORITES_0: wire::Schema = wire::Schema::Dyn;
static S_GROUP_PROVIDER_MODEL_OPTIONS_WITH_FAVORITES_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("key", wire::Schema::Str16),
        ("label", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "options",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("slug", wire::Schema::Str16),
                ("name", wire::Schema::Str16),
                ("description", wire::Schema::OptJs(&wire::Schema::Str16)),
                (
                    "upstreamProviderId",
                    wire::Schema::OptJs(&wire::Schema::Str16),
                ),
                (
                    "upstreamProviderName",
                    wire::Schema::OptJs(&wire::Schema::Str16),
                ),
            ])),
        ),
    ]));

/// `groupProviderModelOptionsWithFavorites` over the binary wire.
#[napi(js_name = "groupProviderModelOptionsWithFavoritesWire")]
pub fn group_provider_model_options_with_favorites_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_GROUP_PROVIDER_MODEL_OPTIONS_WITH_FAVORITES_0)?;
        let out = &port::group_provider_model_options_with_favorites(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(
                w,
                x0,
                S_GROUP_PROVIDER_MODEL_OPTIONS_WITH_FAVORITES_OUT.inner(),
            )?;
        }
        Ok(())
    })
}

/// `shouldUseCollapsibleModelGroups` over the binary wire.
#[napi(js_name = "shouldUseCollapsibleModelGroupsWire")]
pub fn should_use_collapsible_model_groups_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let group_count: f64 = r.f64()?;
        let is_searching: bool = r.bool()?;
        let out = &port::should_use_collapsible_model_groups(group_count, is_searching)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_MODEL_GROUP_DEFAULT_OPEN_0: wire::Schema = wire::Schema::Dyn;

/// `resolveModelGroupDefaultOpen` over the binary wire.
#[napi(js_name = "resolveModelGroupDefaultOpenWire")]
pub fn resolve_model_group_default_open_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Value = wire::read_value(r, &S_RESOLVE_MODEL_GROUP_DEFAULT_OPEN_0)?;
        let out = &port::resolve_model_group_default_open(input)?;
        w.bool(*out);
        Ok(())
    })
}

static S_BUILD_NEXT_PROVIDER_OPTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_NEXT_PROVIDER_OPTIONS_2: wire::Schema = wire::Schema::Dyn;
static S_BUILD_NEXT_PROVIDER_OPTIONS_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildNextProviderOptions` over the binary wire.
#[napi(js_name = "buildNextProviderOptionsWire")]
pub fn build_next_provider_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: Value = wire::read_value(r, &S_BUILD_NEXT_PROVIDER_OPTIONS_0)?;
        let model_options: Option<Value> = wire::read_dyn_opt(r)?;
        let patch: Value = wire::read_value(r, &S_BUILD_NEXT_PROVIDER_OPTIONS_2)?;
        let out = &port::build_next_provider_options(provider, model_options, patch)?;
        wire::write_value(w, out, &S_BUILD_NEXT_PROVIDER_OPTIONS_OUT)?;
        Ok(())
    })
}

static S_BUILD_PROVIDER_OPTION_PATCH_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_PROVIDER_OPTION_PATCH_2: wire::Schema = wire::Schema::Dyn;
static S_BUILD_PROVIDER_OPTION_PATCH_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Dyn);

/// `buildProviderOptionPatch` over the binary wire.
#[napi(js_name = "buildProviderOptionPatchWire")]
pub fn build_provider_option_patch_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: Value = wire::read_value(r, &S_BUILD_PROVIDER_OPTION_PATCH_0)?;
        let option_id: String = r.string16()?;
        let value: Value = wire::read_value(r, &S_BUILD_PROVIDER_OPTION_PATCH_2)?;
        let out = &port::build_provider_option_patch(provider, option_id, value)?;
        wire::write_value(w, out, &S_BUILD_PROVIDER_OPTION_PATCH_OUT)?;
        Ok(())
    })
}

static S_BUILD_MODEL_SELECTION_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_MODEL_SELECTION_3: wire::Schema = wire::Schema::Dyn;
static S_BUILD_MODEL_SELECTION_4: wire::Schema = wire::Schema::Dyn;
static S_BUILD_MODEL_SELECTION_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildModelSelection` over the binary wire.
#[napi(js_name = "buildModelSelectionWire")]
pub fn build_model_selection_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: Value = wire::read_value(r, &S_BUILD_MODEL_SELECTION_0)?;
        let model: String = r.string16()?;
        let options: Option<Value> = wire::read_dyn_opt(r)?;
        let supports_auto_mode_or_metadata: Value =
            wire::read_value(r, &S_BUILD_MODEL_SELECTION_3)?;
        let explicit_metadata: Value = wire::read_value(r, &S_BUILD_MODEL_SELECTION_4)?;
        let out = &port::build_model_selection(
            provider,
            model,
            options,
            supports_auto_mode_or_metadata,
            explicit_metadata,
        )?;
        wire::write_value(w, out, &S_BUILD_MODEL_SELECTION_OUT)?;
        Ok(())
    })
}
