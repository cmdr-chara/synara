//! JS bindings for `apps/web/src/storeProjection.ts`: generated from the signatures in `port.rs` by rust-anything.
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

/// `upsertProject`.
#[napi(js_name = "upsertProject")]
pub fn upsert_project(
    state: String,
    incoming: String,
    match_policy: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let incoming = crate::js::from_json(&incoming)?;
        let out = port::upsert_project(state, incoming, match_policy)?;
        crate::js::to_json(&out)
    })
}

/// `upsertSpace`.
#[napi(js_name = "upsertSpace")]
pub fn upsert_space(state: String, incoming: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let incoming = crate::js::from_json(&incoming)?;
        let out = port::upsert_space(state, incoming)?;
        crate::js::to_json(&out)
    })
}

/// `removeSpace`.
#[napi(js_name = "removeSpace")]
pub fn remove_space(
    state: String,
    space_id: String,
    assignment_updated_at: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let space_id = crate::js::from_json(&space_id)?;
        let assignment_updated_at = crate::js::from_json(&assignment_updated_at)?;
        let out = port::remove_space(state, space_id, assignment_updated_at)?;
        crate::js::to_json(&out)
    })
}

/// `applySpaceOrder`.
#[napi(js_name = "applySpaceOrder")]
pub fn apply_space_order(
    state: String,
    ordered_space_ids: String,
    updated_at: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let ordered_space_ids = crate::js::from_json(&ordered_space_ids)?;
        let updated_at = crate::js::from_json(&updated_at)?;
        let out = port::apply_space_order(state, ordered_space_ids, updated_at)?;
        crate::js::to_json(&out)
    })
}

/// `markThreadDetailSyncFailedInClientState`.
#[napi(js_name = "markThreadDetailSyncFailedInClientState")]
pub fn mark_thread_detail_sync_failed_in_client_state(
    state: String,
    thread_id: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::mark_thread_detail_sync_failed_in_client_state(state, thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `clearThreadDetailSyncFailureInClientState`.
#[napi(js_name = "clearThreadDetailSyncFailureInClientState")]
pub fn clear_thread_detail_sync_failure_in_client_state(
    state: String,
    thread_id: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::clear_thread_detail_sync_failure_in_client_state(state, thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `evictThreadDetailFromClientState`.
#[napi(js_name = "evictThreadDetailFromClientState")]
pub fn evict_thread_detail_from_client_state(
    state: String,
    thread_id: String,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out = port::evict_thread_detail_from_client_state(state, thread_id)?;
        crate::js::to_json(&out)
    })
}

/// `removeDeletedThreadFromClientState`.
#[napi(js_name = "removeDeletedThreadFromClientState")]
pub fn remove_deleted_thread_from_client_state(
    state: String,
    thread_id: String,
    deleted_at_sequence: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let thread_id = crate::js::from_json(&thread_id)?;
        let out =
            port::remove_deleted_thread_from_client_state(state, thread_id, deleted_at_sequence)?;
        crate::js::to_json(&out)
    })
}

/// `removeDeletedProjectFromClientState`.
#[napi(js_name = "removeDeletedProjectFromClientState")]
pub fn remove_deleted_project_from_client_state(
    state: String,
    project_id: String,
    deleted_at_sequence: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let project_id = crate::js::from_json(&project_id)?;
        let out =
            port::remove_deleted_project_from_client_state(state, project_id, deleted_at_sequence)?;
        crate::js::to_json(&out)
    })
}

/// `syncServerShellSnapshot`.
#[napi(js_name = "syncServerShellSnapshot")]
pub fn sync_server_shell_snapshot(state: String, snapshot: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let snapshot = crate::js::from_json(&snapshot)?;
        let out = port::sync_server_shell_snapshot(state, snapshot)?;
        crate::js::to_json(&out)
    })
}

/// `syncServerThreadDetail`.
#[napi(js_name = "syncServerThreadDetail")]
pub fn sync_server_thread_detail(state: String, thread: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let thread = crate::js::from_json(&thread)?;
        let out = port::sync_server_thread_detail(state, thread)?;
        crate::js::to_json(&out)
    })
}

/// `syncServerThreadDetailHotPath`.
#[napi(js_name = "syncServerThreadDetailHotPath")]
pub fn sync_server_thread_detail_hot_path(
    state: String,
    thread: String,
    snapshot_sequence: Option<f64>,
    history_revision_sequence: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let thread = crate::js::from_json(&thread)?;
        let out = port::sync_server_thread_detail_hot_path(
            state,
            thread,
            snapshot_sequence,
            history_revision_sequence,
        )?;
        crate::js::to_json(&out)
    })
}

