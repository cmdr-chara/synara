//! JS bindings for `apps/desktop/src/cuaFixtures/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parseFocusProbeLine`.
#[napi(js_name = "parseFocusProbeLine")]
pub fn parse_focus_probe_line(line: String) -> napi::Result<String> {
    run(move || {
        let out = port::parse_focus_probe_line(line)?;
        crate::js::to_json(&out)
    })
}

/// `analyzeFocusSamples`.
#[napi(js_name = "analyzeFocusSamples")]
pub fn analyze_focus_samples(all_samples: String, options: String) -> napi::Result<String> {
    run(move || {
        let all_samples = crate::js::from_json(&all_samples)?;
        let options = crate::js::from_json(&options)?;
        let out = port::analyze_focus_samples(all_samples, options)?;
        crate::js::to_json(&out)
    })
}

/// `runLiveFixture`.
#[napi(js_name = "runLiveFixture")]
pub async fn run_live_fixture(directory: String, binary_path: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::run_live_fixture(directory, binary_path).await?;
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

/// `parseFocusProbeLine` over the binary wire.
#[napi(js_name = "parseFocusProbeLineWire")]
pub fn parse_focus_probe_line_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: String = r.string16()?;
        let out = &port::parse_focus_probe_line(line)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_ANALYZE_FOCUS_SAMPLES_0: wire::Schema = wire::Schema::List(&wire::Schema::Obj(&[
    ("t", wire::Schema::Num),
    ("pid", wire::Schema::OptJs(&wire::Schema::Num)),
    ("app", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("keyWin", wire::Schema::OptJs(&wire::Schema::Num)),
    ("keyTitle", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("topWin", wire::Schema::OptJs(&wire::Schema::Num)),
    ("topTitle", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("space", wire::Schema::OptJs(&wire::Schema::Num)),
    ("focusedPid", wire::Schema::OptJs(&wire::Schema::Num)),
    ("focused", wire::Schema::OptJs(&wire::Schema::Str16)),
]));
static S_ANALYZE_FOCUS_SAMPLES_OUT: wire::Schema = wire::Schema::Obj(&[
    ("ok", wire::Schema::Bool),
    ("theftFree", wire::Schema::Bool),
    (
        "baseline",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("pid", wire::Schema::OptJs(&wire::Schema::Num)),
            ("keyWin", wire::Schema::OptJs(&wire::Schema::Num)),
            ("topWin", wire::Schema::OptJs(&wire::Schema::Num)),
            ("space", wire::Schema::OptJs(&wire::Schema::Num)),
            ("focusedPid", wire::Schema::OptJs(&wire::Schema::Num)),
            ("focused", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    ("sampleCount", wire::Schema::Num),
    ("warmupCount", wire::Schema::Num),
    ("measuredMs", wire::Schema::Num),
    ("settleMs", wire::Schema::Num),
    ("strictFocus", wire::Schema::Bool),
    (
        "offBaseline",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("index", wire::Schema::Num),
            ("tMs", wire::Schema::Num),
            ("durationMs", wire::Schema::Num),
            ("sampleCount", wire::Schema::Num),
            ("changedFields", wire::Schema::List(&wire::Schema::Dyn)),
            ("observed", wire::Schema::List(&wire::Schema::Dyn)),
        ])),
    ),
    ("offBaselineSamples", wire::Schema::Num),
    (
        "drift",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("index", wire::Schema::Num),
            ("tMs", wire::Schema::Num),
            ("from", wire::Schema::Str16),
            ("to", wire::Schema::Str16),
        ])),
    ),
    ("driftSamples", wire::Schema::Num),
    ("uncertainSamples", wire::Schema::Num),
    ("coverage", wire::Schema::Map16(&wire::Schema::Num)),
    ("issues", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `analyzeFocusSamples` over the binary wire.
#[napi(js_name = "analyzeFocusSamplesWire")]
pub fn analyze_focus_samples_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let all_samples: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_ANALYZE_FOCUS_SAMPLES_0.inner()))?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::analyze_focus_samples(all_samples, options)?;
        wire::write_value(w, out, &S_ANALYZE_FOCUS_SAMPLES_OUT)?;
        Ok(())
    })
}
