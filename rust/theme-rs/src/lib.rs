//! JS bindings for `apps/web/src/theme/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isThemeMode`.
#[napi(js_name = "isThemeMode")]
pub fn is_theme_mode(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_theme_mode(value)?;
        Ok(out)
    })
}

/// `isThemeVariant`.
#[napi(js_name = "isThemeVariant")]
pub fn is_theme_variant(value: String) -> napi::Result<bool> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::is_theme_variant(value)?;
        Ok(out)
    })
}

/// `getAvailableCodeThemes`.
#[napi(js_name = "getAvailableCodeThemes")]
pub fn get_available_code_themes(variant: String) -> napi::Result<String> {
    run(move || {
        let out = port::get_available_code_themes(variant)?;
        crate::js::to_json(&out)
    })
}

/// `isCodeThemeAvailable`.
#[napi(js_name = "isCodeThemeAvailable")]
pub fn is_code_theme_available(code_theme_id: String, variant: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_code_theme_available(code_theme_id, variant)?;
        Ok(out)
    })
}

/// `normalizeCodeThemeId`.
#[napi(js_name = "normalizeCodeThemeId")]
pub fn normalize_code_theme_id(
    code_theme_id: String,
    variant: String,
    fallback: String,
) -> napi::Result<String> {
    run(move || {
        let code_theme_id = crate::js::from_json(&code_theme_id)?;
        let fallback = crate::js::from_json(&fallback)?;
        let out = port::normalize_code_theme_id(code_theme_id, variant, fallback)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeThemeFonts`.
#[napi(js_name = "normalizeThemeFonts")]
pub fn normalize_theme_fonts(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_theme_fonts(value)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeSemanticColors`.
#[napi(js_name = "normalizeSemanticColors")]
pub fn normalize_semantic_colors(value: String, fallback: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let fallback = crate::js::from_json(&fallback)?;
        let out = port::normalize_semantic_colors(value, fallback)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeChromeTheme`.
#[napi(js_name = "normalizeChromeTheme")]
pub fn normalize_chrome_theme(value: String, variant: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_chrome_theme(value, variant)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeWindowTranslucency`.
#[napi(js_name = "normalizeWindowTranslucency")]
pub fn normalize_window_translucency(value: String, variant: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_window_translucency(value, variant)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeThemePack`.
#[napi(js_name = "normalizeThemePack")]
pub fn normalize_theme_pack(value: String, variant: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_theme_pack(value, variant)?;
        crate::js::to_json(&out)
    })
}

/// `getCodeThemeSeed`.
#[napi(js_name = "getCodeThemeSeed")]
pub fn get_code_theme_seed(code_theme_id: String, variant: String) -> napi::Result<String> {
    run(move || {
        let out = port::get_code_theme_seed(code_theme_id, variant)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeThemeState`.
#[napi(js_name = "normalizeThemeState")]
pub fn normalize_theme_state(value: String) -> napi::Result<String> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_theme_state(value)?;
        crate::js::to_json(&out)
    })
}

/// `parseStoredThemeState`.
#[napi(js_name = "parseStoredThemeState")]
pub fn parse_stored_theme_state(raw_value: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::parse_stored_theme_state(raw_value)?;
        crate::js::to_json(&out)
    })
}

/// `serializeThemeState`.
#[napi(js_name = "serializeThemeState")]
pub fn serialize_theme_state(state: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::serialize_theme_state(state)?;
        Ok(out)
    })
}

/// `createThemeShareString`.
#[napi(js_name = "createThemeShareString")]
pub fn create_theme_share_string(variant: String, pack: String) -> napi::Result<String> {
    run(move || {
        let pack = crate::js::from_json(&pack)?;
        let out = port::create_theme_share_string(variant, pack)?;
        Ok(out)
    })
}

/// `parseThemeShareString`.
#[napi(js_name = "parseThemeShareString")]
pub fn parse_theme_share_string(raw_value: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_theme_share_string(raw_value)?;
        crate::js::to_json(&out)
    })
}

/// `canParseThemeShareString`.
#[napi(js_name = "canParseThemeShareString")]
pub fn can_parse_theme_share_string(
    value: String,
    target_variant: Option<String>,
) -> napi::Result<bool> {
    run(move || {
        let out = port::can_parse_theme_share_string(value, target_variant)?;
        Ok(out)
    })
}

/// `parseThemeShareStringForVariant`.
#[napi(js_name = "parseThemeShareStringForVariant")]
pub fn parse_theme_share_string_for_variant(
    value: String,
    target_variant: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::parse_theme_share_string_for_variant(value, target_variant)?;
        crate::js::to_json(&out)
    })
}

