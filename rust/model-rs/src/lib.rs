//! JS bindings for `packages/shared/src/model.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `getModelOptions`.
#[napi(js_name = "getModelOptions")]
pub fn get_model_options(provider: String) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::get_model_options(provider)?;
        crate::js::to_json(&out)
    })
}

/// `getDefaultModel`.
#[napi(js_name = "getDefaultModel")]
pub fn get_default_model(provider: String) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::get_default_model(provider)?;
        crate::js::to_json(&out)
    })
}

/// `formatModelDisplayName`.
#[napi(js_name = "formatModelDisplayName")]
pub fn format_model_display_name(
    model: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::format_model_display_name(model)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `parseCursorCliReasoningEffort`.
#[napi(js_name = "parseCursorCliReasoningEffort")]
pub fn parse_cursor_cli_reasoning_effort(
    model: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::parse_cursor_cli_reasoning_effort(model)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `hasEffortLevel`.
#[napi(js_name = "hasEffortLevel")]
pub fn has_effort_level(caps: String, value: String) -> napi::Result<bool> {
    run(move || {
        let caps = crate::js::from_json(&caps)?;
        let out = port::has_effort_level(caps, value)?;
        Ok(out)
    })
}

/// `getDefaultEffort`.
#[napi(js_name = "getDefaultEffort")]
pub fn get_default_effort(caps: String) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let caps = crate::js::from_json(&caps)?;
        let out = port::get_default_effort(caps)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `hasContextWindowOption`.
#[napi(js_name = "hasContextWindowOption")]
pub fn has_context_window_option(caps: String, value: String) -> napi::Result<bool> {
    run(move || {
        let caps = crate::js::from_json(&caps)?;
        let out = port::has_context_window_option(caps, value)?;
        Ok(out)
    })
}

/// `getDefaultContextWindow`.
#[napi(js_name = "getDefaultContextWindow")]
pub fn get_default_context_window(
    caps: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let caps = crate::js::from_json(&caps)?;
        let out = port::get_default_context_window(caps)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `getDevinStaticModelVariants`.
#[napi(js_name = "getDevinStaticModelVariants")]
pub fn get_devin_static_model_variants(model: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::get_devin_static_model_variants(model)?;
        crate::js::to_json(&out)
    })
}

/// `resolveDevinModelVariant`.
#[napi(js_name = "resolveDevinModelVariant")]
pub fn resolve_devin_model_variant(
    input: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_devin_model_variant(input)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `hasAutoCompactWindowOption`.
#[napi(js_name = "hasAutoCompactWindowOption")]
pub fn has_auto_compact_window_option(caps: String, value: String) -> napi::Result<bool> {
    run(move || {
        let caps = crate::js::from_json(&caps)?;
        let out = port::has_auto_compact_window_option(caps, value)?;
        Ok(out)
    })
}

/// `getClaudeContextWindowSuffix`.
#[napi(js_name = "getClaudeContextWindowSuffix")]
pub fn get_claude_context_window_suffix(
    model: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::get_claude_context_window_suffix(model)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `stripClaudeContextWindowSuffix`.
#[napi(js_name = "stripClaudeContextWindowSuffix")]
pub fn strip_claude_context_window_suffix(model: String) -> napi::Result<String> {
    run(move || {
        let out = port::strip_claude_context_window_suffix(model)?;
        Ok(out)
    })
}

/// `getDefaultAutoCompactWindow`.
#[napi(js_name = "getDefaultAutoCompactWindow")]
pub fn get_default_auto_compact_window(
    caps: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let caps = crate::js::from_json(&caps)?;
        let out = port::get_default_auto_compact_window(caps)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `resolveLabeledOptionValue`.
#[napi(js_name = "resolveLabeledOptionValue")]
pub fn resolve_labeled_option_value(
    options: String,
    raw_value: Option<String>,
) -> napi::Result<Option<String>> {
    run(move || {
        let options: serde_json::Value = crate::js::from_json(&options)?;
        let out = port::resolve_labeled_option_value(options, raw_value)?;
        Ok(out)
    })
}

/// `getProviderOptionBooleanSelectionValue`.
#[napi(js_name = "getProviderOptionBooleanSelectionValue")]
pub fn get_provider_option_boolean_selection_value(
    selections: String,
    id: String,
) -> napi::Result<napi::bindgen_prelude::Either<bool, ()>> {
    run(move || {
        let selections = crate::js::from_json(&selections)?;
        let out = port::get_provider_option_boolean_selection_value(selections, id)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `getModelSelectionStringOptionValue`.
#[napi(js_name = "getModelSelectionStringOptionValue")]
pub fn get_model_selection_string_option_value(
    model_selection: String,
    id: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let model_selection = crate::js::from_json(&model_selection)?;
        let out = port::get_model_selection_string_option_value(model_selection, id)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `getModelSelectionBooleanOptionValue`.
#[napi(js_name = "getModelSelectionBooleanOptionValue")]
pub fn get_model_selection_boolean_option_value(
    model_selection: String,
    id: String,
) -> napi::Result<napi::bindgen_prelude::Either<bool, ()>> {
    run(move || {
        let model_selection = crate::js::from_json(&model_selection)?;
        let out = port::get_model_selection_boolean_option_value(model_selection, id)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `getProviderOptionDescriptors`.
#[napi(js_name = "getProviderOptionDescriptors")]
pub fn get_provider_option_descriptors(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::get_provider_option_descriptors(input)?;
        crate::js::to_json(&out)
    })
}

/// `getProviderOptionCurrentValue`.
#[napi(js_name = "getProviderOptionCurrentValue")]
pub fn get_provider_option_current_value(descriptor: String) -> napi::Result<String> {
    run(move || {
        let descriptor = crate::js::from_json(&descriptor)?;
        let out = port::get_provider_option_current_value(descriptor)?;
        crate::js::to_json(&out)
    })
}

/// `getModelCapabilities`.
#[napi(js_name = "getModelCapabilities")]
pub fn get_model_capabilities(provider: String, model: Option<String>) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::get_model_capabilities(provider, model)?;
        crate::js::to_json(&out)
    })
}

/// `resolveNewestKnownClaudeFamilyModel`.
#[napi(js_name = "resolveNewestKnownClaudeFamilyModel")]
pub fn resolve_newest_known_claude_family_model(model: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_newest_known_claude_family_model(model)?;
        crate::js::to_json(&out)
    })
}

/// `resolveGrokEffortFamily`.
#[napi(js_name = "resolveGrokEffortFamily")]
pub fn resolve_grok_effort_family(model: String) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_grok_effort_family(model)?;
        Ok(out)
    })
}

/// `isClaudeUltrathinkPrompt`.
#[napi(js_name = "isClaudeUltrathinkPrompt")]
pub fn is_claude_ultrathink_prompt(text: Option<String>) -> napi::Result<bool> {
    run(move || {
        let out = port::is_claude_ultrathink_prompt(text)?;
        Ok(out)
    })
}

/// `normalizeModelSlug`.
#[napi(js_name = "normalizeModelSlug")]
pub fn normalize_model_slug(model: Option<String>, provider: String) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::normalize_model_slug(model, provider)?;
        crate::js::to_json(&out)
    })
}

