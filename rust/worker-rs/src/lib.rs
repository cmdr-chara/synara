//! JS bindings for `apps/analytics/worker.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;
pub mod corpus;
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

/// `normalizeProductEvent`.
#[napi(js_name = "normalizeProductEvent")]
pub fn normalize_product_event(raw: String, now: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let now = crate::js::from_json(&now)?;
        let out = port::normalize_product_event(raw, now)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeEvent`.
#[napi(js_name = "normalizeEvent")]
pub fn normalize_event(raw: String, now: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let now = crate::js::from_json(&now)?;
        let out = port::normalize_event(raw, now)?;
        crate::js::to_json(&out)
    })
}

/// `signSession`.
#[napi(js_name = "signSession")]
pub async fn sign_session(secret: String, expires_at_seconds: f64) -> napi::Result<String> {
    run_async(async move {
        let out = port::sign_session(secret, expires_at_seconds).await?;
        Ok(out)
    })
    .await
}

/// `verifySessionCookie`.
#[napi(js_name = "verifySessionCookie")]
pub async fn verify_session_cookie(
    cookie_header: Option<String>,
    secret: String,
    now_seconds: String,
) -> napi::Result<bool> {
    run_async(async move {
        let now_seconds = crate::js::from_json(&now_seconds)?;
        let out = port::verify_session_cookie(cookie_header, secret, now_seconds).await?;
        Ok(out)
    })
    .await
}

/// `parseApiFilters`.
#[napi(js_name = "parseApiFilters")]
pub fn parse_api_filters(url: String, now: String) -> napi::Result<String> {
    run(move || {
        let url = crate::js::from_json(&url)?;
        let now = crate::js::from_json(&now)?;
        let out = port::parse_api_filters(url, now)?;
        crate::js::to_json(&out)
    })
}

/// `compareVersionsDesc`.
#[napi(js_name = "compareVersionsDesc")]
pub fn compare_versions_desc(a: String, b: String) -> napi::Result<f64> {
    run(move || {
        let out = port::compare_versions_desc(a, b)?;
        Ok(out)
    })
}

/// `releaseHealth`.
#[napi(js_name = "releaseHealth")]
pub fn release_health(
    crash_free_pct: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let crash_free_pct = crate::js::from_json(&crash_free_pct)?;
        let out = port::release_health(crash_free_pct)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `rangeDays`.
#[napi(js_name = "rangeDays")]
pub fn range_days(since: String, now: f64) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::range_days(since, now)?;
        Ok(out)
    })
}

/// `purgeLegacyDiagnosticContent`.
#[napi(js_name = "purgeLegacyDiagnosticContent")]
pub async fn purge_legacy_diagnostic_content(env: String) -> napi::Result<String> {
    run_async(async move {
        let env = crate::js::from_json(&env)?;
        let out = port::purge_legacy_diagnostic_content(env).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `purgeLegacyDiagnosticDumps`.
#[napi(js_name = "purgeLegacyDiagnosticDumps")]
pub async fn purge_legacy_diagnostic_dumps(env: String) -> napi::Result<String> {
    run_async(async move {
        let env = crate::js::from_json(&env)?;
        let out = port::purge_legacy_diagnostic_dumps(env).await?;
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

static S_NORMALIZE_PRODUCT_EVENT_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PRODUCT_EVENT_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("id", wire::Schema::Str16),
    ("ts", wire::Schema::Str16),
    ("installId", wire::Schema::Str16),
    ("cohortDay", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("cohortStage", wire::Schema::Dyn),
    ("channel", wire::Schema::Str16),
    ("surface", wire::Schema::Str16),
    ("platform", wire::Schema::Str16),
    ("appVersion", wire::Schema::Str16),
    ("event", wire::Schema::Str16),
    ("outcome", wire::Schema::Str16),
    ("feature", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("mode", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("provider", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("reason", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("failure", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("httpStatus", wire::Schema::OptJs(&wire::Schema::Num)),
    ("command", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("durationMs", wire::Schema::OptJs(&wire::Schema::Num)),
    ("inputTokens", wire::Schema::OptJs(&wire::Schema::Num)),
    ("outputTokens", wire::Schema::OptJs(&wire::Schema::Num)),
    ("cachedInputTokens", wire::Schema::OptJs(&wire::Schema::Num)),
]));

/// `normalizeProductEvent` over the binary wire.
#[napi(js_name = "normalizeProductEventWire")]
pub fn normalize_product_event_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: serde_json::Value = wire::read_value(r, &S_NORMALIZE_PRODUCT_EVENT_0)?;
        let now: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::normalize_product_event(raw, now)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_NORMALIZE_PRODUCT_EVENT_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_NORMALIZE_EVENT_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_EVENT_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("clientId", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("ts", wire::Schema::Str16),
    ("installId", wire::Schema::Str16),
    ("flavor", wire::Schema::Str16),
    ("appVersion", wire::Schema::Str16),
    ("platform", wire::Schema::Str16),
    ("arch", wire::Schema::Str16),
    ("event", wire::Schema::Str16),
    ("kind", wire::Schema::Str16),
    ("outcome", wire::Schema::Str16),
    ("processType", wire::Schema::Str16),
    ("reason", wire::Schema::Str16),
    ("errorContext", wire::Schema::Str16),
    ("targetVersion", wire::Schema::Str16),
    ("durationMs", wire::Schema::OptJs(&wire::Schema::Num)),
    ("source", wire::Schema::Str16),
    ("fingerprint", wire::Schema::Str16),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("stack", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("logTail", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("osVersion", wire::Schema::Str16),
    ("locale", wire::Schema::Str16),
    ("projects", wire::Schema::OptJs(&wire::Schema::Num)),
    ("activeThreads", wire::Schema::OptJs(&wire::Schema::Num)),
]));

/// `normalizeEvent` over the binary wire.
#[napi(js_name = "normalizeEventWire")]
pub fn normalize_event_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: serde_json::Value = wire::read_value(r, &S_NORMALIZE_EVENT_0)?;
        let now: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::normalize_event(raw, now)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_NORMALIZE_EVENT_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_PARSE_API_FILTERS_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_API_FILTERS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("days", wire::Schema::Num),
    ("since", wire::Schema::Str16),
    ("prevSince", wire::Schema::Str16),
    ("bucketSince", wire::Schema::Str16),
    ("now", wire::Schema::Num),
    ("version", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("platform", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("flavor", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("kind", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("source", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("q", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("clause", wire::Schema::Str16),
    ("binds", wire::Schema::List(&wire::Schema::Str16)),
    ("scopeClause", wire::Schema::Str16),
    ("scopeBinds", wire::Schema::List(&wire::Schema::Str16)),
    ("eventScopeClause", wire::Schema::Str16),
    ("platformClause", wire::Schema::Str16),
    ("platformBinds", wire::Schema::List(&wire::Schema::Str16)),
]);

/// `parseApiFilters` over the binary wire.
#[napi(js_name = "parseApiFiltersWire")]
pub fn parse_api_filters_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: serde_json::Value = wire::read_value(r, &S_PARSE_API_FILTERS_0)?;
        let now: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::parse_api_filters(url, now)?;
        wire::write_value(w, out, &S_PARSE_API_FILTERS_OUT)?;
        Ok(())
    })
}

/// `compareVersionsDesc` over the binary wire.
#[napi(js_name = "compareVersionsDescWire")]
pub fn compare_versions_desc_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: String = r.string16()?;
        let b: String = r.string16()?;
        let out = &port::compare_versions_desc(a, b)?;
        w.f64(*out);
        Ok(())
    })
}

static S_RELEASE_HEALTH_0: wire::Schema = wire::Schema::Dyn;

/// `releaseHealth` over the binary wire.
#[napi(js_name = "releaseHealthWire")]
pub fn release_health_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let crash_free_pct: serde_json::Value = wire::read_value(r, &S_RELEASE_HEALTH_0)?;
        let out = &port::release_health(crash_free_pct)?;
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

/// `rangeDays` over the binary wire.
#[napi(js_name = "rangeDaysWire")]
pub fn range_days_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let since: String = r.string16()?;
        let now: f64 = r.f64()?;
        let out = &port::range_days(since, now)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}
