//! JS bindings for `apps/web/src/wsTransport.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `projectFileChangeStreamKey`.
#[napi(js_name = "projectFileChangeStreamKey")]
pub fn project_file_change_stream_key(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::project_file_change_stream_key(input)?;
        Ok(out)
    })
}

/// `getReconnectRetryDelayMs`.
#[napi(js_name = "getReconnectRetryDelayMs")]
pub fn get_reconnect_retry_delay_ms(attempt: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::get_reconnect_retry_delay_ms(attempt)?;
        Ok(out)
    })
}

/// `rawSocketUrl`.
#[napi(js_name = "rawSocketUrl")]
pub fn raw_socket_url(explicit_url: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::raw_socket_url(explicit_url)?;
        Ok(out)
    })
}

/// `shouldDeclineCompressionForLocalBackend`.
#[napi(js_name = "shouldDeclineCompressionForLocalBackend")]
pub fn should_decline_compression_for_local_backend(input: String) -> napi::Result<bool> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::should_decline_compression_for_local_backend(input)?;
        Ok(out)
    })
}

/// `makeSocketUrl`.
#[napi(js_name = "makeSocketUrl")]
pub fn make_socket_url(explicit_url: Option<String>, path: String) -> napi::Result<String> {
    run(move || {
        let out = port::make_socket_url(explicit_url, path)?;
        Ok(out)
    })
}

/// `makeFeatureSocketUrl`.
#[napi(js_name = "makeFeatureSocketUrl")]
pub fn make_feature_socket_url(
    explicit_url: Option<String>,
    compatibility: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let compatibility = crate::js::from_json(&compatibility)?;
        let options = crate::js::from_json(&options)?;
        let out = port::make_feature_socket_url(explicit_url, compatibility, options)?;
        Ok(out)
    })
}

/// `makeNegotiateHttpUrl`.
#[napi(js_name = "makeNegotiateHttpUrl")]
pub fn make_negotiate_http_url(explicit_url: Option<String>) -> napi::Result<String> {
    run(move || {
        let out = port::make_negotiate_http_url(explicit_url)?;
        Ok(out)
    })
}

