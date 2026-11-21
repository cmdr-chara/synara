//! JS bindings for `apps/server/src/usagePricing.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parseRateTable`.
#[napi(js_name = "parseRateTable")]
pub fn parse_rate_table(document: String) -> napi::Result<String> {
    run(move || {
        let document = crate::js::from_json(&document)?;
        let out = port::parse_rate_table(document)?;
        crate::js::to_json(&out)
    })
}

/// `lookupRate`.
#[napi(js_name = "lookupRate")]
pub fn lookup_rate(table: String, model: String) -> napi::Result<String> {
    run(move || {
        let table = crate::js::from_json(&table)?;
        let out = port::lookup_rate(table, model)?;
        crate::js::to_json(&out)
    })
}

/// `priceTokens`.
#[napi(js_name = "priceTokens")]
pub fn price_tokens(
    table: String,
    model: String,
    tokens: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let table = crate::js::from_json(&table)?;
        let tokens = crate::js::from_json(&tokens)?;
        let out = port::price_tokens(table, model, tokens)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `priceTranscriptUsage`.
#[napi(js_name = "priceTranscriptUsage")]
pub fn price_transcript_usage(table: String, usage: String) -> napi::Result<String> {
    run(move || {
        let table = crate::js::from_json(&table)?;
        let usage = crate::js::from_json(&usage)?;
        let out = port::price_transcript_usage(table, usage)?;
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

static S_PARSE_RATE_TABLE_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_RATE_TABLE_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseRateTable` over the binary wire.
#[napi(js_name = "parseRateTableWire")]
pub fn parse_rate_table_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let document: Value = wire::read_value(r, &S_PARSE_RATE_TABLE_0)?;
        let out = &port::parse_rate_table(document)?;
        wire::write_value(w, out, &S_PARSE_RATE_TABLE_OUT)?;
        Ok(())
    })
}

static S_LOOKUP_RATE_0: wire::Schema = wire::Schema::Dyn;
static S_LOOKUP_RATE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    (
        "tiers",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("aboveTokens", wire::Schema::Num),
            ("rates", wire::Schema::Dyn),
        ])),
    ),
    ("fastMultiplier", wire::Schema::Num),
]));

/// `lookupRate` over the binary wire.
#[napi(js_name = "lookupRateWire")]
pub fn lookup_rate_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let table: Value = wire::read_value(r, &S_LOOKUP_RATE_0)?;
        let model: String = r.string16()?;
        let out = &port::lookup_rate(table, model)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_LOOKUP_RATE_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_PRICE_TOKENS_0: wire::Schema = wire::Schema::Dyn;
static S_PRICE_TOKENS_2: wire::Schema = wire::Schema::Dyn;

/// `priceTokens` over the binary wire.
#[napi(js_name = "priceTokensWire")]
pub fn price_tokens_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let table: Value = wire::read_value(r, &S_PRICE_TOKENS_0)?;
        let model: String = r.string16()?;
        let tokens: Value = wire::read_value(r, &S_PRICE_TOKENS_2)?;
        let out = &port::price_tokens(table, model, tokens)?;
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

static S_PRICE_TRANSCRIPT_USAGE_0: wire::Schema = wire::Schema::Dyn;
static S_PRICE_TRANSCRIPT_USAGE_1: wire::Schema = wire::Schema::Dyn;
static S_PRICE_TRANSCRIPT_USAGE_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Num);

/// `priceTranscriptUsage` over the binary wire.
#[napi(js_name = "priceTranscriptUsageWire")]
pub fn price_transcript_usage_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let table: Value = wire::read_value(r, &S_PRICE_TRANSCRIPT_USAGE_0)?;
        let usage: Value = wire::read_value(r, &S_PRICE_TRANSCRIPT_USAGE_1)?;
        let out = &port::price_transcript_usage(table, usage)?;
        wire::write_value(w, out, &S_PRICE_TRANSCRIPT_USAGE_OUT)?;
        Ok(())
    })
}
