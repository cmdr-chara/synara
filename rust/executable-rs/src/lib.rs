//! JS bindings for `packages/shared/src/executable.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `envPathKeyFor`.
#[napi(js_name = "envPathKeyFor")]
pub fn env_path_key_for(env: String, platform: String) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::env_path_key_for(env, platform)?;
        Ok(out)
    })
}

/// `hasPathSeparator`.
#[napi(js_name = "hasPathSeparator")]
pub fn has_path_separator(command: String) -> napi::Result<bool> {
    run(move || {
        let out = port::has_path_separator(command)?;
        Ok(out)
    })
}

/// `windowsPathExtensions`.
#[napi(js_name = "windowsPathExtensions")]
pub fn windows_path_extensions(env: String) -> napi::Result<Vec<String>> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::windows_path_extensions(env)?;
        Ok(out)
    })
}

/// `pathEntries`.
#[napi(js_name = "pathEntries")]
pub fn path_entries(env: String, platform: String) -> napi::Result<Vec<String>> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::path_entries(env, platform)?;
        Ok(out)
    })
}

/// `executableNameCandidates`.
#[napi(js_name = "executableNameCandidates")]
pub fn executable_name_candidates(
    command: String,
    platform: String,
    env: String,
    allow_extensionless: Option<bool>,
) -> napi::Result<Vec<String>> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let env = crate::js::from_json(&env)?;
        let out = port::executable_name_candidates(command, platform, env, allow_extensionless)?;
        Ok(out)
    })
}

/// `isExecutableFile`.
#[napi(js_name = "isExecutableFile")]
pub fn is_executable_file(file_path: String, options: String) -> napi::Result<bool> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::is_executable_file(file_path, options)?;
        Ok(out)
    })
}

/// `resolveExecutable`.
#[napi(js_name = "resolveExecutable")]
pub fn resolve_executable(
    command: String,
    options: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let options = crate::js::from_json(&options)?;
        let out = port::resolve_executable(command, options)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `executableIdentity`.
#[napi(js_name = "executableIdentity")]
pub fn executable_identity(
    file_path: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let out = port::executable_identity(file_path)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
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

static S_ENV_PATH_KEY_FOR_0: wire::Schema = wire::Schema::Dyn;

/// `envPathKeyFor` over the binary wire.
#[napi(js_name = "envPathKeyForWire")]
pub fn env_path_key_for_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_ENV_PATH_KEY_FOR_0)?;
        let platform: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::env_path_key_for(env, platform)?;
        w.string16(out);
        Ok(())
    })
}

/// `hasPathSeparator` over the binary wire.
#[napi(js_name = "hasPathSeparatorWire")]
pub fn has_path_separator_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: String = r.string16()?;
        let out = &port::has_path_separator(command)?;
        w.bool(*out);
        Ok(())
    })
}

static S_WINDOWS_PATH_EXTENSIONS_0: wire::Schema = wire::Schema::Dyn;

/// `windowsPathExtensions` over the binary wire.
#[napi(js_name = "windowsPathExtensionsWire")]
pub fn windows_path_extensions_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_WINDOWS_PATH_EXTENSIONS_0)?;
        let out = &port::windows_path_extensions(env)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_PATH_ENTRIES_0: wire::Schema = wire::Schema::Dyn;
static S_PATH_ENTRIES_1: wire::Schema = wire::Schema::Dyn;

/// `pathEntries` over the binary wire.
#[napi(js_name = "pathEntriesWire")]
pub fn path_entries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_PATH_ENTRIES_0)?;
        let platform: serde_json::Value = wire::read_value(r, &S_PATH_ENTRIES_1)?;
        let out = &port::path_entries(env, platform)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_EXECUTABLE_NAME_CANDIDATES_1: wire::Schema = wire::Schema::Dyn;
static S_EXECUTABLE_NAME_CANDIDATES_2: wire::Schema = wire::Schema::Dyn;

/// `executableNameCandidates` over the binary wire.
#[napi(js_name = "executableNameCandidatesWire")]
pub fn executable_name_candidates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: String = r.string16()?;
        let platform: serde_json::Value = wire::read_value(r, &S_EXECUTABLE_NAME_CANDIDATES_1)?;
        let env: serde_json::Value = wire::read_value(r, &S_EXECUTABLE_NAME_CANDIDATES_2)?;
        let allow_extensionless: Option<bool> = r.opt_js(|r| r.bool())?;
        let out = &port::executable_name_candidates(command, platform, env, allow_extensionless)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_IS_EXECUTABLE_FILE_1: wire::Schema = wire::Schema::Dyn;

/// `isExecutableFile` over the binary wire.
#[napi(js_name = "isExecutableFileWire")]
pub fn is_executable_file_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let options: serde_json::Value = wire::read_value(r, &S_IS_EXECUTABLE_FILE_1)?;
        let out = &port::is_executable_file(file_path, options)?;
        w.bool(*out);
        Ok(())
    })
}

static S_RESOLVE_EXECUTABLE_1: wire::Schema = wire::Schema::Dyn;

/// `resolveExecutable` over the binary wire.
#[napi(js_name = "resolveExecutableWire")]
pub fn resolve_executable_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let command: String = r.string16()?;
        let options: serde_json::Value = wire::read_value(r, &S_RESOLVE_EXECUTABLE_1)?;
        let out = &port::resolve_executable(command, options)?;
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

/// `executableIdentity` over the binary wire.
#[napi(js_name = "executableIdentityWire")]
pub fn executable_identity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let out = &port::executable_identity(file_path)?;
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
