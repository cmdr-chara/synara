//! JS bindings for `packages/shared/src/threadPullRequestLinks.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `pullRequestLinkKey`.
#[napi(js_name = "pullRequestLinkKey")]
pub fn pull_request_link_key(url: String) -> napi::Result<String> {
    run(move || {
        let out = port::pull_request_link_key(url)?;
        Ok(out)
    })
}

/// `isExplicitPullRequestLinkSource`.
#[napi(js_name = "isExplicitPullRequestLinkSource")]
pub fn is_explicit_pull_request_link_source(source: String) -> napi::Result<bool> {
    run(move || {
        let source = crate::js::from_json(&source)?;
        let out = port::is_explicit_pull_request_link_source(source)?;
        Ok(out)
    })
}

/// `isPullRequestLinkVisible`.
#[napi(js_name = "isPullRequestLinkVisible")]
pub fn is_pull_request_link_visible(link: String) -> napi::Result<bool> {
    run(move || {
        let link = crate::js::from_json(&link)?;
        let out = port::is_pull_request_link_visible(link)?;
        Ok(out)
    })
}

/// `visiblePullRequestLinks`.
#[napi(js_name = "visiblePullRequestLinks")]
pub fn visible_pull_request_links(links: String) -> napi::Result<String> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let out = port::visible_pull_request_links(links)?;
        crate::js::to_json(&out)
    })
}

/// `findPullRequestLink`.
#[napi(js_name = "findPullRequestLink")]
pub fn find_pull_request_link(links: String, url: String) -> napi::Result<String> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let out = port::find_pull_request_link(links, url)?;
        crate::js::to_json(&out)
    })
}

/// `isPullRequestDismissed`.
#[napi(js_name = "isPullRequestDismissed")]
pub fn is_pull_request_dismissed(links: String, url: String) -> napi::Result<bool> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let out = port::is_pull_request_dismissed(links, url)?;
        Ok(out)
    })
}

/// `compareLinkedPullRequests`.
#[napi(js_name = "compareLinkedPullRequests")]
pub fn compare_linked_pull_requests(left: String, right: String) -> napi::Result<f64> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::compare_linked_pull_requests(left, right)?;
        Ok(out)
    })
}

/// `resolveCurrentPullRequestLink`.
#[napi(js_name = "resolveCurrentPullRequestLink")]
pub fn resolve_current_pull_request_link(links: String) -> napi::Result<String> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let out = port::resolve_current_pull_request_link(links)?;
        crate::js::to_json(&out)
    })
}

/// `resolveCurrentPullRequest`.
#[napi(js_name = "resolveCurrentPullRequest")]
pub fn resolve_current_pull_request(links: String) -> napi::Result<String> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let out = port::resolve_current_pull_request(links)?;
        crate::js::to_json(&out)
    })
}

/// `resolveBranchManagedPullRequest`.
#[napi(js_name = "resolveBranchManagedPullRequest")]
pub fn resolve_branch_managed_pull_request(links: String) -> napi::Result<String> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let out = port::resolve_branch_managed_pull_request(links)?;
        crate::js::to_json(&out)
    })
}

/// `legacyPullRequestLinks`.
#[napi(js_name = "legacyPullRequestLinks")]
pub fn legacy_pull_request_links(pull_request: String, linked_at: String) -> napi::Result<String> {
    run(move || {
        let pull_request = crate::js::from_json(&pull_request)?;
        let out = port::legacy_pull_request_links(pull_request, linked_at)?;
        crate::js::to_json(&out)
    })
}

/// `pullRequestLinksFromEventPayload`.
#[napi(js_name = "pullRequestLinksFromEventPayload")]
pub fn pull_request_links_from_event_payload(
    payload: String,
    occurred_at: String,
) -> napi::Result<String> {
    run(move || {
        let payload = crate::js::from_json(&payload)?;
        let out = port::pull_request_links_from_event_payload(payload, occurred_at)?;
        crate::js::to_json(&out)
    })
}

/// `linkPullRequest`.
#[napi(js_name = "linkPullRequest")]
pub fn link_pull_request(links: String, input: String) -> napi::Result<String> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let input = crate::js::from_json(&input)?;
        let out = port::link_pull_request(links, input)?;
        crate::js::to_json(&out)
    })
}