/// `negotiateOverHttp`.
#[napi(js_name = "negotiateOverHttp")]
pub async fn negotiate_over_http(
    explicit_url: Option<String>,
    lifetime_signal: String,
) -> napi::Result<String> {
    run_async(async move {
        let lifetime_signal = crate::js::from_json(&lifetime_signal)?;
        let out = port::negotiate_over_http(explicit_url, lifetime_signal).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `getSnapshotFaultRetryDelayMs`.
#[napi(js_name = "getSnapshotFaultRetryDelayMs")]
pub fn get_snapshot_fault_retry_delay_ms(
    cause: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let cause = crate::js::from_json(&cause)?;
        let out = port::get_snapshot_fault_retry_delay_ms(cause)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `isTerminalCompatibilityFailure`.
#[napi(js_name = "isTerminalCompatibilityFailure")]
pub fn is_terminal_compatibility_failure(error: String) -> napi::Result<bool> {
    run(move || {
        let error = crate::js::from_json(&error)?;
        let out = port::is_terminal_compatibility_failure(error)?;
        Ok(out)
    })
}

/// `serverIdentityChanged`.
#[napi(js_name = "serverIdentityChanged")]
pub fn server_identity_changed(
    last_known_server_instance_id: String,
    negotiated_server_instance_id: String,
) -> napi::Result<bool> {
    run(move || {
        let last_known_server_instance_id = crate::js::from_json(&last_known_server_instance_id)?;
        let out = port::server_identity_changed(
            last_known_server_instance_id,
            negotiated_server_instance_id,
        )?;
        Ok(out)
    })
}

/// `getTerminalCompatibilityError`.
#[napi(js_name = "getTerminalCompatibilityError")]
pub fn get_terminal_compatibility_error(error: String) -> napi::Result<String> {
    run(move || {
        let error = crate::js::from_json(&error)?;
        let out = port::get_terminal_compatibility_error(error)?;
        crate::js::to_json(&out)
    })
}

/// `shouldReconnectAfterStreamFailure`.
#[napi(js_name = "shouldReconnectAfterStreamFailure")]
pub fn should_reconnect_after_stream_failure(cause: String) -> napi::Result<bool> {
    run(move || {
        let cause = crate::js::from_json(&cause)?;
        let out = port::should_reconnect_after_stream_failure(cause)?;
        Ok(out)
    })
}

/// `getProjectFileWatchRetryDelayMs`.
#[napi(js_name = "getProjectFileWatchRetryDelayMs")]
pub fn get_project_file_watch_retry_delay_ms(
    cause: String,
    previous_attempts: f64,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let cause = crate::js::from_json(&cause)?;
        let out = port::get_project_file_watch_retry_delay_ms(cause, previous_attempts)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `getUnexpectedStreamCompletionRetryDelayMs`.
#[napi(js_name = "getUnexpectedStreamCompletionRetryDelayMs")]
pub fn get_unexpected_stream_completion_retry_delay_ms(attempt: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::get_unexpected_stream_completion_retry_delay_ms(attempt)?;
        Ok(out)
    })
}

/// `getStreamCapacityRetryDelayMs`.
#[napi(js_name = "getStreamCapacityRetryDelayMs")]
pub fn get_stream_capacity_retry_delay_ms(
    cause: String,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let cause = crate::js::from_json(&cause)?;
        let out = port::get_stream_capacity_retry_delay_ms(cause)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `getStreamDuplicateRetryDelayMs`.
#[napi(js_name = "getStreamDuplicateRetryDelayMs")]
pub fn get_stream_duplicate_retry_delay_ms(
    cause: String,
    previous_attempts: f64,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let cause = crate::js::from_json(&cause)?;
        let out = port::get_stream_duplicate_retry_delay_ms(cause, previous_attempts)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `getThreadSnapshotBootstrapRetryDelayMs`.
#[napi(js_name = "getThreadSnapshotBootstrapRetryDelayMs")]
pub fn get_thread_snapshot_bootstrap_retry_delay_ms(
    cause: String,
    previous_attempts: f64,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let cause = crate::js::from_json(&cause)?;
        let out = port::get_thread_snapshot_bootstrap_retry_delay_ms(cause, previous_attempts)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `getResnapshotRetryDelayMs`.
#[napi(js_name = "getResnapshotRetryDelayMs")]
pub fn get_resnapshot_retry_delay_ms(
    cause: String,
    previous_attempts: f64,
) -> napi::Result<napi::bindgen_prelude::Either<f64, ()>> {
    run(move || {
        let cause = crate::js::from_json(&cause)?;
        let out = port::get_resnapshot_retry_delay_ms(cause, previous_attempts)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `resolveStreamAdmissionRetry`.
#[napi(js_name = "resolveStreamAdmissionRetry")]
pub fn resolve_stream_admission_retry(
    cause: String,
    capacity_attempts: f64,
    duplicate_attempts: f64,
    thread_bootstrap_attempts: Option<f64>,
    resnapshot_attempts: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let cause = crate::js::from_json(&cause)?;
        let out = port::resolve_stream_admission_retry(
            cause,
            capacity_attempts,
            duplicate_attempts,
            thread_bootstrap_attempts,
            resnapshot_attempts,
        )?;
        crate::js::to_json(&out)
    })
}

/// `threadStreamInputsEqual`.
#[napi(js_name = "threadStreamInputsEqual")]
pub fn thread_stream_inputs_equal(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::thread_stream_inputs_equal(left, right)?;
        Ok(out)
    })
}

/// `isServerLifecyclePushChannel`.
#[napi(js_name = "isServerLifecyclePushChannel")]
pub fn is_server_lifecycle_push_channel(channel: String) -> napi::Result<bool> {
    run(move || {
        let out = port::is_server_lifecycle_push_channel(channel)?;
        Ok(out)
    })
}

/// `shouldKeepServerLifecycleStream`.
#[napi(js_name = "shouldKeepServerLifecycleStream")]
pub fn should_keep_server_lifecycle_stream(active_channels: String) -> napi::Result<bool> {
    run(move || {
        let active_channels = crate::js::from_json(&active_channels)?;
        let out = port::should_keep_server_lifecycle_stream(active_channels)?;
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

static S_PROJECT_FILE_CHANGE_STREAM_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `projectFileChangeStreamKey` over the binary wire.
#[napi(js_name = "projectFileChangeStreamKeyWire")]
pub fn project_file_change_stream_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PROJECT_FILE_CHANGE_STREAM_KEY_0)?;
        let out = &port::project_file_change_stream_key(input)?;
        w.string16(out);
        Ok(())
    })
}

/// `getReconnectRetryDelayMs` over the binary wire.
#[napi(js_name = "getReconnectRetryDelayMsWire")]
pub fn get_reconnect_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let attempt: f64 = r.f64()?;
        let out = &port::get_reconnect_retry_delay_ms(attempt)?;
        w.f64(*out);
        Ok(())
    })
}

/// `rawSocketUrl` over the binary wire.
#[napi(js_name = "rawSocketUrlWire")]
pub fn raw_socket_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let explicit_url: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::raw_socket_url(explicit_url)?;
        w.string16(out);
        Ok(())
    })
}