/// `resolveSelectableModel`.
#[napi(js_name = "resolveSelectableModel")]
pub fn resolve_selectable_model(
    provider: String,
    value: Option<String>,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let options = crate::js::from_json(&options)?;
        let out = port::resolve_selectable_model(provider, value, options)?;
        crate::js::to_json(&out)
    })
}

/// `resolveModelSlug`.
#[napi(js_name = "resolveModelSlug")]
pub fn resolve_model_slug(model: Option<String>, provider: String) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::resolve_model_slug(model, provider)?;
        crate::js::to_json(&out)
    })
}

/// `resolveModelSlugForProvider`.
#[napi(js_name = "resolveModelSlugForProvider")]
pub fn resolve_model_slug_for_provider(
    provider: String,
    model: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::resolve_model_slug_for_provider(provider, model)?;
        crate::js::to_json(&out)
    })
}

/// `trimOrNull`.
#[napi(js_name = "trimOrNull")]
pub fn trim_or_null(
    value: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::trim_or_null(value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `normalizeClaudeModelOptions`.
#[napi(js_name = "normalizeClaudeModelOptions")]
pub fn normalize_claude_model_options(
    model: Option<String>,
    model_options: String,
    caps: String,
) -> napi::Result<String> {
    run(move || {
        let model_options = crate::js::from_json(&model_options)?;
        let caps = crate::js::from_json(&caps)?;
        let out = port::normalize_claude_model_options(model, model_options, caps)?;
        crate::js::to_json(&out)
    })
}

/// `resolveApiModelId`.
#[napi(js_name = "resolveApiModelId")]
pub fn resolve_api_model_id(
    model_selection: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let model_selection = crate::js::from_json(&model_selection)?;
        let out = port::resolve_api_model_id(model_selection)?;
        Ok(out
            .as_str()
            .map_or(napi::bindgen_prelude::Either::B(()), |s| {
                napi::bindgen_prelude::Either::A(s.to_string())
            }))
    })
}

