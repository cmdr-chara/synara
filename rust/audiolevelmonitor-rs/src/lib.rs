//! JS bindings for `apps/desktop/src/audioLevelMonitor.ts`: generated from the signatures in `port.rs` by rust-anything.
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

#[allow(dead_code)]
async fn run_async<T>(f: impl std::future::Future<Output = Result<T, String>>) -> napi::Result<T> {
    use futures::FutureExt as _;
    match std::panic::AssertUnwindSafe(f).catch_unwind().await {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `parseAudioLevelMessage`.
#[napi(js_name = "parseAudioLevelMessage")]
pub fn parse_audio_level_message(line: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_audio_level_message(line)?;
        crate::js::to_json(&out)
    })
}

/// `combineAudioLevelSources`.
#[napi(js_name = "combineAudioLevelSources")]
pub fn combine_audio_level_sources(
    sources: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let sources = crate::js::from_json(&sources)?;
        let out = port::combine_audio_level_sources(sources)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `combineAudioLevelSubscriptions`.
#[napi(js_name = "combineAudioLevelSubscriptions")]
pub fn combine_audio_level_subscriptions(subscriptions: String) -> napi::Result<String> {
    run(move || {
        let subscriptions = crate::js::from_json(&subscriptions)?;
        let out = port::combine_audio_level_subscriptions(subscriptions)?;
        crate::js::to_json(&out)
    })
}

/// `parseAudioInputsMessage`.
#[napi(js_name = "parseAudioInputsMessage")]
pub fn parse_audio_inputs_message(output: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_audio_inputs_message(output)?;
        crate::js::to_json(&out)
    })
}

/// `listAudioInputDevices`.
#[napi(js_name = "listAudioInputDevices")]
pub async fn list_audio_input_devices(
    helper_path: String,
    exec_file: String,
) -> napi::Result<String> {
    run_async(async move {
        let exec_file = crate::js::from_json(&exec_file)?;
        let out = port::list_audio_input_devices(helper_path, exec_file).await?;
        crate::js::to_json(&out)
    })
    .await
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

/// `parseAudioLevelMessage` over the binary wire.
#[napi(js_name = "parseAudioLevelMessageWire")]
pub fn parse_audio_level_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: String = r.string16()?;
        let out = &port::parse_audio_level_message(line)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_COMBINE_AUDIO_LEVEL_SOURCES_0: wire::Schema = wire::Schema::Dyn;

/// `combineAudioLevelSources` over the binary wire.
#[napi(js_name = "combineAudioLevelSourcesWire")]
pub fn combine_audio_level_sources_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sources: serde_json::Value = wire::read_value(r, &S_COMBINE_AUDIO_LEVEL_SOURCES_0)?;
        let out = &port::combine_audio_level_sources(sources)?;
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

static S_COMBINE_AUDIO_LEVEL_SUBSCRIPTIONS_0: wire::Schema = wire::Schema::Dyn;
static S_COMBINE_AUDIO_LEVEL_SUBSCRIPTIONS_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Obj(&[
        ("source", wire::Schema::Str16),
        ("microphoneId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]));

/// `combineAudioLevelSubscriptions` over the binary wire.
#[napi(js_name = "combineAudioLevelSubscriptionsWire")]
pub fn combine_audio_level_subscriptions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let subscriptions: serde_json::Value =
            wire::read_value(r, &S_COMBINE_AUDIO_LEVEL_SUBSCRIPTIONS_0)?;
        let out = &port::combine_audio_level_subscriptions(subscriptions)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_COMBINE_AUDIO_LEVEL_SUBSCRIPTIONS_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_PARSE_AUDIO_INPUTS_MESSAGE_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("name", wire::Schema::Str16),
    ("bluetooth", wire::Schema::Bool),
    ("default", wire::Schema::Bool),
]));

/// `parseAudioInputsMessage` over the binary wire.
#[napi(js_name = "parseAudioInputsMessageWire")]
pub fn parse_audio_inputs_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let output: String = r.string16()?;
        let out = &port::parse_audio_inputs_message(output)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_PARSE_AUDIO_INPUTS_MESSAGE_OUT.inner())?;
        }
        Ok(())
    })
}
