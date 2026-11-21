//! JS bindings for `apps/web/src/projectScripts.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `commandForProjectScript`.
#[napi(js_name = "commandForProjectScript")]
pub fn command_for_project_script(script_id: String) -> napi::Result<String> {
    run(move || {
        let out = port::command_for_project_script(script_id)?;
        crate::js::to_json(&out)
    })
}

/// `projectScriptIdFromCommand`.
#[napi(js_name = "projectScriptIdFromCommand")]
pub fn project_script_id_from_command(
    command: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::project_script_id_from_command(command)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `nextProjectScriptId`.
#[napi(js_name = "nextProjectScriptId")]
pub fn next_project_script_id(name: String, existing_ids: String) -> napi::Result<String> {
    run(move || {
        let existing_ids = crate::js::from_json(&existing_ids)?;
        let out = port::next_project_script_id(name, existing_ids)?;
        Ok(out)
    })
}

/// `projectScriptRuntimeEnv`.
#[napi(js_name = "projectScriptRuntimeEnv")]
pub fn project_script_runtime_env(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::project_script_runtime_env(input)?;
        crate::js::to_json(&out)
    })
}

/// `primaryProjectScript`.
#[napi(js_name = "primaryProjectScript")]
pub fn primary_project_script(scripts: String) -> napi::Result<String> {
    run(move || {
        let scripts = crate::js::from_json(&scripts)?;
        let out = port::primary_project_script(scripts)?;
        crate::js::to_json(&out)
    })
}

/// `setupProjectScript`.
#[napi(js_name = "setupProjectScript")]
pub fn setup_project_script(scripts: String) -> napi::Result<String> {
    run(move || {
        let scripts = crate::js::from_json(&scripts)?;
        let out = port::setup_project_script(scripts)?;
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

static S_COMMAND_FOR_PROJECT_SCRIPT_OUT: wire::Schema = wire::Schema::Dyn;

/// `commandForProjectScript` over the binary wire.
#[napi(js_name = "commandForProjectScriptWire")]
pub fn command_for_project_script_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let script_id: String = r.string16()?;
        let out = &port::command_for_project_script(script_id)?;
        wire::write_value(w, out, &S_COMMAND_FOR_PROJECT_SCRIPT_OUT)?;
        Ok(())
    })
}

/// `projectScriptIdFromCommand` over the binary wire.
#[napi(js_name = "projectScriptIdFromCommandWire")]
pub fn project_script_id_from_command_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: String = r.string16()?;
        let out = &port::project_script_id_from_command(command)?;
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

static S_NEXT_PROJECT_SCRIPT_ID_1: wire::Schema = wire::Schema::Dyn;

/// `nextProjectScriptId` over the binary wire.
#[napi(js_name = "nextProjectScriptIdWire")]
pub fn next_project_script_id_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: String = r.string16()?;
        let existing_ids: serde_json::Value = wire::read_value(r, &S_NEXT_PROJECT_SCRIPT_ID_1)?;
        let out = &port::next_project_script_id(name, existing_ids)?;
        w.string16(out);
        Ok(())
    })
}

static S_PROJECT_SCRIPT_RUNTIME_ENV_0: wire::Schema = wire::Schema::Dyn;
static S_PROJECT_SCRIPT_RUNTIME_ENV_OUT: wire::Schema = wire::Schema::Map16(&wire::Schema::Str16);

/// `projectScriptRuntimeEnv` over the binary wire.
#[napi(js_name = "projectScriptRuntimeEnvWire")]
pub fn project_script_runtime_env_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PROJECT_SCRIPT_RUNTIME_ENV_0)?;
        let out = &port::project_script_runtime_env(input)?;
        wire::write_value(w, out, &S_PROJECT_SCRIPT_RUNTIME_ENV_OUT)?;
        Ok(())
    })
}

static S_PRIMARY_PROJECT_SCRIPT_0: wire::Schema = wire::Schema::Dyn;

/// `primaryProjectScript` over the binary wire.
#[napi(js_name = "primaryProjectScriptWire")]
pub fn primary_project_script_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let scripts: serde_json::Value = wire::read_value(r, &S_PRIMARY_PROJECT_SCRIPT_0)?;
        let out = &port::primary_project_script(scripts)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SETUP_PROJECT_SCRIPT_0: wire::Schema = wire::Schema::Dyn;

/// `setupProjectScript` over the binary wire.
#[napi(js_name = "setupProjectScriptWire")]
pub fn setup_project_script_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let scripts: serde_json::Value = wire::read_value(r, &S_SETUP_PROJECT_SCRIPT_0)?;
        let out = &port::setup_project_script(scripts)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}
