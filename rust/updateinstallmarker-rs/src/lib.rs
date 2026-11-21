//! JS bindings for `apps/desktop/src/updateInstallMarker.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `installMarkerMatchesHandoffExpectation`.
#[napi(js_name = "installMarkerMatchesHandoffExpectation")]
pub fn install_marker_matches_handoff_expectation(
    marker: String,
    expected: String,
) -> napi::Result<bool> {
    run(move || {
        let marker = crate::js::from_json(&marker)?;
        let expected = crate::js::from_json(&expected)?;
        let out = port::install_marker_matches_handoff_expectation(marker, expected)?;
        Ok(out)
    })
}

/// `createUpdateInstallMarker`.
#[napi(js_name = "createUpdateInstallMarker")]
pub fn create_update_install_marker(args: String) -> napi::Result<String> {
    run(move || {
        let args = crate::js::from_json(&args)?;
        let out = port::create_update_install_marker(args)?;
        crate::js::to_json(&out)
    })
}

/// `readInstallMarker`.
#[napi(js_name = "readInstallMarker")]
pub fn read_install_marker(file_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::read_install_marker(file_path)?;
        crate::js::to_json(&out)
    })
}

/// `writeInstallMarker`.
#[napi(js_name = "writeInstallMarker")]
pub fn write_install_marker(file_path: String, marker: String) -> napi::Result<()> {
    run(move || {
        let marker = crate::js::from_json(&marker)?;
        port::write_install_marker(file_path, marker)?;
        Ok(())
    })
}

/// `markInstallHandoffSync`.
#[napi(js_name = "markInstallHandoffSync")]
pub fn mark_install_handoff_sync(
    file_path: String,
    expected: String,
    now_iso: String,
) -> napi::Result<String> {
    run(move || {
        let expected = crate::js::from_json(&expected)?;
        let now_iso = crate::js::from_json(&now_iso)?;
        let out = port::mark_install_handoff_sync(file_path, expected, now_iso)?;
        crate::js::to_json(&out)
    })
}

/// `recordInstallMarkerFailureSync`.
#[napi(js_name = "recordInstallMarkerFailureSync")]
pub fn record_install_marker_failure_sync(
    file_path: String,
    expected: String,
    now_iso: String,
) -> napi::Result<String> {
    run(move || {
        let expected = crate::js::from_json(&expected)?;
        let out = port::record_install_marker_failure_sync(file_path, expected, now_iso)?;
        crate::js::to_json(&out)
    })
}

/// `resolveInstallMarkerOutcome`.
#[napi(js_name = "resolveInstallMarkerOutcome")]
pub fn resolve_install_marker_outcome(
    marker: String,
    current_version: String,
    now_iso: String,
) -> napi::Result<String> {
    run(move || {
        let marker = crate::js::from_json(&marker)?;
        let out = port::resolve_install_marker_outcome(marker, current_version, now_iso)?;
        Ok(out)
    })
}

/// `clearInstallMarker`.
#[napi(js_name = "clearInstallMarker")]
pub fn clear_install_marker(file_path: String) -> napi::Result<()> {
    run(move || {
        port::clear_install_marker(file_path)?;
        Ok(())
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

static S_INSTALL_MARKER_MATCHES_HANDOFF_EXPECTATION_0: wire::Schema = wire::Schema::Dyn;
static S_INSTALL_MARKER_MATCHES_HANDOFF_EXPECTATION_1: wire::Schema = wire::Schema::Dyn;

/// `installMarkerMatchesHandoffExpectation` over the binary wire.
#[napi(js_name = "installMarkerMatchesHandoffExpectationWire")]
pub fn install_marker_matches_handoff_expectation_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let marker: serde_json::Value =
            wire::read_value(r, &S_INSTALL_MARKER_MATCHES_HANDOFF_EXPECTATION_0)?;
        let expected: serde_json::Value =
            wire::read_value(r, &S_INSTALL_MARKER_MATCHES_HANDOFF_EXPECTATION_1)?;
        let out = &port::install_marker_matches_handoff_expectation(marker, expected)?;
        w.bool(*out);
        Ok(())
    })
}

static S_CREATE_UPDATE_INSTALL_MARKER_0: wire::Schema = wire::Schema::Dyn;
static S_CREATE_UPDATE_INSTALL_MARKER_OUT: wire::Schema = wire::Schema::Obj(&[
    ("schemaVersion", wire::Schema::Num),
    ("attemptId", wire::Schema::Str16),
    ("fromVersion", wire::Schema::Str16),
    ("toVersion", wire::Schema::Str16),
    ("requestedAt", wire::Schema::Str16),
    ("handoffAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("phase", wire::Schema::Str16),
    ("consecutiveFailures", wire::Schema::Num),
    ("lastFailureAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "artifact",
        wire::Schema::Obj(&[
            ("path", wire::Schema::Str16),
            ("size", wire::Schema::Num),
            ("sha512", wire::Schema::Str16),
        ]),
    ),
]);

/// `createUpdateInstallMarker` over the binary wire.
#[napi(js_name = "createUpdateInstallMarkerWire")]
pub fn create_update_install_marker_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let args: serde_json::Value = wire::read_value(r, &S_CREATE_UPDATE_INSTALL_MARKER_0)?;
        let out = &port::create_update_install_marker(args)?;
        wire::write_value(w, out, &S_CREATE_UPDATE_INSTALL_MARKER_OUT)?;
        Ok(())
    })
}

