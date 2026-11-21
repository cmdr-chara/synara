//! JS bindings for `scripts/dev-runner.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `readDevRunnerBooleanEnvironment`.
#[napi(js_name = "readDevRunnerBooleanEnvironment")]
pub fn read_dev_runner_boolean_environment(environment: String) -> napi::Result<String> {
    run(move || {
        let environment = crate::js::from_json(&environment)?;
        let out = port::read_dev_runner_boolean_environment(environment)?;
        crate::js::to_json(&out)
    })
}

/// `resolveOffset`.
#[napi(js_name = "resolveOffset")]
pub fn resolve_offset(config: String) -> napi::Result<String> {
    run(move || {
        let config = crate::js::from_json(&config)?;
        let out = port::resolve_offset(config)?;
        crate::js::to_json(&out)
    })
}

/// `createDevRunnerEnv`.
#[napi(js_name = "createDevRunnerEnv")]
pub fn create_dev_runner_env(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::create_dev_runner_env(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `resolveDevRunnerBooleanOverrides`.
#[napi(js_name = "resolveDevRunnerBooleanOverrides")]
pub fn resolve_dev_runner_boolean_overrides(
    input: String,
    environment: String,
) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let environment = crate::js::from_json(&environment)?;
        let out = port::resolve_dev_runner_boolean_overrides(input, environment)?;
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

static S_READ_DEV_RUNNER_BOOLEAN_ENVIRONMENT_0: wire::Schema = wire::Schema::Dyn;
static S_READ_DEV_RUNNER_BOOLEAN_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `readDevRunnerBooleanEnvironment` over the binary wire.
#[napi(js_name = "readDevRunnerBooleanEnvironmentWire")]
pub fn read_dev_runner_boolean_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let environment: serde_json::Value =
            wire::read_value(r, &S_READ_DEV_RUNNER_BOOLEAN_ENVIRONMENT_0)?;
        let out = &port::read_dev_runner_boolean_environment(environment)?;
        wire::write_value(w, out, &S_READ_DEV_RUNNER_BOOLEAN_ENVIRONMENT_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_OFFSET_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_OFFSET_OUT: wire::Schema = wire::Schema::Obj(&[
    ("offset", wire::Schema::Num),
    ("source", wire::Schema::Str16),
]);

/// `resolveOffset` over the binary wire.
#[napi(js_name = "resolveOffsetWire")]
pub fn resolve_offset_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let config: serde_json::Value = wire::read_value(r, &S_RESOLVE_OFFSET_0)?;
        let out = &port::resolve_offset(config)?;
        wire::write_value(w, out, &S_RESOLVE_OFFSET_OUT)?;
        Ok(())
    })
}

static S_CREATE_DEV_RUNNER_ENV_0: wire::Schema = wire::Schema::Dyn;
static S_CREATE_DEV_RUNNER_ENV_OUT: wire::Schema = wire::Schema::Dyn;

/// `createDevRunnerEnv` over the binary wire.
#[napi(js_name = "createDevRunnerEnvWire")]
pub fn create_dev_runner_env_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: serde_json::Value = wire::read_value(r, &S_CREATE_DEV_RUNNER_ENV_0)?;
        let out = &port::create_dev_runner_env(arg0)?;
        wire::write_value(w, out, &S_CREATE_DEV_RUNNER_ENV_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_DEV_RUNNER_BOOLEAN_OVERRIDES_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_DEV_RUNNER_BOOLEAN_OVERRIDES_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_DEV_RUNNER_BOOLEAN_OVERRIDES_OUT: wire::Schema = wire::Schema::Obj(&[
    ("noBrowser", wire::Schema::OptJs(&wire::Schema::Bool)),
    (
        "autoBootstrapProjectFromCwd",
        wire::Schema::OptJs(&wire::Schema::Bool),
    ),
    (
        "logWebSocketEvents",
        wire::Schema::OptJs(&wire::Schema::Bool),
    ),
]);

/// `resolveDevRunnerBooleanOverrides` over the binary wire.
#[napi(js_name = "resolveDevRunnerBooleanOverridesWire")]
pub fn resolve_dev_runner_boolean_overrides_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_DEV_RUNNER_BOOLEAN_OVERRIDES_0)?;
        let environment: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_DEV_RUNNER_BOOLEAN_OVERRIDES_1)?;
        let out = &port::resolve_dev_runner_boolean_overrides(input, environment)?;
        wire::write_value(w, out, &S_RESOLVE_DEV_RUNNER_BOOLEAN_OVERRIDES_OUT)?;
        Ok(())
    })
}
