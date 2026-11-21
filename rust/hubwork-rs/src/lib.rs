//! JS bindings for `packages/contracts/src/hubWork.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use wasm_bindgen::prelude::*;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

/// `hubWorkItem`.
fn hub_work_item_inner(record: String) -> Result<String, String> {
    let record = crate::js::from_json(&record)?;
    let out = port::hub_work_item(record)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "hubWorkItem")]
pub fn hub_work_item(record: String) -> Result<String, JsError> {
    hub_work_item_inner(record).map_err(|e| JsError::new(&e))
}

/// `hubWorkLogicalTaskKey`.
fn hub_work_logical_task_key_inner(record: String) -> Result<String, String> {
    let record = crate::js::from_json(&record)?;
    let out = port::hub_work_logical_task_key(record)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "hubWorkLogicalTaskKey")]
pub fn hub_work_logical_task_key(record: String) -> Result<String, JsError> {
    hub_work_logical_task_key_inner(record).map_err(|e| JsError::new(&e))
}

/// `hubWorkPositionalTaskKey`.
fn hub_work_positional_task_key_inner(position: String) -> Result<String, String> {
    let position = crate::js::from_json(&position)?;
    let out = port::hub_work_positional_task_key(position)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "hubWorkPositionalTaskKey")]
pub fn hub_work_positional_task_key(position: String) -> Result<String, JsError> {
    hub_work_positional_task_key_inner(position).map_err(|e| JsError::new(&e))
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

static S_HUB_WORK_ITEM_0: wire::Schema = wire::Schema::Dyn;
static S_HUB_WORK_ITEM_OUT: wire::Schema = wire::Schema::Dyn;

/// `hubWorkItem` over the binary wire.
#[wasm_bindgen(js_name = "hubWorkItemWire")]
pub fn hub_work_item_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let record: Value = wire::read_value(r, &S_HUB_WORK_ITEM_0)?;
        let out = &port::hub_work_item(record)?;
        wire::write_value(w, out, &S_HUB_WORK_ITEM_OUT)?;
        Ok(())
    })
}

static S_HUB_WORK_LOGICAL_TASK_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_HUB_WORK_LOGICAL_TASK_KEY_OUT: wire::Schema = wire::Schema::Str16;

/// `hubWorkLogicalTaskKey` over the binary wire.
#[wasm_bindgen(js_name = "hubWorkLogicalTaskKeyWire")]
pub fn hub_work_logical_task_key_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let record: Value = wire::read_value(r, &S_HUB_WORK_LOGICAL_TASK_KEY_0)?;
        let out = &port::hub_work_logical_task_key(record)?;
        wire::write_value(w, out, &S_HUB_WORK_LOGICAL_TASK_KEY_OUT)?;
        Ok(())
    })
}

static S_HUB_WORK_POSITIONAL_TASK_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_HUB_WORK_POSITIONAL_TASK_KEY_OUT: wire::Schema = wire::Schema::Str16;

/// `hubWorkPositionalTaskKey` over the binary wire.
#[wasm_bindgen(js_name = "hubWorkPositionalTaskKeyWire")]
pub fn hub_work_positional_task_key_wire(len: u32) -> Result<u32, JsError> {
    wire_io(len, |r, w| {
        let position: Value = wire::read_value(r, &S_HUB_WORK_POSITIONAL_TASK_KEY_0)?;
        let out = &port::hub_work_positional_task_key(position)?;
        wire::write_value(w, out, &S_HUB_WORK_POSITIONAL_TASK_KEY_OUT)?;
        Ok(())
    })
}