static S_READ_INSTALL_MARKER_OUT: wire::Schema = wire::Schema::Dyn;

/// `readInstallMarker` over the binary wire.
#[napi(js_name = "readInstallMarkerWire")]
pub fn read_install_marker_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let out = &port::read_install_marker(file_path)?;
        wire::write_value(w, out, &S_READ_INSTALL_MARKER_OUT)?;
        Ok(())
    })
}

static S_WRITE_INSTALL_MARKER_1: wire::Schema = wire::Schema::Dyn;

/// `writeInstallMarker` over the binary wire.
#[napi(js_name = "writeInstallMarkerWire")]
pub fn write_install_marker_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let file_path: String = r.string16()?;
        let marker: serde_json::Value = wire::read_value(r, &S_WRITE_INSTALL_MARKER_1)?;
        let () = port::write_install_marker(file_path, marker)?;
        Ok(())
    })
}

static S_MARK_INSTALL_HANDOFF_SYNC_1: wire::Schema = wire::Schema::Dyn;
static S_MARK_INSTALL_HANDOFF_SYNC_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Obj(&[
    ("schemaVersion", wire::Schema::Num),
    ("attemptId", wire::Schema::Str16),
    ("fromVersion", wire::Schema::Str16),
    ("toVersion", wire::Schema::Str16),
    ("requestedAt", wire::Schema::Str16),
    ("handoffAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    ("phase", wire::Schema::Str16),
    ("consecutiveFailures", wire::Schema::Num),
    ("lastFailureAt", wire::Schema::OptJs(&wire::Schema::Str16)),
    (
        "artifact",
        wire::Schema::Obj(&[
            ("path", wire::Schema::Str16),
            ("size", wire::Schema::Num),
            ("sha512", wire::Schema::Str16),
        ]),
    ),
]));

/// `markInstallHandoffSync` over the binary wire.
#[napi(js_name = "markInstallHandoffSyncWire")]
pub fn mark_install_handoff_sync_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let expected: serde_json::Value = wire::read_value(r, &S_MARK_INSTALL_HANDOFF_SYNC_1)?;
        let now_iso: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::mark_install_handoff_sync(file_path, expected, now_iso)?;
        match out {
            Some(x0) => {
                w.tag(1);
                wire::write_value(w, x0, S_MARK_INSTALL_HANDOFF_SYNC_OUT.inner())?;
            }
            None => w.tag(2),
        }
        Ok(())
    })
}

static S_RECORD_INSTALL_MARKER_FAILURE_SYNC_1: wire::Schema = wire::Schema::Dyn;
static S_RECORD_INSTALL_MARKER_FAILURE_SYNC_OUT: wire::Schema = wire::Schema::Dyn;

/// `recordInstallMarkerFailureSync` over the binary wire.
#[napi(js_name = "recordInstallMarkerFailureSyncWire")]
pub fn record_install_marker_failure_sync_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let file_path: String = r.string16()?;
        let expected: serde_json::Value =
            wire::read_value(r, &S_RECORD_INSTALL_MARKER_FAILURE_SYNC_1)?;
        let now_iso: String = r.string16()?;
        let out = &port::record_install_marker_failure_sync(file_path, expected, now_iso)?;
        wire::write_value(w, out, &S_RECORD_INSTALL_MARKER_FAILURE_SYNC_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_INSTALL_MARKER_OUTCOME_0: wire::Schema = wire::Schema::Dyn;

/// `resolveInstallMarkerOutcome` over the binary wire.
#[napi(js_name = "resolveInstallMarkerOutcomeWire")]
pub fn resolve_install_marker_outcome_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let marker: serde_json::Value = wire::read_value(r, &S_RESOLVE_INSTALL_MARKER_OUTCOME_0)?;
        let current_version: String = r.string16()?;
        let now_iso: String = r.string16()?;
        let out = &port::resolve_install_marker_outcome(marker, current_version, now_iso)?;
        w.string16(out);
        Ok(())
    })
}

/// `clearInstallMarker` over the binary wire.
#[napi(js_name = "clearInstallMarkerWire")]
pub fn clear_install_marker_wire() -> napi::Result<f64> {
    wire_io(|r, _w| {
        let file_path: String = r.string16()?;
        let () = port::clear_install_marker(file_path)?;
        Ok(())
    })
}
