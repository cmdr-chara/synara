//! JS bindings for `packages/contracts/src/browserAutomationCssSelector.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use wasm_bindgen::prelude::*;

pub mod port;
pub mod utf16;

/// `isBrowserCssSelector`.
fn is_browser_css_selector_inner(value: String) -> Result<bool, String> {
    let out = port::is_browser_css_selector(value)?;
    Ok(out)
}

#[wasm_bindgen(js_name = "isBrowserCssSelector")]
pub fn is_browser_css_selector(value: String) -> Result<bool, JsError> {
    is_browser_css_selector_inner(value).map_err(|e| JsError::new(&e))
}
