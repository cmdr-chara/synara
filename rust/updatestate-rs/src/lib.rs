//! JS bindings for `apps/desktop/src/updateState.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `canCheckForDesktopUpdates`.
#[napi(js_name = "canCheckForDesktopUpdates")]
pub fn can_check_for_desktop_updates(state: String) -> napi::Result<bool> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let out = port::can_check_for_desktop_updates(state)?;
        Ok(out)
    })
}

/// `getDownloadStallTimeoutMessage`.
#[napi(js_name = "getDownloadStallTimeoutMessage")]
pub fn get_download_stall_timeout_message(timeout_ms: f64) -> napi::Result<String> {
    run(move || {
        let out = port::get_download_stall_timeout_message(timeout_ms)?;
        Ok(out)
    })
}

/// `isExpectedStalledDownloadCancellationError`.
#[napi(js_name = "isExpectedStalledDownloadCancellationError")]
pub fn is_expected_stalled_download_cancellation_error(args: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::is_expected_stalled_download_cancellation_error(args)?;
        crate::js::to_json(&out)
    })
}

/// `hasDownloadProgressAdvanced`.
#[napi(js_name = "hasDownloadProgressAdvanced")]
pub fn has_download_progress_advanced(previous: String, next: String) -> napi::Result<bool> {
    run(move || {
        let previous = crate::js::from_json(&previous)?;
        let next = crate::js::from_json(&next)?;
        let out = port::has_download_progress_advanced(previous, next)?;
        Ok(out)
    })
}

/// `shouldBroadcastDownloadProgress`.
#[napi(js_name = "shouldBroadcastDownloadProgress")]
pub fn should_broadcast_download_progress(
    current_state: String,
    next_percent: f64,
) -> napi::Result<bool> {
    run(move || {
        let current_state = crate::js::from_json(&current_state)?;
        let out = port::should_broadcast_download_progress(current_state, next_percent)?;
        Ok(out)
    })
}

/// `isUpdateVersionNewer`.
#[napi(js_name = "isUpdateVersionNewer")]
pub fn is_update_version_newer(
    current_version: String,
    candidate_version: String,
) -> napi::Result<bool> {
    run(move || {
        let out = port::is_update_version_newer(current_version, candidate_version)?;
        Ok(out)
    })
}

/// `isUpdateVersionAllowedForFlavor`.
#[napi(js_name = "isUpdateVersionAllowedForFlavor")]
pub fn is_update_version_allowed_for_flavor(
    candidate_version: String,
    flavor: String,
) -> napi::Result<bool> {
    run(move || {
        let out = port::is_update_version_allowed_for_flavor(candidate_version, flavor)?;
        Ok(out)
    })
}

/// `nextStatusAfterDownloadFailure`.
#[napi(js_name = "nextStatusAfterDownloadFailure")]
pub fn next_status_after_download_failure(current_state: String) -> napi::Result<String> {
    run(move || {
        let current_state = crate::js::from_json(&current_state)?;
        let out = port::next_status_after_download_failure(current_state)?;
        crate::js::to_json(&out)
    })
}

/// `getCanRetryAfterDownloadFailure`.
#[napi(js_name = "getCanRetryAfterDownloadFailure")]
pub fn get_can_retry_after_download_failure(current_state: String) -> napi::Result<bool> {
    run(move || {
        let current_state = crate::js::from_json(&current_state)?;
        let out = port::get_can_retry_after_download_failure(current_state)?;
        Ok(out)
    })
}

/// `shouldCheckForUpdatesOnForeground`.
#[napi(js_name = "shouldCheckForUpdatesOnForeground")]
pub fn should_check_for_updates_on_foreground(args: String) -> napi::Result<bool> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::should_check_for_updates_on_foreground(args)?;
        Ok(out)
    })
}