static S_SHOULD_DECLINE_COMPRESSION_FOR_LOCAL_BACKEND_0: wire::Schema = wire::Schema::Dyn;

/// `shouldDeclineCompressionForLocalBackend` over the binary wire.
#[napi(js_name = "shouldDeclineCompressionForLocalBackendWire")]
pub fn should_decline_compression_for_local_backend_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_SHOULD_DECLINE_COMPRESSION_FOR_LOCAL_BACKEND_0)?;
        let out = &port::should_decline_compression_for_local_backend(input)?;
        w.bool(*out);
        Ok(())
    })
}

/// `makeSocketUrl` over the binary wire.
#[napi(js_name = "makeSocketUrlWire")]
pub fn make_socket_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let explicit_url: Option<String> = r.opt_js(|r| r.string16())?;
        let path: String = r.string16()?;
        let out = &port::make_socket_url(explicit_url, path)?;
        w.string16(out);
        Ok(())
    })
}

static S_MAKE_FEATURE_SOCKET_URL_1: wire::Schema = wire::Schema::Dyn;

/// `makeFeatureSocketUrl` over the binary wire.
#[napi(js_name = "makeFeatureSocketUrlWire")]
pub fn make_feature_socket_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let explicit_url: Option<String> = r.opt_js(|r| r.string16())?;
        let compatibility: serde_json::Value = wire::read_value(r, &S_MAKE_FEATURE_SOCKET_URL_1)?;
        let options: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::make_feature_socket_url(explicit_url, compatibility, options)?;
        w.string16(out);
        Ok(())
    })
}

/// `makeNegotiateHttpUrl` over the binary wire.
#[napi(js_name = "makeNegotiateHttpUrlWire")]
pub fn make_negotiate_http_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let explicit_url: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::make_negotiate_http_url(explicit_url)?;
        w.string16(out);
        Ok(())
    })
}

static S_GET_SNAPSHOT_FAULT_RETRY_DELAY_MS_0: wire::Schema = wire::Schema::Dyn;

/// `getSnapshotFaultRetryDelayMs` over the binary wire.
#[napi(js_name = "getSnapshotFaultRetryDelayMsWire")]
pub fn get_snapshot_fault_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cause: serde_json::Value = wire::read_value(r, &S_GET_SNAPSHOT_FAULT_RETRY_DELAY_MS_0)?;
        let out = &port::get_snapshot_fault_retry_delay_ms(cause)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_IS_TERMINAL_COMPATIBILITY_FAILURE_0: wire::Schema = wire::Schema::Dyn;