/// `applyShellEvent`.
#[napi(js_name = "applyShellEvent")]
pub fn apply_shell_event(state: String, event: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let event = crate::js::from_json(&event)?;
        let out = port::apply_shell_event(state, event)?;
        crate::js::to_json(&out)
    })
}

/// `syncServerReadModel`.
#[napi(js_name = "syncServerReadModel")]
pub fn sync_server_read_model(state: String, read_model: String) -> napi::Result<String> {
    run(move || {
        let state = crate::js::from_json(&state)?;
        let read_model = crate::js::from_json(&read_model)?;
        let out = port::sync_server_read_model(state, read_model)?;
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

static S_UPSERT_PROJECT_0: wire::Schema = wire::Schema::Dyn;
static S_UPSERT_PROJECT_1: wire::Schema = wire::Schema::Dyn;
static S_UPSERT_PROJECT_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `upsertProject` over the binary wire.
#[napi(js_name = "upsertProjectWire")]
pub fn upsert_project_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_UPSERT_PROJECT_0)?;
        let incoming: serde_json::Value = wire::read_value(r, &S_UPSERT_PROJECT_1)?;
        let match_policy: String = r.string16()?;
        let out = &port::upsert_project(state, incoming, match_policy)?;
        wire::write_value(w, out, &S_UPSERT_PROJECT_OUT)?;
        Ok(())
    })
}

static S_UPSERT_SPACE_0: wire::Schema = wire::Schema::Dyn;
static S_UPSERT_SPACE_1: wire::Schema = wire::Schema::Dyn;
static S_UPSERT_SPACE_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `upsertSpace` over the binary wire.
#[napi(js_name = "upsertSpaceWire")]
pub fn upsert_space_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_UPSERT_SPACE_0)?;
        let incoming: serde_json::Value = wire::read_value(r, &S_UPSERT_SPACE_1)?;
        let out = &port::upsert_space(state, incoming)?;
        wire::write_value(w, out, &S_UPSERT_SPACE_OUT)?;
        Ok(())
    })
}

static S_REMOVE_SPACE_0: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_SPACE_1: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_SPACE_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `removeSpace` over the binary wire.
#[napi(js_name = "removeSpaceWire")]
pub fn remove_space_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_REMOVE_SPACE_0)?;
        let space_id: serde_json::Value = wire::read_value(r, &S_REMOVE_SPACE_1)?;
        let assignment_updated_at: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::remove_space(state, space_id, assignment_updated_at)?;
        wire::write_value(w, out, &S_REMOVE_SPACE_OUT)?;
        Ok(())
    })
}

static S_APPLY_SPACE_ORDER_0: wire::Schema = wire::Schema::Dyn;
static S_APPLY_SPACE_ORDER_1: wire::Schema = wire::Schema::Dyn;
static S_APPLY_SPACE_ORDER_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `applySpaceOrder` over the binary wire.
#[napi(js_name = "applySpaceOrderWire")]
pub fn apply_space_order_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_APPLY_SPACE_ORDER_0)?;
        let ordered_space_ids: serde_json::Value = wire::read_value(r, &S_APPLY_SPACE_ORDER_1)?;
        let updated_at: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::apply_space_order(state, ordered_space_ids, updated_at)?;
        wire::write_value(w, out, &S_APPLY_SPACE_ORDER_OUT)?;
        Ok(())
    })
}

static S_MARK_THREAD_DETAIL_SYNC_FAILED_IN_CLIENT_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_MARK_THREAD_DETAIL_SYNC_FAILED_IN_CLIENT_STATE_1: wire::Schema = wire::Schema::Dyn;
static S_MARK_THREAD_DETAIL_SYNC_FAILED_IN_CLIENT_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `markThreadDetailSyncFailedInClientState` over the binary wire.
#[napi(js_name = "markThreadDetailSyncFailedInClientStateWire")]
pub fn mark_thread_detail_sync_failed_in_client_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_MARK_THREAD_DETAIL_SYNC_FAILED_IN_CLIENT_STATE_0)?;
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_MARK_THREAD_DETAIL_SYNC_FAILED_IN_CLIENT_STATE_1)?;
        let out = &port::mark_thread_detail_sync_failed_in_client_state(state, thread_id)?;
        wire::write_value(
            w,
            out,
            &S_MARK_THREAD_DETAIL_SYNC_FAILED_IN_CLIENT_STATE_OUT,
        )?;
        Ok(())
    })
}

