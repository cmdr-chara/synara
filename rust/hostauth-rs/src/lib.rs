//! JS bindings for `apps/server/src/hostAuth/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `verifySessionCredential`.
#[napi(js_name = "verifySessionCredential")]
pub async fn verify_session_credential(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::verify_session_credential(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `assertBoundedLifetime`.
#[napi(js_name = "assertBoundedLifetime")]
pub fn assert_bounded_lifetime(
    claims: String,
    maximum_seconds: f64,
    label: String,
    now_seconds: f64,
) -> napi::Result<()> {
    run(move || {
        let claims = crate::js::from_json(&claims)?;
        port::assert_bounded_lifetime(claims, maximum_seconds, label, now_seconds)?;
        Ok(())
    })
}

/// Class `JwtReplayCache`.
#[napi(js_name = "JwtReplayCache")]
pub struct JwtReplayCache {
    inner: port::JwtReplayCache,
}

#[napi]
impl JwtReplayCache {
    #[napi(constructor)]
    pub fn new() -> napi::Result<Self> {
        run(move || {
            let out = port::JwtReplayCache::new()?;
            Ok(Self { inner: out })
        })
    }

    #[napi(js_name = "consume")]
    pub fn consume(
        &mut self,
        jti: String,
        expires_at_seconds: f64,
        now_seconds: String,
    ) -> napi::Result<()> {
        let inner = &mut self.inner;
        run(move || {
            let now_seconds = crate::js::from_json(&now_seconds)?;
            inner.consume(jti, expires_at_seconds, now_seconds)?;
            Ok(())
        })
    }
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

static S_ASSERT_BOUNDED_LIFETIME_0: wire::Schema = wire::Schema::Dyn;

/// `assertBoundedLifetime` over the binary wire.
#[napi(js_name = "assertBoundedLifetimeWire")]
pub fn assert_bounded_lifetime_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let claims: serde_json::Value = wire::read_value(r, &S_ASSERT_BOUNDED_LIFETIME_0)?;
        let maximum_seconds: f64 = r.f64()?;
        let label: String = r.string16()?;
        let now_seconds: f64 = r.f64()?;
        let () = port::assert_bounded_lifetime(claims, maximum_seconds, label, now_seconds)?;
        Ok(())
    })
}