/// `updateThemePackFromShareString`.
#[napi(js_name = "updateThemePackFromShareString")]
pub fn update_theme_pack_from_share_string(
    state: String,
    value: String,
    target_variant: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::update_theme_pack_from_share_string(state, value, target_variant)?;
        crate::js::to_json(&out)
    })
}

/// `updateChromeTheme`.
#[napi(js_name = "updateChromeTheme")]
pub fn update_chrome_theme(state: String, variant: String, patch: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let patch = crate::js::from_json(&patch)?;
        let out = port::update_chrome_theme(state, variant, patch)?;
        crate::js::to_json(&out)
    })
}

/// `setThemeCodeThemeId`.
#[napi(js_name = "setThemeCodeThemeId")]
pub fn set_theme_code_theme_id(
    state: String,
    variant: String,
    code_theme_id: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::set_theme_code_theme_id(state, variant, code_theme_id)?;
        crate::js::to_json(&out)
    })
}

/// `setThemeFonts`.
#[napi(js_name = "setThemeFonts")]
pub fn set_theme_fonts(state: String, variant: String, patch: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let patch = crate::js::from_json(&patch)?;
        let out = port::set_theme_fonts(state, variant, patch)?;
        crate::js::to_json(&out)
    })
}

/// `resetThemeVariant`.
#[napi(js_name = "resetThemeVariant")]
pub fn reset_theme_variant(state: String, variant: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reset_theme_variant(state, variant)?;
        crate::js::to_json(&out)
    })
}

/// `setWindowTranslucency`.
#[napi(js_name = "setWindowTranslucency")]
pub fn set_window_translucency(
    state: String,
    variant: String,
    patch: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let patch = crate::js::from_json(&patch)?;
        let out = port::set_window_translucency(state, variant, patch)?;
        crate::js::to_json(&out)
    })
}

/// `areWindowTranslucenciesEqual`.
#[napi(js_name = "areWindowTranslucenciesEqual")]
pub fn are_window_translucencies_equal(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::are_window_translucencies_equal(left, right)?;
        Ok(out)
    })
}

/// `resolveThemePack`.
#[napi(js_name = "resolveThemePack")]
pub fn resolve_theme_pack(state: String, variant: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::resolve_theme_pack(state, variant)?;
        crate::js::to_json(&out)
    })
}

/// `resolveProfileThemeAccent`.
#[napi(js_name = "resolveProfileThemeAccent")]
pub fn resolve_profile_theme_accent(state: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::resolve_profile_theme_accent(state)?;
        crate::js::to_json(&out)
    })
}

/// `areThemePacksEqual`.
#[napi(js_name = "areThemePacksEqual")]
pub fn are_theme_packs_equal(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::are_theme_packs_equal(left, right)?;
        Ok(out)
    })
}

/// `resolveThemeVariant`.
#[napi(js_name = "resolveThemeVariant")]
pub fn resolve_theme_variant(mode: String, system_dark: bool) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_theme_variant(mode, system_dark)?;
        Ok(out)
    })
}

/// `buildResolvedThemeTokens`.
#[napi(js_name = "buildResolvedThemeTokens")]
pub fn build_resolved_theme_tokens(pack: String, variant: String) -> napi::Result<String> {
    run(move || {
        let pack = crate::js::from_json(&pack)?;
        let out = port::build_resolved_theme_tokens(pack, variant)?;
        crate::js::to_json(&out)
    })
}

/// `buildThemeCssVariables`.
#[napi(js_name = "buildThemeCssVariables")]
pub fn build_theme_css_variables(
    pack: String,
    variant: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let pack = crate::js::from_json(&pack)?;
        let options = crate::js::from_json(&options)?;
        let out = port::build_theme_css_variables(pack, variant, options)?;
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

static S_IS_THEME_MODE_0: wire::Schema = wire::Schema::Dyn;

/// `isThemeMode` over the binary wire.
#[napi(js_name = "isThemeModeWire")]
pub fn is_theme_mode_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_IS_THEME_MODE_0)?;
        let out = &port::is_theme_mode(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_THEME_VARIANT_0: wire::Schema = wire::Schema::Dyn;

/// `isThemeVariant` over the binary wire.
#[napi(js_name = "isThemeVariantWire")]
pub fn is_theme_variant_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_IS_THEME_VARIANT_0)?;
        let out = &port::is_theme_variant(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_GET_AVAILABLE_CODE_THEMES_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("label", wire::Schema::Str16),
    ("variants", wire::Schema::List(&wire::Schema::Str16)),
]));

/// `getAvailableCodeThemes` over the binary wire.
#[napi(js_name = "getAvailableCodeThemesWire")]
pub fn get_available_code_themes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let variant: String = r.string16()?;
        let out = &port::get_available_code_themes(variant)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_GET_AVAILABLE_CODE_THEMES_OUT.inner())?;
        }
        Ok(())
    })
}

