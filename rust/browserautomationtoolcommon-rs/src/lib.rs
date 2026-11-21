//! JS bindings for `packages/contracts/src/browserAutomationToolCommon.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use wasm_bindgen::prelude::*;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

/// `browserClosedStruct`.
fn browser_closed_struct_inner(fields: String) -> Result<String, String> {
    let fields = crate::js::from_json(&fields)?;
    let out = port::browser_closed_struct(fields)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "browserClosedStruct")]
pub fn browser_closed_struct(fields: String) -> Result<String, JsError> {
    browser_closed_struct_inner(fields).map_err(|e| JsError::new(&e))
}

/// `browserBoundedInt`.
fn browser_bounded_int_inner(minimum: f64, maximum: f64) -> Result<String, String> {
    let out = port::browser_bounded_int(minimum, maximum)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "browserBoundedInt")]
pub fn browser_bounded_int(minimum: f64, maximum: f64) -> Result<String, JsError> {
    browser_bounded_int_inner(minimum, maximum).map_err(|e| JsError::new(&e))
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

static S_BROWSER_CLOSED_STRUCT_0: wire::Schema = wire::Schema::Dyn;
static S_BROWSER_CLOSED_STRUCT_OUT: wire::Schema = wire::Schema::Dyn;

/// `browserClosedStruct` over the binary wire.
#[wasm_bindgen(js_name = "browserClosedStructWire")]
pub fn browser_closed_struct_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let fields: serde_json::Value = wire::read_value(r, &S_BROWSER_CLOSED_STRUCT_0)?;
        let out = &port::browser_closed_struct(fields)?;
        wire::write_value(w, out, &S_BROWSER_CLOSED_STRUCT_OUT)?;
        Ok(())
    })
}

static S_BROWSER_BOUNDED_INT_OUT: wire::Schema = wire::Schema::Dyn;

/// `browserBoundedInt` over the binary wire.
#[wasm_bindgen(js_name = "browserBoundedIntWire")]
pub fn browser_bounded_int_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let minimum: f64 = r.f64()?;
        let maximum: f64 = r.f64()?;
        let out = &port::browser_bounded_int(minimum, maximum)?;
        wire::write_value(w, out, &S_BROWSER_BOUNDED_INT_OUT)?;
        Ok(())
    })
}
