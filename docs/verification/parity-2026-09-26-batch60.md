# Parity batch 60 — Turn task lists (`turn.tasks.updated` read model + cards)

Date: 2026-09-26
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Ports upstream's per-turn task list: the ACP `session/plan` update path
(`AcpRuntimeModel.ts` case `"plan"` → `makeAcpPlanUpdatedEvent` →
`turn.tasks.updated`), the `deriveActiveTaskListState` visibility read model
(`session-logic.ts`), the composer `ActiveTaskListCard` /
`ComposerActiveTaskListCard` (`chat/ActiveTaskListCard.tsx`), the PlanSidebar
"Steps" section, and the `planSidebarDismissedForTurnRef` dismiss key.

- **`ThreadEvent::PlanChanged { entries, explanation }`** — upstream attaches
  `explanation` and `turnId` to `turn.tasks.updated`; the port records the
  explanation on the event and derives `plan_turn_id` from the latest
  `PromptStarted` turn at apply time (`thread.rs`), with `plan_created_at_ms`
  from the event envelope timestamp. The snapshot wholesale-replaces
  `thread.plan`, matching upstream keep-latest semantics
  (`collapseKey: taskList:{turnId}` in `workLog.ts`).
- **`TaskStatus` + `normalize_plan_step_status`** — upstream
  `normalizePlanStepStatus`: `"completed"` → completed,
  `"in_progress" | "inProgress"` → in-progress, else pending.
- **`Thread::active_task_list`** — upstream `deriveActiveTaskListState`: the
  latest list shows when it belongs to the latest turn (even all-completed);
  a prior-turn list shows only while some task is unfinished; an explicit
  empty snapshot hides it; `None` when no plan was emitted.
- **Composer card** — `active_task_list_card` mounted beside the
  plan-follow-up banner: hidden while `plan_sidebar_open` (upstream
  `showComposerActiveTaskListCard`), header icon + "{done} out of {total}
  tasks completed" + open-sidebar and collapse buttons
  (`active_task_list_compact`), numbered rows with status marks (check /
  accent indicator / hollow ring) and line-through on completed.
- **PlanSidebar "Steps"** — header gains the `formatTimestamp(createdAt)`
  stamp; body order is explanation → "Steps" label + rows (20px status
  circle: success-tinted check / accent-tinted indicator / ring with dot;
  in-progress rows get a 5% accent wash, completed a 5% success wash and
  line-through) → "Full Plan" collapsible; empty state only when neither a
  task list nor plan markdown exists.
- **Dismiss semantics** — `plan_sidebar_dismissed_turn` records
  `active_task_list.turn_id ?? sidebar_plan.turn_id ?? "__dismissed__"` on
  close and clears on open, mirroring `planSidebarDismissedForTurnRef`;
  reset on thread switch. Implement sends open the sidebar optimistically
  (upstream `useChatTurnFollowUps` / `useChatTurnExecution`).
- **Transcript row** — `RenderRow::Plan` now renders the upstream
  work-log summary ("{done} out of {total} tasks completed" + in-progress
  task detail) instead of the earlier card-shaped invention.

Divergences kept deliberately: upstream spins a `LoaderIcon`; the port has no
animated spinner glyph, so in-progress steps use `Glyph::Clock`. Upstream
`backgroundTaskCount` footer is omitted — the port has no background-agent
surface to count yet.

## Gates

- `cargo fmt --all -- --check` — clean.
- `cargo clippy --locked --workspace --all-targets` — 25 warnings, identical
  to the HEAD baseline (pre-existing).
- `cargo test --locked --workspace` — green including new
  `thread::tests::active_task_list_follows_upstream_visibility_rules`
  (current-turn shows all-complete; prior-turn persists only while
  unfinished; empty snapshot hides; latest-turn tagging). Only failure is
  the documented environment failure
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (gitconfig rewrite on this box).

## Live test — real app

Release binary `target/release/synara-app --data-dir /tmp/synara-e2e/data-plan2`,
`opencode` profile → `/tmp/synara-e2e/stub-agent-plan.py` (ACP stub; on
"PLEASE IMPLEMENT THIS PLAN" emits `session/plan` with
1-completed/1-in-progress/1-pending + explanation, a second update after
10 s with 2-completed/1-in-progress, then `end_turn`).

Acceptance gesture: a `session/plan` update surfaces a task list in the
transcript, composer, and sidebar — and updates live on the next snapshot.

- Transcript row: "1 out of 3 tasks completed — Invalidate cached responses
  on save", then "2 out of 3 tasks completed — Regression-test reload" after
  the second update.
- Composer card: header "2 out of 3 tasks completed" + numbered rows with
  check/ring icons; completed rows struck through.
- Card open-sidebar button → 340 px sidebar with "Plan" badge, timestamp,
  "Steps" label and per-step status circles; the card hides while the
  sidebar is open (mutual exclusivity matches upstream).
- "Hide Tasks" pill closes the sidebar (dismiss key recorded) and the card
  returns; collapse/expand toggles the numbered rows.

## Next tick candidates

- `direct_model_controls` review, followups excision → busy-queue,
  Hub→Studio rename, env-panel inline Notes, auto-mode provider gating.
- Live-verify deferred: batches 48/50/51 (ACP permissions), 49 (AppSnap),
  54, 55.