/// `isCodeThemeAvailable` over the binary wire.
#[napi(js_name = "isCodeThemeAvailableWire")]
pub fn is_code_theme_available_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let code_theme_id: String = r.string16()?;
        let variant: String = r.string16()?;
        let out = &port::is_code_theme_available(code_theme_id, variant)?;
        w.bool(*out);
        Ok(())
    })
}

static S_NORMALIZE_CODE_THEME_ID_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_CODE_THEME_ID_2: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_CODE_THEME_ID_OUT: wire::Schema = wire::Schema::Str16;

/// `normalizeCodeThemeId` over the binary wire.
#[napi(js_name = "normalizeCodeThemeIdWire")]
pub fn normalize_code_theme_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let code_theme_id: Value = wire::read_value(r, &S_NORMALIZE_CODE_THEME_ID_0)?;
        let variant: String = r.string16()?;
        let fallback: Value = wire::read_value(r, &S_NORMALIZE_CODE_THEME_ID_2)?;
        let out = &port::normalize_code_theme_id(code_theme_id, variant, fallback)?;
        wire::write_value(w, out, &S_NORMALIZE_CODE_THEME_ID_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_THEME_FONTS_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THEME_FONTS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("ui", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("code", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `normalizeThemeFonts` over the binary wire.
#[napi(js_name = "normalizeThemeFontsWire")]
pub fn normalize_theme_fonts_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NORMALIZE_THEME_FONTS_0)?;
        let out = &port::normalize_theme_fonts(value)?;
        wire::write_value(w, out, &S_NORMALIZE_THEME_FONTS_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_SEMANTIC_COLORS_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_SEMANTIC_COLORS_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_SEMANTIC_COLORS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("diffAdded", wire::Schema::Str16),
    ("diffRemoved", wire::Schema::Str16),
    ("skill", wire::Schema::Str16),
]);

/// `normalizeSemanticColors` over the binary wire.
#[napi(js_name = "normalizeSemanticColorsWire")]
pub fn normalize_semantic_colors_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NORMALIZE_SEMANTIC_COLORS_0)?;
        let fallback: Value = wire::read_value(r, &S_NORMALIZE_SEMANTIC_COLORS_1)?;
        let out = &port::normalize_semantic_colors(value, fallback)?;
        wire::write_value(w, out, &S_NORMALIZE_SEMANTIC_COLORS_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_CHROME_THEME_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_CHROME_THEME_OUT: wire::Schema = wire::Schema::Obj(&[
    ("accent", wire::Schema::Str16),
    ("contrast", wire::Schema::Num),
    (
        "fonts",
        wire::Schema::Obj(&[
            ("ui", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("code", wire::Schema::OptJs(&wire::Schema::Str16)),
        ]),
    ),
    ("ink", wire::Schema::Str16),
    ("opaqueWindows", wire::Schema::Bool),
    (
        "semanticColors",
        wire::Schema::Obj(&[
            ("diffAdded", wire::Schema::Str16),
            ("diffRemoved", wire::Schema::Str16),
            ("skill", wire::Schema::Str16),
        ]),
    ),
    ("surface", wire::Schema::Str16),
]);

/// `normalizeChromeTheme` over the binary wire.
#[napi(js_name = "normalizeChromeThemeWire")]
pub fn normalize_chrome_theme_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NORMALIZE_CHROME_THEME_0)?;
        let variant: String = r.string16()?;
        let out = &port::normalize_chrome_theme(value, variant)?;
        wire::write_value(w, out, &S_NORMALIZE_CHROME_THEME_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_WINDOW_TRANSLUCENCY_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_WINDOW_TRANSLUCENCY_OUT: wire::Schema = wire::Schema::Obj(&[
    ("opacity", wire::Schema::Num),
    ("blur", wire::Schema::OptJs(&wire::Schema::Num)),
    ("sidebarOnly", wire::Schema::Bool),
]);

/// `normalizeWindowTranslucency` over the binary wire.
#[napi(js_name = "normalizeWindowTranslucencyWire")]
pub fn normalize_window_translucency_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NORMALIZE_WINDOW_TRANSLUCENCY_0)?;
        let variant: String = r.string16()?;
        let out = &port::normalize_window_translucency(value, variant)?;
        wire::write_value(w, out, &S_NORMALIZE_WINDOW_TRANSLUCENCY_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_THEME_PACK_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THEME_PACK_OUT: wire::Schema = wire::Schema::Obj(&[
    ("codeThemeId", wire::Schema::Str16),
    (
        "theme",
        wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            (
                "fonts",
                wire::Schema::Obj(&[
                    ("ui", wire::Schema::OptJs(&wire::Schema::Str16)),
                    ("code", wire::Schema::OptJs(&wire::Schema::Str16)),
                ]),
            ),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            (
                "semanticColors",
                wire::Schema::Obj(&[
                    ("diffAdded", wire::Schema::Str16),
                    ("diffRemoved", wire::Schema::Str16),
                    ("skill", wire::Schema::Str16),
                ]),
            ),
            ("surface", wire::Schema::Str16),
        ]),
    ),
]);