/// `unlinkPullRequest`.
#[napi(js_name = "unlinkPullRequest")]
pub fn unlink_pull_request(
    links: String,
    url: String,
    dismissed_at: String,
) -> napi::Result<String> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let out = port::unlink_pull_request(links, url, dismissed_at)?;
        crate::js::to_json(&out)
    })
}

/// `syncDiscoveredPullRequest`.
#[napi(js_name = "syncDiscoveredPullRequest")]
pub fn sync_discovered_pull_request(
    links: String,
    pull_request: String,
    observed_at: String,
) -> napi::Result<String> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let pull_request = crate::js::from_json(&pull_request)?;
        let out = port::sync_discovered_pull_request(links, pull_request, observed_at)?;
        crate::js::to_json(&out)
    })
}

/// `refreshPullRequestSnapshots`.
#[napi(js_name = "refreshPullRequestSnapshots")]
pub fn refresh_pull_request_snapshots(
    links: String,
    pull_requests: String,
) -> napi::Result<String> {
    run(move || {
        let links = crate::js::from_json(&links)?;
        let pull_requests = crate::js::from_json(&pull_requests)?;
        let out = port::refresh_pull_request_snapshots(links, pull_requests)?;
        crate::js::to_json(&out)
    })
}

/// `threadPullRequestsEqual`.
#[napi(js_name = "threadPullRequestsEqual")]
pub fn thread_pull_requests_equal(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::thread_pull_requests_equal(left, right)?;
        Ok(out)
    })
}

/// `pullRequestLinksEqual`.
#[napi(js_name = "pullRequestLinksEqual")]
pub fn pull_request_links_equal(left: String, right: String) -> napi::Result<bool> {
    run(move || {
        let left = crate::js::from_json(&left)?;
        let right = crate::js::from_json(&right)?;
        let out = port::pull_request_links_equal(left, right)?;
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

/// `pullRequestLinkKey` over the binary wire.
#[napi(js_name = "pullRequestLinkKeyWire")]
pub fn pull_request_link_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url: String = r.string16()?;
        let out = &port::pull_request_link_key(url)?;
        w.string16(out);
        Ok(())
    })
}

static S_IS_EXPLICIT_PULL_REQUEST_LINK_SOURCE_0: wire::Schema = wire::Schema::Dyn;

/// `isExplicitPullRequestLinkSource` over the binary wire.
#[napi(js_name = "isExplicitPullRequestLinkSourceWire")]
pub fn is_explicit_pull_request_link_source_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let source: serde_json::Value =
            wire::read_value(r, &S_IS_EXPLICIT_PULL_REQUEST_LINK_SOURCE_0)?;
        let out = &port::is_explicit_pull_request_link_source(source)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_PULL_REQUEST_LINK_VISIBLE_0: wire::Schema = wire::Schema::Dyn;

/// `isPullRequestLinkVisible` over the binary wire.
#[napi(js_name = "isPullRequestLinkVisibleWire")]
pub fn is_pull_request_link_visible_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let link: serde_json::Value = wire::read_value(r, &S_IS_PULL_REQUEST_LINK_VISIBLE_0)?;
        let out = &port::is_pull_request_link_visible(link)?;
        w.bool(*out);
        Ok(())
    })
}

static S_VISIBLE_PULL_REQUEST_LINKS_0: wire::Schema = wire::Schema::Dyn;

/// `visiblePullRequestLinks` over the binary wire.
#[napi(js_name = "visiblePullRequestLinksWire")]
pub fn visible_pull_request_links_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value = wire::read_value(r, &S_VISIBLE_PULL_REQUEST_LINKS_0)?;
        let out = &port::visible_pull_request_links(links)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_FIND_PULL_REQUEST_LINK_0: wire::Schema = wire::Schema::Dyn;

/// `findPullRequestLink` over the binary wire.
#[napi(js_name = "findPullRequestLinkWire")]
pub fn find_pull_request_link_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value = wire::read_value(r, &S_FIND_PULL_REQUEST_LINK_0)?;
        let url: String = r.string16()?;
        let out = &port::find_pull_request_link(links, url)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_IS_PULL_REQUEST_DISMISSED_0: wire::Schema = wire::Schema::Dyn;

