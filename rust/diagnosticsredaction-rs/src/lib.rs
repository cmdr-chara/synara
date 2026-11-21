//! JS bindings for `packages/shared/src/diagnosticsRedaction.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `sanitizeDesktopUpdateFailureReason`.
#[napi(js_name = "sanitizeDesktopUpdateFailureReason")]
pub fn sanitize_desktop_update_failure_reason(reason: String) -> napi::Result<String> {
    run(move || {
        let reason = crate::js::from_json(&reason)?;
        let out = port::sanitize_desktop_update_failure_reason(reason)?;
        crate::js::to_json(&out)
    })
}

/// `classifyDesktopUpdateFailure`.
#[napi(js_name = "classifyDesktopUpdateFailure")]
pub fn classify_desktop_update_failure(message: String) -> napi::Result<String> {
    run(move || {
        let message = crate::js::from_json(&message)?;
        let out = port::classify_desktop_update_failure(message)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizeDiagnosticError`.
#[napi(js_name = "sanitizeDiagnosticError")]
pub fn sanitize_diagnostic_error(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::sanitize_diagnostic_error(input)?;
        crate::js::to_json(&out)
    })
}

/// `sanitizeDiagnosticCrash`.
#[napi(js_name = "sanitizeDiagnosticCrash")]
pub fn sanitize_diagnostic_crash(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::sanitize_diagnostic_crash(input)?;
        crate::js::to_json(&out)
    })
}

/// `diagnosticErrorFingerprint`.
#[napi(js_name = "diagnosticErrorFingerprint")]
pub fn diagnostic_error_fingerprint(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::diagnostic_error_fingerprint(input)?;
        Ok(out)
    })
}

/// `diagnosticPayloadKind`.
#[napi(js_name = "diagnosticPayloadKind")]
pub fn diagnostic_payload_kind(event: String) -> napi::Result<String> {
    run(move || {
        let out = port::diagnostic_payload_kind(event)?;
        crate::js::to_json(&out)
    })
}

/// `redactDiagnosticText`.
#[napi(js_name = "redactDiagnosticText")]
pub fn redact_diagnostic_text(text: String, opts: String) -> napi::Result<String> {
    run(move || {
        let opts = crate::js::from_json(&opts)?;
        let out = port::redact_diagnostic_text(text, opts)?;
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

static S_SANITIZE_DESKTOP_UPDATE_FAILURE_REASON_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_DESKTOP_UPDATE_FAILURE_REASON_OUT: wire::Schema = wire::Schema::Dyn;

/// `sanitizeDesktopUpdateFailureReason` over the binary wire.
#[napi(js_name = "sanitizeDesktopUpdateFailureReasonWire")]
pub fn sanitize_desktop_update_failure_reason_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let reason: serde_json::Value =
            wire::read_value(r, &S_SANITIZE_DESKTOP_UPDATE_FAILURE_REASON_0)?;
        let out = &port::sanitize_desktop_update_failure_reason(reason)?;
        wire::write_value(w, out, &S_SANITIZE_DESKTOP_UPDATE_FAILURE_REASON_OUT)?;
        Ok(())
    })
}

static S_CLASSIFY_DESKTOP_UPDATE_FAILURE_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_DESKTOP_UPDATE_FAILURE_OUT: wire::Schema = wire::Schema::Dyn;

/// `classifyDesktopUpdateFailure` over the binary wire.
#[napi(js_name = "classifyDesktopUpdateFailureWire")]
pub fn classify_desktop_update_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: serde_json::Value = wire::read_value(r, &S_CLASSIFY_DESKTOP_UPDATE_FAILURE_0)?;
        let out = &port::classify_desktop_update_failure(message)?;
        wire::write_value(w, out, &S_CLASSIFY_DESKTOP_UPDATE_FAILURE_OUT)?;
        Ok(())
    })
}

static S_SANITIZE_DIAGNOSTIC_ERROR_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_DIAGNOSTIC_ERROR_OUT: wire::Schema = wire::Schema::Obj(&[
    ("message", wire::Schema::Str16),
    ("stack", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `sanitizeDiagnosticError` over the binary wire.
#[napi(js_name = "sanitizeDiagnosticErrorWire")]
pub fn sanitize_diagnostic_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_SANITIZE_DIAGNOSTIC_ERROR_0)?;
        let out = &port::sanitize_diagnostic_error(input)?;
        wire::write_value(w, out, &S_SANITIZE_DIAGNOSTIC_ERROR_OUT)?;
        Ok(())
    })
}

static S_SANITIZE_DIAGNOSTIC_CRASH_0: wire::Schema = wire::Schema::Dyn;
static S_SANITIZE_DIAGNOSTIC_CRASH_OUT: wire::Schema = wire::Schema::Obj(&[
    ("processType", wire::Schema::Str16),
    ("reason", wire::Schema::Str16),
]);

/// `sanitizeDiagnosticCrash` over the binary wire.
#[napi(js_name = "sanitizeDiagnosticCrashWire")]
pub fn sanitize_diagnostic_crash_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_SANITIZE_DIAGNOSTIC_CRASH_0)?;
        let out = &port::sanitize_diagnostic_crash(input)?;
        wire::write_value(w, out, &S_SANITIZE_DIAGNOSTIC_CRASH_OUT)?;
        Ok(())
    })
}

static S_DIAGNOSTIC_ERROR_FINGERPRINT_0: wire::Schema = wire::Schema::Dyn;

/// `diagnosticErrorFingerprint` over the binary wire.
#[napi(js_name = "diagnosticErrorFingerprintWire")]
pub fn diagnostic_error_fingerprint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DIAGNOSTIC_ERROR_FINGERPRINT_0)?;
        let out = &port::diagnostic_error_fingerprint(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_DIAGNOSTIC_PAYLOAD_KIND_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `diagnosticPayloadKind` over the binary wire.
#[napi(js_name = "diagnosticPayloadKindWire")]
pub fn diagnostic_payload_kind_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event: String = r.string16()?;
        let out = &port::diagnostic_payload_kind(event)?;
        wire::write_value(w, out, &S_DIAGNOSTIC_PAYLOAD_KIND_OUT)?;
        Ok(())
    })
}

static S_REDACT_DIAGNOSTIC_TEXT_1: wire::Schema = wire::Schema::Dyn;
static S_REDACT_DIAGNOSTIC_TEXT_OUT: wire::Schema = wire::Schema::Str16;

/// `redactDiagnosticText` over the binary wire.
#[napi(js_name = "redactDiagnosticTextWire")]
pub fn redact_diagnostic_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let opts: serde_json::Value = wire::read_value(r, &S_REDACT_DIAGNOSTIC_TEXT_1)?;
        let out = &port::redact_diagnostic_text(text, opts)?;
        wire::write_value(w, out, &S_REDACT_DIAGNOSTIC_TEXT_OUT)?;
        Ok(())
    })
}
