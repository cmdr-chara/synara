//! JS bindings for `packages/shared/src/shell.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isShellEnvironmentHydrated`.
#[napi(js_name = "isShellEnvironmentHydrated")]
pub fn is_shell_environment_hydrated(env: String) -> napi::Result<bool> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::is_shell_environment_hydrated(env)?;
        Ok(out)
    })
}

/// `applyShellEnvironmentHydrationMarker`.
#[napi(js_name = "applyShellEnvironmentHydrationMarker")]
pub fn apply_shell_environment_hydration_marker(
    env: String,
    hydrated: bool,
) -> napi::Result<String> {
    run(move || {
        let env = crate::js::from_json(&env)?;
        let out = port::apply_shell_environment_hydration_marker(env, hydrated)?;
        crate::js::to_json(&out)
    })
}

/// `listLoginShellCandidates`.
#[napi(js_name = "listLoginShellCandidates")]
pub fn list_login_shell_candidates(
    platform: String,
    shell: Option<String>,
    user_shell: String,
) -> napi::Result<Vec<String>> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let user_shell = crate::js::from_json(&user_shell)?;
        let out = port::list_login_shell_candidates(platform, shell, user_shell)?;
        Ok(out)
    })
}

/// `resolveLoginShell`.
#[napi(js_name = "resolveLoginShell")]
pub fn resolve_login_shell(platform: String, shell: Option<String>) -> napi::Result<String> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let out = port::resolve_login_shell(platform, shell)?;
        crate::js::to_json(&out)
    })
}

/// `mergePathEntries`.
#[napi(js_name = "mergePathEntries")]
pub fn merge_path_entries(
    preferred_path: Option<String>,
    inherited_path: Option<String>,
    platform: String,
) -> napi::Result<String> {
    run(move || {
        let platform = crate::js::from_json(&platform)?;
        let out = port::merge_path_entries(preferred_path, inherited_path, platform)?;
        crate::js::to_json(&out)
    })
}

/// `readEnvironmentFromLoginShell`.
#[napi(js_name = "readEnvironmentFromLoginShell")]
pub fn read_environment_from_login_shell(
    shell: String,
    names: String,
    exec_file: String,
) -> napi::Result<String> {
    run(move || {
        let shell = crate::js::from_json(&shell)?;
        let names = crate::js::from_json(&names)?;
        let exec_file = crate::js::from_json(&exec_file)?;
        let out = port::read_environment_from_login_shell(shell, names, exec_file)?;
        crate::js::to_json(&out)
    })
}

/// `isPathName`.
#[napi(js_name = "isPathName")]
pub fn is_path_name(name: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_path_name(name)?;
        Ok(out)
    })
}

/// `mergeWindowsScopes`.
#[napi(js_name = "mergeWindowsScopes")]
pub fn merge_windows_scopes(machine: String, user: String) -> napi::Result<String> {
    run(move || {
        let machine = crate::js::from_json(&machine)?;
        let user = crate::js::from_json(&user)?;
        let out = port::merge_windows_scopes(machine, user)?;
        crate::js::to_json(&out)
    })
}

/// `readWindowsPersistentEnvironment`.
#[napi(js_name = "readWindowsPersistentEnvironment")]
pub fn read_windows_persistent_environment(exec_file: String) -> napi::Result<String> {
    run(move || {
        let exec_file = crate::js::from_json(&exec_file)?;
        let out = port::read_windows_persistent_environment(exec_file)?;
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

static S_IS_SHELL_ENVIRONMENT_HYDRATED_0: wire::Schema = wire::Schema::Dyn;

/// `isShellEnvironmentHydrated` over the binary wire.
#[napi(js_name = "isShellEnvironmentHydratedWire")]
pub fn is_shell_environment_hydrated_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value = wire::read_value(r, &S_IS_SHELL_ENVIRONMENT_HYDRATED_0)?;
        let out = &port::is_shell_environment_hydrated(env)?;
        w.bool(*out);
        Ok(())
    })
}

static S_APPLY_SHELL_ENVIRONMENT_HYDRATION_MARKER_0: wire::Schema = wire::Schema::Dyn;
static S_APPLY_SHELL_ENVIRONMENT_HYDRATION_MARKER_OUT: wire::Schema = wire::Schema::Dyn;

/// `applyShellEnvironmentHydrationMarker` over the binary wire.
#[napi(js_name = "applyShellEnvironmentHydrationMarkerWire")]
pub fn apply_shell_environment_hydration_marker_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let env: serde_json::Value =
            wire::read_value(r, &S_APPLY_SHELL_ENVIRONMENT_HYDRATION_MARKER_0)?;
        let hydrated: bool = r.bool()?;
        let out = &port::apply_shell_environment_hydration_marker(env, hydrated)?;
        wire::write_value(w, out, &S_APPLY_SHELL_ENVIRONMENT_HYDRATION_MARKER_OUT)?;
        Ok(())
    })
}

static S_LIST_LOGIN_SHELL_CANDIDATES_0: wire::Schema = wire::Schema::Dyn;
static S_LIST_LOGIN_SHELL_CANDIDATES_2: wire::Schema = wire::Schema::Dyn;

