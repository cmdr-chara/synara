//! JS bindings for `apps/server/src/providerUsageSnapshot.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `stopProviderUsageScans`.
#[napi(js_name = "stopProviderUsageScans")]
pub fn stop_provider_usage_scans() -> napi::Result<()> {
    run(move || {
        port::stop_provider_usage_scans()?;
        Ok(())
    })
}

/// `resumeProviderUsageScans`.
#[napi(js_name = "resumeProviderUsageScans")]
pub fn resume_provider_usage_scans() -> napi::Result<()> {
    run(move || {
        port::resume_provider_usage_scans()?;
        Ok(())
    })
}

/// `readCodexSessionSummary`.
#[napi(js_name = "readCodexSessionSummary")]
pub async fn read_codex_session_summary(path: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::read_codex_session_summary(path).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `resolveClaudeProjectsRoot`.
#[napi(js_name = "resolveClaudeProjectsRoot")]
pub fn resolve_claude_projects_root(home_dir: String) -> napi::Result<String> {
    run(move || {
        let out = port::resolve_claude_projects_root(home_dir)?;
        Ok(out)
    })
}

/// `readClaudeUsageSamples`.
#[napi(js_name = "readClaudeUsageSamples")]
pub async fn read_claude_usage_samples(path: String) -> napi::Result<String> {
    run_async(async move {
        let out = port::read_claude_usage_samples(path).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `readRecentClaudeUsageSamples`.
#[napi(js_name = "readRecentClaudeUsageSamples")]
pub async fn read_recent_claude_usage_samples(
    projects_root: String,
    now_ms: Option<f64>,
    signal: String,
) -> napi::Result<String> {
    run_async(async move {
        let signal = crate::js::from_json(&signal)?;
        let out = port::read_recent_claude_usage_samples(projects_root, now_ms, signal).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `loadLocalProviderUsageLines`.
#[napi(js_name = "loadLocalProviderUsageLines")]
pub async fn load_local_provider_usage_lines(input: String) -> napi::Result<String> {
    run_async(async move {
        let input = crate::js::from_json(&input)?;
        let out = port::load_local_provider_usage_lines(input).await?;
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

/// `stopProviderUsageScans` over the binary wire.
#[napi(js_name = "stopProviderUsageScansWire")]
pub fn stop_provider_usage_scans_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::stop_provider_usage_scans()?;
        Ok(())
    })
}

/// `resumeProviderUsageScans` over the binary wire.
#[napi(js_name = "resumeProviderUsageScansWire")]
pub fn resume_provider_usage_scans_wire() -> napi::Result<f64> {
    wire_io(|_r, _w| {
        let () = port::resume_provider_usage_scans()?;
        Ok(())
    })
}

/// `resolveClaudeProjectsRoot` over the binary wire.
#[napi(js_name = "resolveClaudeProjectsRootWire")]
pub fn resolve_claude_projects_root_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let home_dir: String = r.string16()?;
        let out = &port::resolve_claude_projects_root(home_dir)?;
        w.string16(out);
        Ok(())
    })
}
