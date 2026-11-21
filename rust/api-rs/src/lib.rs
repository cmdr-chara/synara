//! JS bindings for `apps/analytics/src/api.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use wasm_bindgen::prelude::*;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

/// `pickFilters`.
fn pick_filters_inner(f: String, fields: String) -> Result<String, String> {
    let f = crate::js::from_json(&f)?;
    let fields = crate::js::from_json(&fields)?;
    let out = port::pick_filters(f, fields)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "pickFilters")]
pub fn pick_filters(f: String, fields: String) -> Result<String, JsError> {
    pick_filters_inner(f, fields).map_err(|e| JsError::new(&e))
}

/// `filtersQuery`.
fn filters_query_inner(f: String) -> Result<String, String> {
    let f = crate::js::from_json(&f)?;
    let out = port::filters_query(f)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "filtersQuery")]
pub fn filters_query(f: String) -> Result<String, JsError> {
    filters_query_inner(f).map_err(|e| JsError::new(&e))
}

/// `loginErrorMessage`.
fn login_error_message_inner(res: String) -> Result<String, String> {
    let res = crate::js::from_json(&res)?;
    let out = port::login_error_message(res)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "loginErrorMessage")]
pub fn login_error_message(res: String) -> Result<String, JsError> {
    login_error_message_inner(res).map_err(|e| JsError::new(&e))
}

/// `productFiltersQuery`.
fn product_filters_query_inner(f: String) -> Result<String, String> {
    let f = crate::js::from_json(&f)?;
    let out = port::product_filters_query(f)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "productFiltersQuery")]
pub fn product_filters_query(f: String) -> Result<String, JsError> {
    product_filters_query_inner(f).map_err(|e| JsError::new(&e))
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

static S_PICK_FILTERS_0: wire::Schema = wire::Schema::Dyn;
static S_PICK_FILTERS_1: wire::Schema = wire::Schema::Dyn;
static S_PICK_FILTERS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("days", wire::Schema::Num),
    ("version", wire::Schema::Str16),
    ("platform", wire::Schema::Str16),
    ("flavor", wire::Schema::Str16),
    ("kind", wire::Schema::Str16),
    ("q", wire::Schema::Str16),
]);

/// `pickFilters` over the binary wire.
#[wasm_bindgen(js_name = "pickFiltersWire")]
pub fn pick_filters_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let f: serde_json::Value = wire::read_value(r, &S_PICK_FILTERS_0)?;
        let fields: serde_json::Value = wire::read_value(r, &S_PICK_FILTERS_1)?;
        let out = &port::pick_filters(f, fields)?;
        wire::write_value(w, out, &S_PICK_FILTERS_OUT)?;
        Ok(())
    })
}

static S_FILTERS_QUERY_0: wire::Schema = wire::Schema::Dyn;

/// `filtersQuery` over the binary wire.
#[wasm_bindgen(js_name = "filtersQueryWire")]
pub fn filters_query_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let f: serde_json::Value = wire::read_value(r, &S_FILTERS_QUERY_0)?;
        let out = &port::filters_query(f)?;
        w.string16(out);
        Ok(())
    })
}

static S_LOGIN_ERROR_MESSAGE_0: wire::Schema = wire::Schema::Dyn;

/// `loginErrorMessage` over the binary wire.
#[wasm_bindgen(js_name = "loginErrorMessageWire")]
pub fn login_error_message_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let res: serde_json::Value = wire::read_value(r, &S_LOGIN_ERROR_MESSAGE_0)?;
        let out = &port::login_error_message(res)?;
        w.string16(out);
        Ok(())
    })
}

static S_PRODUCT_FILTERS_QUERY_0: wire::Schema = wire::Schema::Dyn;

/// `productFiltersQuery` over the binary wire.
#[wasm_bindgen(js_name = "productFiltersQueryWire")]
pub fn product_filters_query_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let f: serde_json::Value = wire::read_value(r, &S_PRODUCT_FILTERS_QUERY_0)?;
        let out = &port::product_filters_query(f)?;
        w.string16(out);
        Ok(())
    })
}