/// `isTerminalCompatibilityFailure` over the binary wire.
#[napi(js_name = "isTerminalCompatibilityFailureWire")]
pub fn is_terminal_compatibility_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let error: serde_json::Value = wire::read_value(r, &S_IS_TERMINAL_COMPATIBILITY_FAILURE_0)?;
        let out = &port::is_terminal_compatibility_failure(error)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SERVER_IDENTITY_CHANGED_0: wire::Schema = wire::Schema::Dyn;

/// `serverIdentityChanged` over the binary wire.
#[napi(js_name = "serverIdentityChangedWire")]
pub fn server_identity_changed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let last_known_server_instance_id: serde_json::Value =
            wire::read_value(r, &S_SERVER_IDENTITY_CHANGED_0)?;
        let negotiated_server_instance_id: String = r.string16()?;
        let out = &port::server_identity_changed(
            last_known_server_instance_id,
            negotiated_server_instance_id,
        )?;
        w.bool(*out);
        Ok(())
    })
}

static S_GET_TERMINAL_COMPATIBILITY_ERROR_0: wire::Schema = wire::Schema::Dyn;

/// `getTerminalCompatibilityError` over the binary wire.
#[napi(js_name = "getTerminalCompatibilityErrorWire")]
pub fn get_terminal_compatibility_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let error: serde_json::Value = wire::read_value(r, &S_GET_TERMINAL_COMPATIBILITY_ERROR_0)?;
        let out = &port::get_terminal_compatibility_error(error)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SHOULD_RECONNECT_AFTER_STREAM_FAILURE_0: wire::Schema = wire::Schema::Dyn;

/// `shouldReconnectAfterStreamFailure` over the binary wire.
#[napi(js_name = "shouldReconnectAfterStreamFailureWire")]
pub fn should_reconnect_after_stream_failure_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cause: serde_json::Value =
            wire::read_value(r, &S_SHOULD_RECONNECT_AFTER_STREAM_FAILURE_0)?;
        let out = &port::should_reconnect_after_stream_failure(cause)?;
        w.bool(*out);
        Ok(())
    })
}

static S_GET_PROJECT_FILE_WATCH_RETRY_DELAY_MS_0: wire::Schema = wire::Schema::Dyn;

/// `getProjectFileWatchRetryDelayMs` over the binary wire.
#[napi(js_name = "getProjectFileWatchRetryDelayMsWire")]
pub fn get_project_file_watch_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cause: serde_json::Value =
            wire::read_value(r, &S_GET_PROJECT_FILE_WATCH_RETRY_DELAY_MS_0)?;
        let previous_attempts: f64 = r.f64()?;
        let out = &port::get_project_file_watch_retry_delay_ms(cause, previous_attempts)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

/// `getUnexpectedStreamCompletionRetryDelayMs` over the binary wire.
#[napi(js_name = "getUnexpectedStreamCompletionRetryDelayMsWire")]
pub fn get_unexpected_stream_completion_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let attempt: f64 = r.f64()?;
        let out = &port::get_unexpected_stream_completion_retry_delay_ms(attempt)?;
        w.f64(*out);
        Ok(())
    })
}

static S_GET_STREAM_CAPACITY_RETRY_DELAY_MS_0: wire::Schema = wire::Schema::Dyn;

/// `getStreamCapacityRetryDelayMs` over the binary wire.
#[napi(js_name = "getStreamCapacityRetryDelayMsWire")]
pub fn get_stream_capacity_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cause: serde_json::Value =
            wire::read_value(r, &S_GET_STREAM_CAPACITY_RETRY_DELAY_MS_0)?;
        let out = &port::get_stream_capacity_retry_delay_ms(cause)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_GET_STREAM_DUPLICATE_RETRY_DELAY_MS_0: wire::Schema = wire::Schema::Dyn;

/// `getStreamDuplicateRetryDelayMs` over the binary wire.
#[napi(js_name = "getStreamDuplicateRetryDelayMsWire")]
pub fn get_stream_duplicate_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cause: serde_json::Value =
            wire::read_value(r, &S_GET_STREAM_DUPLICATE_RETRY_DELAY_MS_0)?;
        let previous_attempts: f64 = r.f64()?;
        let out = &port::get_stream_duplicate_retry_delay_ms(cause, previous_attempts)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_GET_THREAD_SNAPSHOT_BOOTSTRAP_RETRY_DELAY_MS_0: wire::Schema = wire::Schema::Dyn;

