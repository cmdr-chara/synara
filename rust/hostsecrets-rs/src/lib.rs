//! JS bindings for `packages/shared/src/hostSecrets.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `encodeBase64Url`.
#[napi(js_name = "encodeBase64Url")]
pub fn encode_base64_url(bytes: String) -> napi::Result<String> {
    run(move || {
        let bytes = crate::js::from_json(&bytes)?;
        let out = port::encode_base64_url(bytes)?;
        Ok(out)
    })
}

/// `decodeBase64Url`.
#[napi(js_name = "decodeBase64Url")]
pub fn decode_base64_url(text: String) -> napi::Result<String> {
    run(move || {
        let out = port::decode_base64_url(text)?;
        crate::js::to_json(&out)
    })
}

/// `generateSyncKey`.
#[napi(js_name = "generateSyncKey")]
pub async fn generate_sync_key() -> napi::Result<String> {
    run_async(async move {
        let out = port::generate_sync_key().await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `sealHostSecret`.
#[napi(js_name = "sealHostSecret")]
pub async fn seal_host_secret(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::seal_host_secret(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `openHostSecret`.
#[napi(js_name = "openHostSecret")]
pub async fn open_host_secret(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::open_host_secret(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `generatePairingKeyPair`.
#[napi(js_name = "generatePairingKeyPair")]
pub async fn generate_pairing_key_pair() -> napi::Result<String> {
    run_async(async move {
        let out = port::generate_pairing_key_pair().await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `wrapSyncKey`.
#[napi(js_name = "wrapSyncKey")]
pub async fn wrap_sync_key(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::wrap_sync_key(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `unwrapSyncKey`.
#[napi(js_name = "unwrapSyncKey")]
pub async fn unwrap_sync_key(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::unwrap_sync_key(input).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `pairingVerificationCode`.
#[napi(js_name = "pairingVerificationCode")]
pub async fn pairing_verification_code(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::pairing_verification_code(input).await?;
        Ok(out)
    })
    .await
}

/// `rotateHostSecrets`.
#[napi(js_name = "rotateHostSecrets")]
pub async fn rotate_host_secrets(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::rotate_host_secrets(input).await?;
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

static S_ENCODE_BASE64_URL_0: wire::Schema = wire::Schema::Dyn;

/// `encodeBase64Url` over the binary wire.
#[napi(js_name = "encodeBase64UrlWire")]
pub fn encode_base64_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let bytes: serde_json::Value = wire::read_value(r, &S_ENCODE_BASE64_URL_0)?;
        let out = &port::encode_base64_url(bytes)?;
        w.string16(out);
        Ok(())
    })
}

static S_DECODE_BASE64_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `decodeBase64Url` over the binary wire.
#[napi(js_name = "decodeBase64UrlWire")]
pub fn decode_base64_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: String = r.string16()?;
        let out = &port::decode_base64_url(text)?;
        wire::write_value(w, out, &S_DECODE_BASE64_URL_OUT)?;
        Ok(())
    })
}