/// `normalizeThemePack` over the binary wire.
#[napi(js_name = "normalizeThemePackWire")]
pub fn normalize_theme_pack_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NORMALIZE_THEME_PACK_0)?;
        let variant: String = r.string16()?;
        let out = &port::normalize_theme_pack(value, variant)?;
        wire::write_value(w, out, &S_NORMALIZE_THEME_PACK_OUT)?;
        Ok(())
    })
}

static S_GET_CODE_THEME_SEED_OUT: wire::Schema = wire::Schema::Obj(&[
    ("accent", wire::Schema::Str16),
    ("contrast", wire::Schema::Num),
    (
        "fonts",
        wire::Schema::Obj(&[
            ("ui", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("code", wire::Schema::OptJs(&wire::Schema::Str16)),
        ]),
    ),
    ("ink", wire::Schema::Str16),
    ("opaqueWindows", wire::Schema::Bool),
    (
        "semanticColors",
        wire::Schema::Obj(&[
            ("diffAdded", wire::Schema::Str16),
            ("diffRemoved", wire::Schema::Str16),
            ("skill", wire::Schema::Str16),
        ]),
    ),
    ("surface", wire::Schema::Str16),
]);

/// `getCodeThemeSeed` over the binary wire.
#[napi(js_name = "getCodeThemeSeedWire")]
pub fn get_code_theme_seed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let code_theme_id: String = r.string16()?;
        let variant: String = r.string16()?;
        let out = &port::get_code_theme_seed(code_theme_id, variant)?;
        wire::write_value(w, out, &S_GET_CODE_THEME_SEED_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_THEME_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_THEME_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "chromeThemes",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            ("fonts", wire::Schema::Dyn),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            ("semanticColors", wire::Schema::Dyn),
            ("surface", wire::Schema::Str16),
        ])),
    ),
    ("codeThemeIds", wire::Schema::Map16(&wire::Schema::Str16)),
    ("mode", wire::Schema::Str16),
    ("systemUiFont", wire::Schema::Bool),
    (
        "translucency",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("opacity", wire::Schema::Num),
            ("blur", wire::Schema::OptJs(&wire::Schema::Num)),
            ("sidebarOnly", wire::Schema::Bool),
        ])),
    ),
]);

/// `normalizeThemeState` over the binary wire.
#[napi(js_name = "normalizeThemeStateWire")]
pub fn normalize_theme_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: Value = wire::read_value(r, &S_NORMALIZE_THEME_STATE_0)?;
        let out = &port::normalize_theme_state(value)?;
        wire::write_value(w, out, &S_NORMALIZE_THEME_STATE_OUT)?;
        Ok(())
    })
}

static S_PARSE_STORED_THEME_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "chromeThemes",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            ("fonts", wire::Schema::Dyn),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            ("semanticColors", wire::Schema::Dyn),
            ("surface", wire::Schema::Str16),
        ])),
    ),
    ("codeThemeIds", wire::Schema::Map16(&wire::Schema::Str16)),
    ("mode", wire::Schema::Str16),
    ("systemUiFont", wire::Schema::Bool),
    (
        "translucency",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("opacity", wire::Schema::Num),
            ("blur", wire::Schema::OptJs(&wire::Schema::Num)),
            ("sidebarOnly", wire::Schema::Bool),
        ])),
    ),
]);

/// `parseStoredThemeState` over the binary wire.
#[napi(js_name = "parseStoredThemeStateWire")]
pub fn parse_stored_theme_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_value: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::parse_stored_theme_state(raw_value)?;
        wire::write_value(w, out, &S_PARSE_STORED_THEME_STATE_OUT)?;
        Ok(())
    })
}

static S_SERIALIZE_THEME_STATE_0: wire::Schema = wire::Schema::Dyn;

/// `serializeThemeState` over the binary wire.
#[napi(js_name = "serializeThemeStateWire")]
pub fn serialize_theme_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_SERIALIZE_THEME_STATE_0)?;
        let out = &port::serialize_theme_state(state)?;
        w.string16(out);
        Ok(())
    })
}

static S_CREATE_THEME_SHARE_STRING_1: wire::Schema = wire::Schema::Dyn;