/// `getAutoUpdateDisabledReason`.
#[napi(js_name = "getAutoUpdateDisabledReason")]
pub fn get_auto_update_disabled_reason(
    args: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::get_auto_update_disabled_reason(args)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `updateFilesMatchRuntimeArch`.
#[napi(js_name = "updateFilesMatchRuntimeArch")]
pub fn update_files_match_runtime_arch(
    info: String,
    platform: String,
    arch: String,
) -> napi::Result<bool> {
    run(move || {
        let info = crate::js::from_json(&info)?;
        let platform = crate::js::from_json(&platform)?;
        let out = port::update_files_match_runtime_arch(info, platform, arch)?;
        Ok(out)
    })
}

/// `shouldAcceptAvailableUpdate`.
#[napi(js_name = "shouldAcceptAvailableUpdate")]
pub fn should_accept_available_update(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::should_accept_available_update(input)?;
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

static S_CAN_CHECK_FOR_DESKTOP_UPDATES_0: wire::Schema = wire::Schema::Dyn;

/// `canCheckForDesktopUpdates` over the binary wire.
#[napi(js_name = "canCheckForDesktopUpdatesWire")]
pub fn can_check_for_desktop_updates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_CAN_CHECK_FOR_DESKTOP_UPDATES_0)?;
        let out = &port::can_check_for_desktop_updates(state)?;
        w.bool(*out);
        Ok(())
    })
}

/// `getDownloadStallTimeoutMessage` over the binary wire.
#[napi(js_name = "getDownloadStallTimeoutMessageWire")]
pub fn get_download_stall_timeout_message_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let timeout_ms: f64 = r.f64()?;
        let out = &port::get_download_stall_timeout_message(timeout_ms)?;
        w.string16(out);
        Ok(())
    })
}

static S_IS_EXPECTED_STALLED_DOWNLOAD_CANCELLATION_ERROR_0: wire::Schema = wire::Schema::Dyn;
static S_IS_EXPECTED_STALLED_DOWNLOAD_CANCELLATION_ERROR_OUT: wire::Schema = wire::Schema::Bool;

/// `isExpectedStalledDownloadCancellationError` over the binary wire.
#[napi(js_name = "isExpectedStalledDownloadCancellationErrorWire")]
pub fn is_expected_stalled_download_cancellation_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value =
            wire::read_value(r, &S_IS_EXPECTED_STALLED_DOWNLOAD_CANCELLATION_ERROR_0)?;
        let out = &port::is_expected_stalled_download_cancellation_error(args)?;
        wire::write_value(
            w,
            out,
            &S_IS_EXPECTED_STALLED_DOWNLOAD_CANCELLATION_ERROR_OUT,
        )?;
        Ok(())
    })
}

static S_HAS_DOWNLOAD_PROGRESS_ADVANCED_1: wire::Schema = wire::Schema::Dyn;

/// `hasDownloadProgressAdvanced` over the binary wire.
#[napi(js_name = "hasDownloadProgressAdvancedWire")]
pub fn has_download_progress_advanced_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let previous: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let next: serde_json::Value = wire::read_value(r, &S_HAS_DOWNLOAD_PROGRESS_ADVANCED_1)?;
        let out = &port::has_download_progress_advanced(previous, next)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_BROADCAST_DOWNLOAD_PROGRESS_0: wire::Schema = wire::Schema::Dyn;

/// `shouldBroadcastDownloadProgress` over the binary wire.
#[napi(js_name = "shouldBroadcastDownloadProgressWire")]
pub fn should_broadcast_download_progress_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let current_state: serde_json::Value =
            wire::read_value(r, &S_SHOULD_BROADCAST_DOWNLOAD_PROGRESS_0)?;
        let next_percent: f64 = r.f64()?;
        let out = &port::should_broadcast_download_progress(current_state, next_percent)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isUpdateVersionNewer` over the binary wire.
#[napi(js_name = "isUpdateVersionNewerWire")]
pub fn is_update_version_newer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let current_version: String = r.string16()?;
        let candidate_version: String = r.string16()?;
        let out = &port::is_update_version_newer(current_version, candidate_version)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isUpdateVersionAllowedForFlavor` over the binary wire.
