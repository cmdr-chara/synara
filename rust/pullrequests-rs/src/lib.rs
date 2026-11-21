//! JS bindings for `apps/server/src/pullRequests.logic.ts`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod corpus;
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

/// `repositoryPullRequestIdentityKey`.
#[napi(js_name = "repositoryPullRequestIdentityKey")]
pub fn repository_pull_request_identity_key(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::repository_pull_request_identity_key(input)?;
        Ok(out)
    })
}

/// `projectPullRequestIdentityKey`.
#[napi(js_name = "projectPullRequestIdentityKey")]
pub fn project_pull_request_identity_key(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::project_pull_request_identity_key(input)?;
        Ok(out)
    })
}

/// `selectRecoverablePullRequestPins`.
#[napi(js_name = "selectRecoverablePullRequestPins")]
pub fn select_recoverable_pull_request_pins(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::select_recoverable_pull_request_pins(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildPullRequestListEntry`.
#[napi(js_name = "buildPullRequestListEntry")]
pub fn build_pull_request_list_entry(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_pull_request_list_entry(input)?;
        crate::js::to_json(&out)
    })
}

/// `orderPullRequestListEntries`.
#[napi(js_name = "orderPullRequestListEntries")]
pub fn order_pull_request_list_entries(entries: String) -> napi::Result<String> {
    run(move || {
        let entries = crate::js::from_json(&entries)?;
        let out = port::order_pull_request_list_entries(entries)?;
        crate::js::to_json(&out)
    })
}

/// `isViewerReviewRequested`.
#[napi(js_name = "isViewerReviewRequested")]
pub fn is_viewer_review_requested(
    author: String,
    review_request_logins: Vec<String>,
    viewer: String,
    matched_reviewing_query: Option<bool>,
) -> napi::Result<bool> {
    run(move || {
        let author = crate::js::from_json(&author)?;
        let out = port::is_viewer_review_requested(
            author,
            review_request_logins,
            viewer,
            matched_reviewing_query,
        )?;
        Ok(out)
    })
}

/// `isPullRequestMergeMethodAllowed`.
#[napi(js_name = "isPullRequestMergeMethodAllowed")]
pub fn is_pull_request_merge_method_allowed(
    capabilities: String,
    method: String,
) -> napi::Result<String> {
    run(move || {
        let capabilities = crate::js::from_json(&capabilities)?;
        let method = crate::js::from_json(&method)?;
        let out = port::is_pull_request_merge_method_allowed(capabilities, method)?;
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

static S_REPOSITORY_PULL_REQUEST_IDENTITY_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `repositoryPullRequestIdentityKey` over the binary wire.
#[napi(js_name = "repositoryPullRequestIdentityKeyWire")]
pub fn repository_pull_request_identity_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_REPOSITORY_PULL_REQUEST_IDENTITY_KEY_0)?;
        let out = &port::repository_pull_request_identity_key(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_PROJECT_PULL_REQUEST_IDENTITY_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `projectPullRequestIdentityKey` over the binary wire.
#[napi(js_name = "projectPullRequestIdentityKeyWire")]
pub fn project_pull_request_identity_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_PROJECT_PULL_REQUEST_IDENTITY_KEY_0)?;
        let out = &port::project_pull_request_identity_key(input)?;
        w.string16(out);
        Ok(())
    })
}

static S_SELECT_RECOVERABLE_PULL_REQUEST_PINS_0: wire::Schema = wire::Schema::Dyn;
static S_SELECT_RECOVERABLE_PULL_REQUEST_PINS_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("projectId", wire::Schema::Dyn),
        ("repositoryKey", wire::Schema::Str16),
        ("number", wire::Schema::Num),
    ]));

/// `selectRecoverablePullRequestPins` over the binary wire.
#[napi(js_name = "selectRecoverablePullRequestPinsWire")]
pub fn select_recoverable_pull_request_pins_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value =
            wire::read_value(r, &S_SELECT_RECOVERABLE_PULL_REQUEST_PINS_0)?;
        let out = &port::select_recoverable_pull_request_pins(input)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_SELECT_RECOVERABLE_PULL_REQUEST_PINS_OUT.inner())?;
        }
        Ok(())
    })
}

static S_BUILD_PULL_REQUEST_LIST_ENTRY_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_PULL_REQUEST_LIST_ENTRY_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildPullRequestListEntry` over the binary wire.
#[napi(js_name = "buildPullRequestListEntryWire")]
pub fn build_pull_request_list_entry_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_PULL_REQUEST_LIST_ENTRY_0)?;
        let out = &port::build_pull_request_list_entry(input)?;
        wire::write_value(w, out, &S_BUILD_PULL_REQUEST_LIST_ENTRY_OUT)?;
        Ok(())
    })
}

static S_ORDER_PULL_REQUEST_LIST_ENTRIES_0: wire::Schema = wire::Schema::Dyn;
static S_ORDER_PULL_REQUEST_LIST_ENTRIES_OUT: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("isPinned", wire::Schema::OptJs(&wire::Schema::Bool)),
        ("updatedAt", wire::Schema::Str16),
    ]));

/// `orderPullRequestListEntries` over the binary wire.
#[napi(js_name = "orderPullRequestListEntriesWire")]
pub fn order_pull_request_list_entries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let entries: serde_json::Value = wire::read_value(r, &S_ORDER_PULL_REQUEST_LIST_ENTRIES_0)?;
        let out = &port::order_pull_request_list_entries(entries)?;
        w.tag(1);
        w.count(out.len());
        for x0 in out.iter() {
            wire::write_value(w, x0, S_ORDER_PULL_REQUEST_LIST_ENTRIES_OUT.inner())?;
        }
        Ok(())
    })
}

/// `isViewerReviewRequested` over the binary wire.
#[napi(js_name = "isViewerReviewRequestedWire")]
pub fn is_viewer_review_requested_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let author: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let review_request_logins: Vec<String> = r.list(|r| r.string16())?;
        let viewer: String = r.string16()?;
        let matched_reviewing_query: Option<bool> = r.opt_js(|r| r.bool())?;
        let out = &port::is_viewer_review_requested(
            author,
            review_request_logins,
            viewer,
            matched_reviewing_query,
        )?;
        w.bool(*out);
        Ok(())
    })
}

static S_IS_PULL_REQUEST_MERGE_METHOD_ALLOWED_0: wire::Schema = wire::Schema::Dyn;
static S_IS_PULL_REQUEST_MERGE_METHOD_ALLOWED_1: wire::Schema = wire::Schema::Dyn;
static S_IS_PULL_REQUEST_MERGE_METHOD_ALLOWED_OUT: wire::Schema = wire::Schema::Bool;

/// `isPullRequestMergeMethodAllowed` over the binary wire.
#[napi(js_name = "isPullRequestMergeMethodAllowedWire")]
pub fn is_pull_request_merge_method_allowed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let capabilities: serde_json::Value =
            wire::read_value(r, &S_IS_PULL_REQUEST_MERGE_METHOD_ALLOWED_0)?;
        let method: serde_json::Value =
            wire::read_value(r, &S_IS_PULL_REQUEST_MERGE_METHOD_ALLOWED_1)?;
        let out = &port::is_pull_request_merge_method_allowed(capabilities, method)?;
        wire::write_value(w, out, &S_IS_PULL_REQUEST_MERGE_METHOD_ALLOWED_OUT)?;
        Ok(())
    })
}
