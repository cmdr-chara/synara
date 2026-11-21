//! JS bindings for `packages/profile-ui/src/formatting.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use wasm_bindgen::prelude::*;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;
pub mod utf16;

/// `formatCompact`.
fn format_compact_inner(value: Option<f64>, locale: Option<String>) -> Result<String, String> {
    let out = port::format_compact(value, locale)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "formatCompact")]
pub fn format_compact(value: Option<f64>, locale: Option<String>) -> Result<String, JsError> {
    format_compact_inner(value, locale).map_err(|e| JsError::new(&e))
}

/// `formatNumber`.
fn format_number_inner(value: Option<f64>, locale: Option<String>) -> Result<String, String> {
    let out = port::format_number(value, locale)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "formatNumber")]
pub fn format_number(value: Option<f64>, locale: Option<String>) -> Result<String, JsError> {
    format_number_inner(value, locale).map_err(|e| JsError::new(&e))
}

/// `formatDays`.
fn format_days_inner(value: f64, locale: Option<String>) -> Result<String, String> {
    let out = port::format_days(value, locale)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "formatDays")]
pub fn format_days(value: f64, locale: Option<String>) -> Result<String, JsError> {
    format_days_inner(value, locale).map_err(|e| JsError::new(&e))
}

/// `toDisplayName`.
fn to_display_name_inner(basename: String) -> Result<String, String> {
    let out = port::to_display_name(basename)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "toDisplayName")]
pub fn to_display_name(basename: String) -> Result<String, JsError> {
    to_display_name_inner(basename).map_err(|e| JsError::new(&e))
}

/// `normalizeHandle`.
fn normalize_handle_inner(value: String) -> Result<String, String> {
    let out = port::normalize_handle(value)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "normalizeHandle")]
pub fn normalize_handle(value: String) -> Result<String, JsError> {
    normalize_handle_inner(value).map_err(|e| JsError::new(&e))
}

/// `formatShortDate`.
fn format_short_date_inner(day: String, locale: Option<String>) -> Result<String, String> {
    let day = crate::js::from_json(&day)?;
    let out = port::format_short_date(day, locale)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "formatShortDate")]
pub fn format_short_date(day: String, locale: Option<String>) -> Result<String, JsError> {
    format_short_date_inner(day, locale).map_err(|e| JsError::new(&e))
}

/// `formatHourLabel`.
fn format_hour_label_inner(hour: f64, locale: Option<String>) -> Result<String, String> {
    let out = port::format_hour_label(hour, locale)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "formatHourLabel")]
pub fn format_hour_label(hour: f64, locale: Option<String>) -> Result<String, JsError> {
    format_hour_label_inner(hour, locale).map_err(|e| JsError::new(&e))
}

/// `formatProviderLabel`.
fn format_provider_label_inner(provider: String) -> Result<String, String> {
    let provider = crate::js::from_json(&provider)?;
    let out = port::format_provider_label(provider)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "formatProviderLabel")]
pub fn format_provider_label(provider: String) -> Result<String, JsError> {
    format_provider_label_inner(provider).map_err(|e| JsError::new(&e))
}

/// `formatProfileUsageBasis`.
fn format_profile_usage_basis_inner(metric: String) -> Result<String, String> {
    let out = port::format_profile_usage_basis(metric)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "formatProfileUsageBasis")]
pub fn format_profile_usage_basis(metric: String) -> Result<String, JsError> {
    format_profile_usage_basis_inner(metric).map_err(|e| JsError::new(&e))
}

/// `deriveInitials`.
fn derive_initials_inner(name: String) -> Result<String, String> {
    let out = port::derive_initials(name)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "deriveInitials")]
pub fn derive_initials(name: String) -> Result<String, JsError> {
    derive_initials_inner(name).map_err(|e| JsError::new(&e))
}

// ---- the binary wire ------------------------------------------------------------------------

thread_local! {
    static WIRE_IN: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static WIRE_OUT: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// This module's memory, where `index.cjs` reads and writes the wire buffers.
#[wasm_bindgen(js_name = "rustAnythingWireMemory")]
pub fn rust_anything_wire_memory() -> JsValue {
    wasm_bindgen::memory()
}

/// The address of an argument buffer of `len` bytes.
#[wasm_bindgen(js_name = "rustAnythingWireIn")]
pub fn rust_anything_wire_in(len: u32) -> u32 {
    WIRE_IN.with(|b| {
        let mut b = b.borrow_mut();
        b.resize(len as usize, 0);
        b.as_mut_ptr() as usize as u32
    })
}

/// The address of the last result.
#[wasm_bindgen(js_name = "rustAnythingWireOut")]
pub fn rust_anything_wire_out() -> u32 {
    WIRE_OUT.with(|b| b.borrow().as_ptr() as usize as u32)
}

/// Reads `len` argument bytes, runs `f`, and keeps its result for `rustAnythingWireOut`.
fn wire_io(
    len: u32,
    f: impl FnOnce(&mut wire::Reader, &mut wire::Writer) -> Result<(), String>,
) -> Result<u32, JsError> {
    let input = WIRE_IN.with(|b| std::mem::take(&mut *b.borrow_mut()));
    let mut w = wire::Writer::default();
    let result = f(
        &mut wire::Reader::new(&input[..(len as usize).min(input.len())]),
        &mut w,
    );
    WIRE_IN.with(|b| *b.borrow_mut() = input);
    match result {
        // A result of another shape than the TS type: JS runs the call through JSON.
        Err(e) if e.starts_with(wire::MISMATCH) => return Ok(u32::MAX),
        other => other.map_err(|e| JsError::new(&e))?,
    }
    let n = w.0.len();
    WIRE_OUT.with(|b| *b.borrow_mut() = w.0);
    Ok(u32::try_from(n).unwrap_or(u32::MAX))
}

static S_FORMAT_SHORT_DATE_0: wire::Schema = wire::Schema::Dyn;
static S_FORMAT_SHORT_DATE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `formatShortDate` over the binary wire.
#[wasm_bindgen(js_name = "formatShortDateWire")]
pub fn format_short_date_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let day: serde_json::Value = wire::read_value(r, &S_FORMAT_SHORT_DATE_0)?;
        let locale: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::format_short_date(day, locale)?;
        wire::write_value(w, out, &S_FORMAT_SHORT_DATE_OUT)?;
        Ok(())
    })
}

static S_FORMAT_PROVIDER_LABEL_0: wire::Schema = wire::Schema::Dyn;
static S_FORMAT_PROVIDER_LABEL_OUT: wire::Schema = wire::Schema::Str16;

/// `formatProviderLabel` over the binary wire.
#[wasm_bindgen(js_name = "formatProviderLabelWire")]
pub fn format_provider_label_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let provider: serde_json::Value = wire::read_value(r, &S_FORMAT_PROVIDER_LABEL_0)?;
        let out = &port::format_provider_label(provider)?;
        wire::write_value(w, out, &S_FORMAT_PROVIDER_LABEL_OUT)?;
        Ok(())
    })
}

static S_DERIVE_INITIALS_OUT: wire::Schema = wire::Schema::Str16;

/// `deriveInitials` over the binary wire.
#[wasm_bindgen(js_name = "deriveInitialsWire")]
pub fn derive_initials_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let name: String = r.string16()?;
        let out = &port::derive_initials(name)?;
        wire::write_value(w, out, &S_DERIVE_INITIALS_OUT)?;
        Ok(())
    })
}