/// `isPullRequestDismissed` over the binary wire.
#[napi(js_name = "isPullRequestDismissedWire")]
pub fn is_pull_request_dismissed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value = wire::read_value(r, &S_IS_PULL_REQUEST_DISMISSED_0)?;
        let url: String = r.string16()?;
        let out = &port::is_pull_request_dismissed(links, url)?;
        w.bool(*out);
        Ok(())
    })
}

static S_COMPARE_LINKED_PULL_REQUESTS_0: wire::Schema = wire::Schema::Dyn;
static S_COMPARE_LINKED_PULL_REQUESTS_1: wire::Schema = wire::Schema::Dyn;

/// `compareLinkedPullRequests` over the binary wire.
#[napi(js_name = "compareLinkedPullRequestsWire")]
pub fn compare_linked_pull_requests_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: serde_json::Value = wire::read_value(r, &S_COMPARE_LINKED_PULL_REQUESTS_0)?;
        let right: serde_json::Value = wire::read_value(r, &S_COMPARE_LINKED_PULL_REQUESTS_1)?;
        let out = &port::compare_linked_pull_requests(left, right)?;
        w.f64(*out);
        Ok(())
    })
}

static S_RESOLVE_CURRENT_PULL_REQUEST_LINK_0: wire::Schema = wire::Schema::Dyn;

/// `resolveCurrentPullRequestLink` over the binary wire.
#[napi(js_name = "resolveCurrentPullRequestLinkWire")]
pub fn resolve_current_pull_request_link_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value = wire::read_value(r, &S_RESOLVE_CURRENT_PULL_REQUEST_LINK_0)?;
        let out = &port::resolve_current_pull_request_link(links)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_CURRENT_PULL_REQUEST_0: wire::Schema = wire::Schema::Dyn;

/// `resolveCurrentPullRequest` over the binary wire.
#[napi(js_name = "resolveCurrentPullRequestWire")]
pub fn resolve_current_pull_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value = wire::read_value(r, &S_RESOLVE_CURRENT_PULL_REQUEST_0)?;
        let out = &port::resolve_current_pull_request(links)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_RESOLVE_BRANCH_MANAGED_PULL_REQUEST_0: wire::Schema = wire::Schema::Dyn;

/// `resolveBranchManagedPullRequest` over the binary wire.
#[napi(js_name = "resolveBranchManagedPullRequestWire")]
pub fn resolve_branch_managed_pull_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value =
            wire::read_value(r, &S_RESOLVE_BRANCH_MANAGED_PULL_REQUEST_0)?;
        let out = &port::resolve_branch_managed_pull_request(links)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_LEGACY_PULL_REQUEST_LINKS_0: wire::Schema = wire::Schema::Dyn;

/// `legacyPullRequestLinks` over the binary wire.
#[napi(js_name = "legacyPullRequestLinksWire")]
pub fn legacy_pull_request_links_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let pull_request: serde_json::Value = wire::read_value(r, &S_LEGACY_PULL_REQUEST_LINKS_0)?;
        let linked_at: String = r.string16()?;
        let out = &port::legacy_pull_request_links(pull_request, linked_at)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_PULL_REQUEST_LINKS_FROM_EVENT_PAYLOAD_0: wire::Schema = wire::Schema::Dyn;
static S_PULL_REQUEST_LINKS_FROM_EVENT_PAYLOAD_OUT: wire::Schema = wire::Schema::Dyn;

/// `pullRequestLinksFromEventPayload` over the binary wire.
#[napi(js_name = "pullRequestLinksFromEventPayloadWire")]
pub fn pull_request_links_from_event_payload_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let payload: serde_json::Value =
            wire::read_value(r, &S_PULL_REQUEST_LINKS_FROM_EVENT_PAYLOAD_0)?;
        let occurred_at: String = r.string16()?;
        let out = &port::pull_request_links_from_event_payload(payload, occurred_at)?;
        wire::write_value(w, out, &S_PULL_REQUEST_LINKS_FROM_EVENT_PAYLOAD_OUT)?;
        Ok(())
    })
}

