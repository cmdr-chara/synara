//! JS bindings for `apps/web/src/providerOrdering.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isProviderKind`.
#[napi(js_name = "isProviderKind")]
pub fn is_provider_kind(value: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_provider_kind(value)?;
        Ok(out)
    })
}

/// `normalizeHiddenProviders`.
#[napi(js_name = "normalizeHiddenProviders")]
pub fn normalize_hidden_providers(hidden_providers: Vec<String>) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_hidden_providers(hidden_providers)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeProviderOrder`.
#[napi(js_name = "normalizeProviderOrder")]
pub fn normalize_provider_order(provider_order: Vec<String>) -> napi::Result<String> {
    run(move || {
        let out = port::normalize_provider_order(provider_order)?;
        crate::js::to_json(&out)
    })
}

/// `sameProviderOrder`.
#[napi(js_name = "sameProviderOrder")]
pub fn same_provider_order(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::same_provider_order(left, right)?;
        Ok(out)
    })
}

/// `compareProvidersByOrder`.
#[napi(js_name = "compareProvidersByOrder")]
pub fn compare_providers_by_order(
    provider_order: String,
    left: String,
    right: String,
) -> napi::Result<f64> {
    run(move || {
        let provider_order = crate::js::from_json(&provider_order)?;
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::compare_providers_by_order(provider_order, left, right)?;
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

/// `isProviderKind` over the binary wire.
#[napi(js_name = "isProviderKindWire")]
pub fn is_provider_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let value: String = r.string16()?;
        let out = &port::is_provider_kind(value)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `normalizeHiddenProviders` over the binary wire.
#[napi(js_name = "normalizeHiddenProvidersWire")]
pub fn normalize_hidden_providers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let hidden_providers: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::normalize_hidden_providers(hidden_providers)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

/// `normalizeProviderOrder` over the binary wire.
#[napi(js_name = "normalizeProviderOrderWire")]
pub fn normalize_provider_order_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider_order: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::normalize_provider_order(provider_order)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SAME_PROVIDER_ORDER_0: wire::Schema = wire::Schema::Dyn;
static S_SAME_PROVIDER_ORDER_1: wire::Schema = wire::Schema::Dyn;

/// `sameProviderOrder` over the binary wire.
#[napi(js_name = "sameProviderOrderWire")]
pub fn same_provider_order_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: serde_json::Value = wire::read_value(r, &S_SAME_PROVIDER_ORDER_0)?;
        let right: serde_json::Value = wire::read_value(r, &S_SAME_PROVIDER_ORDER_1)?;
        let out = &port::same_provider_order(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPARE_PROVIDERS_BY_ORDER_0: wire::Schema = wire::Schema::Dyn;
static S_COMPARE_PROVIDERS_BY_ORDER_1: wire::Schema = wire::Schema::Dyn;
static S_COMPARE_PROVIDERS_BY_ORDER_2: wire::Schema = wire::Schema::Dyn;

/// `compareProvidersByOrder` over the binary wire.
#[napi(js_name = "compareProvidersByOrderWire")]
pub fn compare_providers_by_order_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider_order: serde_json::Value =
            wire::read_value(r, &S_COMPARE_PROVIDERS_BY_ORDER_0)?;
        let left: serde_json::Value = wire::read_value(r, &S_COMPARE_PROVIDERS_BY_ORDER_1)?;
        let right: serde_json::Value = wire::read_value(r, &S_COMPARE_PROVIDERS_BY_ORDER_2)?;
        let out = &port::compare_providers_by_order(provider_order, left, right)?;
        w.f64(*out);
        Ok(())
    })
}
