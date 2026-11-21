//! JS bindings for `apps/web/src/worktreeCleanup.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `getOrphanedWorktreePathForThread`.
#[napi(js_name = "getOrphanedWorktreePathForThread")]
pub fn get_orphaned_worktree_path_for_thread(
    threads: String,
    thread_id: String,
) -> napi::Result<String> {
    run(move || {
        let threads = crate::js::from_json(&threads)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::get_orphaned_worktree_path_for_thread(threads, thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `isThreadAssociatedWithWorktree`.
#[napi(js_name = "isThreadAssociatedWithWorktree")]
pub fn is_thread_associated_with_worktree(
    thread: String,
    worktree_path: String,
) -> napi::Result<bool> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::is_thread_associated_with_worktree(thread, worktree_path)?;
        Ok(out)
    })
}

/// `formatWorktreePathForDisplay`.
#[napi(js_name = "formatWorktreePathForDisplay")]
pub fn format_worktree_path_for_display(worktree_path: String) -> napi::Result<String> {
    run(move || {
        let out = port::format_worktree_path_for_display(worktree_path)?;
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

static S_GET_ORPHANED_WORKTREE_PATH_FOR_THREAD_0: wire::Schema =
    wire::Schema::List(&wire::Schema::Obj(&[
        ("isProjectImport", wire::Schema::OptJs(&wire::Schema::Bool)),
        ("id", wire::Schema::Dyn),
        ("codexThreadId", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("projectId", wire::Schema::Dyn),
        ("title", wire::Schema::Str16),
        ("modelSelection", wire::Schema::Dyn),
        ("runtimeMode", wire::Schema::Dyn),
        ("interactionMode", wire::Schema::Dyn),
        (
            "session",
            wire::Schema::OptJs(&wire::Schema::Obj(&[
                ("provider", wire::Schema::Dyn),
                ("runtimeMode", wire::Schema::Dyn),
                ("providerInstanceId", wire::Schema::Dyn),
                ("status", wire::Schema::Dyn),
                ("activeTurnId", wire::Schema::Dyn),
                ("createdAt", wire::Schema::Str16),
                ("updatedAt", wire::Schema::Str16),
                ("lastError", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("orchestrationStatus", wire::Schema::Dyn),
            ])),
        ),
        (
            "messages",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("id", wire::Schema::Dyn),
                ("role", wire::Schema::Str16),
                ("text", wire::Schema::Str16),
                (
                    "textSegments",
                    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
                ),
                ("asyncUserInput", wire::Schema::Dyn),
                (
                    "attachments",
                    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
                ),
                (
                    "skills",
                    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
                ),
                (
                    "mentions",
                    wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
                ),
                ("dispatchMode", wire::Schema::Dyn),
                ("dispatchOrigin", wire::Schema::Dyn),
                ("startsNewTurn", wire::Schema::OptJs(&wire::Schema::Bool)),
                ("turnId", wire::Schema::Dyn),
                ("createdAt", wire::Schema::Str16),
                ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("completedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("streaming", wire::Schema::Bool),
                ("source", wire::Schema::Dyn),
            ])),
        ),
        (
            "proposedPlans",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("id", wire::Schema::Dyn),
                ("turnId", wire::Schema::Dyn),
                ("planMarkdown", wire::Schema::Str16),
                ("implementedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("implementationThreadId", wire::Schema::Dyn),
                ("createdAt", wire::Schema::Str16),
                ("updatedAt", wire::Schema::Str16),
            ])),
        ),
        ("error", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("createdAt", wire::Schema::Str16),
        ("archivedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("settledAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("snoozedUntil", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "snoozeReminderAt",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("isPinned", wire::Schema::OptJs(&wire::Schema::Bool)),
        (
            "pinnedMessages",
            wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
        ),
        ("notes", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("goal", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("goalStartedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("goalPausedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "goalAchievements",
            wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
        ),
        ("latestTurn", wire::Schema::Dyn),
        ("pendingSourceProposedPlan", wire::Schema::Dyn),
        ("pendingTurnStartMessageId", wire::Schema::Dyn),
        ("lastVisitedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("parentThreadId", wire::Schema::Dyn),
        ("creationSource", wire::Schema::Dyn),
        ("sourceThreadId", wire::Schema::Dyn),
        ("subagentAgentId", wire::Schema::OptJs(&wire::Schema::Str16)),
        (
            "subagentNickname",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("subagentRole", wire::Schema::OptJs(&wire::Schema::Str16)),
        ("forkSourceThreadId", wire::Schema::Dyn),
        ("sidechatSourceThreadId", wire::Schema::Dyn),
        ("sidechatContext", wire::Schema::Dyn),
        (
            "sidechatLastActivityAt",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        (
            "sidechatExpiredAt",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        ("handoff", wire::Schema::Dyn),
        ("usageLimitRecovery", wire::Schema::Dyn),
        (
            "usageLimitRecoverySequence",
            wire::Schema::OptJs(&wire::Schema::Num),
        ),
        ("claudeCacheReview", wire::Schema::Dyn),
        (
            "claudeCacheReviewSequence",
            wire::Schema::OptJs(&wire::Schema::Num),
        ),
        ("snoozeSequence", wire::Schema::OptJs(&wire::Schema::Num)),
        ("lastKnownPr", wire::Schema::Dyn),
        (
            "pullRequests",
            wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
        ),
        (
            "latestUserMessageAt",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        (
            "latestHumanMessageAt",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        (
            "hasPendingApprovals",
            wire::Schema::OptJs(&wire::Schema::Bool),
        ),
        (
            "hasPendingUserInput",
            wire::Schema::OptJs(&wire::Schema::Bool),
        ),
        (
            "hasActionableProposedPlan",
            wire::Schema::OptJs(&wire::Schema::Bool),
        ),
        (
            "pendingBackgroundTaskCount",
            wire::Schema::OptJs(&wire::Schema::Num),
        ),
        (
            "latestReplyPreview",
            wire::Schema::OptJs(&wire::Schema::Str16),
        ),
        (
            "pendingInteractions",
            wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
        ),
        (
            "turnDiffSummaries",
            wire::Schema::List(&wire::Schema::Obj(&[
                ("turnId", wire::Schema::Dyn),
                ("startedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("completedAt", wire::Schema::Str16),
                ("status", wire::Schema::OptJs(&wire::Schema::Str16)),
                ("files", wire::Schema::List(&wire::Schema::Dyn)),
                ("checkpointRef", wire::Schema::Dyn),
                ("assistantMessageId", wire::Schema::Dyn),
                (
                    "checkpointTurnCount",
                    wire::Schema::OptJs(&wire::Schema::Num),
                ),
                ("checkpointTurnCounts", wire::Schema::Dyn),
            ])),
        ),
        ("activities", wire::Schema::List(&wire::Schema::Dyn)),
    ]));
static S_GET_ORPHANED_WORKTREE_PATH_FOR_THREAD_1: wire::Schema = wire::Schema::Dyn;
static S_GET_ORPHANED_WORKTREE_PATH_FOR_THREAD_OUT: wire::Schema =
    wire::Schema::OptJs(&wire::Schema::Str16);

/// `getOrphanedWorktreePathForThread` over the binary wire.
#[napi(js_name = "getOrphanedWorktreePathForThreadWire")]
pub fn get_orphaned_worktree_path_for_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let threads: Vec<serde_json::Value> =
            r.list(|r| wire::read_value(r, S_GET_ORPHANED_WORKTREE_PATH_FOR_THREAD_0.inner()))?;
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_GET_ORPHANED_WORKTREE_PATH_FOR_THREAD_1)?;
        let out = &port::get_orphaned_worktree_path_for_thread(threads, thread_id)?;
        wire::write_value(w, out, &S_GET_ORPHANED_WORKTREE_PATH_FOR_THREAD_OUT)?;
        Ok(())
    })
}

static S_IS_THREAD_ASSOCIATED_WITH_WORKTREE_0: wire::Schema = wire::Schema::Dyn;

/// `isThreadAssociatedWithWorktree` over the binary wire.
#[napi(js_name = "isThreadAssociatedWithWorktreeWire")]
pub fn is_thread_associated_with_worktree_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value =
            wire::read_value(r, &S_IS_THREAD_ASSOCIATED_WITH_WORKTREE_0)?;
        let worktree_path: String = r.string16()?;
        let out = &port::is_thread_associated_with_worktree(thread, worktree_path)?;
        w.bool(*out);
        Ok(())
    })
}

/// `formatWorktreePathForDisplay` over the binary wire.
#[napi(js_name = "formatWorktreePathForDisplayWire")]
pub fn format_worktree_path_for_display_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let worktree_path: String = r.string16()?;
        let out = &port::format_worktree_path_for_display(worktree_path)?;
        w.string16(out);
        Ok(())
    })
}