/// `getEffectiveClaudeCodeEffort`.
#[napi(js_name = "getEffectiveClaudeCodeEffort")]
pub fn get_effective_claude_code_effort(effort: String) -> napi::Result<String> {
    run(move || {
        let effort = crate::js::from_json(&effort)?;
        let out = port::get_effective_claude_code_effort(effort)?;
        crate::js::to_json(&out)
    })
}

/// `claudeSelectionRequiresRestart`.
#[napi(js_name = "claudeSelectionRequiresRestart")]
pub fn claude_selection_requires_restart(previous: String, next: String) -> napi::Result<bool> {
    run(move || {
        let previous = crate::js::from_json(&previous)?;
        let next = crate::js::from_json(&next)?;
        let out = port::claude_selection_requires_restart(previous, next)?;
        Ok(out)
    })
}

/// `normalizeCursorModelOptions`.
#[napi(js_name = "normalizeCursorModelOptions")]
pub fn normalize_cursor_model_options(
    model: Option<String>,
    model_options: String,
    capabilities: String,
) -> napi::Result<String> {
    run(move || {
        let model_options = crate::js::from_json(&model_options)?;
        let capabilities = crate::js::from_json(&capabilities)?;
        let out = port::normalize_cursor_model_options(model, model_options, capabilities)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeGrokModelOptions`.
#[napi(js_name = "normalizeGrokModelOptions")]
pub fn normalize_grok_model_options(
    model: Option<String>,
    model_options: String,
) -> napi::Result<String> {
    run(move || {
        let model_options = crate::js::from_json(&model_options)?;
        let out = port::normalize_grok_model_options(model, model_options)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeAntigravityModelOptions`.
#[napi(js_name = "normalizeAntigravityModelOptions")]
pub fn normalize_antigravity_model_options(
    model: Option<String>,
    model_options: String,
    capabilities: String,
) -> napi::Result<String> {
    run(move || {
        let model_options = crate::js::from_json(&model_options)?;
        let capabilities = crate::js::from_json(&capabilities)?;
        let out = port::normalize_antigravity_model_options(model, model_options, capabilities)?;
        crate::js::to_json(&out)
    })
}

/// `normalizePiModelOptions`.
#[napi(js_name = "normalizePiModelOptions")]
pub fn normalize_pi_model_options(model_options: String) -> napi::Result<String> {
    run(move || {
        let model_options = crate::js::from_json(&model_options)?;
        let out = port::normalize_pi_model_options(model_options)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeOmpModelOptions`.
#[napi(js_name = "normalizeOmpModelOptions")]
pub fn normalize_omp_model_options(model_options: String) -> napi::Result<String> {
    run(move || {
        let model_options = crate::js::from_json(&model_options)?;
        let out = port::normalize_omp_model_options(model_options)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeOpenCodeModelOptions`.
#[napi(js_name = "normalizeOpenCodeModelOptions")]
pub fn normalize_open_code_model_options(model_options: String) -> napi::Result<String> {
    run(move || {
        let model_options = crate::js::from_json(&model_options)?;
        let out = port::normalize_open_code_model_options(model_options)?;
        crate::js::to_json(&out)
    })
}

/// `applyClaudePromptEffortPrefix`.
#[napi(js_name = "applyClaudePromptEffortPrefix")]
pub fn apply_claude_prompt_effort_prefix(text: String, effort: String) -> napi::Result<String> {
    run(move || {
        let effort = crate::js::from_json(&effort)?;
        let out = port::apply_claude_prompt_effort_prefix(text, effort)?;
        Ok(out)
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

static S_GET_MODEL_OPTIONS_OUT: wire::Schema = wire::Schema::Dyn;

/// `getModelOptions` over the binary wire.
#[napi(js_name = "getModelOptionsWire")]
pub fn get_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::get_model_options(provider)?;
        wire::write_value(w, out, &S_GET_MODEL_OPTIONS_OUT)?;
        Ok(())
    })
}

/// `getDefaultModel` over the binary wire.
#[napi(js_name = "getDefaultModelWire")]
pub fn get_default_model_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::get_default_model(provider)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `formatModelDisplayName` over the binary wire.
#[napi(js_name = "formatModelDisplayNameWire")]
pub fn format_model_display_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::format_model_display_name(model)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

/// `parseCursorCliReasoningEffort` over the binary wire.
#[napi(js_name = "parseCursorCliReasoningEffortWire")]
pub fn parse_cursor_cli_reasoning_effort_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: String = r.string16()?;
        let out = &port::parse_cursor_cli_reasoning_effort(model)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_HAS_EFFORT_LEVEL_0: wire::Schema = wire::Schema::Dyn;

/// `hasEffortLevel` over the binary wire.
#[napi(js_name = "hasEffortLevelWire")]
pub fn has_effort_level_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let caps: serde_json::Value = wire::read_value(r, &S_HAS_EFFORT_LEVEL_0)?;
        let value: String = r.string16()?;
        let out = &port::has_effort_level(caps, value)?;
        w.bool(*out);
        Ok(())
    })
}

static S_GET_DEFAULT_EFFORT_0: wire::Schema = wire::Schema::Dyn;

/// `getDefaultEffort` over the binary wire.
#[napi(js_name = "getDefaultEffortWire")]
pub fn get_default_effort_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let caps: serde_json::Value = wire::read_value(r, &S_GET_DEFAULT_EFFORT_0)?;
        let out = &port::get_default_effort(caps)?;
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

static S_HAS_CONTEXT_WINDOW_OPTION_0: wire::Schema = wire::Schema::Dyn;

/// `hasContextWindowOption` over the binary wire.
#[napi(js_name = "hasContextWindowOptionWire")]
pub fn has_context_window_option_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let caps: serde_json::Value = wire::read_value(r, &S_HAS_CONTEXT_WINDOW_OPTION_0)?;
        let value: String = r.string16()?;
        let out = &port::has_context_window_option(caps, value)?;
        w.bool(*out);
        Ok(())
    })
}