/// `listLoginShellCandidates` over the binary wire.
#[napi(js_name = "listLoginShellCandidatesWire")]
pub fn list_login_shell_candidates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: serde_json::Value = wire::read_value(r, &S_LIST_LOGIN_SHELL_CANDIDATES_0)?;
        let shell: Option<String> = r.opt_js(|r| r.string16())?;
        let user_shell: serde_json::Value = wire::read_value(r, &S_LIST_LOGIN_SHELL_CANDIDATES_2)?;
        let out = &port::list_login_shell_candidates(platform, shell, user_shell)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            w.string16(x0);
        }
        Ok(())
    })
}

static S_RESOLVE_LOGIN_SHELL_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_LOGIN_SHELL_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveLoginShell` over the binary wire.
#[napi(js_name = "resolveLoginShellWire")]
pub fn resolve_login_shell_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let platform: serde_json::Value = wire::read_value(r, &S_RESOLVE_LOGIN_SHELL_0)?;
        let shell: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::resolve_login_shell(platform, shell)?;
        wire::write_value(w, out, &S_RESOLVE_LOGIN_SHELL_OUT)?;
        Ok(())
    })
}

static S_MERGE_PATH_ENTRIES_2: wire::Schema = wire::Schema::Dyn;
static S_MERGE_PATH_ENTRIES_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `mergePathEntries` over the binary wire.
#[napi(js_name = "mergePathEntriesWire")]
pub fn merge_path_entries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let preferred_path: Option<String> = r.opt_js(|r| r.string16())?;
        let inherited_path: Option<String> = r.opt_js(|r| r.string16())?;
        let platform: serde_json::Value = wire::read_value(r, &S_MERGE_PATH_ENTRIES_2)?;
        let out = &port::merge_path_entries(preferred_path, inherited_path, platform)?;
        wire::write_value(w, out, &S_MERGE_PATH_ENTRIES_OUT)?;
        Ok(())
    })
}

static S_READ_ENVIRONMENT_FROM_LOGIN_SHELL_0: wire::Schema = wire::Schema::Dyn;
static S_READ_ENVIRONMENT_FROM_LOGIN_SHELL_1: wire::Schema = wire::Schema::Dyn;
static S_READ_ENVIRONMENT_FROM_LOGIN_SHELL_2: wire::Schema = wire::Schema::Dyn;
static S_READ_ENVIRONMENT_FROM_LOGIN_SHELL_OUT: wire::Schema = wire::Schema::Dyn;

/// `readEnvironmentFromLoginShell` over the binary wire.
#[napi(js_name = "readEnvironmentFromLoginShellWire")]
pub fn read_environment_from_login_shell_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let shell: serde_json::Value = wire::read_value(r, &S_READ_ENVIRONMENT_FROM_LOGIN_SHELL_0)?;
        let names: serde_json::Value = wire::read_value(r, &S_READ_ENVIRONMENT_FROM_LOGIN_SHELL_1)?;
        let exec_file: serde_json::Value =
            wire::read_value(r, &S_READ_ENVIRONMENT_FROM_LOGIN_SHELL_2)?;
        let out = &port::read_environment_from_login_shell(shell, names, exec_file)?;
        wire::write_value(w, out, &S_READ_ENVIRONMENT_FROM_LOGIN_SHELL_OUT)?;
        Ok(())
    })
}

/// `isPathName` over the binary wire.
#[napi(js_name = "isPathNameWire")]
pub fn is_path_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let name: String = r.string16()?;
        let out = &port::is_path_name(name)?;
        w.bool(*out);
        Ok(())
    })
}

static S_MERGE_WINDOWS_SCOPES_0: wire::Schema = wire::Schema::Dyn;
static S_MERGE_WINDOWS_SCOPES_1: wire::Schema = wire::Schema::Dyn;
static S_MERGE_WINDOWS_SCOPES_OUT: wire::Schema = wire::Schema::Dyn;

/// `mergeWindowsScopes` over the binary wire.
#[napi(js_name = "mergeWindowsScopesWire")]
pub fn merge_windows_scopes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let machine: serde_json::Value = wire::read_value(r, &S_MERGE_WINDOWS_SCOPES_0)?;
        let user: serde_json::Value = wire::read_value(r, &S_MERGE_WINDOWS_SCOPES_1)?;
        let out = &port::merge_windows_scopes(machine, user)?;
        wire::write_value(w, out, &S_MERGE_WINDOWS_SCOPES_OUT)?;
        Ok(())
    })
}

static S_READ_WINDOWS_PERSISTENT_ENVIRONMENT_0: wire::Schema = wire::Schema::Dyn;
static S_READ_WINDOWS_PERSISTENT_ENVIRONMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `readWindowsPersistentEnvironment` over the binary wire.
#[napi(js_name = "readWindowsPersistentEnvironmentWire")]
pub fn read_windows_persistent_environment_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let exec_file: serde_json::Value =
            wire::read_value(r, &S_READ_WINDOWS_PERSISTENT_ENVIRONMENT_0)?;
        let out = &port::read_windows_persistent_environment(exec_file)?;
        wire::write_value(w, out, &S_READ_WINDOWS_PERSISTENT_ENVIRONMENT_OUT)?;
        Ok(())
    })
}
