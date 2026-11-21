//! JS bindings for `scripts/verify-packaged-desktop-startup.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `parsePackagedDesktopStartupArgs`.
#[napi(js_name = "parsePackagedDesktopStartupArgs")]
pub fn parse_packaged_desktop_startup_args(argv: String) -> napi::Result<String> {
    run(move || {
        let argv = crate::js::from_json(&argv)?;
        let out = port::parse_packaged_desktop_startup_args(argv)?;
        crate::js::to_json(&out)
    })
}

/// `verifyPackagedRuntimeDependencies`.
#[napi(js_name = "verifyPackagedRuntimeDependencies")]
pub async fn verify_packaged_runtime_dependencies(
    runtime: String,
    isolated_environment: String,
    timeout_ms: f64,
) -> napi::Result<()> {
    run_async(async move {
        let runtime = crate::js::from_json(&runtime)?;
        let isolated_environment = crate::js::from_json(&isolated_environment)?;
        port::verify_packaged_runtime_dependencies(runtime, isolated_environment, timeout_ms)
            .await?;
        Ok(())
    })
    .await
}

/// `createPackagedDesktopSmokeEnvironment`.
#[napi(js_name = "createPackagedDesktopSmokeEnvironment")]
pub fn create_packaged_desktop_smoke_environment(
    root: String,
    options: String,
    inherited_environment: String,
) -> napi::Result<String> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let inherited_environment = crate::js::from_json(&inherited_environment)?;
        let out =
            port::create_packaged_desktop_smoke_environment(root, options, inherited_environment)?;
        crate::js::to_json(&out)
    })
}

/// `readPackagedStartupLogTails`.
#[napi(js_name = "readPackagedStartupLogTails")]
pub fn read_packaged_startup_log_tails(log_directory: String) -> napi::Result<String> {
    run(move || {
        let out = port::read_packaged_startup_log_tails(log_directory)?;
        Ok(out)
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

static S_PARSE_PACKAGED_DESKTOP_STARTUP_ARGS_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_PACKAGED_DESKTOP_STARTUP_ARGS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("assetsDirectory", wire::Schema::Str16),
    ("platform", wire::Schema::Str16),
    ("arch", wire::Schema::Str16),
    ("version", wire::Schema::Str16),
    ("timeoutMs", wire::Schema::Num),
    ("executableName", wire::Schema::Str16),
]);

/// `parsePackagedDesktopStartupArgs` over the binary wire.
#[napi(js_name = "parsePackagedDesktopStartupArgsWire")]
pub fn parse_packaged_desktop_startup_args_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let argv: Value = wire::read_value(r, &S_PARSE_PACKAGED_DESKTOP_STARTUP_ARGS_0)?;
        let out = &port::parse_packaged_desktop_startup_args(argv)?;
        wire::write_value(w, out, &S_PARSE_PACKAGED_DESKTOP_STARTUP_ARGS_OUT)?;
        Ok(())
    })
}

static S_CREATE_PACKAGED_DESKTOP_SMOKE_ENVIRONMENT_1: wire::Schema = wire::Schema::Dyn;
static S_CREATE_PACKAGED_DESKTOP_SMOKE_ENVIRONMENT_2: wire::Schema = wire::Schema::Dyn;
static S_CREATE_PACKAGED_DESKTOP_SMOKE_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `createPackagedDesktopSmokeEnvironment` over the binary wire.
#[napi(js_name = "createPackagedDesktopSmokeEnvironmentWire")]
pub fn create_packaged_desktop_smoke_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let root: String = r.string16()?;
        let options: Value = wire::read_value(r, &S_CREATE_PACKAGED_DESKTOP_SMOKE_ENVIRONMENT_1)?;
        let inherited_environment: Value =
            wire::read_value(r, &S_CREATE_PACKAGED_DESKTOP_SMOKE_ENVIRONMENT_2)?;
        let out =
            &port::create_packaged_desktop_smoke_environment(root, options, inherited_environment)?;
        wire::write_value(w, out, &S_CREATE_PACKAGED_DESKTOP_SMOKE_ENVIRONMENT_OUT)?;
        Ok(())
    })
}

/// `readPackagedStartupLogTails` over the binary wire.
#[napi(js_name = "readPackagedStartupLogTailsWire")]
pub fn read_packaged_startup_log_tails_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let log_directory: String = r.string16()?;
        let out = &port::read_packaged_startup_log_tails(log_directory)?;
        w.string16(out);
        Ok(())
    })
}