static S_CLEAR_THREAD_DETAIL_SYNC_FAILURE_IN_CLIENT_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_CLEAR_THREAD_DETAIL_SYNC_FAILURE_IN_CLIENT_STATE_1: wire::Schema = wire::Schema::Dyn;
static S_CLEAR_THREAD_DETAIL_SYNC_FAILURE_IN_CLIENT_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `clearThreadDetailSyncFailureInClientState` over the binary wire.
#[napi(js_name = "clearThreadDetailSyncFailureInClientStateWire")]
pub fn clear_thread_detail_sync_failure_in_client_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_CLEAR_THREAD_DETAIL_SYNC_FAILURE_IN_CLIENT_STATE_0)?;
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_CLEAR_THREAD_DETAIL_SYNC_FAILURE_IN_CLIENT_STATE_1)?;
        let out = &port::clear_thread_detail_sync_failure_in_client_state(state, thread_id)?;
        wire::write_value(
            w,
            out,
            &S_CLEAR_THREAD_DETAIL_SYNC_FAILURE_IN_CLIENT_STATE_OUT,
        )?;
        Ok(())
    })
}

static S_EVICT_THREAD_DETAIL_FROM_CLIENT_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_EVICT_THREAD_DETAIL_FROM_CLIENT_STATE_1: wire::Schema = wire::Schema::Dyn;
static S_EVICT_THREAD_DETAIL_FROM_CLIENT_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `evictThreadDetailFromClientState` over the binary wire.
#[napi(js_name = "evictThreadDetailFromClientStateWire")]
pub fn evict_thread_detail_from_client_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_EVICT_THREAD_DETAIL_FROM_CLIENT_STATE_0)?;
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_EVICT_THREAD_DETAIL_FROM_CLIENT_STATE_1)?;
        let out = &port::evict_thread_detail_from_client_state(state, thread_id)?;
        wire::write_value(w, out, &S_EVICT_THREAD_DETAIL_FROM_CLIENT_STATE_OUT)?;
        Ok(())
    })
}

static S_REMOVE_DELETED_THREAD_FROM_CLIENT_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_DELETED_THREAD_FROM_CLIENT_STATE_1: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_DELETED_THREAD_FROM_CLIENT_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `removeDeletedThreadFromClientState` over the binary wire.
#[napi(js_name = "removeDeletedThreadFromClientStateWire")]
pub fn remove_deleted_thread_from_client_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REMOVE_DELETED_THREAD_FROM_CLIENT_STATE_0)?;
        let thread_id: serde_json::Value =
            wire::read_value(r, &S_REMOVE_DELETED_THREAD_FROM_CLIENT_STATE_1)?;
        let deleted_at_sequence: Option<f64> = r.opt_js(|r| r.f64())?;
        let out =
            &port::remove_deleted_thread_from_client_state(state, thread_id, deleted_at_sequence)?;
        wire::write_value(w, out, &S_REMOVE_DELETED_THREAD_FROM_CLIENT_STATE_OUT)?;
        Ok(())
    })
}

static S_REMOVE_DELETED_PROJECT_FROM_CLIENT_STATE_0: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_DELETED_PROJECT_FROM_CLIENT_STATE_1: wire::Schema = wire::Schema::Dyn;
static S_REMOVE_DELETED_PROJECT_FROM_CLIENT_STATE_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `removeDeletedProjectFromClientState` over the binary wire.
#[napi(js_name = "removeDeletedProjectFromClientStateWire")]
pub fn remove_deleted_project_from_client_state_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_REMOVE_DELETED_PROJECT_FROM_CLIENT_STATE_0)?;
        let project_id: serde_json::Value =
            wire::read_value(r, &S_REMOVE_DELETED_PROJECT_FROM_CLIENT_STATE_1)?;
        let deleted_at_sequence: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::remove_deleted_project_from_client_state(
            state,
            project_id,
            deleted_at_sequence,
        )?;
        wire::write_value(w, out, &S_REMOVE_DELETED_PROJECT_FROM_CLIENT_STATE_OUT)?;
        Ok(())
    })
}

