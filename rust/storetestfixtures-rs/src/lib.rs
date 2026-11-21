//! JS bindings for `apps/web/src/storeTestFixtures.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `makeThread`.
#[napi(js_name = "makeThread")]
pub fn make_thread(overrides: String) -> napi::Result<String> {
    run(move || {
        let overrides = crate::js::from_json(&overrides)?;
        let out = port::make_thread(overrides)?;
        crate::js::to_json(&out)
    })
}

/// `makeDomainEvent`.
#[napi(js_name = "makeDomainEvent")]
pub fn make_domain_event(
    r#type: String,
    payload: String,
    overrides: String,
) -> napi::Result<String> {
    run(move || {
        let r#type = crate::js::from_json(&r#type)?;
        let payload = crate::js::from_json(&payload)?;
        let overrides = crate::js::from_json(&overrides)?;
        let out = port::make_domain_event(r#type, payload, overrides)?;
        crate::js::to_json(&out)
    })
}

/// `makeActivity`.
#[napi(js_name = "makeActivity")]
pub fn make_activity(overrides: String) -> napi::Result<String> {
    run(move || {
        let overrides = crate::js::from_json(&overrides)?;
        let out = port::make_activity(overrides)?;
        crate::js::to_json(&out)
    })
}

/// `makeState`.
#[napi(js_name = "makeState")]
pub fn make_state(thread: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::make_state(thread)?;
        crate::js::to_json(&out)
    })
}

/// `makeProject`.
#[napi(js_name = "makeProject")]
pub fn make_project(overrides: String) -> napi::Result<String> {
    run(move || {
        let overrides = crate::js::from_json(&overrides)?;
        let out = port::make_project(overrides)?;
        crate::js::to_json(&out)
    })
}

/// `makeReadModelThread`.
#[napi(js_name = "makeReadModelThread")]
pub fn make_read_model_thread(overrides: String) -> napi::Result<String> {
    run(move || {
        let overrides = crate::js::from_json(&overrides)?;
        let out = port::make_read_model_thread(overrides)?;
        crate::js::to_json(&out)
    })
}

/// `makeReadModel`.
#[napi(js_name = "makeReadModel")]
pub fn make_read_model(thread: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::make_read_model(thread)?;
        crate::js::to_json(&out)
    })
}

/// `makeShellSnapshot`.
#[napi(js_name = "makeShellSnapshot")]
pub fn make_shell_snapshot(thread: String) -> napi::Result<String> {
    run(move || {
        let thread = crate::js::from_json(&thread)?;
        let out = port::make_shell_snapshot(thread)?;
        crate::js::to_json(&out)
    })
}

/// `makeReadModelProject`.
#[napi(js_name = "makeReadModelProject")]
pub fn make_read_model_project(overrides: String) -> napi::Result<String> {
    run(move || {
        let overrides = crate::js::from_json(&overrides)?;
        let out = port::make_read_model_project(overrides)?;
        crate::js::to_json(&out)
    })
}

/// `makeUsageLimitRecovery`.
#[napi(js_name = "makeUsageLimitRecovery")]
pub fn make_usage_limit_recovery(overrides: String) -> napi::Result<String> {
    run(move || {
        let overrides = crate::js::from_json(&overrides)?;
        let out = port::make_usage_limit_recovery(overrides)?;
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

static S_MAKE_THREAD_OUT: wire::Schema = wire::Schema::Obj(&[
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
            (
                "checkpointTurnCounts",
                wire::Schema::OptJs(&wire::Schema::List(&wire::Schema::Dyn)),
            ),
        ])),
    ),
    ("activities", wire::Schema::List(&wire::Schema::Dyn)),
]);

/// `makeThread` over the binary wire.
#[napi(js_name = "makeThreadWire")]
pub fn make_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let overrides: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::make_thread(overrides)?;
        wire::write_value(w, out, &S_MAKE_THREAD_OUT)?;
        Ok(())
    })
}

