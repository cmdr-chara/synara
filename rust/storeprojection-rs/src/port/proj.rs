//! Port of `apps/web/src/storeProjection.ts` + `threadDerivation.ts`.

use crate::port::jv::*;
use crate::port::norm::*;
use std::rc::Rc;

type R<T> = Result<T, String>;

// ---------- threadDerivation ----------

fn collect_by_ids(ids: &JV, by_id: &JV) -> JV {
    let items = units_arr(ids);
    let obj = match by_id {
        JV::O(_, m) => m.clone(),
        _ => return a(vec![]),
    };
    if matches!(ids, JV::U) || items.is_empty() || !matches!(by_id, JV::O(..)) {
        return a(vec![]);
    }
    let mut out = Vec::new();
    for id in &items {
        let value = obj.get(&pkey(id)).cloned().unwrap_or(JV::U);
        if truthy(&value) {
            out.push(value);
        }
    }
    a(out)
}
/// `getThreadFromState` -> JV::U or a Thread JV.
pub fn get_thread_from_state(state: &JV, thread_id: &JV) -> R<JV> {
    let shells = gv(state, "threadShellById", "state.threadShellById")?;
    let shell = gk(&shells, thread_id);
    if !truthy(&shell) {
        return Ok(JV::U);
    }
    let sessions = g(state, "threadSessionById");
    let session = or(&gk(&sessions, thread_id), JV::N);
    let turn_states = g(state, "threadTurnStateById");
    let turn_state = gk(&turn_states, thread_id);
    let messages = collect_by_ids(
        &gk(&g(state, "messageIdsByThreadId"), thread_id),
        &gk(&g(state, "messageByThreadId"), thread_id),
    );
    let activities = collect_by_ids(
        &gk(&g(state, "activityIdsByThreadId"), thread_id),
        &gk(&g(state, "activityByThreadId"), thread_id),
    );
    let proposed_plans = collect_by_ids(
        &gk(&g(state, "proposedPlanIdsByThreadId"), thread_id),
        &gk(&g(state, "proposedPlanByThreadId"), thread_id),
    );
    let turn_diffs = collect_by_ids(
        &gk(&g(state, "turnDiffIdsByThreadId"), thread_id),
        &gk(&g(state, "turnDiffSummaryByThreadId"), thread_id),
    );
    let mut m = spread_map(&shell);
    mset(&mut m, "session", session);
    mset(
        &mut m,
        "latestTurn",
        or(&g(&turn_state, "latestTurn"), JV::N),
    );
    mset(
        &mut m,
        "pendingSourceProposedPlan",
        g(&turn_state, "pendingSourceProposedPlan"),
    );
    let pts = g(&turn_state, "pendingTurnStartMessageId");
    if !matches!(pts, JV::U) {
        mset(&mut m, "pendingTurnStartMessageId", pts);
    }
    mset(&mut m, "messages", messages);
    mset(&mut m, "activities", activities);
    mset(&mut m, "proposedPlans", proposed_plans);
    mset(&mut m, "turnDiffSummaries", turn_diffs);
    Ok(o(m))
}
/// `getThreadsFromState` — threadIds are raw JS values.
pub fn get_threads_from_state(state: &JV) -> Vec<JV> {
    let thread_ids = units_arr(&g(state, "threadIds"));
    thread_ids
        .iter()
        .filter_map(|id| {
            let t = get_thread_from_state(state, id).unwrap_or(JV::U);
            if matches!(t, JV::U) { None } else { Some(t) }
        })
        .collect()
}

// ---------- storeProjection helpers ----------