static S_GET_DEFAULT_CONTEXT_WINDOW_0: wire::Schema = wire::Schema::Dyn;

/// `getDefaultContextWindow` over the binary wire.
#[napi(js_name = "getDefaultContextWindowWire")]
pub fn get_default_context_window_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let caps: serde_json::Value = wire::read_value(r, &S_GET_DEFAULT_CONTEXT_WINDOW_0)?;
        let out = &port::get_default_context_window(caps)?;
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

/// `getDevinStaticModelVariants` over the binary wire.
#[napi(js_name = "getDevinStaticModelVariantsWire")]
pub fn get_devin_static_model_variants_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::get_devin_static_model_variants(model)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_DEVIN_MODEL_VARIANT_0: wire::Schema = wire::Schema::Dyn;

/// `resolveDevinModelVariant` over the binary wire.
#[napi(js_name = "resolveDevinModelVariantWire")]
pub fn resolve_devin_model_variant_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_DEVIN_MODEL_VARIANT_0)?;
        let out = &port::resolve_devin_model_variant(input)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_HAS_AUTO_COMPACT_WINDOW_OPTION_0: wire::Schema = wire::Schema::Dyn;

/// `hasAutoCompactWindowOption` over the binary wire.
#[napi(js_name = "hasAutoCompactWindowOptionWire")]
pub fn has_auto_compact_window_option_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let caps: serde_json::Value = wire::read_value(r, &S_HAS_AUTO_COMPACT_WINDOW_OPTION_0)?;
        let value: String = r.string16()?;
        let out = &port::has_auto_compact_window_option(caps, value)?;
        w.bool(*out);
        Ok(())
    })
}

