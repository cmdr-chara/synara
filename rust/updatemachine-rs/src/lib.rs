//! JS bindings for `apps/desktop/src/updateMachine.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `createInitialDesktopUpdateState`.
#[napi(js_name = "createInitialDesktopUpdateState")]
pub fn create_initial_desktop_update_state(
    current_version: String,
    runtime_info: String,
    flavor: String,
) -> napi::Result<String> {
    run(move || {
        let runtime_info = crate::js::from_json(&runtime_info)?;
        let flavor = crate::js::from_json(&flavor)?;
        let out = port::create_initial_desktop_update_state(current_version, runtime_info, flavor)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnCheckStart`.
#[napi(js_name = "reduceDesktopUpdateStateOnCheckStart")]
pub fn reduce_desktop_update_state_on_check_start(
    state: String,
    checked_at: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_check_start(state, checked_at)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnCheckFailure`.
#[napi(js_name = "reduceDesktopUpdateStateOnCheckFailure")]
pub fn reduce_desktop_update_state_on_check_failure(
    state: String,
    message: String,
    checked_at: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_check_failure(state, message, checked_at)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnUpdateAvailable`.
#[napi(js_name = "reduceDesktopUpdateStateOnUpdateAvailable")]
pub fn reduce_desktop_update_state_on_update_available(
    state: String,
    version: String,
    checked_at: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out =
            port::reduce_desktop_update_state_on_update_available(state, version, checked_at)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnNoUpdate`.
#[napi(js_name = "reduceDesktopUpdateStateOnNoUpdate")]
pub fn reduce_desktop_update_state_on_no_update(
    state: String,
    checked_at: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_no_update(state, checked_at)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnDownloadStart`.
#[napi(js_name = "reduceDesktopUpdateStateOnDownloadStart")]
pub fn reduce_desktop_update_state_on_download_start(state: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_download_start(state)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnDownloadFailure`.
#[napi(js_name = "reduceDesktopUpdateStateOnDownloadFailure")]
pub fn reduce_desktop_update_state_on_download_failure(
    state: String,
    message: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_download_failure(state, message)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnDownloadProgress`.
#[napi(js_name = "reduceDesktopUpdateStateOnDownloadProgress")]
pub fn reduce_desktop_update_state_on_download_progress(
    state: String,
    percent: f64,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_download_progress(state, percent)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnDownloadComplete`.
#[napi(js_name = "reduceDesktopUpdateStateOnDownloadComplete")]
pub fn reduce_desktop_update_state_on_download_complete(
    state: String,
    version: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_download_complete(state, version)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnInstallStart`.
#[napi(js_name = "reduceDesktopUpdateStateOnInstallStart")]
pub fn reduce_desktop_update_state_on_install_start(state: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_install_start(state)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnInstallFailure`.
#[napi(js_name = "reduceDesktopUpdateStateOnInstallFailure")]
pub fn reduce_desktop_update_state_on_install_failure(
    state: String,
    message: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_install_failure(state, message)?;
        crate::js::to_json(&out)
    })
}

/// `reduceDesktopUpdateStateOnInstallRestartFailure`.
#[napi(js_name = "reduceDesktopUpdateStateOnInstallRestartFailure")]
pub fn reduce_desktop_update_state_on_install_restart_failure(
    state: String,
    to_version: String,
    consecutive_failures: f64,
    message: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::reduce_desktop_update_state_on_install_restart_failure(
            state,
            to_version,
            consecutive_failures,
            message,
        )?;
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

static S_CREATE_INITIAL_DESKTOP_UPDATE_STATE_1: wire::Schema = wire::Schema::Dyn;
static S_CREATE_INITIAL_DESKTOP_UPDATE_STATE_2: wire::Schema = wire::Schema::Dyn;
static S_CREATE_INITIAL_DESKTOP_UPDATE_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `createInitialDesktopUpdateState` over the binary wire.
#[napi(js_name = "createInitialDesktopUpdateStateWire")]
pub fn create_initial_desktop_update_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let current_version: String = r.string16()?;
        let runtime_info: serde_json::Value =
            wire::read_value(r, &S_CREATE_INITIAL_DESKTOP_UPDATE_STATE_1)?;
        let flavor: serde_json::Value =
            wire::read_value(r, &S_CREATE_INITIAL_DESKTOP_UPDATE_STATE_2)?;
        let out =
            &port::create_initial_desktop_update_state(current_version, runtime_info, flavor)?;
        wire::write_value(w, out, &S_CREATE_INITIAL_DESKTOP_UPDATE_STATE_OUT)?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_CHECK_START_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_CHECK_START_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnCheckStart` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnCheckStartWire")]
pub fn reduce_desktop_update_state_on_check_start_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_CHECK_START_0)?;
        let checked_at: String = r.string16()?;
        let out = &port::reduce_desktop_update_state_on_check_start(state, checked_at)?;
        wire::write_value(w, out, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_CHECK_START_OUT)?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_CHECK_FAILURE_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_CHECK_FAILURE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnCheckFailure` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnCheckFailureWire")]
pub fn reduce_desktop_update_state_on_check_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_CHECK_FAILURE_0)?;
        let message: String = r.string16()?;
        let checked_at: String = r.string16()?;
        let out = &port::reduce_desktop_update_state_on_check_failure(state, message, checked_at)?;
        wire::write_value(w, out, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_CHECK_FAILURE_OUT)?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_UPDATE_AVAILABLE_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_UPDATE_AVAILABLE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnUpdateAvailable` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnUpdateAvailableWire")]
pub fn reduce_desktop_update_state_on_update_available_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_UPDATE_AVAILABLE_0)?;
        let version: String = r.string16()?;
        let checked_at: String = r.string16()?;
        let out =
            &port::reduce_desktop_update_state_on_update_available(state, version, checked_at)?;
        wire::write_value(
            w,
            out,
            &S_REDUCE_DESKTOP_UPDATE_STATE_ON_UPDATE_AVAILABLE_OUT,
        )?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_NO_UPDATE_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_NO_UPDATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnNoUpdate` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnNoUpdateWire")]
pub fn reduce_desktop_update_state_on_no_update_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_NO_UPDATE_0)?;
        let checked_at: String = r.string16()?;
        let out = &port::reduce_desktop_update_state_on_no_update(state, checked_at)?;
        wire::write_value(w, out, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_NO_UPDATE_OUT)?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_START_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_START_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnDownloadStart` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnDownloadStartWire")]
pub fn reduce_desktop_update_state_on_download_start_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_START_0)?;
        let out = &port::reduce_desktop_update_state_on_download_start(state)?;
        wire::write_value(w, out, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_START_OUT)?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_FAILURE_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_FAILURE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnDownloadFailure` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnDownloadFailureWire")]
pub fn reduce_desktop_update_state_on_download_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_FAILURE_0)?;
        let message: String = r.string16()?;
        let out = &port::reduce_desktop_update_state_on_download_failure(state, message)?;
        wire::write_value(
            w,
            out,
            &S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_FAILURE_OUT,
        )?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_PROGRESS_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_PROGRESS_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnDownloadProgress` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnDownloadProgressWire")]
pub fn reduce_desktop_update_state_on_download_progress_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_PROGRESS_0)?;
        let percent: f64 = r.f64()?;
        let out = &port::reduce_desktop_update_state_on_download_progress(state, percent)?;
        wire::write_value(
            w,
            out,
            &S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_PROGRESS_OUT,
        )?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_COMPLETE_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_COMPLETE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnDownloadComplete` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnDownloadCompleteWire")]
pub fn reduce_desktop_update_state_on_download_complete_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_COMPLETE_0)?;
        let version: String = r.string16()?;
        let out = &port::reduce_desktop_update_state_on_download_complete(state, version)?;
        wire::write_value(
            w,
            out,
            &S_REDUCE_DESKTOP_UPDATE_STATE_ON_DOWNLOAD_COMPLETE_OUT,
        )?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_START_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_START_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnInstallStart` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnInstallStartWire")]
pub fn reduce_desktop_update_state_on_install_start_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_START_0)?;
        let out = &port::reduce_desktop_update_state_on_install_start(state)?;
        wire::write_value(w, out, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_START_OUT)?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_FAILURE_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_FAILURE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("enabled", wire::Schema::Bool),
    ("status", wire::Schema::Str16),
    ("currentVersion", wire::Schema::Str16),
    ("hostArch", wire::Schema::Str16),
    ("appArch", wire::Schema::Str16),
    ("runningUnderArm64Translation", wire::Schema::Bool),
    (
        "availableVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "downloadedVersion",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
    ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("canRetry", wire::Schema::Bool),
    ("installFailureCount", wire::Schema::Num),
    ("flavor", wire::Schema::Str16),
    ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
]);

/// `reduceDesktopUpdateStateOnInstallFailure` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnInstallFailureWire")]
pub fn reduce_desktop_update_state_on_install_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_FAILURE_0)?;
        let message: String = r.string16()?;
        let out = &port::reduce_desktop_update_state_on_install_failure(state, message)?;
        wire::write_value(
            w,
            out,
            &S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_FAILURE_OUT,
        )?;
        Ok(())
    })
}

static S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_RESTART_FAILURE_0: wire::Schema = wire::Schema::Dyn;
static S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_RESTART_FAILURE_OUT: wire::Schema =
    wire::Schema::Obj(&[
        ("enabled", wire::Schema::Bool),
        ("status", wire::Schema::Str16),
        ("currentVersion", wire::Schema::Str16),
        ("hostArch", wire::Schema::Str16),
        ("appArch", wire::Schema::Str16),
        ("runningUnderArm64Translation", wire::Schema::Bool),
        (
            "availableVersion",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        (
            "downloadedVersion",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("downloadPercent", wire::Schema::OptJs(&wire::Schema::Num)),
        ("checkedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("message", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("errorContext", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("canRetry", wire::Schema::Bool),
        ("installFailureCount", wire::Schema::Num),
        ("flavor", wire::Schema::Str16),
        ("releaseUrl", wire::Schema::OptJs(&wire::Schema::Str16)),
    ]);

/// `reduceDesktopUpdateStateOnInstallRestartFailure` over the binary wire.
#[napi(js_name = "reduceDesktopUpdateStateOnInstallRestartFailureWire")]
pub fn reduce_desktop_update_state_on_install_restart_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(
            r,
            &S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_RESTART_FAILURE_0,
        )?;
        let to_version: String = r.string16()?;
        let consecutive_failures: f64 = r.f64()?;
        let message: String = r.string16()?;
        let out = &port::reduce_desktop_update_state_on_install_restart_failure(
            state,
            to_version,
            consecutive_failures,
            message,
        )?;
        wire::write_value(
            w,
            out,
            &S_REDUCE_DESKTOP_UPDATE_STATE_ON_INSTALL_RESTART_FAILURE_OUT,
        )?;
        Ok(())
    })
}
