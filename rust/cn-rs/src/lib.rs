//! JS bindings for `packages/profile-ui/src/cn.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use wasm_bindgen::prelude::*;

pub mod js;
pub mod port;

/// `cn`.
fn cn_inner(inputs: String) -> Result<String, String> {
    let inputs = crate::js::from_json(&inputs)?;
    let out = port::cn(inputs)?;
    crate::js::to_json(&out)
}

#[wasm_bindgen(js_name = "cn")]
pub fn cn(inputs: String) -> Result<String, JsError> {
    cn_inner(inputs).map_err(|e| JsError::new(&e))
}