/// `getClaudeContextWindowSuffix` over the binary wire.
#[napi(js_name = "getClaudeContextWindowSuffixWire")]
pub fn get_claude_context_window_suffix_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::get_claude_context_window_suffix(model)?;
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

/// `stripClaudeContextWindowSuffix` over the binary wire.
#[napi(js_name = "stripClaudeContextWindowSuffixWire")]
pub fn strip_claude_context_window_suffix_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: String = r.string16()?;
        let out = &port::strip_claude_context_window_suffix(model)?;
        w.string16(out);
        Ok(())
    })
}

static S_GET_DEFAULT_AUTO_COMPACT_WINDOW_0: wire::Schema = wire::Schema::Dyn;

/// `getDefaultAutoCompactWindow` over the binary wire.
#[napi(js_name = "getDefaultAutoCompactWindowWire")]
pub fn get_default_auto_compact_window_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let caps: serde_json::Value = wire::read_value(r, &S_GET_DEFAULT_AUTO_COMPACT_WINDOW_0)?;
        let out = &port::get_default_auto_compact_window(caps)?;
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

static S_RESOLVE_LABELED_OPTION_VALUE_0: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Obj(&[
        ("value", wire::Schema::Str16),
        ("isDefault", wire::Schema::OptJs(&wire::Schema::Bool)),
    ])));

/// `resolveLabeledOptionValue` over the binary wire.
#[napi(js_name = "resolveLabeledOptionValueWire")]
pub fn resolve_labeled_option_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let options: Option<Vec<serde_json::Value>> = r.opt_js(|r| {
            r.list(|r| wire::read_value(r, S_RESOLVE_LABELED_OPTION_VALUE_0.inner().inner()))
        })?;
        let raw_value: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::resolve_labeled_option_value(
            options
                .map(serde_json::Value::Array)
                .unwrap_or(serde_json::Value::Null),
            raw_value,
        )?;
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

static S_GET_PROVIDER_OPTION_BOOLEAN_SELECTION_VALUE_0: wire::Schema = wire::Schema::Dyn;

/// `getProviderOptionBooleanSelectionValue` over the binary wire.
#[napi(js_name = "getProviderOptionBooleanSelectionValueWire")]
pub fn get_provider_option_boolean_selection_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let selections: serde_json::Value =
            wire::read_value(r, &S_GET_PROVIDER_OPTION_BOOLEAN_SELECTION_VALUE_0)?;
        let id: String = r.string16()?;
        let out = &port::get_provider_option_boolean_selection_value(selections, id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.bool(*x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

/// `getModelSelectionStringOptionValue` over the binary wire.
#[napi(js_name = "getModelSelectionStringOptionValueWire")]
pub fn get_model_selection_string_option_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_selection: serde_json::Value = wire::read_dyn(r)?;
        let id: String = r.string16()?;
        let out = &port::get_model_selection_string_option_value(model_selection, id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.string16(x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

/// `getModelSelectionBooleanOptionValue` over the binary wire.
#[napi(js_name = "getModelSelectionBooleanOptionValueWire")]
pub fn get_model_selection_boolean_option_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_selection: serde_json::Value = wire::read_dyn(r)?;
        let id: String = r.string16()?;
        let out = &port::get_model_selection_boolean_option_value(model_selection, id)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.bool(*x0);
            }
            None => w.tag(0),
        }
        Ok(())
    })
}

