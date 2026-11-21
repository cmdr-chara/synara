//! JS bindings for `packages/shared/src/browserSession.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveFloatingBrowserGuestLayout`.
#[napi(js_name = "resolveFloatingBrowserGuestLayout")]
pub fn resolve_floating_browser_guest_layout(slot: String) -> napi::Result<String> {
    run(move || {
        let slot = crate::js::from_json(&slot)?;
        let out = port::resolve_floating_browser_guest_layout(slot)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeBrowserPageZoomFactor`.
#[napi(js_name = "normalizeBrowserPageZoomFactor")]
pub fn normalize_browser_page_zoom_factor(value: String) -> napi::Result<f64> {
    run(move || {
        let value = crate::js::from_json(&value)?;
        let out = port::normalize_browser_page_zoom_factor(value)?;
        Ok(out)
    })
}

/// `normalizeBrowserUrlInput`.
#[napi(js_name = "normalizeBrowserUrlInput")]
pub fn normalize_browser_url_input(input: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_browser_url_input(input)?;
        Ok(out)
    })
}

/// `resolveCopyableBrowserTabUrl`.
#[napi(js_name = "resolveCopyableBrowserTabUrl")]
pub fn resolve_copyable_browser_tab_url(
    tab: String,
    live_url: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let tab = crate::js::from_json(&tab)?;
        let out = port::resolve_copyable_browser_tab_url(tab, live_url)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `isBlankBrowserTabUrl`.
#[napi(js_name = "isBlankBrowserTabUrl")]
pub fn is_blank_browser_tab_url(tab: String) -> napi::Result<bool> {
    run(move || {
        let tab = crate::js::from_json(&tab)?;
        let out = port::is_blank_browser_tab_url(tab)?;
        Ok(out)
    })
}

/// `classifyBrowserWindowOpen`.
#[napi(js_name = "classifyBrowserWindowOpen")]
pub fn classify_browser_window_open(intent: String) -> napi::Result<String> {
    run(move || {
        let intent = crate::js::from_json(&intent)?;
        let out = port::classify_browser_window_open(intent)?;
        Ok(out)
    })
}

/// `deriveChromeUserAgent`.
#[napi(js_name = "deriveChromeUserAgent")]
pub fn derive_chrome_user_agent(
    base_user_agent: String,
    app_product_tokens: String,
) -> napi::Result<String> {
    run(move || {
        let app_product_tokens = crate::js::from_json(&app_product_tokens)?;
        let out = port::derive_chrome_user_agent(base_user_agent, app_product_tokens)?;
        Ok(out)
    })
}

/// `chromeClientHintPlatform`.
#[napi(js_name = "chromeClientHintPlatform")]
pub fn chrome_client_hint_platform(platform: String) -> napi::Result<String> {
    run(move || {
        let out = port::chrome_client_hint_platform(platform)?;
        Ok(out)
    })
}

/// `buildChromeClientHints`.
#[napi(js_name = "buildChromeClientHints")]
pub fn build_chrome_client_hints(user_agent: String, platform: String) -> napi::Result<String> {
    run(move || {
        let out = port::build_chrome_client_hints(user_agent, platform)?;
        crate::js::to_json(&out)
    })
}

/// `buildAcceptLanguageHeader`.
#[napi(js_name = "buildAcceptLanguageHeader")]
pub fn build_accept_language_header(
    languages: Vec<String>,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::build_accept_language_header(languages)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
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

static S_RESOLVE_FLOATING_BROWSER_GUEST_LAYOUT_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_FLOATING_BROWSER_GUEST_LAYOUT_OUT: wire::Schema = wire::Schema::Obj(&[
    ("width", wire::Schema::Num),
    ("height", wire::Schema::Num),
    ("scale", wire::Schema::Num),
    ("x", wire::Schema::Num),
    ("y", wire::Schema::Num),
]);

/// `resolveFloatingBrowserGuestLayout` over the binary wire.
#[napi(js_name = "resolveFloatingBrowserGuestLayoutWire")]
pub fn resolve_floating_browser_guest_layout_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let slot: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_FLOATING_BROWSER_GUEST_LAYOUT_0)?;
        let out = &port::resolve_floating_browser_guest_layout(slot)?;
        wire::write_value(w, out, &S_RESOLVE_FLOATING_BROWSER_GUEST_LAYOUT_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_BROWSER_PAGE_ZOOM_FACTOR_0: wire::Schema = wire::Schema::Dyn;

/// `normalizeBrowserPageZoomFactor` over the binary wire.
#[napi(js_name = "normalizeBrowserPageZoomFactorWire")]
pub fn normalize_browser_page_zoom_factor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_BROWSER_PAGE_ZOOM_FACTOR_0)?;
        let out = &port::normalize_browser_page_zoom_factor(value)?;
        w.f64(*out);
        Ok(())
    })
}

/// `normalizeBrowserUrlInput` over the binary wire.
#[napi(js_name = "normalizeBrowserUrlInputWire")]
pub fn normalize_browser_url_input_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::normalize_browser_url_input(input)?;
        w.string16(out);
        Ok(())
    })
}

/// `resolveCopyableBrowserTabUrl` over the binary wire.
#[napi(js_name = "resolveCopyableBrowserTabUrlWire")]
pub fn resolve_copyable_browser_tab_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tab: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let live_url: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::resolve_copyable_browser_tab_url(tab, live_url)?;
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

/// `isBlankBrowserTabUrl` over the binary wire.
#[napi(js_name = "isBlankBrowserTabUrlWire")]
pub fn is_blank_browser_tab_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tab: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_blank_browser_tab_url(tab)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CLASSIFY_BROWSER_WINDOW_OPEN_0: wire::Schema = wire::Schema::Dyn;

/// `classifyBrowserWindowOpen` over the binary wire.
#[napi(js_name = "classifyBrowserWindowOpenWire")]
pub fn classify_browser_window_open_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let intent: serde_json::Value = wire::read_value(r, &S_CLASSIFY_BROWSER_WINDOW_OPEN_0)?;
        let out = &port::classify_browser_window_open(intent)?;
        w.string16(out);
        Ok(())
    })
}

/// `deriveChromeUserAgent` over the binary wire.
#[napi(js_name = "deriveChromeUserAgentWire")]
pub fn derive_chrome_user_agent_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let base_user_agent: String = r.string16()?;
        let app_product_tokens: Option<Vec<String>> = r.opt_js(|r| r.list(|r| r.string16()))?;
        let out = &port::derive_chrome_user_agent(base_user_agent, app_product_tokens)?;
        w.string16(out);
        Ok(())
    })
}

/// `chromeClientHintPlatform` over the binary wire.
#[napi(js_name = "chromeClientHintPlatformWire")]
pub fn chrome_client_hint_platform_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: String = r.string16()?;
        let out = &port::chrome_client_hint_platform(platform)?;
        w.string16(out);
        Ok(())
    })
}

static S_BUILD_CHROME_CLIENT_HINTS_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("sec-ch-ua", wire::Schema::Str16),
    ("sec-ch-ua-mobile", wire::Schema::Str16),
    ("sec-ch-ua-platform", wire::Schema::Str16),
]));

/// `buildChromeClientHints` over the binary wire.
#[napi(js_name = "buildChromeClientHintsWire")]
pub fn build_chrome_client_hints_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let user_agent: String = r.string16()?;
        let platform: String = r.string16()?;
        let out = &port::build_chrome_client_hints(user_agent, platform)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_BUILD_CHROME_CLIENT_HINTS_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `buildAcceptLanguageHeader` over the binary wire.
#[napi(js_name = "buildAcceptLanguageHeaderWire")]
pub fn build_accept_language_header_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let languages: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::build_accept_language_header(languages)?;
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
