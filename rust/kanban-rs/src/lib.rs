//! JS bindings for `packages/shared/src/kanban.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isKanbanTurnSettled`.
#[napi(js_name = "isKanbanTurnSettled")]
pub fn is_kanban_turn_settled(t: String) -> napi::Result<bool> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::is_kanban_turn_settled(t)?;
        Ok(out)
    })
}

/// `hasKanbanLiveLatestTurn`.
#[napi(js_name = "hasKanbanLiveLatestTurn")]
pub fn has_kanban_live_latest_turn(t: String) -> napi::Result<bool> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::has_kanban_live_latest_turn(t)?;
        Ok(out)
    })
}

/// `hasKanbanLiveWork`.
#[napi(js_name = "hasKanbanLiveWork")]
pub fn has_kanban_live_work(t: String) -> napi::Result<bool> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::has_kanban_live_work(t)?;
        Ok(out)
    })
}

/// `kanbanHeartbeatTimestampMs`.
#[napi(js_name = "kanbanHeartbeatTimestampMs")]
pub fn kanban_heartbeat_timestamp_ms(
    t: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::kanban_heartbeat_timestamp_ms(t)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `deriveKanbanAwaitingYouReason`.
#[napi(js_name = "deriveKanbanAwaitingYouReason")]
pub fn derive_kanban_awaiting_you_reason(
    t: String,
    opts: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let opts = crate::js::from_json(&opts)?;
        let out = port::derive_kanban_awaiting_you_reason(t, opts)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `deriveKanbanColumnV2`.
#[napi(js_name = "deriveKanbanColumnV2")]
pub fn derive_kanban_column_v2(t: String, opts: String) -> napi::Result<String> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let opts = crate::js::from_json(&opts)?;
        let out = port::derive_kanban_column_v2(t, opts)?;
        Ok(out)
    })
}

/// `deriveKanbanAttention`.
#[napi(js_name = "deriveKanbanAttention")]
pub fn derive_kanban_attention(t: String, opts: String) -> napi::Result<Vec<String>> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let opts = crate::js::from_json(&opts)?;
        let out = port::derive_kanban_attention(t, opts)?;
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

static S_IS_KANBAN_TURN_SETTLED_0: wire::Schema = wire::Schema::Dyn;

/// `isKanbanTurnSettled` over the binary wire.
#[napi(js_name = "isKanbanTurnSettledWire")]
pub fn is_kanban_turn_settled_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_IS_KANBAN_TURN_SETTLED_0)?;
        let out = &port::is_kanban_turn_settled(t)?;
        w.bool(*out);
        Ok(())
    })
}

static S_HAS_KANBAN_LIVE_LATEST_TURN_0: wire::Schema = wire::Schema::Dyn;

/// `hasKanbanLiveLatestTurn` over the binary wire.
#[napi(js_name = "hasKanbanLiveLatestTurnWire")]
pub fn has_kanban_live_latest_turn_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_HAS_KANBAN_LIVE_LATEST_TURN_0)?;
        let out = &port::has_kanban_live_latest_turn(t)?;
        w.bool(*out);
        Ok(())
    })
}

static S_HAS_KANBAN_LIVE_WORK_0: wire::Schema = wire::Schema::Dyn;

/// `hasKanbanLiveWork` over the binary wire.
#[napi(js_name = "hasKanbanLiveWorkWire")]
pub fn has_kanban_live_work_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_HAS_KANBAN_LIVE_WORK_0)?;
        let out = &port::has_kanban_live_work(t)?;
        w.bool(*out);
        Ok(())
    })
}

static S_KANBAN_HEARTBEAT_TIMESTAMP_MS_0: wire::Schema = wire::Schema::Dyn;

/// `kanbanHeartbeatTimestampMs` over the binary wire.
#[napi(js_name = "kanbanHeartbeatTimestampMsWire")]
pub fn kanban_heartbeat_timestamp_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_KANBAN_HEARTBEAT_TIMESTAMP_MS_0)?;
        let out = &port::kanban_heartbeat_timestamp_ms(t)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_DERIVE_KANBAN_AWAITING_YOU_REASON_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_KANBAN_AWAITING_YOU_REASON_1: wire::Schema = wire::Schema::Dyn;

/// `deriveKanbanAwaitingYouReason` over the binary wire.
#[napi(js_name = "deriveKanbanAwaitingYouReasonWire")]
pub fn derive_kanban_awaiting_you_reason_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_DERIVE_KANBAN_AWAITING_YOU_REASON_0)?;
        let opts: serde_json::Value = wire::read_value(r, &S_DERIVE_KANBAN_AWAITING_YOU_REASON_1)?;
        let out = &port::derive_kanban_awaiting_you_reason(t, opts)?;
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

static S_DERIVE_KANBAN_COLUMN_V2_0: wire::Schema = wire::Schema::Dyn;

/// `deriveKanbanColumnV2` over the binary wire.
#[napi(js_name = "deriveKanbanColumnV2Wire")]
pub fn derive_kanban_column_v2_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_DERIVE_KANBAN_COLUMN_V2_0)?;
        let opts: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::derive_kanban_column_v2(t, opts)?;
        w.string16(out);
        Ok(())
    })
}

static S_DERIVE_KANBAN_ATTENTION_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_KANBAN_ATTENTION_1: wire::Schema = wire::Schema::Dyn;

/// `deriveKanbanAttention` over the binary wire.
#[napi(js_name = "deriveKanbanAttentionWire")]
pub fn derive_kanban_attention_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_DERIVE_KANBAN_ATTENTION_0)?;
        let opts: serde_json::Value = wire::read_value(r, &S_DERIVE_KANBAN_ATTENTION_1)?;
        let out = &port::derive_kanban_attention(t, opts)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}