static S_MAKE_DOMAIN_EVENT_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_DOMAIN_EVENT_1: wire::Schema = wire::Schema::Dyn;
static S_MAKE_DOMAIN_EVENT_2: wire::Schema = wire::Schema::Dyn;
static S_MAKE_DOMAIN_EVENT_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeDomainEvent` over the binary wire.
#[napi(js_name = "makeDomainEventWire")]
pub fn make_domain_event_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let r#type: serde_json::Value = wire::read_value(r, &S_MAKE_DOMAIN_EVENT_0)?;
        let payload: serde_json::Value = wire::read_value(r, &S_MAKE_DOMAIN_EVENT_1)?;
        let overrides: serde_json::Value = wire::read_value(r, &S_MAKE_DOMAIN_EVENT_2)?;
        let out = &port::make_domain_event(r#type, payload, overrides)?;
        wire::write_value(w, out, &S_MAKE_DOMAIN_EVENT_OUT)?;
        Ok(())
    })
}

static S_MAKE_ACTIVITY_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_ACTIVITY_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeActivity` over the binary wire.
#[napi(js_name = "makeActivityWire")]
pub fn make_activity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let overrides: serde_json::Value = wire::read_value(r, &S_MAKE_ACTIVITY_0)?;
        let out = &port::make_activity(overrides)?;
        wire::write_value(w, out, &S_MAKE_ACTIVITY_OUT)?;
        Ok(())
    })
}

static S_MAKE_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `makeState` over the binary wire.
#[napi(js_name = "makeStateWire")]
pub fn make_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value = wire::read_value(r, &S_MAKE_STATE_0)?;
        let out = &port::make_state(thread)?;
        wire::write_value(w, out, &S_MAKE_STATE_OUT)?;
        Ok(())
    })
}

static S_MAKE_PROJECT_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeProject` over the binary wire.
#[napi(js_name = "makeProjectWire")]
pub fn make_project_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let overrides: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::make_project(overrides)?;
        wire::write_value(w, out, &S_MAKE_PROJECT_OUT)?;
        Ok(())
    })
}

static S_MAKE_READ_MODEL_THREAD_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_READ_MODEL_THREAD_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeReadModelThread` over the binary wire.
#[napi(js_name = "makeReadModelThreadWire")]
pub fn make_read_model_thread_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let overrides: serde_json::Value = wire::read_value(r, &S_MAKE_READ_MODEL_THREAD_0)?;
        let out = &port::make_read_model_thread(overrides)?;
        wire::write_value(w, out, &S_MAKE_READ_MODEL_THREAD_OUT)?;
        Ok(())
    })
}

static S_MAKE_READ_MODEL_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_READ_MODEL_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeReadModel` over the binary wire.
#[napi(js_name = "makeReadModelWire")]
pub fn make_read_model_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value = wire::read_value(r, &S_MAKE_READ_MODEL_0)?;
        let out = &port::make_read_model(thread)?;
        wire::write_value(w, out, &S_MAKE_READ_MODEL_OUT)?;
        Ok(())
    })
}

static S_MAKE_SHELL_SNAPSHOT_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_SHELL_SNAPSHOT_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeShellSnapshot` over the binary wire.
#[napi(js_name = "makeShellSnapshotWire")]
pub fn make_shell_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let thread: serde_json::Value = wire::read_value(r, &S_MAKE_SHELL_SNAPSHOT_0)?;
        let out = &port::make_shell_snapshot(thread)?;
        wire::write_value(w, out, &S_MAKE_SHELL_SNAPSHOT_OUT)?;
        Ok(())
    })
}

static S_MAKE_READ_MODEL_PROJECT_0: wire::Schema = wire::Schema::Dyn;
static S_MAKE_READ_MODEL_PROJECT_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeReadModelProject` over the binary wire.
#[napi(js_name = "makeReadModelProjectWire")]
pub fn make_read_model_project_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let overrides: serde_json::Value = wire::read_value(r, &S_MAKE_READ_MODEL_PROJECT_0)?;
        let out = &port::make_read_model_project(overrides)?;
        wire::write_value(w, out, &S_MAKE_READ_MODEL_PROJECT_OUT)?;
        Ok(())
    })
}

static S_MAKE_USAGE_LIMIT_RECOVERY_OUT: wire::Schema = wire::Schema::Dyn;

/// `makeUsageLimitRecovery` over the binary wire.
#[napi(js_name = "makeUsageLimitRecoveryWire")]
pub fn make_usage_limit_recovery_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let overrides: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::make_usage_limit_recovery(overrides)?;
        wire::write_value(w, out, &S_MAKE_USAGE_LIMIT_RECOVERY_OUT)?;
        Ok(())
    })
}