#[napi(js_name = "isUpdateVersionAllowedForFlavorWire")]
pub fn is_update_version_allowed_for_flavor_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let candidate_version: String = r.string16()?;
        let flavor: String = r.string16()?;
        let out = &port::is_update_version_allowed_for_flavor(candidate_version, flavor)?;
        w.bool(*out);
        Ok(())
    })
}

static S_NEXT_STATUS_AFTER_DOWNLOAD_FAILURE_0: wire::Schema = wire::Schema::Dyn;
static S_NEXT_STATUS_AFTER_DOWNLOAD_FAILURE_OUT: wire::Schema = wire::Schema::Dyn;

/// `nextStatusAfterDownloadFailure` over the binary wire.
#[napi(js_name = "nextStatusAfterDownloadFailureWire")]
pub fn next_status_after_download_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let current_state: serde_json::Value =
            wire::read_value(r, &S_NEXT_STATUS_AFTER_DOWNLOAD_FAILURE_0)?;
        let out = &port::next_status_after_download_failure(current_state)?;
        wire::write_value(w, out, &S_NEXT_STATUS_AFTER_DOWNLOAD_FAILURE_OUT)?;
        Ok(())
    })
}

static S_GET_CAN_RETRY_AFTER_DOWNLOAD_FAILURE_0: wire::Schema = wire::Schema::Dyn;

/// `getCanRetryAfterDownloadFailure` over the binary wire.
#[napi(js_name = "getCanRetryAfterDownloadFailureWire")]
pub fn get_can_retry_after_download_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let current_state: serde_json::Value =
            wire::read_value(r, &S_GET_CAN_RETRY_AFTER_DOWNLOAD_FAILURE_0)?;
        let out = &port::get_can_retry_after_download_failure(current_state)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_CHECK_FOR_UPDATES_ON_FOREGROUND_0: wire::Schema = wire::Schema::Dyn;

/// `shouldCheckForUpdatesOnForeground` over the binary wire.
#[napi(js_name = "shouldCheckForUpdatesOnForegroundWire")]
pub fn should_check_for_updates_on_foreground_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value =
            wire::read_value(r, &S_SHOULD_CHECK_FOR_UPDATES_ON_FOREGROUND_0)?;
        let out = &port::should_check_for_updates_on_foreground(args)?;
        w.bool(*out);
        Ok(())
    })
}

static S_GET_AUTO_UPDATE_DISABLED_REASON_0: wire::Schema = wire::Schema::Dyn;

/// `getAutoUpdateDisabledReason` over the binary wire.
#[napi(js_name = "getAutoUpdateDisabledReasonWire")]
pub fn get_auto_update_disabled_reason_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_GET_AUTO_UPDATE_DISABLED_REASON_0)?;
        let out = &port::get_auto_update_disabled_reason(args)?;
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

static S_UPDATE_FILES_MATCH_RUNTIME_ARCH_0: wire::Schema = wire::Schema::Dyn;
static S_UPDATE_FILES_MATCH_RUNTIME_ARCH_1: wire::Schema = wire::Schema::Dyn;

/// `updateFilesMatchRuntimeArch` over the binary wire.
#[napi(js_name = "updateFilesMatchRuntimeArchWire")]
pub fn update_files_match_runtime_arch_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let info: serde_json::Value = wire::read_value(r, &S_UPDATE_FILES_MATCH_RUNTIME_ARCH_0)?;
        let platform: serde_json::Value =
            wire::read_value(r, &S_UPDATE_FILES_MATCH_RUNTIME_ARCH_1)?;
        let arch: String = r.string16()?;
        let out = &port::update_files_match_runtime_arch(info, platform, arch)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_ACCEPT_AVAILABLE_UPDATE_0: wire::Schema = wire::Schema::Dyn;

/// `shouldAcceptAvailableUpdate` over the binary wire.
#[napi(js_name = "shouldAcceptAvailableUpdateWire")]
pub fn should_accept_available_update_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_SHOULD_ACCEPT_AVAILABLE_UPDATE_0)?;
        let out = &port::should_accept_available_update(input)?;
        w.bool(*out);
        Ok(())
    })
}