static S_GET_PROVIDER_OPTION_DESCRIPTORS_0: wire::Schema = wire::Schema::Dyn;

/// `getProviderOptionDescriptors` over the binary wire.
#[napi(js_name = "getProviderOptionDescriptorsWire")]
pub fn get_provider_option_descriptors_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_GET_PROVIDER_OPTION_DESCRIPTORS_0)?;
        let out = &port::get_provider_option_descriptors(input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_GET_PROVIDER_OPTION_CURRENT_VALUE_OUT: wire::Schema = wire::Schema::Dyn;

/// `getProviderOptionCurrentValue` over the binary wire.
#[napi(js_name = "getProviderOptionCurrentValueWire")]
pub fn get_provider_option_current_value_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let descriptor: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::get_provider_option_current_value(descriptor)?;
        wire::write_value(w, out, &S_GET_PROVIDER_OPTION_CURRENT_VALUE_OUT)?;
        Ok(())
    })
}

static S_GET_MODEL_CAPABILITIES_0: wire::Schema = wire::Schema::Dyn;
static S_GET_MODEL_CAPABILITIES_OUT: wire::Schema = wire::Schema::Dyn;

/// `getModelCapabilities` over the binary wire.
#[napi(js_name = "getModelCapabilitiesWire")]
pub fn get_model_capabilities_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_value(r, &S_GET_MODEL_CAPABILITIES_0)?;
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::get_model_capabilities(provider, model)?;
        wire::write_value(w, out, &S_GET_MODEL_CAPABILITIES_OUT)?;
        Ok(())
    })
}

/// `resolveNewestKnownClaudeFamilyModel` over the binary wire.
#[napi(js_name = "resolveNewestKnownClaudeFamilyModelWire")]
pub fn resolve_newest_known_claude_family_model_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::resolve_newest_known_claude_family_model(model)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `resolveGrokEffortFamily` over the binary wire.
#[napi(js_name = "resolveGrokEffortFamilyWire")]
pub fn resolve_grok_effort_family_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: String = r.string16()?;
        let out = &port::resolve_grok_effort_family(model)?;
        w.string16(out);
        Ok(())
    })
}

/// `isClaudeUltrathinkPrompt` over the binary wire.
#[napi(js_name = "isClaudeUltrathinkPromptWire")]
pub fn is_claude_ultrathink_prompt_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_claude_ultrathink_prompt(text)?;
        w.bool(*out);
        Ok(())
    })
}

/// `normalizeModelSlug` over the binary wire.
#[napi(js_name = "normalizeModelSlugWire")]
pub fn normalize_model_slug_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let provider: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::normalize_model_slug(model, provider)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_SELECTABLE_MODEL_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_SELECTABLE_MODEL_2: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("slug", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
]));

/// `resolveSelectableModel` over the binary wire.
#[napi(js_name = "resolveSelectableModelWire")]
pub fn resolve_selectable_model_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_value(r, &S_RESOLVE_SELECTABLE_MODEL_0)?;
        let value: Option<String> = r.opt_js(|r| r.string16())?;
        let options: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_RESOLVE_SELECTABLE_MODEL_2.inner()))?;
        let out =
            &port::resolve_selectable_model(provider, value, serde_json::Value::Array(options))?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `resolveModelSlug` over the binary wire.
#[napi(js_name = "resolveModelSlugWire")]
pub fn resolve_model_slug_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let provider: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::resolve_model_slug(model, provider)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_MODEL_SLUG_FOR_PROVIDER_0: wire::Schema = wire::Schema::Dyn;