/// `getThreadSnapshotBootstrapRetryDelayMs` over the binary wire.
#[napi(js_name = "getThreadSnapshotBootstrapRetryDelayMsWire")]
pub fn get_thread_snapshot_bootstrap_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cause: serde_json::Value =
            wire::read_value(r, &S_GET_THREAD_SNAPSHOT_BOOTSTRAP_RETRY_DELAY_MS_0)?;
        let previous_attempts: f64 = r.f64()?;
        let out = &port::get_thread_snapshot_bootstrap_retry_delay_ms(cause, previous_attempts)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_GET_RESNAPSHOT_RETRY_DELAY_MS_0: wire::Schema = wire::Schema::Dyn;

/// `getResnapshotRetryDelayMs` over the binary wire.
#[napi(js_name = "getResnapshotRetryDelayMsWire")]
pub fn get_resnapshot_retry_delay_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cause: serde_json::Value = wire::read_value(r, &S_GET_RESNAPSHOT_RETRY_DELAY_MS_0)?;
        let previous_attempts: f64 = r.f64()?;
        let out = &port::get_resnapshot_retry_delay_ms(cause, previous_attempts)?;
        match out {
            Some(x0) => {
                w.tag(1);
                w.f64(*x0);
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_RESOLVE_STREAM_ADMISSION_RETRY_0: wire::Schema = wire::Schema::Dyn;

/// `resolveStreamAdmissionRetry` over the binary wire.
#[napi(js_name = "resolveStreamAdmissionRetryWire")]
pub fn resolve_stream_admission_retry_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let cause: serde_json::Value = wire::read_value(r, &S_RESOLVE_STREAM_ADMISSION_RETRY_0)?;
        let capacity_attempts: f64 = r.f64()?;
        let duplicate_attempts: f64 = r.f64()?;
        let thread_bootstrap_attempts: Option<f64> = r.opt_js(|r| r.f64())?;
        let resnapshot_attempts: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::resolve_stream_admission_retry(
            cause,
            capacity_attempts,
            duplicate_attempts,
            thread_bootstrap_attempts,
            resnapshot_attempts,
        )?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_THREAD_STREAM_INPUTS_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_THREAD_STREAM_INPUTS_EQUAL_1: wire::Schema = wire::Schema::Dyn;

/// `threadStreamInputsEqual` over the binary wire.
#[napi(js_name = "threadStreamInputsEqualWire")]
pub fn thread_stream_inputs_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: serde_json::Value = wire::read_value(r, &S_THREAD_STREAM_INPUTS_EQUAL_0)?;
        let right: serde_json::Value = wire::read_value(r, &S_THREAD_STREAM_INPUTS_EQUAL_1)?;
        let out = &port::thread_stream_inputs_equal(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

/// `isServerLifecyclePushChannel` over the binary wire.
#[napi(js_name = "isServerLifecyclePushChannelWire")]
pub fn is_server_lifecycle_push_channel_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let channel: String = r.string16()?;
        let out = &port::is_server_lifecycle_push_channel(channel)?;
        w.bool(*out);
        Ok(())
    })
}

static S_SHOULD_KEEP_SERVER_LIFECYCLE_STREAM_0: wire::Schema = wire::Schema::Dyn;

/// `shouldKeepServerLifecycleStream` over the binary wire.
#[napi(js_name = "shouldKeepServerLifecycleStreamWire")]
pub fn should_keep_server_lifecycle_stream_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let active_channels: serde_json::Value =
            wire::read_value(r, &S_SHOULD_KEEP_SERVER_LIFECYCLE_STREAM_0)?;
        let out = &port::should_keep_server_lifecycle_stream(active_channels)?;
        w.bool(*out);
        Ok(())
    })
}