/// `createThemeShareString` over the binary wire.
#[napi(js_name = "createThemeShareStringWire")]
pub fn create_theme_share_string_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let variant: String = r.string16()?;
        let pack: Value = wire::read_value(r, &S_CREATE_THEME_SHARE_STRING_1)?;
        let out = &port::create_theme_share_string(variant, pack)?;
        w.string16(out);
        Ok(())
    })
}

static S_PARSE_THEME_SHARE_STRING_OUT: wire::Schema = wire::Schema::Obj(&[
    ("codeThemeId", wire::Schema::Str16),
    (
        "theme",
        wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            (
                "fonts",
                wire::Schema::Obj(&[
                    ("ui", wire::Schema::OptJs(&wire::Schema::Str16)),
                    ("code", wire::Schema::OptJs(&wire::Schema::Str16)),
                ]),
            ),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            (
                "semanticColors",
                wire::Schema::Obj(&[
                    ("diffAdded", wire::Schema::Str16),
                    ("diffRemoved", wire::Schema::Str16),
                    ("skill", wire::Schema::Str16),
                ]),
            ),
            ("surface", wire::Schema::Str16),
        ]),
    ),
    ("variant", wire::Schema::Str16),
]);

/// `parseThemeShareString` over the binary wire.
#[napi(js_name = "parseThemeShareStringWire")]
pub fn parse_theme_share_string_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_value: String = r.string16()?;
        let out = &port::parse_theme_share_string(raw_value)?;
        wire::write_value(w, out, &S_PARSE_THEME_SHARE_STRING_OUT)?;
        Ok(())
    })
}

/// `canParseThemeShareString` over the binary wire.
#[napi(js_name = "canParseThemeShareStringWire")]
pub fn can_parse_theme_share_string_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let target_variant: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::can_parse_theme_share_string(value, target_variant)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PARSE_THEME_SHARE_STRING_FOR_VARIANT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("codeThemeId", wire::Schema::Str16),
    (
        "theme",
        wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            (
                "fonts",
                wire::Schema::Obj(&[
                    ("ui", wire::Schema::OptJs(&wire::Schema::Str16)),
                    ("code", wire::Schema::OptJs(&wire::Schema::Str16)),
                ]),
            ),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            (
                "semanticColors",
                wire::Schema::Obj(&[
                    ("diffAdded", wire::Schema::Str16),
                    ("diffRemoved", wire::Schema::Str16),
                    ("skill", wire::Schema::Str16),
                ]),
            ),
            ("surface", wire::Schema::Str16),
        ]),
    ),
    ("variant", wire::Schema::Str16),
]);

/// `parseThemeShareStringForVariant` over the binary wire.
#[napi(js_name = "parseThemeShareStringForVariantWire")]
pub fn parse_theme_share_string_for_variant_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let target_variant: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::parse_theme_share_string_for_variant(value, target_variant)?;
        wire::write_value(w, out, &S_PARSE_THEME_SHARE_STRING_FOR_VARIANT_OUT)?;
        Ok(())
    })
}

static S_UPDATE_THEME_PACK_FROM_SHARE_STRING_0: wire::Schema = wire::Schema::Dyn;
static S_UPDATE_THEME_PACK_FROM_SHARE_STRING_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "chromeThemes",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            ("fonts", wire::Schema::Dyn),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            ("semanticColors", wire::Schema::Dyn),
            ("surface", wire::Schema::Str16),
        ])),
    ),
    ("codeThemeIds", wire::Schema::Map16(&wire::Schema::Str16)),
    ("mode", wire::Schema::Str16),
    ("systemUiFont", wire::Schema::Bool),
    (
        "translucency",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("opacity", wire::Schema::Num),
            ("blur", wire::Schema::OptJs(&wire::Schema::Num)),
            ("sidebarOnly", wire::Schema::Bool),
        ])),
    ),
]);

/// `updateThemePackFromShareString` over the binary wire.
#[napi(js_name = "updateThemePackFromShareStringWire")]
pub fn update_theme_pack_from_share_string_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_UPDATE_THEME_PACK_FROM_SHARE_STRING_0)?;
        let value: String = r.string16()?;
        let target_variant: String = r.string16()?;
        let out = &port::update_theme_pack_from_share_string(state, value, target_variant)?;
        wire::write_value(w, out, &S_UPDATE_THEME_PACK_FROM_SHARE_STRING_OUT)?;
        Ok(())
    })
}

