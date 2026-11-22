//! JS bindings for `apps/server/src/composerPrediction/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveComposerPredictionTurnBlocker`.
#[napi(js_name = "resolveComposerPredictionTurnBlocker")]
pub fn resolve_composer_prediction_turn_blocker(
    thread: String,
    expected_turn_id: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let expected_turn_id = crate::js::from_json(&expected_turn_id)?;
        let out = port::resolve_composer_prediction_turn_blocker(thread, expected_turn_id)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `isComposerPredictionTurnCurrent`.
#[napi(js_name = "isComposerPredictionTurnCurrent")]
pub fn is_composer_prediction_turn_current(
    thread: String,
    expected_turn_id: String,
) -> napi::Result<bool> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let expected_turn_id = crate::js::from_json(&expected_turn_id)?;
        let out = port::is_composer_prediction_turn_current(thread, expected_turn_id)?;
        Ok(out)
    })
}

/// `buildComposerPredictionConversation`.
#[napi(js_name = "buildComposerPredictionConversation")]
pub fn build_composer_prediction_conversation(
    input: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_composer_prediction_conversation(input)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `lowestSupportedEffort`.
#[napi(js_name = "lowestSupportedEffort")]
pub fn lowest_supported_effort(
    provider: String,
    model: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let provider = crate::js::from_json(&provider)?;
        let out = port::lowest_supported_effort(provider, model)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `resolveComposerPredictionHedgeDelayMs`.
#[napi(js_name = "resolveComposerPredictionHedgeDelayMs")]
pub fn resolve_composer_prediction_hedge_delay_ms(
    input: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_composer_prediction_hedge_delay_ms(input)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `resolveComposerPredictionRouting`.
#[napi(js_name = "resolveComposerPredictionRouting")]
pub fn resolve_composer_prediction_routing(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_composer_prediction_routing(input)?;
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

static S_RESOLVE_COMPOSER_PREDICTION_TURN_BLOCKER_1: wire::Schema = wire::Schema::Dyn;

/// `resolveComposerPredictionTurnBlocker` over the binary wire.
#[napi(js_name = "resolveComposerPredictionTurnBlockerWire")]
pub fn resolve_composer_prediction_turn_blocker_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let expected_turn_id: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_COMPOSER_PREDICTION_TURN_BLOCKER_1)?;
        let out = &port::resolve_composer_prediction_turn_blocker(thread, expected_turn_id)?;
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

static S_IS_COMPOSER_PREDICTION_TURN_CURRENT_1: wire::Schema = wire::Schema::Dyn;

/// `isComposerPredictionTurnCurrent` over the binary wire.
#[napi(js_name = "isComposerPredictionTurnCurrentWire")]
pub fn is_composer_prediction_turn_current_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let expected_turn_id: serde_json::Value =
            wire::read_value(r, &S_IS_COMPOSER_PREDICTION_TURN_CURRENT_1)?;
        let out = &port::is_composer_prediction_turn_current(thread, expected_turn_id)?;
        w.bool(*out);
        Ok(())
    })
}

static S_BUILD_COMPOSER_PREDICTION_CONVERSATION_0: wire::Schema = wire::Schema::Dyn;

/// `buildComposerPredictionConversation` over the binary wire.
#[napi(js_name = "buildComposerPredictionConversationWire")]
pub fn build_composer_prediction_conversation_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_BUILD_COMPOSER_PREDICTION_CONVERSATION_0)?;
        let out = &port::build_composer_prediction_conversation(input)?;
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

static S_LOWEST_SUPPORTED_EFFORT_0: wire::Schema = wire::Schema::Dyn;

/// `lowestSupportedEffort` over the binary wire.
#[napi(js_name = "lowestSupportedEffortWire")]
pub fn lowest_supported_effort_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let provider: serde_json::Value = wire::read_value(r, &S_LOWEST_SUPPORTED_EFFORT_0)?;
        let model: String = r.string16()?;
        let out = &port::lowest_supported_effort(provider, model)?;
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

static S_RESOLVE_COMPOSER_PREDICTION_HEDGE_DELAY_MS_0: wire::Schema = wire::Schema::Dyn;

/// `resolveComposerPredictionHedgeDelayMs` over the binary wire.
#[napi(js_name = "resolveComposerPredictionHedgeDelayMsWire")]
pub fn resolve_composer_prediction_hedge_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_COMPOSER_PREDICTION_HEDGE_DELAY_MS_0)?;
        let out = &port::resolve_composer_prediction_hedge_delay_ms(input)?;
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

static S_RESOLVE_COMPOSER_PREDICTION_ROUTING_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_COMPOSER_PREDICTION_ROUTING_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("modelSelection", wire::Schema::Dyn),
        ("providerOptions", wire::Schema::Dyn),
        ("codexHomePath", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `resolveComposerPredictionRouting` over the binary wire.
#[napi(js_name = "resolveComposerPredictionRoutingWire")]
pub fn resolve_composer_prediction_routing_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_COMPOSER_PREDICTION_ROUTING_0)?;
        let out = &port::resolve_composer_prediction_routing(input)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_RESOLVE_COMPOSER_PREDICTION_ROUTING_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}