fn to_thread_shell(thread: &JV) -> JV {
    let mut m = Obj::new();
    let copy = |m: &mut Obj, k: &str| mset(m, k, g(thread, k));
    copy(&mut m, "id");
    copy(&mut m, "codexThreadId");
    copy(&mut m, "projectId");
    copy(&mut m, "title");
    copy(&mut m, "modelSelection");
    copy(&mut m, "runtimeMode");
    copy(&mut m, "interactionMode");
    copy(&mut m, "error");
    copy(&mut m, "createdAt");
    for k in [
        "archivedAt",
        "settledAt",
        "snoozedUntil",
        "snoozeReminderAt",
    ] {
        mset(&mut m, k, gv_or_null(thread, k));
    }
    copy(&mut m, "updatedAt");
    mset(&mut m, "isPinned", or(&g(thread, "isPinned"), b(false)));
    copy(&mut m, "envMode");
    copy(&mut m, "branch");
    copy(&mut m, "worktreePath");
    for k in [
        "workingDirectory",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
        "parentThreadId",
        "creationSource",
        "sourceThreadId",
        "subagentAgentId",
        "subagentNickname",
        "subagentRole",
        "forkSourceThreadId",
        "sidechatSourceThreadId",
        "sidechatContext",
        "sidechatLastActivityAt",
        "sidechatExpiredAt",
        "lastKnownPr",
        "handoff",
    ] {
        mset(&mut m, k, gv_or_null(thread, k));
    }
    mset(
        &mut m,
        "createBranchFlowCompleted",
        or(&g(thread, "createBranchFlowCompleted"), b(false)),
    );
    mset(
        &mut m,
        "pullRequests",
        or(&g(thread, "pullRequests"), a(vec![])),
    );
    if truthy(&g(thread, "isProjectImport")) {
        mset(&mut m, "isProjectImport", b(true));
    }
    mset(
        &mut m,
        "usageLimitRecovery",
        gv_or_null(thread, "usageLimitRecovery"),
    );
    if !matches!(g(thread, "usageLimitRecoverySequence"), JV::U) {
        mset(
            &mut m,
            "usageLimitRecoverySequence",
            g(thread, "usageLimitRecoverySequence"),
        );
    }
    mset(
        &mut m,
        "claudeCacheReview",
        gv_or_null(thread, "claudeCacheReview"),
    );
    if !matches!(g(thread, "snoozeSequence"), JV::U) {
        mset(&mut m, "snoozeSequence", g(thread, "snoozeSequence"));
    }
    if !matches!(g(thread, "claudeCacheReviewSequence"), JV::U) {
        mset(
            &mut m,
            "claudeCacheReviewSequence",
            g(thread, "claudeCacheReviewSequence"),
        );
    }
    for k in [
        "pinnedMessages",
        "notes",
        "goal",
        "goalStartedAt",
        "goalPausedAt",
        "goalAchievements",
        "latestHumanMessageAt",
        "latestUserMessageAt",
        "hasPendingApprovals",
        "hasPendingUserInput",
        "hasActionableProposedPlan",
        "pendingBackgroundTaskCount",
        "latestReplyPreview",
        "pendingInteractions",
        "lastVisitedAt",
    ] {
        if !matches!(g(thread, k), JV::U) {
            mset(&mut m, k, g(thread, k));
        }
    }
    o(m)
}
fn to_thread_turn_state(thread: &JV) -> JV {
    let mut m = Obj::new();
    mset(&mut m, "latestTurn", g(thread, "latestTurn"));
    if truthy(&g(thread, "pendingSourceProposedPlan")) {
        mset(
            &mut m,
            "pendingSourceProposedPlan",
            g(thread, "pendingSourceProposedPlan"),
        );
    }
    if !matches!(g(thread, "pendingTurnStartMessageId"), JV::U) {
        mset(
            &mut m,
            "pendingTurnStartMessageId",
            g(thread, "pendingTurnStartMessageId"),
        );
    }
    o(m)
}
/// `buildNormalizedSlice` — returns (ids, byId) JVs, reusing previous nodes when eligible.
fn build_normalized_slice(
    items: &[JV],
    id_of: fn(&JV) -> JV,
    previous_items: Option<&Vec<JV>>,
    previous_ids: &JV,
    previous_by_id: &JV,
) -> (JV, JV) {
    let mut reusable_ids = false;
    if let (Some(prev_items), JV::A(_, prev_ids), JV::O(..)) =
        (previous_items, previous_ids, previous_by_id)
        && prev_items.len() == items.len()
        && prev_ids.len() == items.len()
    {
        let first_changed = items
            .iter()
            .enumerate()
            .find(|(i, item)| !eq(&prev_items[*i], item))
            .map(|(i, _)| i);
        match first_changed {
            None => return (previous_ids.clone(), previous_by_id.clone()),
            Some(fc) => {
                reusable_ids = true;
                for (index, item) in items.iter().enumerate().skip(fc) {
                    if !eq(&prev_ids[index], &id_of(item)) {
                        reusable_ids = false;
                        break;
                    }
                }
            }
        }
    }
    let mut by_id = Obj::new();
    if reusable_ids {
        for item in items {
            by_id.insert(pkey(&id_of(item)), item.clone());
        }
        return (previous_ids.clone(), o(by_id));
    }
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let id = id_of(item);
        by_id.insert(pkey(&id), item.clone());
        ids.push(id);
    }
    (a(ids), o(by_id))
}
fn message_id(m: &JV) -> JV {
    g(m, "id")
}
fn activity_id(m: &JV) -> JV {
    g(m, "id")
}
fn proposed_plan_id(m: &JV) -> JV {
    g(m, "id")
}
fn turn_diff_id(m: &JV) -> JV {
    g(m, "turnId")
}
/// `{...state, k: v}` — preserves key order (replaces in place for spread fields).
fn state_set(state: &JV, k: &str, v: JV) -> JV {
    let mut m = spread_map(state);
    mset(&mut m, k, v);
    o(m)
}
fn ensure_thread_registered(state: &JV, thread_id: &JV) -> JV {
    let ids = units_arr(&g(state, "threadIds"));
    if ids.iter().any(|x| same_value(x, thread_id)) {
        return state.clone();
    }
    let mut next = ids.clone();
    next.push(thread_id.clone());
    state_set(state, "threadIds", a(next))
}
fn retain_thread_scoped_record(record: &JV, next_ids: &[JV]) -> JV {
    if is_nullish(record) {
        return o(Obj::new());
    }
    let JV::O(_, m) = record else {
        return o(Obj::new());
    };
    let mut changed = false;
    let mut next = Obj::new();
    for (k, v) in m.iter() {
        let key = su(k.encode_utf16().collect());
        if !next_ids.iter().any(|id| same_value(id, &key)) {
            changed = true;
            continue;
        }
        next.insert(k.clone(), v.clone());
    }
    if changed { o(next) } else { record.clone() }
}
fn resolve_shell_entry(previous: &JV, next: &JV) -> JV {
    if !matches!(previous, JV::U) && thread_shells_eq(previous, next) {
        previous.clone()
    } else {
        next.clone()
    }
}
fn resolve_session_entry(previous: &JV, next: &JV) -> JV {
    if matches!(next, JV::N) {
        return JV::U;
    }
    if !is_nullish(previous) && thread_sessions_eq(previous, next) {
        previous.clone()
    } else {
        next.clone()
    }
}
fn resolve_turn_state_entry(previous: &JV, next: &JV) -> JV {
    if !matches!(previous, JV::U) && thread_turn_states_eq(previous, next) {
        previous.clone()
    } else {
        next.clone()
    }
}
/// `writeThreadShellProjection`.
pub fn write_thread_shell_projection(state: &JV, next_thread: &(JV, JV, JV)) -> R<JV> {
    let (next_shell, next_session, next_turn_state) = next_thread;
    let thread_id = g(next_shell, "id");
    let mut next_state = ensure_thread_registered(state, &thread_id);
    let shells = g(&next_state, "threadShellById");
    let prev_shell = gk(&shells, &thread_id);
    let shell = resolve_shell_entry(&prev_shell, next_shell);
    if !eq(&shell, &prev_shell) {
        let JV::O(_, m) = &shells else {
            return Err("internal: threadShellById not object".into());
        };
        let mut mm = m.as_ref().clone();
        mm.insert(pkey(&thread_id), shell);
        next_state = state_set(&next_state, "threadShellById", o(mm));
    }
    let sessions = g(&next_state, "threadSessionById");
    let prev_session = gk(&sessions, &thread_id);
    let session = resolve_session_entry(&prev_session, next_session);
    if !eq(&session, &prev_session) {
        let mut mm = match &sessions {
            JV::O(_, m) => m.as_ref().clone(),
            _ => Obj::new(),
        };
        if matches!(session, JV::U) {
            mm.remove(&pkey(&thread_id));
        } else {
            mm.insert(pkey(&thread_id), session);
        }
        next_state = state_set(&next_state, "threadSessionById", o(mm));
    }
    let turn_states = g(&next_state, "threadTurnStateById");
    let prev_turn = gk(&turn_states, &thread_id);
    let turn_state = resolve_turn_state_entry(&prev_turn, next_turn_state);
    if !eq(&turn_state, &prev_turn) {
        let mut mm = match &turn_states {
            JV::O(_, m) => m.as_ref().clone(),
            _ => Obj::new(),
        };
        mm.insert(pkey(&thread_id), turn_state);
        next_state = state_set(&next_state, "threadTurnStateById", o(mm));
    }
    Ok(next_state)
}
/// `rebuildThreadShellRecords` -> (shellById, sessionById, turnStateById).
fn rebuild_thread_shell_records(
    state: &JV,
    snapshot_threads: &[JV],
    snapshot_sequence: &JV,
) -> R<(JV, JV, JV)> {
    let prev_shells = g(state, "threadShellById");
    let prev_sessions = g(state, "threadSessionById");
    let prev_turns = g(state, "threadTurnStateById");
    let mut shells = Obj::new();
    let mut sessions = Obj::new();
    let mut turns = Obj::new();
    for thread in snapshot_threads {
        let tid = g(thread, "id");
        let previous_thread = get_thread_from_state(state, &tid)?;
        let seq = if !is_nullish(&g(thread, "claudeCacheReview"))
            || !matches!(g(&previous_thread, "claudeCacheReviewSequence"), JV::U)
            || !is_nullish(&g(thread, "snoozedUntil"))
            || !is_nullish(&g(thread, "snoozeReminderAt"))
            || !matches!(g(&previous_thread, "snoozeSequence"), JV::U)
        {
            snapshot_sequence.clone()
        } else {
            JV::U
        };
        let (shell, session, turn_state) =
            normalize_thread_shell_snapshot(thread, &previous_thread, &seq, true)?;
        let thread_id = g(&shell, "id");
        shells.insert(
            pkey(&thread_id),
            resolve_shell_entry(&gk(&prev_shells, &thread_id), &shell),
        );
        let sess = resolve_session_entry(&gk(&prev_sessions, &thread_id), &session);
        if !matches!(sess, JV::U) {
            sessions.insert(pkey(&thread_id), sess);
        }
        turns.insert(
            pkey(&thread_id),
            resolve_turn_state_entry(&gk(&prev_turns, &thread_id), &turn_state),
        );
    }
    let shells_jv = if records_shallow_eq(&prev_shells, &shells) {
        prev_shells.clone()
    } else {
        o(shells)
    };
    let sessions_jv = if records_shallow_eq(&prev_sessions, &sessions) {
        prev_sessions.clone()
    } else {
        o(sessions)
    };
    let turns_jv = if records_shallow_eq(&prev_turns, &turns) {
        prev_turns.clone()
    } else {
        o(turns)
    };
    Ok((shells_jv, sessions_jv, turns_jv))
}
fn write_thread_detail_sync_state(state: &JV, thread_id: &JV, sync_state: &JV) -> JV {
    let cur = gk(&g(state, "threadDetailSyncById"), thread_id);
    if eq(&cur, sync_state) {
        return state.clone();
    }
    let mut m = match &g(state, "threadDetailSyncById") {
        JV::O(_, mm) => mm.as_ref().clone(),
        _ => Obj::new(),
    };
    m.insert(pkey(thread_id), sync_state.clone());
    state_set(state, "threadDetailSyncById", o(m))
}
fn obj_remove(record: &JV, key: &JV) -> Obj {
    let mut m = match record {
        JV::O(_, mm) => mm.as_ref().clone(),
        _ => Obj::new(),
    };
    m.remove(&pkey(key));
    m
}
fn clear_thread_detail_sync_state(state: &JV, thread_id: &JV) -> JV {
    let sync = g(state, "threadDetailSyncById");
    let rev = g(state, "threadDetailRevisionSequenceById");
    let has_sync = has_key(&sync, thread_id);
    let has_rev = has_key(&rev, thread_id);
    if !has_sync && !has_rev {
        return state.clone();
    }
    let syncs = o(obj_remove(&sync, thread_id));
    let revs = o(obj_remove(&rev, thread_id));
    let next = state_set(state, "threadDetailSyncById", syncs);
    state_set(&next, "threadDetailRevisionSequenceById", revs)
}
pub fn mark_thread_detail_sync_failed(state: &JV, thread_id: &JV) -> R<JV> {
    // `state.threadDetailSyncById?.[id]` — errors on nullish state only via `state.` read.
    let _ = gv(state, "threadDetailSyncById", "state.threadDetailSyncById")?;
    Ok(write_thread_detail_sync_state(
        state,
        thread_id,
        &s("failed"),
    ))
}
pub fn clear_thread_detail_sync_failure(state: &JV, thread_id: &JV) -> R<JV> {
    let sync_rec = gv(state, "threadDetailSyncById", "state.threadDetailSyncById")?;
    if !streq(&gk(&sync_rec, thread_id), "failed") {
        return Ok(state.clone());
    }
    Ok(clear_thread_detail_sync_state(state, thread_id))
}
/// `writeThreadState`.
fn write_thread_state(state: &JV, next_thread: &JV, previous_thread: &JV) -> R<JV> {
    let thread_id = g(next_thread, "id");
    let next_shell = to_thread_shell(next_thread);
    let next_turn_state = to_thread_turn_state(next_thread);
    let prev_shell = gk(&g(state, "threadShellById"), &thread_id);
    let prev_turn_state = gk(&g(state, "threadTurnStateById"), &thread_id);
    let mut next_state = ensure_thread_registered(state, &thread_id);
    if !thread_shells_eq(&prev_shell, &next_shell) {
        let mut m = match &g(&next_state, "threadShellById") {
            JV::O(_, mm) => mm.as_ref().clone(),
            _ => Obj::new(),
        };
        m.insert(pkey(&thread_id), next_shell);
        next_state = state_set(&next_state, "threadShellById", o(m));
    }
    let prev_session = or(&g(previous_thread, "session"), JV::N);
    if !thread_sessions_eq(&prev_session, &g(next_thread, "session")) {
        let mut m = match &g(&next_state, "threadSessionById") {
            JV::O(_, mm) => mm.as_ref().clone(),
            _ => Obj::new(),
        };
        m.insert(pkey(&thread_id), g(next_thread, "session"));
        next_state = state_set(&next_state, "threadSessionById", o(m));
    }
    if !thread_turn_states_eq(&prev_turn_state, &next_turn_state) {
        let mut m = match &g(&next_state, "threadTurnStateById") {
            JV::O(_, mm) => mm.as_ref().clone(),
            _ => Obj::new(),
        };
        m.insert(pkey(&thread_id), next_turn_state);
        next_state = state_set(&next_state, "threadTurnStateById", o(m));
    }
    // detail slices
    macro_rules! slice_write {
        ($items:expr, $prev_items:expr, $ids_key:literal, $by_key:literal, $id_of:expr, $transform:expr) => {{
            let prev_items_jv: &JV = $prev_items;
            if !eq(prev_items_jv, $items) {
                let prev_ids = gk(&g(&next_state, $ids_key), &thread_id);
                let prev_by_id = gk(&g(&next_state, $by_key), &thread_id);
                let items2: JV = $transform($items.clone())?;
                let prev_items_vec: Vec<JV> = units_arr(prev_items_jv);
                let (ids, by_id) = build_normalized_slice(
                    &units_arr(&items2),
                    $id_of,
                    if matches!(prev_items_jv, JV::U) {
                        None
                    } else {
                        Some(&prev_items_vec)
                    },
                    &prev_ids,
                    &prev_by_id,
                );
                if !eq(&ids, &prev_ids) {
                    let mut m = match &g(&next_state, $ids_key) {
                        JV::O(_, mm) => mm.as_ref().clone(),
                        _ => Obj::new(),
                    };
                    m.insert(pkey(&thread_id), ids);
                    next_state = state_set(&next_state, $ids_key, o(m));
                }
                if !eq(&by_id, &prev_by_id) {
                    let mut m = match &g(&next_state, $by_key) {
                        JV::O(_, mm) => mm.as_ref().clone(),
                        _ => Obj::new(),
                    };
                    m.insert(pkey(&thread_id), by_id);
                    next_state = state_set(&next_state, $by_key, o(m));
                }
            }
        }};
    }
    slice_write!(
        &g(next_thread, "messages"),
        &g(previous_thread, "messages"),
        "messageIdsByThreadId",
        "messageByThreadId",
        message_id as fn(&JV) -> JV,
        |x: JV| -> R<JV> { Ok(x) }
    );
    slice_write!(
        &g(next_thread, "activities"),
        &g(previous_thread, "activities"),
        "activityIdsByThreadId",
        "activityByThreadId",
        activity_id as fn(&JV) -> JV,
        |x: JV| -> R<JV> {
            let prev_by_id = gk(&g(&next_state, "activityByThreadId"), &thread_id);
            let prev_by_obj = match &prev_by_id {
                JV::O(_, mm) => Some(mm.as_ref().clone()),
                _ => None,
            };
            let prev_items = units_arr(&g(previous_thread, "activities"));
            let deduped = dedupe_activities_after_append(
                &units_arr(&x),
                if matches!(g(previous_thread, "activities"), JV::A(..)) {
                    Some(&prev_items)
                } else {
                    None
                },
                prev_by_obj.as_ref(),
            );
            Ok(a(cap_thread_activities(&deduped)?))
        }
    );
    slice_write!(
        &g(next_thread, "proposedPlans"),
        &g(previous_thread, "proposedPlans"),
        "proposedPlanIdsByThreadId",
        "proposedPlanByThreadId",
        proposed_plan_id as fn(&JV) -> JV,
        |x: JV| -> R<JV> { Ok(x) }
    );
    slice_write!(
        &g(next_thread, "turnDiffSummaries"),
        &g(previous_thread, "turnDiffSummaries"),
        "turnDiffIdsByThreadId",
        "turnDiffSummaryByThreadId",
        turn_diff_id as fn(&JV) -> JV,
        |x: JV| -> R<JV> { Ok(x) }
    );
    Ok(next_state)
}
/// `removeThreadState`.
fn remove_thread_state(state: &JV, thread_id: &JV) -> R<JV> {
    let shells = gv(state, "threadShellById", "state.threadShellById")?;
    let sessions = gv(state, "threadSessionById", "state.threadSessionById")?;
    let turns = gv(state, "threadTurnStateById", "state.threadTurnStateById")?;
    let msg_ids = gv(state, "messageIdsByThreadId", "state.messageIdsByThreadId")?;
    let msg_by = gv(state, "messageByThreadId", "state.messageByThreadId")?;
    let act_ids = gv(
        state,
        "activityIdsByThreadId",
        "state.activityIdsByThreadId",
    )?;
    let act_by = gv(state, "activityByThreadId", "state.activityByThreadId")?;
    let plan_ids = gv(
        state,
        "proposedPlanIdsByThreadId",
        "state.proposedPlanIdsByThreadId",
    )?;
    let plan_by = gv(
        state,
        "proposedPlanByThreadId",
        "state.proposedPlanByThreadId",
    )?;
    let diff_ids = gv(
        state,
        "turnDiffIdsByThreadId",
        "state.turnDiffIdsByThreadId",
    )?;
    let diff_by = gv(
        state,
        "turnDiffSummaryByThreadId",
        "state.turnDiffSummaryByThreadId",
    )?;
    let sidebar = gv(
        state,
        "sidebarThreadSummaryById",
        "state.sidebarThreadSummaryById",
    )?;
    if is_nullish(&sidebar) {
        return Err("Cannot destructure null or undefined value".into());
    }
    let next_ids: Vec<JV> = units_arr(&g(state, "threadIds"))
        .into_iter()
        .filter(|id| !eq(id, thread_id))
        .collect();
    // `nextThreadIds === state.threadIds` can never hold: `filter` always returns
    // a fresh array, so the source always rebuilds the records.
    let mut next = state_set(state, "threadIds", a(next_ids));
    for (k, rec) in [
        ("threadShellById", &shells),
        ("threadSessionById", &sessions),
        ("threadTurnStateById", &turns),
        ("messageIdsByThreadId", &msg_ids),
        ("messageByThreadId", &msg_by),
        ("activityIdsByThreadId", &act_ids),
        ("activityByThreadId", &act_by),
        ("proposedPlanIdsByThreadId", &plan_ids),
        ("proposedPlanByThreadId", &plan_by),
        ("turnDiffIdsByThreadId", &diff_ids),
        ("turnDiffSummaryByThreadId", &diff_by),
        ("sidebarThreadSummaryById", &sidebar),
    ] {
        next = state_set(&next, k, o(obj_remove(rec, thread_id)));
    }
    Ok(clear_thread_detail_sync_state(&next, thread_id))
}
/// `evictThreadDetailFromClientState`.
pub fn evict_thread_detail(state: &JV, thread_id: &JV) -> R<JV> {
    let detail_keys = [
        "messageIdsByThreadId",
        "messageByThreadId",
        "activityIdsByThreadId",
        "activityByThreadId",
        "proposedPlanIdsByThreadId",
        "proposedPlanByThreadId",
        "turnDiffIdsByThreadId",
        "turnDiffSummaryByThreadId",
    ];
    let mut has_detail = false;
    for k in detail_keys {
        let rec = gv(state, k, &format!("state.{k}"))?;
        if !matches!(rec, JV::U) && has_key(&rec, thread_id) {
            has_detail = true;
        }
    }
    if !has_detail {
        return Ok(clear_thread_detail_sync_state(state, thread_id));
    }
    let mut next = state.clone();
    for k in detail_keys {
        next = state_set(&next, k, o(obj_remove(&g(state, k), thread_id)));
    }
    let next = clear_thread_detail_sync_state(&next, thread_id);
    commit_thread_projection(&next, thread_id, true)
}
fn next_tombstone_sequence(tombstones: &JV, id: &JV, state: &JV, deleted_at: &JV) -> R<JV> {
    let candidate = if !matches!(deleted_at, JV::U) {
        deleted_at.clone()
    } else {
        n(to_num(&or(
            &gv(
                state,
                "shellSnapshotSequence",
                "state.shellSnapshotSequence",
            )?,
            n(0.0),
        )) + 1.0)
    };
    let existing = gk(tombstones, id);
    Ok(if matches!(existing, JV::U) {
        candidate
    } else {
        n(js_max(to_num(&existing), to_num(&candidate)))
    })
}
fn retire_deletion_tombstones(tombstones: &JV, snapshot_sequence: f64, present: &[JV]) -> JV {
    if matches!(tombstones, JV::U) {
        return tombstones.clone();
    }
    let JV::O(_, m) = tombstones else {
        return tombstones.clone();
    };
    let mut retired_any = false;
    let mut retained = Obj::new();
    for (id, seq) in m.iter() {
        let id_jv = su(id.encode_utf16().collect());
        if snapshot_sequence >= to_num(seq) && !present.iter().any(|p| same_value(p, &id_jv)) {
            retired_any = true;
            continue;
        }
        retained.insert(id.clone(), seq.clone());
    }
    if retired_any {
        o(retained)
    } else {
        tombstones.clone()
    }
}
fn is_stale_snapshot(state: &JV, seq: &JV) -> R<bool> {
    Ok(js_lt(
        seq,
        &or(
            &gv(
                state,
                "shellSnapshotSequence",
                "state.shellSnapshotSequence",
            )?,
            n(0.0),
        ),
    ))
}
fn retire_confirmed_deletion_tombstones(
    state: &JV,
    snapshot_sequence: &JV,
    present_thread_ids: &[JV],
    present_project_ids: &[JV],
) -> R<JV> {
    if is_stale_snapshot(state, snapshot_sequence)? {
        return Ok(state.clone());
    }
    let threads = retire_deletion_tombstones(
        &g(state, "deletedThreadIdsById"),
        to_num(snapshot_sequence),
        present_thread_ids,
    );
    let projects = retire_deletion_tombstones(
        &g(state, "deletedProjectIdsById"),
        to_num(snapshot_sequence),
        present_project_ids,
    );
    if eq(&threads, &g(state, "deletedThreadIdsById"))
        && eq(&projects, &g(state, "deletedProjectIdsById"))
    {
        return Ok(state.clone());
    }
    let mut m = spread_map(state);
    if !matches!(threads, JV::U) {
        mset(&mut m, "deletedThreadIdsById", threads);
    }
    if !matches!(projects, JV::U) {
        mset(&mut m, "deletedProjectIdsById", projects);
    }
    Ok(o(m))
}
/// `removeDeletedThreadFromClientState`.
pub fn remove_deleted_thread(state: &JV, thread_id: &JV, deleted_at: &JV) -> R<JV> {
    let tombstones = gv(state, "deletedThreadIdsById", "state.deletedThreadIdsById")?;
    let _seq_state = gv(
        state,
        "shellSnapshotSequence",
        "state.shellSnapshotSequence",
    )?;
    let _ = &_seq_state;
    let sequence = next_tombstone_sequence(&tombstones, thread_id, state, deleted_at)?;
    let deleted = if eq(&gk(&tombstones, thread_id), &sequence) {
        tombstones.clone()
    } else {
        let mut m = match &tombstones {
            JV::O(_, mm) => mm.as_ref().clone(),
            _ => Obj::new(),
        };
        m.insert(pkey(thread_id), sequence);
        o(m)
    };
    let next_state = remove_thread_state(state, thread_id)?;
    if eq(&g(&next_state, "deletedThreadIdsById"), &deleted) {
        Ok(next_state)
    } else {
        Ok(state_set(&next_state, "deletedThreadIdsById", deleted))
    }
}
fn remove_project_state(state: &JV, project_id: &JV) -> R<JV> {
    let mut thread_ids: Vec<JV> = Vec::new();
    let shells_jv = gv(state, "threadShellById", "state.threadShellById")?;
    if let JV::O(_, shells) = &shells_jv {
        for (_, shell) in shells.iter() {
            if eq(&g(shell, "projectId"), project_id) {
                thread_ids.push(g(shell, "id"));
            }
        }
    }
    let projects = gv(state, "projects", "state.projects")?;
    let project_items = arr_recv(
        &projects,
        "state.projects",
        "find",
        "state.projects.find(...)",
    )?;
    let removed = project_items
        .iter()
        .find(|p| eq(&g(p, "id"), project_id))
        .cloned();
    if let Some(rp) = &removed {
        forget_project_state(&g(rp, "cwd"))?;
    }
    let next_projects = if project_items.iter().any(|p| eq(&g(p, "id"), project_id)) {
        a(project_items
            .iter()
            .filter(|p| !eq(&g(p, "id"), project_id))
            .cloned()
            .collect())
    } else {
        projects.clone()
    };
    let mut next_state = state.clone();
    for tid in &thread_ids {
        next_state = remove_thread_state(&next_state, tid)?;
    }
    if eq(&next_projects, &projects) && eq(&next_state, state) {
        return Ok(state.clone());
    }
    if eq(&next_projects, &g(&next_state, "projects")) {
        Ok(next_state)
    } else {
        Ok(state_set(&next_state, "projects", next_projects))
    }
}
/// `removeDeletedProjectFromClientState`.
pub fn remove_deleted_project(state: &JV, project_id: &JV, deleted_at: &JV) -> R<JV> {
    let tombstones = gv(
        state,
        "deletedProjectIdsById",
        "state.deletedProjectIdsById",
    )?;
    let sequence = next_tombstone_sequence(&tombstones, project_id, state, deleted_at)?;
    let deleted = if eq(&gk(&tombstones, project_id), &sequence) {
        tombstones.clone()
    } else {
        let mut m = match &tombstones {
            JV::O(_, mm) => mm.as_ref().clone(),
            _ => Obj::new(),
        };
        m.insert(pkey(project_id), sequence);
        o(m)
    };
    let next_state = remove_project_state(state, project_id)?;
    if eq(&g(&next_state, "deletedProjectIdsById"), &deleted) {
        Ok(next_state)
    } else {
        Ok(state_set(&next_state, "deletedProjectIdsById", deleted))
    }
}
/// `sidebarThreadSummariesEqual`.
fn sidebar_thread_summaries_eq(left: &JV, right: &JV) -> bool {
    if matches!(left, JV::U) {
        return false;
    }
    for k in [
        "id",
        "projectId",
        "title",
        "modelSelection",
        "interactionMode",
        "envMode",
        "branch",
        "worktreePath",
        "session",
        "createdAt",
        "updatedAt",
        "latestTurn",
        "lastVisitedAt",
        "latestUserMessageAt",
        "latestHumanMessageAt",
        "hasPendingApprovals",
        "hasPendingUserInput",
        "hasActionableProposedPlan",
        "hasLiveTailWork",
        "pendingBackgroundWorkCount",
        "latestReplyPreview",
    ] {
        if !eq(&g(left, k), &g(right, k)) {
            return false;
        }
    }
    for k in [
        "workingDirectory",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
        "archivedAt",
        "settledAt",
        "snoozedUntil",
        "snoozeReminderAt",
        "parentThreadId",
        "creationSource",
        "sourceThreadId",
        "subagentAgentId",
        "subagentNickname",
        "subagentRole",
        "forkSourceThreadId",
        "sidechatSourceThreadId",
        "sidechatLastActivityAt",
        "sidechatExpiredAt",
        "handoff",
    ] {
        if !field_eq_or(left, right, k, &JV::N) {
            return false;
        }
    }
    if !field_eq_or(left, right, "isPinned", &b(false)) {
        return false;
    }
    if !eq(
        &gv_or_null(&g(left, "sidechatContext"), "url"),
        &gv_or_null(&g(right, "sidechatContext"), "url"),
    ) {
        return false;
    }
    deep_equal_json(
        &gv_or_null(left, "lastKnownPr"),
        &gv_or_null(right, "lastKnownPr"),
    ) && deep_equal_json(
        &or(&g(left, "pullRequests"), a(vec![])),
        &or(&g(right, "pullRequests"), a(vec![])),
    )
}
/// `resolveThreadSidebarMetadata` -> (latestUserMessageAt, latestHumanMessageAt, hasApprovals,
/// hasUserInput, hasPlan, hasLiveTailWork, pendingBackgroundCount).
fn resolve_thread_sidebar_metadata(thread: &JV) -> R<(JV, JV, JV, JV, JV, JV, JV)> {
    let needs = matches!(g(thread, "latestUserMessageAt"), JV::U)
        || matches!(g(thread, "latestHumanMessageAt"), JV::U)
        || matches!(g(thread, "hasPendingApprovals"), JV::U)
        || matches!(g(thread, "hasPendingUserInput"), JV::U)
        || matches!(g(thread, "hasActionableProposedPlan"), JV::U);
    let derived = if needs {
        Some(derive_thread_summary_metadata(
            &g(thread, "messages"),
            &units_arr(&g(thread, "activities")),
            &g(thread, "proposedPlans"),
            &g(thread, "latestTurn"),
        )?)
    } else {
        None
    };
    let latest_user = or(
        &g(thread, "latestUserMessageAt"),
        or(
            &g(&derived.clone().unwrap_or(JV::U), "latestUserMessageAt"),
            JV::N,
        ),
    );
    let latest_human = if !matches!(g(thread, "latestHumanMessageAt"), JV::U) {
        g(thread, "latestHumanMessageAt")
    } else {
        or(
            &g(&derived.clone().unwrap_or(JV::U), "latestHumanMessageAt"),
            JV::N,
        )
    };
    let approvals = or(
        &g(thread, "hasPendingApprovals"),
        or(
            &g(&derived.clone().unwrap_or(JV::U), "hasPendingApprovals"),
            b(false),
        ),
    );
    let user_input = or(
        &g(thread, "hasPendingUserInput"),
        or(
            &g(&derived.clone().unwrap_or(JV::U), "hasPendingUserInput"),
            b(false),
        ),
    );
    let plan = or(
        &g(thread, "hasActionableProposedPlan"),
        or(
            &g(
                &derived.clone().unwrap_or(JV::U),
                "hasActionableProposedPlan",
            ),
            b(false),
        ),
    );
    let live_tail = b(has_live_turn_tail_work(
        &g(thread, "latestTurn"),
        &g(thread, "messages"),
        &units_arr(&g(thread, "activities")),
        &g(thread, "session"),
    )?);
    let pending_bg = or(
        &g(thread, "pendingBackgroundTaskCount"),
        n(js_max(
            derive_pending_background_work(
                &units_arr(&g(thread, "activities")),
                &g(thread, "latestTurn"),
                &g(thread, "session"),
            )?
            .unwrap_or(0.0),
            count_outstanding_background_work(&g(thread, "activities"), &g(thread, "session"))?,
        )),
    );
    Ok((
        latest_user,
        latest_human,
        approvals,
        user_input,
        plan,
        live_tail,
        pending_bg,
    ))
}
/// `buildSidebarThreadSummary`.
fn build_sidebar_thread_summary(thread: &JV, previous: &JV) -> R<JV> {
    let md = resolve_thread_sidebar_metadata(thread)?;
    let mut m = Obj::new();
    let copy = |m: &mut Obj, k: &str| mset(m, k, g(thread, k));
    for k in [
        "id",
        "projectId",
        "title",
        "modelSelection",
        "interactionMode",
        "envMode",
        "branch",
        "worktreePath",
        "session",
        "createdAt",
        "updatedAt",
        "latestTurn",
        "lastVisitedAt",
    ] {
        copy(&mut m, k);
    }
    for k in [
        "workingDirectory",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
        "archivedAt",
        "settledAt",
        "snoozedUntil",
        "snoozeReminderAt",
        "parentThreadId",
        "creationSource",
        "sourceThreadId",
        "subagentAgentId",
        "subagentNickname",
        "subagentRole",
        "forkSourceThreadId",
        "sidechatSourceThreadId",
        "sidechatContext",
        "sidechatLastActivityAt",
        "sidechatExpiredAt",
        "lastKnownPr",
        "handoff",
    ] {
        mset(&mut m, k, gv_or_null(thread, k));
    }
    mset(&mut m, "isPinned", or(&g(thread, "isPinned"), b(false)));
    mset(&mut m, "latestUserMessageAt", md.0.clone());
    mset(&mut m, "latestHumanMessageAt", md.1.clone());
    mset(&mut m, "hasPendingApprovals", md.2.clone());
    mset(&mut m, "hasPendingUserInput", md.3.clone());
    mset(&mut m, "hasActionableProposedPlan", md.4.clone());
    mset(&mut m, "hasLiveTailWork", md.5.clone());
    mset(&mut m, "pendingBackgroundWorkCount", md.6.clone());
    mset(
        &mut m,
        "pullRequests",
        or(&g(thread, "pullRequests"), a(vec![])),
    );
    if !matches!(g(thread, "latestReplyPreview"), JV::U) {
        mset(
            &mut m,
            "latestReplyPreview",
            g(thread, "latestReplyPreview"),
        );
    }
    let next = o(m);
    if !matches!(previous, JV::U) && sidebar_thread_summaries_eq(previous, &next) {
        return Ok(previous.clone());
    }
    Ok(next)
}
/// `commitThreadProjection`.
fn commit_thread_projection(state: &JV, thread_id: &JV, update_summary: bool) -> R<JV> {
    let previous_summary = gk(&g(state, "sidebarThreadSummaryById"), thread_id);
    if !update_summary && !matches!(previous_summary, JV::U) {
        return Ok(state.clone());
    }
    let next_thread = get_thread_from_state(state, thread_id)?;
    if matches!(next_thread, JV::U) {
        return Ok(state.clone());
    }
    let next_summary = build_sidebar_thread_summary(&next_thread, &previous_summary)?;
    if eq(&next_summary, &previous_summary) {
        return Ok(state.clone());
    }
    let mut m = match &g(state, "sidebarThreadSummaryById") {
        JV::O(_, mm) => mm.as_ref().clone(),
        _ => Obj::new(),
    };
    m.insert(pkey(thread_id), next_summary);
    Ok(state_set(state, "sidebarThreadSummaryById", o(m)))
}