static S_UPDATE_CHROME_THEME_0: wire::Schema = wire::Schema::Dyn;
static S_UPDATE_CHROME_THEME_2: wire::Schema = wire::Schema::Dyn;
static S_UPDATE_CHROME_THEME_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "chromeThemes",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            ("fonts", wire::Schema::Dyn),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            ("semanticColors", wire::Schema::Dyn),
            ("surface", wire::Schema::Str16),
        ])),
    ),
    ("codeThemeIds", wire::Schema::Map16(&wire::Schema::Str16)),
    ("mode", wire::Schema::Str16),
    ("systemUiFont", wire::Schema::Bool),
    (
        "translucency",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("opacity", wire::Schema::Num),
            ("blur", wire::Schema::OptJs(&wire::Schema::Num)),
            ("sidebarOnly", wire::Schema::Bool),
        ])),
    ),
]);

/// `updateChromeTheme` over the binary wire.
#[napi(js_name = "updateChromeThemeWire")]
pub fn update_chrome_theme_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_UPDATE_CHROME_THEME_0)?;
        let variant: String = r.string16()?;
        let patch: Value = wire::read_value(r, &S_UPDATE_CHROME_THEME_2)?;
        let out = &port::update_chrome_theme(state, variant, patch)?;
        wire::write_value(w, out, &S_UPDATE_CHROME_THEME_OUT)?;
        Ok(())
    })
}

static S_SET_THEME_CODE_THEME_ID_0: wire::Schema = wire::Schema::Dyn;
static S_SET_THEME_CODE_THEME_ID_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "chromeThemes",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            ("fonts", wire::Schema::Dyn),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            ("semanticColors", wire::Schema::Dyn),
            ("surface", wire::Schema::Str16),
        ])),
    ),
    ("codeThemeIds", wire::Schema::Map16(&wire::Schema::Str16)),
    ("mode", wire::Schema::Str16),
    ("systemUiFont", wire::Schema::Bool),
    (
        "translucency",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("opacity", wire::Schema::Num),
            ("blur", wire::Schema::OptJs(&wire::Schema::Num)),
            ("sidebarOnly", wire::Schema::Bool),
        ])),
    ),
]);

/// `setThemeCodeThemeId` over the binary wire.
#[napi(js_name = "setThemeCodeThemeIdWire")]
pub fn set_theme_code_theme_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_SET_THEME_CODE_THEME_ID_0)?;
        let variant: String = r.string16()?;
        let code_theme_id: String = r.string16()?;
        let out = &port::set_theme_code_theme_id(state, variant, code_theme_id)?;
        wire::write_value(w, out, &S_SET_THEME_CODE_THEME_ID_OUT)?;
        Ok(())
    })
}

static S_SET_THEME_FONTS_0: wire::Schema = wire::Schema::Dyn;
static S_SET_THEME_FONTS_2: wire::Schema = wire::Schema::Dyn;
static S_SET_THEME_FONTS_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "chromeThemes",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            ("fonts", wire::Schema::Dyn),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            ("semanticColors", wire::Schema::Dyn),
            ("surface", wire::Schema::Str16),
        ])),
    ),
    ("codeThemeIds", wire::Schema::Map16(&wire::Schema::Str16)),
    ("mode", wire::Schema::Str16),
    ("systemUiFont", wire::Schema::Bool),
    (
        "translucency",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("opacity", wire::Schema::Num),
            ("blur", wire::Schema::OptJs(&wire::Schema::Num)),
            ("sidebarOnly", wire::Schema::Bool),
        ])),
    ),
]);

/// `setThemeFonts` over the binary wire.
#[napi(js_name = "setThemeFontsWire")]
pub fn set_theme_fonts_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_SET_THEME_FONTS_0)?;
        let variant: String = r.string16()?;
        let patch: Value = wire::read_value(r, &S_SET_THEME_FONTS_2)?;
        let out = &port::set_theme_fonts(state, variant, patch)?;
        wire::write_value(w, out, &S_SET_THEME_FONTS_OUT)?;
        Ok(())
    })
}

static S_RESET_THEME_VARIANT_0: wire::Schema = wire::Schema::Dyn;
static S_RESET_THEME_VARIANT_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "chromeThemes",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            ("fonts", wire::Schema::Dyn),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            ("semanticColors", wire::Schema::Dyn),
            ("surface", wire::Schema::Str16),
        ])),
    ),
    ("codeThemeIds", wire::Schema::Map16(&wire::Schema::Str16)),
    ("mode", wire::Schema::Str16),
    ("systemUiFont", wire::Schema::Bool),
    (
        "translucency",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("opacity", wire::Schema::Num),
            ("blur", wire::Schema::OptJs(&wire::Schema::Num)),
            ("sidebarOnly", wire::Schema::Bool),
        ])),
    ),
]);

/// `resetThemeVariant` over the binary wire.
#[napi(js_name = "resetThemeVariantWire")]
pub fn reset_theme_variant_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_RESET_THEME_VARIANT_0)?;
        let variant: String = r.string16()?;
        let out = &port::reset_theme_variant(state, variant)?;
        wire::write_value(w, out, &S_RESET_THEME_VARIANT_OUT)?;
        Ok(())
    })
}

