//! JS bindings for `apps/desktop/src/resumableUpdateDownloadPolicy.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `computeProgressInfo`.
#[napi(js_name = "computeProgressInfo")]
pub fn compute_progress_info(args: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::compute_progress_info(args)?;
        crate::js::to_json(&out)
    })
}

/// `parseContentRangeTotal`.
#[napi(js_name = "parseContentRangeTotal")]
pub fn parse_content_range_total(
    header_value: Option<String>,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let out = port::parse_content_range_total(header_value)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `selectSha512Encoding`.
#[napi(js_name = "selectSha512Encoding")]
pub fn select_sha512_encoding(sha512: String) -> napi::Result<String> {
    run(move || {
        let out = port::select_sha512_encoding(sha512)?;
        Ok(out)
    })
}

/// `classifyDownloadResponse`.
#[napi(js_name = "classifyDownloadResponse")]
pub fn classify_download_response(args: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::classify_download_response(args)?;
        crate::js::to_json(&out)
    })
}

/// `computeRetryDelayMs`.
#[napi(js_name = "computeRetryDelayMs")]
pub fn compute_retry_delay_ms(consecutive_stall_count: f64, config: String) -> napi::Result<f64> {
    run(move || {
        let config = crate::js::from_json(&config)?;
        let out = port::compute_retry_delay_ms(consecutive_stall_count, config)?;
        Ok(out)
    })
}

/// `shouldGiveUp`.
#[napi(js_name = "shouldGiveUp")]
pub fn should_give_up(args: String) -> napi::Result<bool> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::should_give_up(args)?;
        Ok(out)
    })
}

/// `isCrossOrigin`.
#[napi(js_name = "isCrossOrigin")]
pub fn is_cross_origin(a: String, b: String) -> napi::Result<bool> {
    run(move || {
        let a = crate::js::from_json(&a)?;
        let b = crate::js::from_json(&b)?;
        let out = port::is_cross_origin(a, b)?;
        Ok(out)
    })
}

/// `buildDownloadHeaders`.
#[napi(js_name = "buildDownloadHeaders")]
pub fn build_download_headers(args: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::build_download_headers(args)?;
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

static S_COMPUTE_PROGRESS_INFO_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTE_PROGRESS_INFO_OUT: wire::Schema = wire::Schema::Obj(&[
    ("total", wire::Schema::Num),
    ("delta", wire::Schema::Num),
    ("transferred", wire::Schema::Num),
    ("percent", wire::Schema::Num),
    ("bytesPerSecond", wire::Schema::Num),
]);

/// `computeProgressInfo` over the binary wire.
#[napi(js_name = "computeProgressInfoWire")]
pub fn compute_progress_info_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_COMPUTE_PROGRESS_INFO_0)?;
        let out = &port::compute_progress_info(args)?;
        wire::write_value(w, out, &S_COMPUTE_PROGRESS_INFO_OUT)?;
        Ok(())
    })
}

/// `parseContentRangeTotal` over the binary wire.
#[napi(js_name = "parseContentRangeTotalWire")]
pub fn parse_content_range_total_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let header_value: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::parse_content_range_total(header_value)?;
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

/// `selectSha512Encoding` over the binary wire.
#[napi(js_name = "selectSha512EncodingWire")]
pub fn select_sha512_encoding_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sha512: String = r.string16()?;
        let out = &port::select_sha512_encoding(sha512)?;
        w.string16(out);
        Ok(())
    })
}

static S_CLASSIFY_DOWNLOAD_RESPONSE_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_DOWNLOAD_RESPONSE_OUT: wire::Schema = wire::Schema::Dyn;

/// `classifyDownloadResponse` over the binary wire.
#[napi(js_name = "classifyDownloadResponseWire")]
pub fn classify_download_response_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_CLASSIFY_DOWNLOAD_RESPONSE_0)?;
        let out = &port::classify_download_response(args)?;
        wire::write_value(w, out, &S_CLASSIFY_DOWNLOAD_RESPONSE_OUT)?;
        Ok(())
    })
}

static S_COMPUTE_RETRY_DELAY_MS_1: wire::Schema = wire::Schema::Dyn;

/// `computeRetryDelayMs` over the binary wire.
#[napi(js_name = "computeRetryDelayMsWire")]
pub fn compute_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let consecutive_stall_count: f64 = r.f64()?;
        let config: serde_json::Value = wire::read_value(r, &S_COMPUTE_RETRY_DELAY_MS_1)?;
        let out = &port::compute_retry_delay_ms(consecutive_stall_count, config)?;
        w.f64(*out);
        Ok(())
    })
}

static S_SHOULD_GIVE_UP_0: wire::Schema = wire::Schema::Dyn;

/// `shouldGiveUp` over the binary wire.
#[napi(js_name = "shouldGiveUpWire")]
pub fn should_give_up_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_SHOULD_GIVE_UP_0)?;
        let out = &port::should_give_up(args)?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_CROSS_ORIGIN_0: wire::Schema = wire::Schema::Dyn;
static S_IS_CROSS_ORIGIN_1: wire::Schema = wire::Schema::Dyn;

/// `isCrossOrigin` over the binary wire.
#[napi(js_name = "isCrossOriginWire")]
pub fn is_cross_origin_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: serde_json::Value = wire::read_value(r, &S_IS_CROSS_ORIGIN_0)?;
        let b: serde_json::Value = wire::read_value(r, &S_IS_CROSS_ORIGIN_1)?;
        let out = &port::is_cross_origin(a, b)?;
        w.bool(*out);
        Ok(())
    })
}

static S_BUILD_DOWNLOAD_HEADERS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_DOWNLOAD_HEADERS_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Str16);

/// `buildDownloadHeaders` over the binary wire.
#[napi(js_name = "buildDownloadHeadersWire")]
pub fn build_download_headers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_BUILD_DOWNLOAD_HEADERS_0)?;
        let out = &port::build_download_headers(args)?;
        wire::write_value(w, out, &S_BUILD_DOWNLOAD_HEADERS_OUT)?;
        Ok(())
    })
}