/// `resolveModelSlugForProvider` over the binary wire.
#[napi(js_name = "resolveModelSlugForProviderWire")]
pub fn resolve_model_slug_for_provider_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_MODEL_SLUG_FOR_PROVIDER_0)?;
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::resolve_model_slug_for_provider(provider, model)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `trimOrNull` over the binary wire.
#[napi(js_name = "trimOrNullWire")]
pub fn trim_or_null_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::trim_or_null(value)?;
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

/// `normalizeClaudeModelOptions` over the binary wire.
#[napi(js_name = "normalizeClaudeModelOptionsWire")]
pub fn normalize_claude_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let model_options: serde_json::Value = wire::read_dyn(r)?;
        let caps: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::normalize_claude_model_options(model, model_options, caps)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_API_MODEL_ID_0: wire::Schema = wire::Schema::Dyn;

/// `resolveApiModelId` over the binary wire.
#[napi(js_name = "resolveApiModelIdWire")]
pub fn resolve_api_model_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_selection: serde_json::Value = wire::read_value(r, &S_RESOLVE_API_MODEL_ID_0)?;
        let out = &port::resolve_api_model_id(model_selection)?;
        match out {
            serde_json::Value::String(s) => {
                w.tag(1);
                w.string16(s);
            }
            _ => w.tag(0),
        }
        Ok(())
    })
}

/// `getEffectiveClaudeCodeEffort` over the binary wire.
#[napi(js_name = "getEffectiveClaudeCodeEffortWire")]
pub fn get_effective_claude_code_effort_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let effort: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::get_effective_claude_code_effort(effort)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_CLAUDE_SELECTION_REQUIRES_RESTART_1: wire::Schema = wire::Schema::Dyn;

/// `claudeSelectionRequiresRestart` over the binary wire.
#[napi(js_name = "claudeSelectionRequiresRestartWire")]
pub fn claude_selection_requires_restart_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let previous: serde_json::Value = wire::read_dyn(r)?;
        let next: serde_json::Value = wire::read_value(r, &S_CLAUDE_SELECTION_REQUIRES_RESTART_1)?;
        let out = &port::claude_selection_requires_restart(previous, next)?;
        w.bool(*out);
        Ok(())
    })
}

/// `normalizeCursorModelOptions` over the binary wire.
#[napi(js_name = "normalizeCursorModelOptionsWire")]
pub fn normalize_cursor_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let model_options: serde_json::Value = wire::read_dyn(r)?;
        let capabilities: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::normalize_cursor_model_options(model, model_options, capabilities)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `normalizeGrokModelOptions` over the binary wire.
#[napi(js_name = "normalizeGrokModelOptionsWire")]
pub fn normalize_grok_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let model_options: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::normalize_grok_model_options(model, model_options)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `normalizeAntigravityModelOptions` over the binary wire.
#[napi(js_name = "normalizeAntigravityModelOptionsWire")]
pub fn normalize_antigravity_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model: Option<String> = r.opt_js(|r| r.string16())?;
        let model_options: serde_json::Value = wire::read_dyn(r)?;
        let capabilities: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::normalize_antigravity_model_options(model, model_options, capabilities)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `normalizePiModelOptions` over the binary wire.
#[napi(js_name = "normalizePiModelOptionsWire")]
pub fn normalize_pi_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_options: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::normalize_pi_model_options(model_options)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `normalizeOmpModelOptions` over the binary wire.
#[napi(js_name = "normalizeOmpModelOptionsWire")]
pub fn normalize_omp_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_options: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::normalize_omp_model_options(model_options)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `normalizeOpenCodeModelOptions` over the binary wire.
#[napi(js_name = "normalizeOpenCodeModelOptionsWire")]
pub fn normalize_open_code_model_options_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let model_options: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::normalize_open_code_model_options(model_options)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `applyClaudePromptEffortPrefix` over the binary wire.
#[napi(js_name = "applyClaudePromptEffortPrefixWire")]
pub fn apply_claude_prompt_effort_prefix_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let effort: serde_json::Value = wire::read_dyn(r)?;
        let out = &port::apply_claude_prompt_effort_prefix(text, effort)?;
        w.string16(out);
        Ok(())
    })
}