static S_SET_WINDOW_TRANSLUCENCY_0: wire::Schema = wire::Schema::Dyn;
static S_SET_WINDOW_TRANSLUCENCY_2: wire::Schema = wire::Schema::Dyn;
static S_SET_WINDOW_TRANSLUCENCY_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "chromeThemes",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            ("fonts", wire::Schema::Dyn),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            ("semanticColors", wire::Schema::Dyn),
            ("surface", wire::Schema::Str16),
        ])),
    ),
    ("codeThemeIds", wire::Schema::Map16(&wire::Schema::Str16)),
    ("mode", wire::Schema::Str16),
    ("systemUiFont", wire::Schema::Bool),
    (
        "translucency",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("opacity", wire::Schema::Num),
            ("blur", wire::Schema::OptJs(&wire::Schema::Num)),
            ("sidebarOnly", wire::Schema::Bool),
        ])),
    ),
]);

/// `setWindowTranslucency` over the binary wire.
#[napi(js_name = "setWindowTranslucencyWire")]
pub fn set_window_translucency_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_SET_WINDOW_TRANSLUCENCY_0)?;
        let variant: String = r.string16()?;
        let patch: Value = wire::read_value(r, &S_SET_WINDOW_TRANSLUCENCY_2)?;
        let out = &port::set_window_translucency(state, variant, patch)?;
        wire::write_value(w, out, &S_SET_WINDOW_TRANSLUCENCY_OUT)?;
        Ok(())
    })
}

static S_ARE_WINDOW_TRANSLUCENCIES_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_ARE_WINDOW_TRANSLUCENCIES_EQUAL_1: wire::Schema = wire::Schema::Dyn;

/// `areWindowTranslucenciesEqual` over the binary wire.
#[napi(js_name = "areWindowTranslucenciesEqualWire")]
pub fn are_window_translucencies_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Value = wire::read_value(r, &S_ARE_WINDOW_TRANSLUCENCIES_EQUAL_0)?;
        let right: Value = wire::read_value(r, &S_ARE_WINDOW_TRANSLUCENCIES_EQUAL_1)?;
        let out = &port::are_window_translucencies_equal(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_THEME_PACK_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_THEME_PACK_OUT: wire::Schema = wire::Schema::Obj(&[
    ("codeThemeId", wire::Schema::Str16),
    (
        "theme",
        wire::Schema::Obj(&[
            ("accent", wire::Schema::Str16),
            ("contrast", wire::Schema::Num),
            (
                "fonts",
                wire::Schema::Obj(&[
                    ("ui", wire::Schema::OptJs(&wire::Schema::Str16)),
                    ("code", wire::Schema::OptJs(&wire::Schema::Str16)),
                ]),
            ),
            ("ink", wire::Schema::Str16),
            ("opaqueWindows", wire::Schema::Bool),
            (
                "semanticColors",
                wire::Schema::Obj(&[
                    ("diffAdded", wire::Schema::Str16),
                    ("diffRemoved", wire::Schema::Str16),
                    ("skill", wire::Schema::Str16),
                ]),
            ),
            ("surface", wire::Schema::Str16),
        ]),
    ),
]);

/// `resolveThemePack` over the binary wire.
#[napi(js_name = "resolveThemePackWire")]
pub fn resolve_theme_pack_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_RESOLVE_THEME_PACK_0)?;
        let variant: String = r.string16()?;
        let out = &port::resolve_theme_pack(state, variant)?;
        wire::write_value(w, out, &S_RESOLVE_THEME_PACK_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_PROFILE_THEME_ACCENT_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_PROFILE_THEME_ACCENT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("light", wire::Schema::Str16),
    ("dark", wire::Schema::Str16),
]);

/// `resolveProfileThemeAccent` over the binary wire.
#[napi(js_name = "resolveProfileThemeAccentWire")]
pub fn resolve_profile_theme_accent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_RESOLVE_PROFILE_THEME_ACCENT_0)?;
        let out = &port::resolve_profile_theme_accent(state)?;
        wire::write_value(w, out, &S_RESOLVE_PROFILE_THEME_ACCENT_OUT)?;
        Ok(())
    })
}

static S_ARE_THEME_PACKS_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_ARE_THEME_PACKS_EQUAL_1: wire::Schema = wire::Schema::Dyn;