// ---------- exports ----------

/// `upsertProject`.
pub fn upsert_project_jv(state: &JV, incoming: &JV, match_policy: &JV) -> R<JV> {
    let deleted = gv(
        state,
        "deletedProjectIdsById",
        "state.deletedProjectIdsById",
    )?;
    let incoming_id = if is_nullish(&deleted) {
        JV::U
    } else {
        gv(incoming, "id", "incoming.id")?
    };
    if !matches!(gk(&deleted, &incoming_id), JV::U) {
        return Ok(state.clone());
    }
    let projects = gv(state, "projects", "state.projects")?;
    let project_items = arr_recv(
        &projects,
        "state.projects",
        "find",
        "state.projects.find(...)",
    )?;
    let mut existing = JV::U;
    for p in project_items {
        if eq(&g(p, "id"), &gv(incoming, "id", "incoming.id")?) {
            existing = p.clone();
            break;
        }
    }
    let existing = if matches!(existing, JV::U) && streq(match_policy, "id-or-cwd") {
        let incoming_key =
            project_cwd_key(&gv(incoming, "workspaceRoot", "incoming.workspaceRoot")?)?;
        project_items
            .iter()
            .find(|p| match project_cwd_key(&g(p, "cwd")) {
                Ok(k) => k == incoming_key,
                Err(_) => false,
            })
            .cloned()
            .unwrap_or(JV::U)
    } else {
        existing
    };
    let next_project = normalize_project(incoming, &existing)?;
    if !matches!(existing, JV::U) {
        if eq(&existing, &next_project) {
            return Ok(state.clone());
        }
        let mapped: Vec<JV> = project_items
            .iter()
            .map(|p| {
                if eq(&g(p, "id"), &g(&existing, "id")) {
                    next_project.clone()
                } else {
                    p.clone()
                }
            })
            .collect();
        return Ok(state_set(state, "projects", a(mapped)));
    }
    let mut items = project_items.to_vec();
    items.push(next_project);
    Ok(state_set(state, "projects", a(items)))
}
/// `upsertSpace`.
pub fn upsert_space_jv(state: &JV, incoming: &JV) -> R<JV> {
    let spaces = gv(state, "spaces", "state.spaces")?;
    let space_items = arr_recv(&spaces, "state.spaces", "find", "state.spaces.find(...)")?;
    let mut existing = JV::U;
    for x in space_items {
        if eq(&g(x, "id"), &gv(incoming, "id", "incoming.id")?) {
            existing = x.clone();
            break;
        }
    }
    let next_space = normalize_space(incoming, &existing)?;
    if eq(&existing, &next_space) {
        return Ok(state.clone());
    }
    let mut next: Vec<JV>;
    if matches!(existing, JV::U) {
        next = space_items.to_vec();
        next.push(next_space);
    } else {
        next = space_items
            .iter()
            .map(|x| {
                if eq(&g(x, "id"), &g(incoming, "id")) {
                    next_space.clone()
                } else {
                    x.clone()
                }
            })
            .collect();
    }
    let mut err = None;
    js_sort(&mut next, |l, r| match space_cmp_pub(l, r) {
        Ok(v) => v,
        Err(e) => {
            if err.is_none() {
                err = Some(e);
            }
            f64::NAN
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(state_set(state, "spaces", a(next)))
}
fn space_cmp_pub(l: &JV, r: &JV) -> R<f64> {
    let d = to_num(&g(l, "sortOrder")) - to_num(&g(r, "sortOrder"));
    if d != 0.0 && !d.is_nan() {
        return Ok(d);
    }
    locale_compare(
        &g(l, "id"),
        &g(r, "id"),
        "left.id",
        "left.id.localeCompare(right.id)",
    )
}
/// `removeSpace`.
pub fn remove_space_jv(state: &JV, space_id: &JV, assignment_updated_at: &JV) -> R<JV> {
    let spaces = gv(state, "spaces", "state.spaces")?;
    let space_items = arr_recv(
        &spaces,
        "state.spaces",
        "filter",
        "state.spaces.filter((space) => ...)",
    )?;
    let next_spaces: Vec<JV> = space_items
        .iter()
        .filter(|x| !eq(&g(x, "id"), space_id))
        .cloned()
        .collect();
    let projects = gv(state, "projects", "state.projects")?;
    let project_items = arr_recv(
        &projects,
        "state.projects",
        "map",
        "state.projects.map(...)",
    )?;
    let mut changed = false;
    let next_projects: Vec<JV> = project_items
        .iter()
        .map(|p| {
            if !eq(&gv_or_null(p, "spaceId"), space_id) {
                return p.clone();
            }
            changed = true;
            let mut m = spread_map(p);
            mset(&mut m, "spaceId", JV::N);
            if !matches!(assignment_updated_at, JV::U) {
                let upd = g(p, "updatedAt");
                mset(
                    &mut m,
                    "updatedAt",
                    if truthy(&upd) && js_gt(&upd, assignment_updated_at) {
                        upd
                    } else {
                        assignment_updated_at.clone()
                    },
                );
            }
            o(m)
        })
        .collect();
    if next_spaces.len() == space_items.len() && !changed {
        return Ok(state.clone());
    }
    let mut m = spread_map(state);
    mset(&mut m, "spaces", a(next_spaces));
    mset(
        &mut m,
        "projects",
        if changed { a(next_projects) } else { projects },
    );
    Ok(o(m))
}
/// `applySpaceOrder`.
pub fn apply_space_order_jv(state: &JV, ordered_space_ids: &JV, updated_at: &JV) -> R<JV> {
    let ids = arr_recv(
        ordered_space_ids,
        "orderedSpaceIds",
        "map",
        "orderedSpaceIds.map((spaceId, index) => [spaceId, index])",
    )?;
    let order_by_id: Vec<(JV, f64)> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), i as f64))
        .collect();
    let spaces = gv(state, "spaces", "state.spaces")?;
    let space_items = arr_recv(&spaces, "state.spaces", "map", "state.spaces.map(...)")?;
    let mut next: Vec<JV> = space_items
        .iter()
        .map(|space| {
            let sid = g(space, "id");
            match order_by_id.iter().rfind(|(id, _)| same_value(id, &sid)) {
                None => space.clone(),
                Some((_, idx)) if eq(&n(*idx), &g(space, "sortOrder")) => space.clone(),
                Some((_, idx)) => {
                    let mut m = spread_map(space);
                    mset(&mut m, "sortOrder", n(*idx));
                    if !matches!(updated_at, JV::U) {
                        mset(&mut m, "updatedAt", updated_at.clone());
                    }
                    o(m)
                }
            }
        })
        .collect();
    let mut err = None;
    js_sort(&mut next, |l, r| match space_cmp_pub(l, r) {
        Ok(v) => v,
        Err(e) => {
            if err.is_none() {
                err = Some(e);
            }
            f64::NAN
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    if space_items.len() == next.len() && space_items.iter().zip(&next).all(|(x, y)| eq(x, y)) {
        return Ok(state.clone());
    }
    Ok(state_set(state, "spaces", a(next)))
}
/// `syncServerShellSnapshot`.
pub fn sync_server_shell_snapshot_jv(state: &JV, snapshot: &JV) -> R<JV> {
    let seq = gv(snapshot, "snapshotSequence", "snapshot.snapshotSequence")?;
    if is_stale_snapshot(state, &seq)? {
        return Ok(state.clone());
    }
    remember_project_state(&gv(state, "projects", "state.projects")?)?;
    let deleted_projects = or(&g(state, "deletedProjectIdsById"), o(Obj::new()));
    let deleted_threads = or(&g(state, "deletedThreadIdsById"), o(Obj::new()));
    let threads_v = gv(snapshot, "threads", "snapshot.threads")?;
    let snapshot_threads = arr_recv(
        &threads_v,
        "snapshot.threads",
        "filter",
        "snapshot.threads.filter(...)",
    )?
    .iter()
    .filter(|t| {
        matches!(gk(&deleted_projects, &g(t, "projectId")), JV::U)
            && matches!(gk(&deleted_threads, &g(t, "id")), JV::U)
    })
    .cloned()
    .collect::<Vec<_>>();
    let projects_v = gv(snapshot, "projects", "snapshot.projects")?;
    let snapshot_projects = arr_recv(
        &projects_v,
        "snapshot.projects",
        "filter",
        "snapshot.projects.filter(...)",
    )?
    .iter()
    .filter(|p| matches!(gk(&deleted_projects, &g(p, "id")), JV::U))
    .cloned()
    .collect::<Vec<_>>();
    let mut cwd_keys = std::collections::HashSet::new();
    for p in &snapshot_projects {
        cwd_keys.insert(project_cwd_key(&g(p, "workspaceRoot"))?);
    }
    reset_stale_remembered(&cwd_keys);
    let spaces = a(map_spaces(
        &units_arr(&or(&g(snapshot, "spaces"), a(vec![]))),
        &units_arr(&g(state, "spaces")),
    )?);
    let spaces = if arrays_shallow_eq(&g(state, "spaces"), &units_arr(&spaces)) {
        g(state, "spaces")
    } else {
        spaces
    };
    let projects_next = a(map_projects(
        &snapshot_projects,
        &units_arr(&g(state, "projects")),
    )?);
    let projects_next = if arrays_shallow_eq(&g(state, "projects"), &units_arr(&projects_next)) {
        g(state, "projects")
    } else {
        projects_next
    };
    remember_project_state(&projects_next)?;
    let next_thread_ids: Vec<JV> = snapshot_threads.iter().map(|t| g(t, "id")).collect();
    let mut normalized = state.clone();
    normalized = state_set(
        &normalized,
        "threadIds",
        reuse_thread_id_registry(&g(state, "threadIds"), &next_thread_ids),
    );
    let (shells, sessions, turns) = rebuild_thread_shell_records(state, &snapshot_threads, &seq)?;
    normalized = state_set(&normalized, "threadShellById", shells);
    normalized = state_set(&normalized, "threadSessionById", sessions);
    normalized = state_set(&normalized, "threadTurnStateById", turns);
    for k in [
        "messageIdsByThreadId",
        "messageByThreadId",
        "activityIdsByThreadId",
        "activityByThreadId",
        "proposedPlanIdsByThreadId",
        "proposedPlanByThreadId",
        "turnDiffIdsByThreadId",
        "turnDiffSummaryByThreadId",
        "threadDetailSyncById",
        "threadDetailRevisionSequenceById",
    ] {
        let retained = retain_thread_scoped_record(&g(state, k), &next_thread_ids);
        normalized = state_set(&normalized, k, retained);
    }
    let threads = get_threads_from_state(&normalized);
    let mut next_summaries = Obj::new();
    for thread in &threads {
        let summary = build_sidebar_thread_summary(
            thread,
            &gk(&g(state, "sidebarThreadSummaryById"), &g(thread, "id")),
        )?;
        next_summaries.insert(pkey(&g(thread, "id")), summary);
    }
    let sidebar = if records_shallow_eq(&g(state, "sidebarThreadSummaryById"), &next_summaries) {
        g(state, "sidebarThreadSummaryById")
    } else {
        o(next_summaries)
    };
    let mut m = spread_map(&normalized);
    mset(
        &mut m,
        "shellSnapshotSequence",
        n(js_max(
            to_num(&or(&g(state, "shellSnapshotSequence"), n(0.0))),
            to_num(&seq),
        )),
    );
    mset(&mut m, "spaces", spaces);
    mset(&mut m, "projects", projects_next);
    mset(&mut m, "sidebarThreadSummaryById", sidebar);
    mset(&mut m, "threadsHydrated", b(true));
    let present_threads: Vec<JV> = arr_recv(
        &threads_v,
        "snapshot.threads",
        "map",
        "snapshot.threads.map(...)",
    )?
    .iter()
    .map(|t| g(t, "id"))
    .collect();
    let present_projects: Vec<JV> = arr_recv(
        &projects_v,
        "snapshot.projects",
        "map",
        "snapshot.projects.map(...)",
    )?
    .iter()
    .map(|p| g(p, "id"))
    .collect();
    retire_confirmed_deletion_tombstones(&o(m), &seq, &present_threads, &present_projects)
}
fn reuse_thread_id_registry(previous: &JV, next: &[JV]) -> JV {
    if let JV::A(_, prev) = previous
        && prev.len() == next.len()
    {
        let identical = prev.iter().zip(next).all(|(a, b)| eq(a, b));
        if identical {
            return previous.clone();
        }
    }
    a(next.to_vec())
}
/// `syncServerThreadDetailWithOptions`.
fn sync_server_thread_detail_with_options(
    state: &JV,
    thread: &JV,
    update_sidebar: bool,
    snapshot_sequence: &JV,
    replace_history: bool,
) -> R<JV> {
    let _ = update_sidebar;
    let previous_thread = get_thread_from_state(state, &gv(thread, "id", "thread.id")?)?;
    let next_detail = if !replace_history {
        merge_read_model_thread_detail(thread, &previous_thread, snapshot_sequence)?
    } else {
        thread.clone()
    };
    let normalized = normalize_thread_from_read_model(
        &next_detail,
        &previous_thread,
        snapshot_sequence,
        !truthy(&g(state, "threadsHydrated")),
    )?;
    let written = write_thread_state(state, &normalized, &previous_thread)?;
    let committed = commit_thread_projection(&written, &gv(thread, "id", "thread.id")?, false)?;
    Ok(write_thread_detail_sync_state(
        &committed,
        &g(thread, "id"),
        &s("synced"),
    ))
}
/// `syncServerThreadDetail`.
pub fn sync_server_thread_detail_jv(state: &JV, thread: &JV) -> R<JV> {
    let del_p = gv(
        state,
        "deletedProjectIdsById",
        "state.deletedProjectIdsById",
    )?;
    let del_t = gv(state, "deletedThreadIdsById", "state.deletedThreadIdsById")?;
    let hit_p = if is_nullish(&del_p) {
        false
    } else {
        !matches!(
            gk(&del_p, &gv(thread, "projectId", "thread.projectId")?),
            JV::U
        )
    };
    let hit_t = if is_nullish(&del_t) {
        false
    } else {
        !matches!(gk(&del_t, &gv(thread, "id", "thread.id")?), JV::U)
    };
    if hit_p || hit_t {
        return remove_thread_state(state, &gv(thread, "id", "thread.id")?);
    }
    sync_server_thread_detail_with_options(state, thread, false, &JV::U, false)
}
/// `syncServerThreadDetailHotPath`.
pub fn sync_server_thread_detail_hot_path_jv(
    state: &JV,
    thread: &JV,
    snapshot_sequence: &JV,
    history_revision: &JV,
) -> R<JV> {
    let del_p = gv(
        state,
        "deletedProjectIdsById",
        "state.deletedProjectIdsById",
    )?;
    let del_t = gv(state, "deletedThreadIdsById", "state.deletedThreadIdsById")?;
    let hit_p = if is_nullish(&del_p) {
        false
    } else {
        !matches!(
            gk(&del_p, &gv(thread, "projectId", "thread.projectId")?),
            JV::U
        )
    };
    let hit_t = if is_nullish(&del_t) {
        false
    } else {
        !matches!(gk(&del_t, &gv(thread, "id", "thread.id")?), JV::U)
    };
    if hit_p || hit_t {
        return remove_thread_state(state, &gv(thread, "id", "thread.id")?);
    }
    let previous_revision = gk(
        &g(state, "threadDetailRevisionSequenceById"),
        &gv(thread, "id", "thread.id")?,
    );
    let replace = !matches!(previous_revision, JV::U)
        && !matches!(history_revision, JV::U)
        && !eq(history_revision, &previous_revision);
    let next =
        sync_server_thread_detail_with_options(state, thread, false, snapshot_sequence, replace)?;
    if matches!(history_revision, JV::U) || eq(history_revision, &previous_revision) {
        return Ok(next);
    }
    let mut m = match &g(&next, "threadDetailRevisionSequenceById") {
        JV::O(_, mm) => mm.as_ref().clone(),
        _ => Obj::new(),
    };
    m.insert(
        pkey(&gv(thread, "id", "thread.id")?),
        history_revision.clone(),
    );
    Ok(state_set(&next, "threadDetailRevisionSequenceById", o(m)))
}
/// `applyShellEvent`.
pub fn apply_shell_event_jv(state: &JV, event: &JV) -> R<JV> {
    let kind = gv(event, "kind", "event.kind")?;
    if streq(&kind, "space-upserted") {
        upsert_space_jv(state, &gv(event, "space", "event.space")?)
    } else if streq(&kind, "space-removed") {
        remove_space_jv(
            state,
            &gv(event, "spaceId", "event.spaceId")?,
            &gv(event, "updatedAt", "event.updatedAt")?,
        )
    } else if streq(&kind, "space-order-updated") {
        apply_space_order_jv(
            state,
            &gv(event, "orderedSpaceIds", "event.orderedSpaceIds")?,
            &JV::U,
        )
    } else if streq(&kind, "project-upserted") {
        upsert_project_jv(
            state,
            &gv(event, "project", "event.project")?,
            &s("id-or-cwd"),
        )
    } else if streq(&kind, "project-removed") {
        remove_deleted_project(
            state,
            &gv(event, "projectId", "event.projectId")?,
            &gv(event, "sequence", "event.sequence")?,
        )
    } else if streq(&kind, "thread-upserted") {
        let thread = gv(event, "thread", "event.thread")?;
        let del_p = gv(
            state,
            "deletedProjectIdsById",
            "state.deletedProjectIdsById",
        )?;
        let hit_p = if is_nullish(&del_p) {
            false
        } else {
            !matches!(
                gk(&del_p, &gv(&thread, "projectId", "event.thread.projectId")?),
                JV::U
            )
        };
        let del_t = gv(state, "deletedThreadIdsById", "state.deletedThreadIdsById")?;
        let hit_t = if is_nullish(&del_t) {
            false
        } else {
            !matches!(gk(&del_t, &gv(&thread, "id", "event.thread.id")?), JV::U)
        };
        if hit_p || hit_t {
            return remove_thread_state(state, &gv(&thread, "id", "event.thread.id")?);
        }
        let previous_thread = get_thread_from_state(state, &gv(&thread, "id", "event.thread.id")?)?;
        let normalized = normalize_thread_shell_snapshot(
            &thread,
            &previous_thread,
            &gv(event, "sequence", "event.sequence")?,
            !truthy(&g(state, "threadsHydrated")),
        )?;
        let next_state = write_thread_shell_projection(state, &normalized)?;
        commit_thread_projection(&next_state, &gv(&thread, "id", "event.thread.id")?, true)
    } else if streq(&kind, "thread-removed") {
        remove_thread_state(state, &gv(event, "threadId", "event.threadId")?)
    } else {
        Ok(JV::U)
    }
}
/// `syncServerReadModel`.
pub fn sync_server_read_model_jv(state: &JV, read_model: &JV) -> R<JV> {
    let seq = gv(read_model, "snapshotSequence", "readModel.snapshotSequence")?;
    if is_stale_snapshot(state, &seq)? {
        return Ok(state.clone());
    }
    remember_project_state(&gv(state, "projects", "state.projects")?)?;
    let deleted_projects = or(&g(state, "deletedProjectIdsById"), o(Obj::new()));
    let deleted_threads = or(&g(state, "deletedThreadIdsById"), o(Obj::new()));
    let threads_v = gv(read_model, "threads", "readModel.threads")?;
    let all_threads = arr_recv(
        &threads_v,
        "readModel.threads",
        "filter",
        "readModel.threads.filter(...)",
    )?;
    let live_present_thread_ids: Vec<JV> = all_threads
        .iter()
        .filter(|t| matches!(g(t, "deletedAt"), JV::N))
        .map(|t| g(t, "id"))
        .collect();
    let projects_v = gv(read_model, "projects", "readModel.projects")?;
    let all_projects = arr_recv(
        &projects_v,
        "readModel.projects",
        "filter",
        "readModel.projects.filter(...)",
    )?;
    let live_present_project_ids: Vec<JV> = all_projects
        .iter()
        .filter(|p| matches!(g(p, "deletedAt"), JV::N))
        .map(|p| g(p, "id"))
        .collect();
    let spaces_in = units_arr(&or(
        &gv(read_model, "spaces", "readModel.spaces")?,
        a(vec![]),
    ))
    .into_iter()
    .filter(|x| matches!(g(x, "deletedAt"), JV::N))
    .collect::<Vec<_>>();
    let spaces = a(map_spaces(&spaces_in, &units_arr(&g(state, "spaces")))?);
    let spaces = if arrays_shallow_eq(&g(state, "spaces"), &units_arr(&spaces)) {
        g(state, "spaces")
    } else {
        spaces
    };
    let live_projects: Vec<JV> = all_projects
        .iter()
        .filter(|p| {
            matches!(g(p, "deletedAt"), JV::N)
                && matches!(gk(&deleted_projects, &g(p, "id")), JV::U)
        })
        .cloned()
        .collect();
    let mut cwd_keys = std::collections::HashSet::new();
    for p in &live_projects {
        cwd_keys.insert(project_cwd_key(&g(p, "workspaceRoot"))?);
    }
    reset_stale_remembered(&cwd_keys);
    let projects_next = a(map_projects(
        &live_projects,
        &units_arr(&g(state, "projects")),
    )?);
    let projects_next = if arrays_shallow_eq(&g(state, "projects"), &units_arr(&projects_next)) {
        g(state, "projects")
    } else {
        projects_next
    };
    remember_project_state(&projects_next)?;
    let mut next_threads: Vec<JV> = Vec::new();
    for thread in all_threads {
        if !matches!(g(thread, "deletedAt"), JV::N)
            || !matches!(gk(&deleted_projects, &g(thread, "projectId")), JV::U)
            || !matches!(gk(&deleted_threads, &g(thread, "id")), JV::U)
        {
            continue;
        }
        let existing = get_thread_from_state(state, &g(thread, "id"))?;
        let seq_arg = if !is_nullish(&g(thread, "claudeCacheReview"))
            || !matches!(g(&existing, "claudeCacheReviewSequence"), JV::U)
            || !is_nullish(&g(thread, "snoozedUntil"))
            || !is_nullish(&g(thread, "snoozeReminderAt"))
            || !matches!(g(&existing, "snoozeSequence"), JV::U)
        {
            seq.clone()
        } else {
            JV::U
        };
        next_threads.push(normalize_thread_from_read_model(
            thread, &existing, &seq_arg, true,
        )?);
    }
    let next_thread_ids: Vec<JV> = next_threads.iter().map(|t| g(t, "id")).collect();
    let mut normalized = state.clone();
    normalized = state_set(
        &normalized,
        "threadIds",
        reuse_thread_id_registry(&g(state, "threadIds"), &next_thread_ids),
    );
    for k in [
        "threadShellById",
        "threadSessionById",
        "threadTurnStateById",
        "messageIdsByThreadId",
        "messageByThreadId",
        "activityIdsByThreadId",
        "activityByThreadId",
        "proposedPlanIdsByThreadId",
        "proposedPlanByThreadId",
        "turnDiffIdsByThreadId",
        "turnDiffSummaryByThreadId",
        "threadDetailSyncById",
        "threadDetailRevisionSequenceById",
    ] {
        let retained = retain_thread_scoped_record(&g(state, k), &next_thread_ids);
        normalized = state_set(&normalized, k, retained);
    }
    for thread in &next_threads {
        let prev = get_thread_from_state(state, &g(thread, "id"))?;
        normalized = write_thread_state(&normalized, thread, &prev)?;
        normalized = write_thread_detail_sync_state(&normalized, &g(thread, "id"), &s("synced"));
    }
    let threads = get_threads_from_state(&normalized);
    let mut next_summaries = Obj::new();
    for thread in &threads {
        let summary = build_sidebar_thread_summary(
            thread,
            &gk(&g(state, "sidebarThreadSummaryById"), &g(thread, "id")),
        )?;
        next_summaries.insert(pkey(&g(thread, "id")), summary);
    }
    let sidebar = if records_shallow_eq(&g(state, "sidebarThreadSummaryById"), &next_summaries) {
        g(state, "sidebarThreadSummaryById")
    } else {
        o(next_summaries)
    };
    let no_change = eq(&spaces, &g(state, "spaces"))
        && eq(&projects_next, &g(state, "projects"))
        && eq(&sidebar, &g(state, "sidebarThreadSummaryById"))
        && eq(&g(&normalized, "threadIds"), &g(state, "threadIds"))
        && eq(
            &g(&normalized, "threadShellById"),
            &g(state, "threadShellById"),
        )
        && eq(
            &g(&normalized, "threadSessionById"),
            &g(state, "threadSessionById"),
        )
        && eq(
            &g(&normalized, "threadTurnStateById"),
            &g(state, "threadTurnStateById"),
        )
        && eq(
            &g(&normalized, "messageIdsByThreadId"),
            &g(state, "messageIdsByThreadId"),
        )
        && eq(
            &g(&normalized, "messageByThreadId"),
            &g(state, "messageByThreadId"),
        )
        && eq(
            &g(&normalized, "activityIdsByThreadId"),
            &g(state, "activityIdsByThreadId"),
        )
        && eq(
            &g(&normalized, "activityByThreadId"),
            &g(state, "activityByThreadId"),
        )
        && eq(
            &g(&normalized, "proposedPlanIdsByThreadId"),
            &g(state, "proposedPlanIdsByThreadId"),
        )
        && eq(
            &g(&normalized, "proposedPlanByThreadId"),
            &g(state, "proposedPlanByThreadId"),
        )
        && eq(
            &g(&normalized, "turnDiffIdsByThreadId"),
            &g(state, "turnDiffIdsByThreadId"),
        )
        && eq(
            &g(&normalized, "turnDiffSummaryByThreadId"),
            &g(state, "turnDiffSummaryByThreadId"),
        )
        && eq(
            &g(&normalized, "threadDetailSyncById"),
            &g(state, "threadDetailSyncById"),
        )
        && eq(
            &g(&normalized, "threadDetailRevisionSequenceById"),
            &g(state, "threadDetailRevisionSequenceById"),
        )
        && truthy(&g(state, "threadsHydrated"));
    if no_change {
        let advanced = if js_gt(&seq, &or(&g(state, "shellSnapshotSequence"), n(0.0))) {
            state_set(state, "shellSnapshotSequence", seq.clone())
        } else {
            state.clone()
        };
        return retire_confirmed_deletion_tombstones(
            &advanced,
            &seq,
            &live_present_thread_ids,
            &live_present_project_ids,
        );
    }
    let mut m = spread_map(&normalized);
    mset(
        &mut m,
        "shellSnapshotSequence",
        n(js_max(
            to_num(&or(&g(state, "shellSnapshotSequence"), n(0.0))),
            to_num(&seq),
        )),
    );
    mset(&mut m, "spaces", spaces);
    mset(&mut m, "projects", projects_next);
    mset(&mut m, "sidebarThreadSummaryById", sidebar);
    mset(&mut m, "threadsHydrated", b(true));
    retire_confirmed_deletion_tombstones(
        &o(m),
        &seq,
        &live_present_thread_ids,
        &live_present_project_ids,
    )
}
/// silence unused warning for Rc import used through macros
#[allow(dead_code)]
fn _t(_: Rc<JV>) {}
