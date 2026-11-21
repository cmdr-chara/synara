//! JS bindings for `packages/shared/src/runtimeMode.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;

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

/// `providerSupportsAutoRuntimeMode`.
#[napi(js_name = "providerSupportsAutoRuntimeMode")]
pub fn provider_supports_auto_runtime_mode(provider: String) -> napi::Result<bool> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::provider_supports_auto_runtime_mode(provider)?;
        Ok(out)
    })
}

/// `unsupportedAutoRuntimeModeMessage`.
#[napi(js_name = "unsupportedAutoRuntimeModeMessage")]
pub fn unsupported_auto_runtime_mode_message(provider: String) -> napi::Result<String> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::unsupported_auto_runtime_mode_message(provider)?;
        Ok(out)
    })
}

/// `autoRuntimeModeSelectionIssue`.
#[napi(js_name = "autoRuntimeModeSelectionIssue")]
pub fn auto_runtime_mode_selection_issue(
    input: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::auto_runtime_mode_selection_issue(input)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `runtimeModeEscalatesPrivilege`.
#[napi(js_name = "runtimeModeEscalatesPrivilege")]
pub fn runtime_mode_escalates_privilege(
    caller_runtime_mode: String,
    target_runtime_mode: String,
) -> napi::Result<bool> {
    run(move || {
        let caller_runtime_mode = crate::js::from_json(&caller_runtime_mode)?;
        let target_runtime_mode = crate::js::from_json(&target_runtime_mode)?;
        let out = port::runtime_mode_escalates_privilege(caller_runtime_mode, target_runtime_mode)?;
        Ok(out)
    })
}

/// `normalizeRuntimeModeForProvider`.
#[napi(js_name = "normalizeRuntimeModeForProvider")]
pub fn normalize_runtime_mode_for_provider(
    runtime_mode: String,
    provider: String,
) -> napi::Result<String> {
    run(move || {
        let runtime_mode = crate::js::from_json(&runtime_mode)?;
        let provider = crate::js::from_json(&provider)?;
        let out = port::normalize_runtime_mode_for_provider(runtime_mode, provider)?;
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

static S_PROVIDER_SUPPORTS_AUTO_RUNTIME_MODE_0: wire::Schema = wire::Schema::Dyn;

/// `providerSupportsAutoRuntimeMode` over the binary wire.
#[napi(js_name = "providerSupportsAutoRuntimeModeWire")]
pub fn provider_supports_auto_runtime_mode_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_PROVIDER_SUPPORTS_AUTO_RUNTIME_MODE_0)?;
        let out = &port::provider_supports_auto_runtime_mode(provider)?;
        w.bool(*out);
        Ok(())
    })
}

static S_UNSUPPORTED_AUTO_RUNTIME_MODE_MESSAGE_0: wire::Schema = wire::Schema::Dyn;

/// `unsupportedAutoRuntimeModeMessage` over the binary wire.
#[napi(js_name = "unsupportedAutoRuntimeModeMessageWire")]
pub fn unsupported_auto_runtime_mode_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value =
            wire::read_value(r, &S_UNSUPPORTED_AUTO_RUNTIME_MODE_MESSAGE_0)?;
        let out = &port::unsupported_auto_runtime_mode_message(provider)?;
        w.string16(out);
        Ok(())
    })
}

static S_AUTO_RUNTIME_MODE_SELECTION_ISSUE_0: wire::Schema = wire::Schema::Dyn;

/// `autoRuntimeModeSelectionIssue` over the binary wire.
#[napi(js_name = "autoRuntimeModeSelectionIssueWire")]
pub fn auto_runtime_mode_selection_issue_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_AUTO_RUNTIME_MODE_SELECTION_ISSUE_0)?;
        let out = &port::auto_runtime_mode_selection_issue(input)?;
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

static S_RUNTIME_MODE_ESCALATES_PRIVILEGE_0: wire::Schema = wire::Schema::Dyn;
static S_RUNTIME_MODE_ESCALATES_PRIVILEGE_1: wire::Schema = wire::Schema::Dyn;

/// `runtimeModeEscalatesPrivilege` over the binary wire.
#[napi(js_name = "runtimeModeEscalatesPrivilegeWire")]
pub fn runtime_mode_escalates_privilege_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let caller_runtime_mode: serde_json::Value =
            wire::read_value(r, &S_RUNTIME_MODE_ESCALATES_PRIVILEGE_0)?;
        let target_runtime_mode: serde_json::Value =
            wire::read_value(r, &S_RUNTIME_MODE_ESCALATES_PRIVILEGE_1)?;
        let out =
            &port::runtime_mode_escalates_privilege(caller_runtime_mode, target_runtime_mode)?;
        w.bool(*out);
        Ok(())
    })
}

static S_NORMALIZE_RUNTIME_MODE_FOR_PROVIDER_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_RUNTIME_MODE_FOR_PROVIDER_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_RUNTIME_MODE_FOR_PROVIDER_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeRuntimeModeForProvider` over the binary wire.
#[napi(js_name = "normalizeRuntimeModeForProviderWire")]
pub fn normalize_runtime_mode_for_provider_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let runtime_mode: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_RUNTIME_MODE_FOR_PROVIDER_0)?;
        let provider: serde_json::Value =
            wire::read_value(r, &S_NORMALIZE_RUNTIME_MODE_FOR_PROVIDER_1)?;
        let out = &port::normalize_runtime_mode_for_provider(runtime_mode, provider)?;
        wire::write_value(w, out, &S_NORMALIZE_RUNTIME_MODE_FOR_PROVIDER_OUT)?;
        Ok(())
    })
}
