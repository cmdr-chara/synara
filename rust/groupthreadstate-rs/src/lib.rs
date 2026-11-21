//! JS bindings for `packages/shared/src/groupThreadState.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `groupThreadStateLabel`.
#[napi(js_name = "groupThreadStateLabel")]
pub fn group_thread_state_label(state: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::group_thread_state_label(state)?;
        crate::js::to_json(&out)
    })
}

/// `canSessionAnswerPendingRequests`.
#[napi(js_name = "canSessionAnswerPendingRequests")]
pub fn can_session_answer_pending_requests(session: String) -> napi::Result<bool> {
    run(move || {
        let session = crate::js::from_json(&session)?;
        let out = port::can_session_answer_pending_requests(session)?;
        Ok(out)
    })
}

/// `isLatestTurnSettled`.
#[napi(js_name = "isLatestTurnSettled")]
pub fn is_latest_turn_settled(latest_turn: String, session: String) -> napi::Result<bool> {
    run(move || {
        let latest_turn = crate::js::from_json(&latest_turn)?;
        let session = crate::js::from_json(&session)?;
        let out = port::is_latest_turn_settled(latest_turn, session)?;
        Ok(out)
    })
}

/// `hasLiveLatestTurn`.
#[napi(js_name = "hasLiveLatestTurn")]
pub fn has_live_latest_turn(latest_turn: String, session: String) -> napi::Result<bool> {
    run(move || {
        let latest_turn = crate::js::from_json(&latest_turn)?;
        let session = crate::js::from_json(&session)?;
        let out = port::has_live_latest_turn(latest_turn, session)?;
        Ok(out)
    })
}

/// `isThreadActivelyWorking`.
#[napi(js_name = "isThreadActivelyWorking")]
pub fn is_thread_actively_working(thread: String) -> napi::Result<bool> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::is_thread_actively_working(thread)?;
        Ok(out)
    })
}

/// `groupThreadNeedsAttention`.
#[napi(js_name = "groupThreadNeedsAttention")]
pub fn group_thread_needs_attention(thread: String) -> napi::Result<bool> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::group_thread_needs_attention(thread)?;
        Ok(out)
    })
}

/// `resolveGroupCoordinatorStatus`.
#[napi(js_name = "resolveGroupCoordinatorStatus")]
pub fn resolve_group_coordinator_status(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_group_coordinator_status(input)?;
        Ok(out)
    })
}

/// `resolveGroupThreadState`.
#[napi(js_name = "resolveGroupThreadState")]
pub fn resolve_group_thread_state(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_group_thread_state(input)?;
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

static S_GROUP_THREAD_STATE_LABEL_0: wire::Schema = wire::Schema::Dyn;
static S_GROUP_THREAD_STATE_LABEL_OUT: wire::Schema = wire::Schema::Str16;

/// `groupThreadStateLabel` over the binary wire.
#[napi(js_name = "groupThreadStateLabelWire")]
pub fn group_thread_state_label_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: Value = wire::read_value(r, &S_GROUP_THREAD_STATE_LABEL_0)?;
        let out = &port::group_thread_state_label(state)?;
        wire::write_value(w, out, &S_GROUP_THREAD_STATE_LABEL_OUT)?;
        Ok(())
    })
}

/// `canSessionAnswerPendingRequests` over the binary wire.
#[napi(js_name = "canSessionAnswerPendingRequestsWire")]
pub fn can_session_answer_pending_requests_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::can_session_answer_pending_requests(session)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isLatestTurnSettled` over the binary wire.
#[napi(js_name = "isLatestTurnSettledWire")]
pub fn is_latest_turn_settled_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let latest_turn: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::is_latest_turn_settled(latest_turn, session)?;
        w.bool(*out);
        Ok(())
    })
}

/// `hasLiveLatestTurn` over the binary wire.
#[napi(js_name = "hasLiveLatestTurnWire")]
pub fn has_live_latest_turn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let latest_turn: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let session: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::has_live_latest_turn(latest_turn, session)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_THREAD_ACTIVELY_WORKING_0: wire::Schema = wire::Schema::Dyn;

/// `isThreadActivelyWorking` over the binary wire.
#[napi(js_name = "isThreadActivelyWorkingWire")]
pub fn is_thread_actively_working_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value = wire::read_value(r, &S_IS_THREAD_ACTIVELY_WORKING_0)?;
        let out = &port::is_thread_actively_working(thread)?;
        w.bool(*out);
        Ok(())
    })
}

static S_GROUP_THREAD_NEEDS_ATTENTION_0: wire::Schema = wire::Schema::Dyn;

/// `groupThreadNeedsAttention` over the binary wire.
#[napi(js_name = "groupThreadNeedsAttentionWire")]
pub fn group_thread_needs_attention_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value = wire::read_value(r, &S_GROUP_THREAD_NEEDS_ATTENTION_0)?;
        let out = &port::group_thread_needs_attention(thread)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_GROUP_COORDINATOR_STATUS_0: wire::Schema = wire::Schema::Dyn;

/// `resolveGroupCoordinatorStatus` over the binary wire.
#[napi(js_name = "resolveGroupCoordinatorStatusWire")]
pub fn resolve_group_coordinator_status_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_GROUP_COORDINATOR_STATUS_0)?;
        let out = &port::resolve_group_coordinator_status(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_RESOLVE_GROUP_THREAD_STATE_0: wire::Schema = wire::Schema::Dyn;

/// `resolveGroupThreadState` over the binary wire.
#[napi(js_name = "resolveGroupThreadStateWire")]
pub fn resolve_group_thread_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_GROUP_THREAD_STATE_0)?;
        let out = &port::resolve_group_thread_state(input)?;
        w.string16(out);
        Ok(())
    })
}
