//! JS bindings for `apps/server/src/usageAnalyticsStats.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveUsageAnalyticsWindow`.
#[napi(js_name = "resolveUsageAnalyticsWindow")]
pub fn resolve_usage_analytics_window(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::resolve_usage_analytics_window(input)?;
        crate::js::to_json(&out)
    })
}

/// `estimateCost`.
#[napi(js_name = "estimateCost")]
pub fn estimate_cost(
    prices: String,
    model: String,
    total: f64,
    row: String,
) -> napi::Result<String> {
    run(move || {
        let prices = crate::js::from_json(&prices)?;
        let row = crate::js::from_json(&row)?;
        let out = port::estimate_cost(prices, model, total, row)?;
        crate::js::to_json(&out)
    })
}

/// `reportingProviders`.
#[napi(js_name = "reportingProviders")]
pub fn reporting_providers(cost_rows: String) -> napi::Result<String> {
    run(move || {
        let cost_rows = crate::js::from_json(&cost_rows)?;
        let out = port::reporting_providers(cost_rows)?;
        crate::js::to_json(&out)
    })
}

/// `needsPrices`.
#[napi(js_name = "needsPrices")]
pub fn needs_prices(token_rows: String, reporting: String) -> napi::Result<bool> {
    run(move || {
        let token_rows = crate::js::from_json(&token_rows)?;
        let reporting = crate::js::from_json(&reporting)?;
        let out = port::needs_prices(token_rows, reporting)?;
        Ok(out)
    })
}

/// `buildUsageAnalytics`.
#[napi(js_name = "buildUsageAnalytics")]
pub fn build_usage_analytics(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_usage_analytics(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildTranscriptRows`.
#[napi(js_name = "buildTranscriptRows")]
pub fn build_transcript_rows(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_transcript_rows(input)?;
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

static S_RESOLVE_USAGE_ANALYTICS_WINDOW_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_USAGE_ANALYTICS_WINDOW_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveUsageAnalyticsWindow` over the binary wire.
#[napi(js_name = "resolveUsageAnalyticsWindowWire")]
pub fn resolve_usage_analytics_window_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_USAGE_ANALYTICS_WINDOW_0)?;
        let out = &port::resolve_usage_analytics_window(input)?;
        wire::write_value(w, out, &S_RESOLVE_USAGE_ANALYTICS_WINDOW_OUT)?;
        Ok(())
    })
}

static S_ESTIMATE_COST_0: wire::Schema = wire::Schema::Dyn;
static S_ESTIMATE_COST_3: wire::Schema = wire::Schema::Dyn;
static S_ESTIMATE_COST_OUT: wire::Schema = wire::Schema::Obj(&[
    ("costUsd", wire::Schema::Num),
    ("unpricedTokens", wire::Schema::Num),
    ("unsplitTokens", wire::Schema::Num),
]);

/// `estimateCost` over the binary wire.
#[napi(js_name = "estimateCostWire")]
pub fn estimate_cost_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let prices: serde_json::Value = wire::read_value(r, &S_ESTIMATE_COST_0)?;
        let model: String = r.string16()?;
        let total: f64 = r.f64()?;
        let row: serde_json::Value = wire::read_value(r, &S_ESTIMATE_COST_3)?;
        let out = &port::estimate_cost(prices, model, total, row)?;
        wire::write_value(w, out, &S_ESTIMATE_COST_OUT)?;
        Ok(())
    })
}

static S_REPORTING_PROVIDERS_0: wire::Schema = wire::Schema::Dyn;
static S_REPORTING_PROVIDERS_OUT: wire::Schema = wire::Schema::List(&wire::Schema::Str16);

/// `reportingProviders` over the binary wire.
#[napi(js_name = "reportingProvidersWire")]
pub fn reporting_providers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cost_rows: serde_json::Value = wire::read_value(r, &S_REPORTING_PROVIDERS_0)?;
        let out = &port::reporting_providers(cost_rows)?;
        wire::write_value(w, out, &S_REPORTING_PROVIDERS_OUT)?;
        Ok(())
    })
}

static S_NEEDS_PRICES_0: wire::Schema = wire::Schema::Dyn;
static S_NEEDS_PRICES_1: wire::Schema = wire::Schema::Dyn;

/// `needsPrices` over the binary wire.
#[napi(js_name = "needsPricesWire")]
pub fn needs_prices_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let token_rows: serde_json::Value = wire::read_value(r, &S_NEEDS_PRICES_0)?;
        let reporting: serde_json::Value = wire::read_value(r, &S_NEEDS_PRICES_1)?;
        let out = &port::needs_prices(token_rows, reporting)?;
        w.bool(*out);
        Ok(())
    })
}

static S_BUILD_USAGE_ANALYTICS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_USAGE_ANALYTICS_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildUsageAnalytics` over the binary wire.
#[napi(js_name = "buildUsageAnalyticsWire")]
pub fn build_usage_analytics_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_USAGE_ANALYTICS_0)?;
        let out = &port::build_usage_analytics(input)?;
        wire::write_value(w, out, &S_BUILD_USAGE_ANALYTICS_OUT)?;
        Ok(())
    })
}

static S_BUILD_TRANSCRIPT_ROWS_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_TRANSCRIPT_ROWS_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildTranscriptRows` over the binary wire.
#[napi(js_name = "buildTranscriptRowsWire")]
pub fn build_transcript_rows_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_TRANSCRIPT_ROWS_0)?;
        let out = &port::build_transcript_rows(input)?;
        wire::write_value(w, out, &S_BUILD_TRANSCRIPT_ROWS_OUT)?;
        Ok(())
    })
}
