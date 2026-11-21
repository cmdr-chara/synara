//! JS bindings for `apps/web/src/storeEventReducer.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `applyOrchestrationEvents`.
#[napi(js_name = "applyOrchestrationEvents")]
pub fn apply_orchestration_events(state: String, events: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let events = crate::js::from_json(&events)?;
        let out = port::apply_orchestration_events(state, events)?;
        crate::js::to_json(&out)
    })
}

/// `applyOrchestrationEventsHotPath`.
#[napi(js_name = "applyOrchestrationEventsHotPath")]
pub fn apply_orchestration_events_hot_path(
    state: String,
    events: String,
    options: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let events = crate::js::from_json(&events)?;
        let options = crate::js::from_json(&options)?;
        let out = port::apply_orchestration_events_hot_path(state, events, options)?;
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

static S_APPLY_ORCHESTRATION_EVENTS_0: wire::Schema = wire::Schema::Dyn;
static S_APPLY_ORCHESTRATION_EVENTS_1: wire::Schema = wire::Schema::Dyn;
static S_APPLY_ORCHESTRATION_EVENTS_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "shellSnapshotSequence",
        wire::Schema::OptJs(&wire::Schema::Num),
    ),
    (
        "spaces",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Dyn),
            ("name", wire::Schema::Str16),
            ("icon", wire::Schema::Dyn),
            ("sortOrder", wire::Schema::Num),
            ("createdAt", wire::Schema::Str16),
            ("updatedAt", wire::Schema::Str16),
        ])),
    ),
    (
        "projects",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Dyn),
            ("kind", wire::Schema::Dyn),
            ("name", wire::Schema::Str16),
            ("remoteName", wire::Schema::Str16),
            ("folderName", wire::Schema::Str16),
            ("localName", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("appearance", wire::Schema::Dyn),
            ("cwd", wire::Schema::Str16),
            ("defaultModelSelection", wire::Schema::Dyn),
            ("expanded", wire::Schema::Bool),
            ("isPinned", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("spaceId", wire::Schema::Dyn),
            ("createdAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("scripts", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "additionalFolders",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
        ])),
    ),
    (
        "sidebarThreadSummaryById",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("id", wire::Schema::Dyn),
            ("projectId", wire::Schema::Dyn),
            ("title", wire::Schema::Str16),
            ("modelSelection", wire::Schema::Dyn),
            ("interactionMode", wire::Schema::Dyn),
            ("envMode", wire::Schema::Dyn),
            ("branch", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("worktreePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            (
                "workingDirectory",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "associatedWorktreePath",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "associatedWorktreeBranch",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "associatedWorktreeRef",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("session", wire::Schema::Dyn),
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
            ("latestTurn", wire::Schema::Dyn),
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
            ("hasLiveTailWork", wire::Schema::Bool),
            ("pendingBackgroundWorkCount", wire::Schema::Num),
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
            ("lastKnownPr", wire::Schema::Dyn),
            ("pullRequests", wire::Schema::Dyn),
            (
                "latestReplyPreview",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
        ])),
    ),
    ("threadsHydrated", wire::Schema::Bool),
    (
        "threadIds",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    (
        "threadShellById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Obj(&[
            ("isProjectImport", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("id", wire::Schema::Dyn),
            ("codexThreadId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("projectId", wire::Schema::Dyn),
            ("title", wire::Schema::Str16),
            ("modelSelection", wire::Schema::Dyn),
            ("runtimeMode", wire::Schema::Dyn),
            ("interactionMode", wire::Schema::Dyn),
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
            ("pullRequests", wire::Schema::Dyn),
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
            ("lastVisitedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ]))),
    ),
    (
        "threadSessionById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::OptJs(
            &wire::Schema::Obj(&[
                ("provider", wire::Schema::Dyn),
                ("runtimeMode", wire::Schema::Dyn),
                ("providerInstanceId", wire::Schema::Dyn),
                ("status", wire::Schema::Dyn),
                ("activeTurnId", wire::Schema::Dyn),
                ("createdAt", wire::Schema::Dyn),
                ("updatedAt", wire::Schema::Dyn),
                ("lastError", wire::Schema::Dyn),
                ("orchestrationStatus", wire::Schema::Dyn),
            ]),
        ))),
    ),
    (
        "threadTurnStateById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Obj(&[
            ("latestTurn", wire::Schema::Dyn),
            ("pendingSourceProposedPlan", wire::Schema::Dyn),
            ("pendingTurnStartMessageId", wire::Schema::Dyn),
        ]))),
    ),
    (
        "messageIdsByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::List(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "messageByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Map16(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "activityIdsByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::List(
            &wire::Schema::Str16,
        ))),
    ),
    (
        "activityByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Map16(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "proposedPlanIdsByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::List(
            &wire::Schema::Str16,
        ))),
    ),
    (
        "proposedPlanByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Map16(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "turnDiffIdsByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::List(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "turnDiffSummaryByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Map16(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "threadDetailSyncById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Str16)),
    ),
    (
        "threadDetailRevisionSequenceById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Num)),
    ),
    (
        "deletedProjectIdsById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Num)),
    ),
    (
        "deletedThreadIdsById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Num)),
    ),
]);

