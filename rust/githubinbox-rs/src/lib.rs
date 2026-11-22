//! JS bindings for `apps/server/src/githubInbox/`: generated from the signatures in `port.rs` by rust-anything.
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

/// `githubInboxFailureBackoffMs`.
#[napi(js_name = "githubInboxFailureBackoffMs")]
pub fn github_inbox_failure_backoff_ms(consecutive_failures: f64) -> napi::Result<f64> {
    run(move || {
        let out = port::github_inbox_failure_backoff_ms(consecutive_failures)?;
        Ok(out)
    })
}

/// `githubInboxRemoteItemState`.
#[napi(js_name = "githubInboxRemoteItemState")]
pub fn github_inbox_remote_item_state(item: String) -> napi::Result<String> {
    run(move || {
        let item = crate::js::from_json(&item)?;
        let out = port::github_inbox_remote_item_state(item)?;
        crate::js::to_json(&out)
    })
}

/// `githubViewerInvolvement`.
#[napi(js_name = "githubViewerInvolvement")]
pub fn github_viewer_involvement(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::github_viewer_involvement(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildIssueListEntry`.
#[napi(js_name = "buildIssueListEntry")]
pub fn build_issue_list_entry(input: String) -> napi::Result<String> {
    run(move || {
        let input = crate::js::from_json(&input)?;
        let out = port::build_issue_list_entry(input)?;
        crate::js::to_json(&out)
    })
}

/// `buildGitHubInboxItem`.
#[napi(js_name = "buildGitHubInboxItem")]
pub fn build_git_hub_inbox_item(context: String, remote: String) -> napi::Result<String> {
    run(move || {
        let context = crate::js::from_json(&context)?;
        let remote = crate::js::from_json(&remote)?;
        let out = port::build_git_hub_inbox_item(context, remote)?;
        crate::js::to_json(&out)
    })
}

/// `githubItemCacheKey`.
#[napi(js_name = "githubItemCacheKey")]
pub fn github_item_cache_key(repository: String, number: f64) -> napi::Result<String> {
    run(move || {
        let out = port::github_item_cache_key(repository, number)?;
        Ok(out)
    })
}

/// `githubInboxItemIdentityKey`.
#[napi(js_name = "githubInboxItemIdentityKey")]
pub fn github_inbox_item_identity_key(item: String) -> napi::Result<String> {
    run(move || {
        let item = crate::js::from_json(&item)?;
        let out = port::github_inbox_item_identity_key(item)?;
        Ok(out)
    })
}

/// `summarizeGitHubInboxError`.
#[napi(js_name = "summarizeGitHubInboxError")]
pub fn summarize_git_hub_inbox_error(error: String) -> napi::Result<String> {
    run(move || {
        let error = crate::js::from_json(&error)?;
        let out = port::summarize_git_hub_inbox_error(error)?;
        Ok(out)
    })
}

/// `mergeRepositoryInbox`.
#[napi(js_name = "mergeRepositoryInbox")]
pub fn merge_repository_inbox(lists: String, involvement: String) -> napi::Result<String> {
    run(move || {
        let lists = crate::js::from_json(&lists)?;
        let involvement = crate::js::from_json(&involvement)?;
        let out = port::merge_repository_inbox(lists, involvement)?;
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

/// `githubInboxFailureBackoffMs` over the binary wire.
#[napi(js_name = "githubInboxFailureBackoffMsWire")]
pub fn github_inbox_failure_backoff_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let consecutive_failures: f64 = r.f64()?;
        let out = &port::github_inbox_failure_backoff_ms(consecutive_failures)?;
        w.f64(*out);
        Ok(())
    })
}

static S_GITHUB_INBOX_REMOTE_ITEM_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_GITHUB_INBOX_REMOTE_ITEM_STATE_OUT: wire::Schema = wire::Schema::Dyn;

/// `githubInboxRemoteItemState` over the binary wire.
#[napi(js_name = "githubInboxRemoteItemStateWire")]
pub fn github_inbox_remote_item_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let item: serde_json::Value = wire::read_value(r, &S_GITHUB_INBOX_REMOTE_ITEM_STATE_0)?;
        let out = &port::github_inbox_remote_item_state(item)?;
        wire::write_value(w, out, &S_GITHUB_INBOX_REMOTE_ITEM_STATE_OUT)?;
        Ok(())
    })
}

static S_GITHUB_VIEWER_INVOLVEMENT_0: wire::Schema = wire::Schema::Dyn;
static S_GITHUB_VIEWER_INVOLVEMENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `githubViewerInvolvement` over the binary wire.
#[napi(js_name = "githubViewerInvolvementWire")]
pub fn github_viewer_involvement_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_GITHUB_VIEWER_INVOLVEMENT_0)?;
        let out = &port::github_viewer_involvement(input)?;
        wire::write_value(w, out, &S_GITHUB_VIEWER_INVOLVEMENT_OUT)?;
        Ok(())
    })
}

static S_BUILD_ISSUE_LIST_ENTRY_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_ISSUE_LIST_ENTRY_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildIssueListEntry` over the binary wire.
#[napi(js_name = "buildIssueListEntryWire")]
pub fn build_issue_list_entry_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let input: serde_json::Value = wire::read_value(r, &S_BUILD_ISSUE_LIST_ENTRY_0)?;
        let out = &port::build_issue_list_entry(input)?;
        wire::write_value(w, out, &S_BUILD_ISSUE_LIST_ENTRY_OUT)?;
        Ok(())
    })
}

