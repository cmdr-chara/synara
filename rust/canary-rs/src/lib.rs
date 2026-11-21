//! JS bindings for `scripts/canary.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `resolveCanaryPaths`.
#[napi(js_name = "resolveCanaryPaths")]
pub fn resolve_canary_paths(env: String, home_directory: String) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let home_directory = crate::js::from_json(&home_directory)?;
        let out = port::resolve_canary_paths(env, home_directory)?;
        crate::js::to_json(&out)
    })
}

/// `parseCanaryArgs`.
#[napi(js_name = "parseCanaryArgs")]
pub fn parse_canary_args(argv: Vec<String>) -> napi::Result<String> {
    run(move || {
        let out = port::parse_canary_args(argv)?;
        crate::js::to_json(&out)
    })
}

/// `resolveCanaryRef`.
#[napi(js_name = "resolveCanaryRef")]
pub fn resolve_canary_ref(input: String, tracked_ref: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let tracked_ref = crate::js::from_json(&tracked_ref)?;
        let out = port::resolve_canary_ref(input, tracked_ref)?;
        crate::js::to_json(&out)
    })
}

/// `canaryCloneArgs`.
#[napi(js_name = "canaryCloneArgs")]
pub fn canary_clone_args(origin_url: String, source: String) -> napi::Result<Vec<String>> {
    run(move || {
        let out = port::canary_clone_args(origin_url, source)?;
        Ok(out)
    })
}

/// `canaryAppCommandPattern`.
#[napi(js_name = "canaryAppCommandPattern")]
pub fn canary_app_command_pattern(source: String) -> napi::Result<String> {
    run(move || {
        let out = port::canary_app_command_pattern(source)?;
        Ok(out)
    })
}

/// `runCanaryCommand`.
#[napi(js_name = "runCanaryCommand")]
pub fn run_canary_command(input: String, paths: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let paths = crate::js::from_json(&paths)?;
        let out = port::run_canary_command(input, paths)?;
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

static S_RESOLVE_CANARY_PATHS_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_CANARY_PATHS_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_CANARY_PATHS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("home", wire::Schema::Str16),
    ("source", wire::Schema::Str16),
    ("state", wire::Schema::Str16),
    ("pid", wire::Schema::Str16),
    ("log", wire::Schema::Str16),
]);

/// `resolveCanaryPaths` over the binary wire.
#[napi(js_name = "resolveCanaryPathsWire")]
pub fn resolve_canary_paths_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_RESOLVE_CANARY_PATHS_0)?;
        let home_directory: serde_json::Value = wire::read_value(r, &S_RESOLVE_CANARY_PATHS_1)?;
        let out = &port::resolve_canary_paths(env, home_directory)?;
        wire::write_value(w, out, &S_RESOLVE_CANARY_PATHS_OUT)?;
        Ok(())
    })
}

static S_PARSE_CANARY_ARGS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("command", wire::Schema::Str16),
    ("ref", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `parseCanaryArgs` over the binary wire.
#[napi(js_name = "parseCanaryArgsWire")]
pub fn parse_canary_args_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let argv: Vec<String> = r.list(|r| r.string16())?;
        let out = &port::parse_canary_args(argv)?;
        wire::write_value(w, out, &S_PARSE_CANARY_ARGS_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_CANARY_REF_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_CANARY_REF_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_CANARY_REF_OUT: wire::Schema = wire::Schema::Str16;

/// `resolveCanaryRef` over the binary wire.
#[napi(js_name = "resolveCanaryRefWire")]
pub fn resolve_canary_ref_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RESOLVE_CANARY_REF_0)?;
        let tracked_ref: serde_json::Value = wire::read_value(r, &S_RESOLVE_CANARY_REF_1)?;
        let out = &port::resolve_canary_ref(input, tracked_ref)?;
        wire::write_value(w, out, &S_RESOLVE_CANARY_REF_OUT)?;
        Ok(())
    })
}

/// `canaryCloneArgs` over the binary wire.
#[napi(js_name = "canaryCloneArgsWire")]
pub fn canary_clone_args_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let origin_url: String = r.string16()?;
        let source: String = r.string16()?;
        let out = &port::canary_clone_args(origin_url, source)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

/// `canaryAppCommandPattern` over the binary wire.
#[napi(js_name = "canaryAppCommandPatternWire")]
pub fn canary_app_command_pattern_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: String = r.string16()?;
        let out = &port::canary_app_command_pattern(source)?;
        w.string16(out);
        Ok(())
    })
}

static S_RUN_CANARY_COMMAND_0: wire::Schema = wire::Schema::Dyn;
static S_RUN_CANARY_COMMAND_1: wire::Schema = wire::Schema::Dyn;
static S_RUN_CANARY_COMMAND_OUT: wire::Schema = wire::Schema::Dyn;

/// `runCanaryCommand` over the binary wire.
#[napi(js_name = "runCanaryCommandWire")]
pub fn run_canary_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_RUN_CANARY_COMMAND_0)?;
        let paths: serde_json::Value = wire::read_value(r, &S_RUN_CANARY_COMMAND_1)?;
        let out = &port::run_canary_command(input, paths)?;
        wire::write_value(w, out, &S_RUN_CANARY_COMMAND_OUT)?;
        Ok(())
    })
}