/// `applyOrchestrationEvents` over the binary wire.
#[napi(js_name = "applyOrchestrationEventsWire")]
pub fn apply_orchestration_events_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_APPLY_ORCHESTRATION_EVENTS_0)?;
        let events: serde_json::Value = wire::read_value(r, &S_APPLY_ORCHESTRATION_EVENTS_1)?;
        let out = &port::apply_orchestration_events(state, events)?;
        wire::write_value(w, out, &S_APPLY_ORCHESTRATION_EVENTS_OUT)?;
        Ok(())
    })
}

static S_APPLY_ORCHESTRATION_EVENTS_HOT_PATH_0: wire::Schema = wire::Schema::Dyn;
static S_APPLY_ORCHESTRATION_EVENTS_HOT_PATH_1: wire::Schema = wire::Schema::Dyn;
static S_APPLY_ORCHESTRATION_EVENTS_HOT_PATH_2: wire::Schema = wire::Schema::Dyn;
static S_APPLY_ORCHESTRATION_EVENTS_HOT_PATH_OUT: wire::Schema = wire::Schema::Obj(&[
    (
        "shellSnapshotSequence",
        wire::Schema::OptJs(&wire::Schema::Num),
    ),
    (
        "spaces",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Dyn),
            ("name", wire::Schema::Str16),
            ("icon", wire::Schema::Dyn),
            ("sortOrder", wire::Schema::Num),
            ("createdAt", wire::Schema::Str16),
            ("updatedAt", wire::Schema::Str16),
        ])),
    ),
    (
        "projects",
        wire::Schema::List(&wire::Schema::Obj(&[
            ("id", wire::Schema::Dyn),
            ("kind", wire::Schema::Dyn),
            ("name", wire::Schema::Str16),
            ("remoteName", wire::Schema::Str16),
            ("folderName", wire::Schema::Str16),
            ("localName", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("appearance", wire::Schema::Dyn),
            ("cwd", wire::Schema::Str16),
            ("defaultModelSelection", wire::Schema::Dyn),
            ("expanded", wire::Schema::Bool),
            ("isPinned", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("spaceId", wire::Schema::Dyn),
            ("createdAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("updatedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("scripts", wire::Schema::List(&wire::Schema::Dyn)),
            (
                "additionalFolders",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
        ])),
    ),
    (
        "sidebarThreadSummaryById",
        wire::Schema::Map16(&wire::Schema::Obj(&[
            ("id", wire::Schema::Dyn),
            ("projectId", wire::Schema::Dyn),
            ("title", wire::Schema::Str16),
            ("modelSelection", wire::Schema::Dyn),
            ("interactionMode", wire::Schema::Dyn),
            ("envMode", wire::Schema::Dyn),
            ("branch", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("worktreePath", wire::Schema::OptJs(&wire::Schema::Str16)),
            (
                "workingDirectory",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "associatedWorktreePath",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "associatedWorktreeBranch",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            (
                "associatedWorktreeRef",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
            ("session", wire::Schema::Dyn),
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
            ("latestTurn", wire::Schema::Dyn),
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
            ("hasLiveTailWork", wire::Schema::Bool),
            ("pendingBackgroundWorkCount", wire::Schema::Num),
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
            ("lastKnownPr", wire::Schema::Dyn),
            ("pullRequests", wire::Schema::Dyn),
            (
                "latestReplyPreview",
                wire::Schema::OptJs(&wire::Schema::Str16),
            ),
        ])),
    ),
    ("threadsHydrated", wire::Schema::Bool),
    (
        "threadIds",
        wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
    ),
    (
        "threadShellById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Obj(&[
            ("isProjectImport", wire::Schema::OptJs(&wire::Schema::Bool)),
            ("id", wire::Schema::Dyn),
            ("codexThreadId", wire::Schema::OptJs(&wire::Schema::Str16)),
            ("projectId", wire::Schema::Dyn),
            ("title", wire::Schema::Str16),
            ("modelSelection", wire::Schema::Dyn),
            ("runtimeMode", wire::Schema::Dyn),
            ("interactionMode", wire::Schema::Dyn),
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
            ("pullRequests", wire::Schema::Dyn),
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
            ("lastVisitedAt", wire::Schema::OptJs(&wire::Schema::Str16)),
        ]))),
    ),
    (
        "threadSessionById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::OptJs(
            &wire::Schema::Obj(&[
                ("provider", wire::Schema::Dyn),
                ("runtimeMode", wire::Schema::Dyn),
                ("providerInstanceId", wire::Schema::Dyn),
                ("status", wire::Schema::Dyn),
                ("activeTurnId", wire::Schema::Dyn),
                ("createdAt", wire::Schema::Dyn),
                ("updatedAt", wire::Schema::Dyn),
                ("lastError", wire::Schema::Dyn),
                ("orchestrationStatus", wire::Schema::Dyn),
            ]),
        ))),
    ),
    (
        "threadTurnStateById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Obj(&[
            ("latestTurn", wire::Schema::Dyn),
            ("pendingSourceProposedPlan", wire::Schema::Dyn),
            ("pendingTurnStartMessageId", wire::Schema::Dyn),
        ]))),
    ),
    (
        "messageIdsByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::List(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "messageByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Map16(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "activityIdsByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::List(
            &wire::Schema::Str16,
        ))),
    ),
    (
        "activityByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Map16(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "proposedPlanIdsByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::List(
            &wire::Schema::Str16,
        ))),
    ),
    (
        "proposedPlanByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Map16(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "turnDiffIdsByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::List(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "turnDiffSummaryByThreadId",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Map16(
            &wire::Schema::Dyn,
        ))),
    ),
    (
        "threadDetailSyncById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Str16)),
    ),
    (
        "threadDetailRevisionSequenceById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Num)),
    ),
    (
        "deletedProjectIdsById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Num)),
    ),
    (
        "deletedThreadIdsById",
        wire::Schema::OptJs(&wire::Schema::Map16(&wire::Schema::Num)),
    ),
]);

/// `applyOrchestrationEventsHotPath` over the binary wire.
#[napi(js_name = "applyOrchestrationEventsHotPathWire")]
pub fn apply_orchestration_events_hot_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_APPLY_ORCHESTRATION_EVENTS_HOT_PATH_0)?;
        let events: serde_json::Value =
            wire::read_value(r, &S_APPLY_ORCHESTRATION_EVENTS_HOT_PATH_1)?;
        let options: serde_json::Value =
            wire::read_value(r, &S_APPLY_ORCHESTRATION_EVENTS_HOT_PATH_2)?;
        let out = &port::apply_orchestration_events_hot_path(state, events, options)?;
        wire::write_value(w, out, &S_APPLY_ORCHESTRATION_EVENTS_HOT_PATH_OUT)?;
        Ok(())
    })
}
