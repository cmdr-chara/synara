//! JS bindings for `packages/shared/src/automationRunLifecycle.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isActiveAutomationRunStatus`.
#[napi(js_name = "isActiveAutomationRunStatus")]
pub fn is_active_automation_run_status(status: String) -> napi::Result<bool> {
    run(move || {
        let status = crate::js::from_json(&status)?;
        let out = port::is_active_automation_run_status(status)?;
        Ok(out)
    })
}

/// `isTerminalAutomationRunStatus`.
#[napi(js_name = "isTerminalAutomationRunStatus")]
pub fn is_terminal_automation_run_status(status: String) -> napi::Result<bool> {
    run(move || {
        let status = crate::js::from_json(&status)?;
        let out = port::is_terminal_automation_run_status(status)?;
        Ok(out)
    })
}

/// `canTransitionAutomationRun`.
#[napi(js_name = "canTransitionAutomationRun")]
pub fn can_transition_automation_run(from: String, to: String) -> napi::Result<bool> {
    run(move || {
        let from = crate::js::from_json(&from)?;
        let to = crate::js::from_json(&to)?;
        let out = port::can_transition_automation_run(from, to)?;
        Ok(out)
    })
}

/// `automationRunStatusesLeadingTo`.
#[napi(js_name = "automationRunStatusesLeadingTo")]
pub fn automation_run_statuses_leading_to(to: String) -> napi::Result<String> {
    run(move || {
        let to = crate::js::from_json(&to)?;
        let out = port::automation_run_statuses_leading_to(to)?;
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

static S_IS_ACTIVE_AUTOMATION_RUN_STATUS_0: wire::Schema = wire::Schema::Dyn;

/// `isActiveAutomationRunStatus` over the binary wire.
#[napi(js_name = "isActiveAutomationRunStatusWire")]
pub fn is_active_automation_run_status_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let status: serde_json::Value = wire::read_value(r, &S_IS_ACTIVE_AUTOMATION_RUN_STATUS_0)?;
        let out = &port::is_active_automation_run_status(status)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_TERMINAL_AUTOMATION_RUN_STATUS_0: wire::Schema = wire::Schema::Dyn;

/// `isTerminalAutomationRunStatus` over the binary wire.
#[napi(js_name = "isTerminalAutomationRunStatusWire")]
pub fn is_terminal_automation_run_status_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let status: serde_json::Value =
            wire::read_value(r, &S_IS_TERMINAL_AUTOMATION_RUN_STATUS_0)?;
        let out = &port::is_terminal_automation_run_status(status)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_CAN_TRANSITION_AUTOMATION_RUN_0: wire::Schema = wire::Schema::Dyn;
static S_CAN_TRANSITION_AUTOMATION_RUN_1: wire::Schema = wire::Schema::Dyn;

/// `canTransitionAutomationRun` over the binary wire.
#[napi(js_name = "canTransitionAutomationRunWire")]
pub fn can_transition_automation_run_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let from: serde_json::Value = wire::read_value(r, &S_CAN_TRANSITION_AUTOMATION_RUN_0)?;
        let to: serde_json::Value = wire::read_value(r, &S_CAN_TRANSITION_AUTOMATION_RUN_1)?;
        let out = &port::can_transition_automation_run(from, to)?;
        w.bool(*out);
        Ok(())
    })
}

static S_AUTOMATION_RUN_STATUSES_LEADING_TO_0: wire::Schema = wire::Schema::Dyn;

/// `automationRunStatusesLeadingTo` over the binary wire.
#[napi(js_name = "automationRunStatusesLeadingToWire")]
pub fn automation_run_statuses_leading_to_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let to: serde_json::Value = wire::read_value(r, &S_AUTOMATION_RUN_STATUSES_LEADING_TO_0)?;
        let out = &port::automation_run_statuses_leading_to(to)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