static S_BUILD_GIT_HUB_INBOX_ITEM_0: wire::Schema = wire::Schema::Dyn;
static S_BUILD_GIT_HUB_INBOX_ITEM_1: wire::Schema = wire::Schema::Dyn;
static S_BUILD_GIT_HUB_INBOX_ITEM_OUT: wire::Schema = wire::Schema::Dyn;

/// `buildGitHubInboxItem` over the binary wire.
#[napi(js_name = "buildGitHubInboxItemWire")]
pub fn build_git_hub_inbox_item_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let context: serde_json::Value = wire::read_value(r, &S_BUILD_GIT_HUB_INBOX_ITEM_0)?;
        let remote: serde_json::Value = wire::read_value(r, &S_BUILD_GIT_HUB_INBOX_ITEM_1)?;
        let out = &port::build_git_hub_inbox_item(context, remote)?;
        wire::write_value(w, out, &S_BUILD_GIT_HUB_INBOX_ITEM_OUT)?;
        Ok(())
    })
}

/// `githubItemCacheKey` over the binary wire.
#[napi(js_name = "githubItemCacheKeyWire")]
pub fn github_item_cache_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let repository: String = r.string16()?;
        let number: f64 = r.f64()?;
        let out = &port::github_item_cache_key(repository, number)?;
        w.string16(out);
        Ok(())
    })
}

static S_GITHUB_INBOX_ITEM_IDENTITY_KEY_0: wire::Schema = wire::Schema::Dyn;

/// `githubInboxItemIdentityKey` over the binary wire.
#[napi(js_name = "githubInboxItemIdentityKeyWire")]
pub fn github_inbox_item_identity_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let item: serde_json::Value = wire::read_value(r, &S_GITHUB_INBOX_ITEM_IDENTITY_KEY_0)?;
        let out = &port::github_inbox_item_identity_key(item)?;
        w.string16(out);
        Ok(())
    })
}

static S_SUMMARIZE_GIT_HUB_INBOX_ERROR_0: wire::Schema = wire::Schema::Dyn;

/// `summarizeGitHubInboxError` over the binary wire.
#[napi(js_name = "summarizeGitHubInboxErrorWire")]
pub fn summarize_git_hub_inbox_error_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let error: serde_json::Value = wire::read_value(r, &S_SUMMARIZE_GIT_HUB_INBOX_ERROR_0)?;
        let out = &port::summarize_git_hub_inbox_error(error)?;
        w.string16(out);
        Ok(())
    })
}

static S_MERGE_REPOSITORY_INBOX_0: wire::Schema = wire::Schema::Dyn;
static S_MERGE_REPOSITORY_INBOX_OUT: wire::Schema = wire::Schema::Obj(&[
    ("viewer", wire::Schema::Str16),
    (
        "pullRequests",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("commentCount", wire::Schema::Num),
            ("assignees", wire::Schema::Dyn),
        ])),
    ),
    (
        "issues",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("number", wire::Schema::Num),
            ("title", wire::Schema::Str16),
            ("url", wire::Schema::Str16),
            ("author", wire::Schema::Dyn),
            ("state", wire::Schema::Dyn),
            ("stateReason", wire::Schema::Dyn),
            ("labels", wire::Schema::Dyn),
            ("assignees", wire::Schema::Dyn),
            ("commentCount", wire::Schema::Num),
            ("createdAt", wire::Schema::Str16),
            ("updatedAt", wire::Schema::Str16),
            ("closedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ])),
    ),
    ("truncatedPullRequests", wire::Schema::Bool),
    ("truncatedIssues", wire::Schema::Bool),
    ("totalPullRequests", wire::Schema::Num),
    ("totalIssues", wire::Schema::Num),
    ("involvedNumbers", wire::Schema::List(&wire::Schema::Num)),
    (
        "reviewRequestedNumbers",
        wire::Schema::List(&wire::Schema::Num),
    ),
    ("reviewRequestedCount", wire::Schema::Num),
    (
        "rateLimit",
        wire::Schema::OptJs(&wire::Schema::Obj(&[
            ("cost", wire::Schema::Num),
            ("remaining", wire::Schema::Num),
            ("resetAt", wire::Schema::Str16),
        ])),
    ),
]);

/// `mergeRepositoryInbox` over the binary wire.
#[napi(js_name = "mergeRepositoryInboxWire")]
pub fn merge_repository_inbox_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let lists: serde_json::Value = wire::read_value(r, &S_MERGE_REPOSITORY_INBOX_0)?;
        let involvement: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::merge_repository_inbox(lists, involvement)?;
        wire::write_value(w, out, &S_MERGE_REPOSITORY_INBOX_OUT)?;
        Ok(())
    })
}