/// `areThemePacksEqual` over the binary wire.
#[napi(js_name = "areThemePacksEqualWire")]
pub fn are_theme_packs_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: Value = wire::read_value(r, &S_ARE_THEME_PACKS_EQUAL_0)?;
        let right: Value = wire::read_value(r, &S_ARE_THEME_PACKS_EQUAL_1)?;
        let out = &port::are_theme_packs_equal(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

/// `resolveThemeVariant` over the binary wire.
#[napi(js_name = "resolveThemeVariantWire")]
pub fn resolve_theme_variant_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let mode: String = r.string16()?;
        let system_dark: bool = r.bool()?;
        let out = &port::resolve_theme_variant(mode, system_dark)?;
        w.string16(out);
        Ok(())
    })
}

static S_BUILD_RESOLVED_THEME_TOKENS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_RESOLVED_THEME_TOKENS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("aliases", wire::Schema::Map16(&wire::Schema::Str16)),
    ("codexVariables", wire::Schema::Map16(&wire::Schema::Str16)),
    (
        "computed",
        wire::Schema::Obj(&[
            ("contrast", wire::Schema::Num),
            ("editorBackground", wire::Schema::Str16),
            ("panel", wire::Schema::Str16),
            ("surfaceUnder", wire::Schema::Str16),
        ]),
    ),
    (
        "derived",
        wire::Schema::Obj(&[
            ("accentBackground", wire::Schema::Str16),
            ("accentBackgroundActive", wire::Schema::Str16),
            ("accentBackgroundHover", wire::Schema::Str16),
            ("border", wire::Schema::Str16),
            ("borderFocus", wire::Schema::Str16),
            ("borderHeavy", wire::Schema::Str16),
            ("borderLight", wire::Schema::Str16),
            ("buttonPrimaryBackground", wire::Schema::Str16),
            ("buttonPrimaryBackgroundActive", wire::Schema::Str16),
            ("buttonPrimaryBackgroundHover", wire::Schema::Str16),
            ("buttonPrimaryBackgroundInactive", wire::Schema::Str16),
            ("buttonSecondaryBackground", wire::Schema::Str16),
            ("buttonSecondaryBackgroundActive", wire::Schema::Str16),
            ("buttonSecondaryBackgroundHover", wire::Schema::Str16),
            ("buttonSecondaryBackgroundInactive", wire::Schema::Str16),
            ("buttonTertiaryBackground", wire::Schema::Str16),
            ("buttonTertiaryBackgroundActive", wire::Schema::Str16),
            ("buttonTertiaryBackgroundHover", wire::Schema::Str16),
            ("controlBackground", wire::Schema::Str16),
            ("controlBackgroundOpaque", wire::Schema::Str16),
            ("elevatedPrimary", wire::Schema::Str16),
            ("elevatedPrimaryOpaque", wire::Schema::Str16),
            ("elevatedSecondary", wire::Schema::Str16),
            ("elevatedSecondaryOpaque", wire::Schema::Str16),
            ("iconAccent", wire::Schema::Str16),
            ("iconPrimary", wire::Schema::Str16),
            ("iconSecondary", wire::Schema::Str16),
            ("iconTertiary", wire::Schema::Str16),
            ("simpleScrim", wire::Schema::Str16),
            ("textAccent", wire::Schema::Str16),
            ("textButtonPrimary", wire::Schema::Str16),
            ("textButtonSecondary", wire::Schema::Str16),
            ("textButtonTertiary", wire::Schema::Str16),
            ("textForeground", wire::Schema::Str16),
            ("textForegroundSecondary", wire::Schema::Str16),
            ("textForegroundTertiary", wire::Schema::Str16),
        ]),
    ),
]);

/// `buildResolvedThemeTokens` over the binary wire.
#[napi(js_name = "buildResolvedThemeTokensWire")]
pub fn build_resolved_theme_tokens_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pack: Value = wire::read_value(r, &S_BUILD_RESOLVED_THEME_TOKENS_0)?;
        let variant: String = r.string16()?;
        let out = &port::build_resolved_theme_tokens(pack, variant)?;
        wire::write_value(w, out, &S_BUILD_RESOLVED_THEME_TOKENS_OUT)?;
        Ok(())
    })
}

static S_BUILD_THEME_CSS_VARIABLES_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_THEME_CSS_VARIABLES_OUT: wire::Schema = wire::Schema::Obj(&[
    ("material", wire::Schema::Str16),
    ("translucencyScope", wire::Schema::Str16),
    ("variables", wire::Schema::Map16(&wire::Schema::Str16)),
]);

/// `buildThemeCssVariables` over the binary wire.
#[napi(js_name = "buildThemeCssVariablesWire")]
pub fn build_theme_css_variables_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pack: Value = wire::read_value(r, &S_BUILD_THEME_CSS_VARIABLES_0)?;
        let variant: String = r.string16()?;
        let options: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::build_theme_css_variables(pack, variant, options)?;
        wire::write_value(w, out, &S_BUILD_THEME_CSS_VARIABLES_OUT)?;
        Ok(())
    })
}
