//! JS bindings for `apps/server/src/accountUsageReporter.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isAccountUsageRelevantEventType`.
#[napi(js_name = "isAccountUsageRelevantEventType")]
pub fn is_account_usage_relevant_event_type(event_type: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_account_usage_relevant_event_type(event_type)?;
        Ok(out)
    })
}

/// `minuteStartIso`.
#[napi(js_name = "minuteStartIso")]
pub fn minute_start_iso(ms: f64) -> napi::Result<String> {
    run(move || {
        let out = port::minute_start_iso(ms)?;
        Ok(out)
    })
}

/// `rawTimestampLowerBound`.
#[napi(js_name = "rawTimestampLowerBound")]
pub fn raw_timestamp_lower_bound(from_minute: String) -> napi::Result<String> {
    run(move || {
        let out = port::raw_timestamp_lower_bound(from_minute)?;
        Ok(out)
    })
}

/// `usageBucketQueries`.
#[napi(js_name = "usageBucketQueries")]
pub fn usage_bucket_queries(sql: String, from_minute: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let out = port::usage_bucket_queries(sql, from_minute)?;
        crate::js::to_json(&out)
    })
}

/// `collectUsageBuckets`.
#[napi(js_name = "collectUsageBuckets")]
pub fn collect_usage_buckets(sql: String, from_minute: String) -> napi::Result<String> {
    run(move || {
        let sql = crate::js::from_json(&sql)?;
        let out = port::collect_usage_buckets(sql, from_minute)?;
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

/// `isAccountUsageRelevantEventType` over the binary wire.
#[napi(js_name = "isAccountUsageRelevantEventTypeWire")]
pub fn is_account_usage_relevant_event_type_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let event_type: String = r.string16()?;
        let out = &port::is_account_usage_relevant_event_type(event_type)?;
        w.bool(*out);
        Ok(())
    })
}

/// `minuteStartIso` over the binary wire.
#[napi(js_name = "minuteStartIsoWire")]
pub fn minute_start_iso_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ms: f64 = r.f64()?;
        let out = &port::minute_start_iso(ms)?;
        w.string16(out);
        Ok(())
    })
}

/// `rawTimestampLowerBound` over the binary wire.
#[napi(js_name = "rawTimestampLowerBoundWire")]
pub fn raw_timestamp_lower_bound_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let from_minute: String = r.string16()?;
        let out = &port::raw_timestamp_lower_bound(from_minute)?;
        w.string16(out);
        Ok(())
    })
}

static S_USAGE_BUCKET_QUERIES_0: wire::Schema = wire::Schema::Dyn;
static S_USAGE_BUCKET_QUERIES_OUT: wire::Schema = wire::Schema::Dyn;

/// `usageBucketQueries` over the binary wire.
#[napi(js_name = "usageBucketQueriesWire")]
pub fn usage_bucket_queries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_USAGE_BUCKET_QUERIES_0)?;
        let from_minute: String = r.string16()?;
        let out = &port::usage_bucket_queries(sql, from_minute)?;
        wire::write_value(w, out, &S_USAGE_BUCKET_QUERIES_OUT)?;
        Ok(())
    })
}

static S_COLLECT_USAGE_BUCKETS_0: wire::Schema = wire::Schema::Dyn;
static S_COLLECT_USAGE_BUCKETS_OUT: wire::Schema = wire::Schema::Dyn;

/// `collectUsageBuckets` over the binary wire.
#[napi(js_name = "collectUsageBucketsWire")]
pub fn collect_usage_buckets_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let sql: serde_json::Value = wire::read_value(r, &S_COLLECT_USAGE_BUCKETS_0)?;
        let from_minute: String = r.string16()?;
        let out = &port::collect_usage_buckets(sql, from_minute)?;
        wire::write_value(w, out, &S_COLLECT_USAGE_BUCKETS_OUT)?;
        Ok(())
    })
}