static S_LINK_PULL_REQUEST_0: wire::Schema = wire::Schema::Dyn;
static S_LINK_PULL_REQUEST_1: wire::Schema = wire::Schema::Dyn;

/// `linkPullRequest` over the binary wire.
#[napi(js_name = "linkPullRequestWire")]
pub fn link_pull_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value = wire::read_value(r, &S_LINK_PULL_REQUEST_0)?;
        let input: serde_json::Value = wire::read_value(r, &S_LINK_PULL_REQUEST_1)?;
        let out = &port::link_pull_request(links, input)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_UNLINK_PULL_REQUEST_0: wire::Schema = wire::Schema::Dyn;

/// `unlinkPullRequest` over the binary wire.
#[napi(js_name = "unlinkPullRequestWire")]
pub fn unlink_pull_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value = wire::read_value(r, &S_UNLINK_PULL_REQUEST_0)?;
        let url: String = r.string16()?;
        let dismissed_at: String = r.string16()?;
        let out = &port::unlink_pull_request(links, url, dismissed_at)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_SYNC_DISCOVERED_PULL_REQUEST_0: wire::Schema = wire::Schema::Dyn;
static S_SYNC_DISCOVERED_PULL_REQUEST_1: wire::Schema = wire::Schema::Dyn;

/// `syncDiscoveredPullRequest` over the binary wire.
#[napi(js_name = "syncDiscoveredPullRequestWire")]
pub fn sync_discovered_pull_request_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value = wire::read_value(r, &S_SYNC_DISCOVERED_PULL_REQUEST_0)?;
        let pull_request: serde_json::Value =
            wire::read_value(r, &S_SYNC_DISCOVERED_PULL_REQUEST_1)?;
        let observed_at: String = r.string16()?;
        let out = &port::sync_discovered_pull_request(links, pull_request, observed_at)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_REFRESH_PULL_REQUEST_SNAPSHOTS_0: wire::Schema = wire::Schema::Dyn;
static S_REFRESH_PULL_REQUEST_SNAPSHOTS_1: wire::Schema = wire::Schema::Dyn;

/// `refreshPullRequestSnapshots` over the binary wire.
#[napi(js_name = "refreshPullRequestSnapshotsWire")]
pub fn refresh_pull_request_snapshots_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let links: serde_json::Value = wire::read_value(r, &S_REFRESH_PULL_REQUEST_SNAPSHOTS_0)?;
        let pull_requests: serde_json::Value =
            wire::read_value(r, &S_REFRESH_PULL_REQUEST_SNAPSHOTS_1)?;
        let out = &port::refresh_pull_request_snapshots(links, pull_requests)?;
        wire::write_dyn(w, &serde_json::to_value(out).map_err(|e| e.to_string())?);
        Ok(())
    })
}

static S_THREAD_PULL_REQUESTS_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_THREAD_PULL_REQUESTS_EQUAL_1: wire::Schema = wire::Schema::Dyn;

/// `threadPullRequestsEqual` over the binary wire.
#[napi(js_name = "threadPullRequestsEqualWire")]
pub fn thread_pull_requests_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: serde_json::Value = wire::read_value(r, &S_THREAD_PULL_REQUESTS_EQUAL_0)?;
        let right: serde_json::Value = wire::read_value(r, &S_THREAD_PULL_REQUESTS_EQUAL_1)?;
        let out = &port::thread_pull_requests_equal(left, right)?;
        w.bool(*out);
        Ok(())
    })
}

static S_PULL_REQUEST_LINKS_EQUAL_0: wire::Schema = wire::Schema::Dyn;
static S_PULL_REQUEST_LINKS_EQUAL_1: wire::Schema = wire::Schema::Dyn;

/// `pullRequestLinksEqual` over the binary wire.
#[napi(js_name = "pullRequestLinksEqualWire")]
pub fn pull_request_links_equal_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let left: serde_json::Value = wire::read_value(r, &S_PULL_REQUEST_LINKS_EQUAL_0)?;
        let right: serde_json::Value = wire::read_value(r, &S_PULL_REQUEST_LINKS_EQUAL_1)?;
        let out = &port::pull_request_links_equal(left, right)?;
        w.bool(*out);
        Ok(())
    })
}
