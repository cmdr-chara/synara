//! JS bindings for `packages/shared/src/threadSummary.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `pendingRequestInstanceKey`.
#[napi(js_name = "pendingRequestInstanceKey")]
pub fn pending_request_instance_key(
    request_id: String,
    lifecycle_generation: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let out = port::pending_request_instance_key(request_id, lifecycle_generation)?;
        Ok(out)
    })
}

/// `approvalRequestKindFromRequestType`.
#[napi(js_name = "approvalRequestKindFromRequestType")]
pub fn approval_request_kind_from_request_type(
    request_type: String,
) -> napi::Result<napi::bindgen_prelude::Either<String, ()>> {
    run(move || {
        let request_type = crate::js::from_json(&request_type)?;
        let out = port::approval_request_kind_from_request_type(request_type)?;
        Ok(out.map_or(
            napi::bindgen_prelude::Either::B(()),
            napi::bindgen_prelude::Either::A,
        ))
    })
}

/// `isStalePendingRequestFailureDetail`.
#[napi(js_name = "isStalePendingRequestFailureDetail")]
pub fn is_stale_pending_request_failure_detail(detail: Option<String>) -> napi::Result<bool> {
    run(move || {
        let out = port::is_stale_pending_request_failure_detail(detail)?;
        Ok(out)
    })
}

/// `buildStalePendingRequestFailureDetail`.
#[napi(js_name = "buildStalePendingRequestFailureDetail")]
pub fn build_stale_pending_request_failure_detail(
    request_kind: String,
    request_id: String,
) -> napi::Result<String> {
    run(move || {
        let out = port::build_stale_pending_request_failure_detail(request_kind, request_id)?;
        Ok(out)
    })
}

/// `derivePendingThreadRequestIds`.
#[napi(js_name = "derivePendingThreadRequestIds")]
pub fn derive_pending_thread_request_ids(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_pending_thread_request_ids(input)?;
        crate::js::to_json(&out)
    })
}

/// `resolveHumanMessageAt`.
#[napi(js_name = "resolveHumanMessageAt")]
pub fn resolve_human_message_at(message: String) -> napi::Result<String> {
    run(move || {
        let message = crate::js::from_json(&message)?;
        let out = port::resolve_human_message_at(message)?;
        crate::js::to_json(&out)
    })
}

/// `deriveThreadSummaryState`.
#[napi(js_name = "deriveThreadSummaryState")]
pub fn derive_thread_summary_state(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_thread_summary_state(input)?;
        crate::js::to_json(&out)
    })
}

/// `deriveThreadSummaryMetadata`.
#[napi(js_name = "deriveThreadSummaryMetadata")]
pub fn derive_thread_summary_metadata(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::derive_thread_summary_metadata(input)?;
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

/// `pendingRequestInstanceKey` over the binary wire.
#[napi(js_name = "pendingRequestInstanceKeyWire")]
pub fn pending_request_instance_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let request_id: String = r.string16()?;
        let lifecycle_generation: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::pending_request_instance_key(request_id, lifecycle_generation)?;
        w.string16(out);
        Ok(())
    })
}

static S_APPROVAL_REQUEST_KIND_FROM_REQUEST_TYPE_0: wire::Schema = wire::Schema::Dyn;

/// `approvalRequestKindFromRequestType` over the binary wire.
#[napi(js_name = "approvalRequestKindFromRequestTypeWire")]
pub fn approval_request_kind_from_request_type_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let request_type: serde_json::Value =
            wire::read_value(r, &S_APPROVAL_REQUEST_KIND_FROM_REQUEST_TYPE_0)?;
        let out = &port::approval_request_kind_from_request_type(request_type)?;
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

/// `isStalePendingRequestFailureDetail` over the binary wire.
#[napi(js_name = "isStalePendingRequestFailureDetailWire")]
pub fn is_stale_pending_request_failure_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let detail: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_stale_pending_request_failure_detail(detail)?;
        w.bool(*out);
        Ok(())
    })
}

/// `buildStalePendingRequestFailureDetail` over the binary wire.
#[napi(js_name = "buildStalePendingRequestFailureDetailWire")]
pub fn build_stale_pending_request_failure_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let request_kind: String = r.string16()?;
        let request_id: String = r.string16()?;
        let out = &port::build_stale_pending_request_failure_detail(request_kind, request_id)?;
        w.string16(out);
        Ok(())
    })
}

static S_DERIVE_PENDING_THREAD_REQUEST_IDS_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_PENDING_THREAD_REQUEST_IDS_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "approvalRequestIds",
        wire::Schema::List(&wire::Schema::Str16),
    ),
    (
        "userInputRequestIds",
        wire::Schema::List(&wire::Schema::Str16),
    ),
]);

/// `derivePendingThreadRequestIds` over the binary wire.
#[napi(js_name = "derivePendingThreadRequestIdsWire")]
pub fn derive_pending_thread_request_ids_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DERIVE_PENDING_THREAD_REQUEST_IDS_0)?;
        let out = &port::derive_pending_thread_request_ids(input)?;
        wire::write_value(w, out, &S_DERIVE_PENDING_THREAD_REQUEST_IDS_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_HUMAN_MESSAGE_AT_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_HUMAN_MESSAGE_AT_OUT: wire::Schema = wire::Schema::OptJs(&wire::Schema::Str16);

/// `resolveHumanMessageAt` over the binary wire.
#[napi(js_name = "resolveHumanMessageAtWire")]
pub fn resolve_human_message_at_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let message: serde_json::Value = wire::read_value(r, &S_RESOLVE_HUMAN_MESSAGE_AT_0)?;
        let out = &port::resolve_human_message_at(message)?;
        wire::write_value(w, out, &S_RESOLVE_HUMAN_MESSAGE_AT_OUT)?;
        Ok(())
    })
}

static S_DERIVE_THREAD_SUMMARY_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_THREAD_SUMMARY_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
    ("pendingApprovalCount", wire::Schema::Num),
    ("pendingUserInputCount", wire::Schema::Num),
]);

/// `deriveThreadSummaryState` over the binary wire.
#[napi(js_name = "deriveThreadSummaryStateWire")]
pub fn derive_thread_summary_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DERIVE_THREAD_SUMMARY_STATE_0)?;
        let out = &port::derive_thread_summary_state(input)?;
        wire::write_value(w, out, &S_DERIVE_THREAD_SUMMARY_STATE_OUT)?;
        Ok(())
    })
}

static S_DERIVE_THREAD_SUMMARY_METADATA_0: wire::Schema = wire::Schema::Dyn;
static S_DERIVE_THREAD_SUMMARY_METADATA_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "latestUserMessageAt",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    (
        "latestHumanMessageAt",
        wire::Schema::OptJs(&wire::Schema::Str16),
    ),
    ("hasPendingApprovals", wire::Schema::Bool),
    ("hasPendingUserInput", wire::Schema::Bool),
    ("hasActionableProposedPlan", wire::Schema::Bool),
]);

/// `deriveThreadSummaryMetadata` over the binary wire.
#[napi(js_name = "deriveThreadSummaryMetadataWire")]
pub fn derive_thread_summary_metadata_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_DERIVE_THREAD_SUMMARY_METADATA_0)?;
        let out = &port::derive_thread_summary_metadata(input)?;
        wire::write_value(w, out, &S_DERIVE_THREAD_SUMMARY_METADATA_OUT)?;
        Ok(())
    })
}