static S_SYNC_SERVER_SHELL_SNAPSHOT_0: wire::Schema = wire::Schema::Dyn;
static S_SYNC_SERVER_SHELL_SNAPSHOT_1: wire::Schema = wire::Schema::Dyn;
static S_SYNC_SERVER_SHELL_SNAPSHOT_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `syncServerShellSnapshot` over the binary wire.
#[napi(js_name = "syncServerShellSnapshotWire")]
pub fn sync_server_shell_snapshot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_SYNC_SERVER_SHELL_SNAPSHOT_0)?;
        let snapshot: serde_json::Value = wire::read_value(r, &S_SYNC_SERVER_SHELL_SNAPSHOT_1)?;
        let out = &port::sync_server_shell_snapshot(state, snapshot)?;
        wire::write_value(w, out, &S_SYNC_SERVER_SHELL_SNAPSHOT_OUT)?;
        Ok(())
    })
}

static S_SYNC_SERVER_THREAD_DETAIL_0: wire::Schema = wire::Schema::Dyn;
static S_SYNC_SERVER_THREAD_DETAIL_1: wire::Schema = wire::Schema::Dyn;
static S_SYNC_SERVER_THREAD_DETAIL_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `syncServerThreadDetail` over the binary wire.
#[napi(js_name = "syncServerThreadDetailWire")]
pub fn sync_server_thread_detail_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_SYNC_SERVER_THREAD_DETAIL_0)?;
        let thread: serde_json::Value = wire::read_value(r, &S_SYNC_SERVER_THREAD_DETAIL_1)?;
        let out = &port::sync_server_thread_detail(state, thread)?;
        wire::write_value(w, out, &S_SYNC_SERVER_THREAD_DETAIL_OUT)?;
        Ok(())
    })
}

static S_SYNC_SERVER_THREAD_DETAIL_HOT_PATH_0: wire::Schema = wire::Schema::Dyn;
static S_SYNC_SERVER_THREAD_DETAIL_HOT_PATH_1: wire::Schema = wire::Schema::Dyn;
static S_SYNC_SERVER_THREAD_DETAIL_HOT_PATH_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `syncServerThreadDetailHotPath` over the binary wire.
#[napi(js_name = "syncServerThreadDetailHotPathWire")]
pub fn sync_server_thread_detail_hot_path_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value =
            wire::read_value(r, &S_SYNC_SERVER_THREAD_DETAIL_HOT_PATH_0)?;
        let thread: serde_json::Value =
            wire::read_value(r, &S_SYNC_SERVER_THREAD_DETAIL_HOT_PATH_1)?;
        let snapshot_sequence: Option<f64> = r.opt_js(|r| r.f64())?;
        let history_revision_sequence: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::sync_server_thread_detail_hot_path(
            state,
            thread,
            snapshot_sequence,
            history_revision_sequence,
        )?;
        wire::write_value(w, out, &S_SYNC_SERVER_THREAD_DETAIL_HOT_PATH_OUT)?;
        Ok(())
    })
}

static S_APPLY_SHELL_EVENT_0: wire::Schema = wire::Schema::Dyn;
static S_APPLY_SHELL_EVENT_1: wire::Schema = wire::Schema::Dyn;
static S_APPLY_SHELL_EVENT_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `applyShellEvent` over the binary wire.
#[napi(js_name = "applyShellEventWire")]
pub fn apply_shell_event_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_APPLY_SHELL_EVENT_0)?;
        let event: serde_json::Value = wire::read_value(r, &S_APPLY_SHELL_EVENT_1)?;
        let out = &port::apply_shell_event(state, event)?;
        wire::write_value(w, out, &S_APPLY_SHELL_EVENT_OUT)?;
        Ok(())
    })
}

static S_SYNC_SERVER_READ_MODEL_0: wire::Schema = wire::Schema::Dyn;
static S_SYNC_SERVER_READ_MODEL_1: wire::Schema = wire::Schema::Dyn;
static S_SYNC_SERVER_READ_MODEL_OUT: wire::Schema = wire::Schema::Obj(&[
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

/// `syncServerReadModel` over the binary wire.
#[napi(js_name = "syncServerReadModelWire")]
pub fn sync_server_read_model_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let state: serde_json::Value = wire::read_value(r, &S_SYNC_SERVER_READ_MODEL_0)?;
        let read_model: serde_json::Value = wire::read_value(r, &S_SYNC_SERVER_READ_MODEL_1)?;
        let out = &port::sync_server_read_model(state, read_model)?;
        wire::write_value(w, out, &S_SYNC_SERVER_READ_MODEL_OUT)?;
        Ok(())
    })
}
