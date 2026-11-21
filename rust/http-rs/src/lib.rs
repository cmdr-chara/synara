//! JS bindings for `apps/server/src/http.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `makeEffectHttpRouteLayer`.
#[napi(js_name = "makeEffectHttpRouteLayer")]
pub fn make_effect_http_route_layer(
    readiness: String,
    shutdown_controller: String,
) -> napi::Result<String> {
    run(move || {
        let readiness = crate::js::from_json(&readiness)?;
        let shutdown_controller = crate::js::from_json(&shutdown_controller)?;
        let out = port::make_effect_http_route_layer(readiness, shutdown_controller)?;
        crate::js::to_json(&out)
    })
}

/// `makeDesktopShutdownEffectRouteLayer`.
#[napi(js_name = "makeDesktopShutdownEffectRouteLayer")]
pub fn make_desktop_shutdown_effect_route_layer(
    shutdown_controller: String,
) -> napi::Result<String> {
    run(move || {
        let shutdown_controller = crate::js::from_json(&shutdown_controller)?;
        let out = port::make_desktop_shutdown_effect_route_layer(shutdown_controller)?;
        crate::js::to_json(&out)
    })
}

/// `makeDesktopComputerEmergencyStopRouteLayer`.
#[napi(js_name = "makeDesktopComputerEmergencyStopRouteLayer")]
pub fn make_desktop_computer_emergency_stop_route_layer() -> napi::Result<String> {
    run(move || {
        let out = port::make_desktop_computer_emergency_stop_route_layer()?;
        crate::js::to_json(&out)
    })
}

/// `makeHealthEffectRouteLayer`.
#[napi(js_name = "makeHealthEffectRouteLayer")]
pub fn make_health_effect_route_layer(readiness: String) -> napi::Result<String> {
    run(move || {
        let readiness = crate::js::from_json(&readiness)?;
        let out = port::make_health_effect_route_layer(readiness)?;
        crate::js::to_json(&out)
    })
}

/// `isLegacyTokenAuthorized`.
#[napi(js_name = "isLegacyTokenAuthorized")]
pub fn is_legacy_token_authorized(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::is_legacy_token_authorized(input)?;
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

static S_MAKE_EFFECT_HTTP_ROUTE_LAYER_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_EFFECT_HTTP_ROUTE_LAYER_1: wire::Schema = wire::Schema::Dyn;
static S_MAKE_EFFECT_HTTP_ROUTE_LAYER_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeEffectHttpRouteLayer` over the binary wire.
#[napi(js_name = "makeEffectHttpRouteLayerWire")]
pub fn make_effect_http_route_layer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let readiness: serde_json::Value = wire::read_value(r, &S_MAKE_EFFECT_HTTP_ROUTE_LAYER_0)?;
        let shutdown_controller: serde_json::Value =
            wire::read_value(r, &S_MAKE_EFFECT_HTTP_ROUTE_LAYER_1)?;
        let out = &port::make_effect_http_route_layer(readiness, shutdown_controller)?;
        wire::write_value(w, out, &S_MAKE_EFFECT_HTTP_ROUTE_LAYER_OUT)?;
        Ok(())
    })
}

static S_MAKE_DESKTOP_SHUTDOWN_EFFECT_ROUTE_LAYER_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_DESKTOP_SHUTDOWN_EFFECT_ROUTE_LAYER_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeDesktopShutdownEffectRouteLayer` over the binary wire.
#[napi(js_name = "makeDesktopShutdownEffectRouteLayerWire")]
pub fn make_desktop_shutdown_effect_route_layer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shutdown_controller: serde_json::Value =
            wire::read_value(r, &S_MAKE_DESKTOP_SHUTDOWN_EFFECT_ROUTE_LAYER_0)?;
        let out = &port::make_desktop_shutdown_effect_route_layer(shutdown_controller)?;
        wire::write_value(w, out, &S_MAKE_DESKTOP_SHUTDOWN_EFFECT_ROUTE_LAYER_OUT)?;
        Ok(())
    })
}

static S_MAKE_DESKTOP_COMPUTER_EMERGENCY_STOP_ROUTE_LAYER_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeDesktopComputerEmergencyStopRouteLayer` over the binary wire.
#[napi(js_name = "makeDesktopComputerEmergencyStopRouteLayerWire")]
pub fn make_desktop_computer_emergency_stop_route_layer_wire() -> napi::Result<f64> {
    wire_io(|_r, w| {
        let out = &port::make_desktop_computer_emergency_stop_route_layer()?;
        wire::write_value(
            w,
            out,
            &S_MAKE_DESKTOP_COMPUTER_EMERGENCY_STOP_ROUTE_LAYER_OUT,
        )?;
        Ok(())
    })
}

static S_MAKE_HEALTH_EFFECT_ROUTE_LAYER_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_HEALTH_EFFECT_ROUTE_LAYER_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeHealthEffectRouteLayer` over the binary wire.
#[napi(js_name = "makeHealthEffectRouteLayerWire")]
pub fn make_health_effect_route_layer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let readiness: serde_json::Value =
            wire::read_value(r, &S_MAKE_HEALTH_EFFECT_ROUTE_LAYER_0)?;
        let out = &port::make_health_effect_route_layer(readiness)?;
        wire::write_value(w, out, &S_MAKE_HEALTH_EFFECT_ROUTE_LAYER_OUT)?;
        Ok(())
    })
}

static S_IS_LEGACY_TOKEN_AUTHORIZED_0: wire::Schema = wire::Schema::Dyn;

/// `isLegacyTokenAuthorized` over the binary wire.
#[napi(js_name = "isLegacyTokenAuthorizedWire")]
pub fn is_legacy_token_authorized_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_IS_LEGACY_TOKEN_AUTHORIZED_0)?;
        let out = &port::is_legacy_token_authorized(input)?;
        w.bool(*out);
        Ok(())
    })
}
