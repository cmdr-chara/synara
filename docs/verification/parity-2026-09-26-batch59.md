# Parity batch 59 — Plan sidebar (`PlanSidebar` + `findSidebarProposedPlan`)

Date: 2026-09-26
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Ports upstream's right-side plan sidebar (`apps/web/src/components/PlanSidebar.tsx`),
its plan resolution (`session-logic.ts` `findSidebarProposedPlan`), the composer
`sidebarAction` toggle (`ChatComposerFooter.tsx`), and the
`planSidebarOpenOnNextThreadRef` auto-open on the implementation thread.

- **`SourceProposedPlan` / `ThreadEvent::ProposedPlanSource`** — upstream
  attaches `sourceProposedPlan` (`SourceProposedPlanReference`) to the
  implementing turn via `thread.turn.start` params. The port records a
  `ProposedPlanSource{source_thread, plan_id}` event on the new thread ahead
  of the prompt; the next `PromptStarted` consumes it onto
  `TurnSummary.source_proposed_plan` (pending slot mirrors
  `pending_acp_routes`). Ordering is preserved because both records are
  awaited before `controller.submit` is dispatched.
- **`find_sidebar_proposed_plan`** — upstream `findSidebarProposedPlan`: while
  the latest turn is unsettled and carries `source_proposed_plan`, resolve the
  plan from the SOURCE thread's `proposed_plans`; otherwise the active
  thread's latest unimplemented plan (`implemented_at_ms.is_none()` +
  `find_latest_proposed_plan` recency/turn preference). The shell holds only
  the active thread, so the source thread's plan is fetched lazily through
  `workspace.thread` into `sidebar_source_plan` (`maybe_load_sidebar_source`,
  driven from `ThreadLoaded` and live `Event` arms).
- **`plan_sidebar`** — upstream `PlanSidebar.tsx`: `w-[340px]` right column,
  `border-l`, header row with an accent-tinted "Plan" badge + copy/download
  (`ProposedPlanActions`) + close (`PanelRightClose` → `Glyph::PanelRight`),
  scrollable body with a collapsible "Full Plan" section (chevron →
  `ui::markdown::render` of `plan_markdown`) and the empty state
  "No active plan yet. / Plans will appear here when generated." Mounted by
  wrapping `conversation()`'s column in a flex row, so dock/zen/split
  placements all get it.
- **Composer `sidebarAction`** — `plan_sidebar_toggle` appended to the
  session-controls left cluster: rendered when `sidebarProposedPlan` exists or
  the sidebar is open; label `sidebarProposedPlan ? "Plan details" : "Tasks"`,
  `Hide {label}` while open; tooltip title `{Show|Hide} {lowercase} sidebar`.
- **`plan_sidebar_open_next`** — upstream `planSidebarOpenOnNextThreadRef`:
  `implement_plan_in_new_thread` sets it, `select_task` consumes it into
  `plan_sidebar_open` on the next thread select; `plan_sidebar_expanded` and
  the source-plan cache reset per thread (upstream remounts `PlanSidebar`).
- **Dispatch ordering fix** — upstream creates the thread, navigates to it,
  THEN dispatches `turn.start`; the prompt keeps running while the user
  watches the source plan in the sidebar. The port previously awaited
  `controller.submit` (which resolves on `end_turn`) inside the creation job,
  so navigation only happened after the implementation turn settled — the
  source-plan sidebar branch was unreachable. `submit` now runs detached on
  the runtime after the durable records land; on failure it still deletes the
  half-created thread and reports `TaskCreationFailed`.

Not ported (feature absent upstream-relative, no stub invented): the sidebar's
"Steps" task-list section (`turn.tasks.updated` task lists do not exist in the
port), the `formatTimestamp` header timestamp (upstream only renders it for
task lists), and `showComposerActiveTaskListCard` interplay.

## Gates

- `cargo fmt --all -- --check` — clean.
- `cargo clippy --locked --workspace --all-targets` — 25 warnings, identical
  to committed HEAD baseline.
- `cargo test --locked --workspace` — green except the documented
  `pull_requests::tests::discovery_does_not_change_worktree_or_index` env
  failure (gitconfig remote rewrite; unchanged by this batch).

## Live test — real app

`target/release/synara-app --data-dir /tmp/synara-e2e/data-plan`,
`opencode` profile → `/tmp/synara-e2e/stub-agent-plan.py` (ACP stub; emits a
real `<proposed_plan>` chunk per prompt; a 10 s delay on
"PLEASE IMPLEMENT THIS PLAN" prompts keeps the implementation turn unsettled
long enough to observe the sidebar).

Acceptance gestures (all observed, screenshots in `/Users/devin/screenshots/`):

1. Plan-mode thread with an actionable plan shows the composer's
   **"Plan details"** toggle (icon + label, left cluster) — `ss_a0eb95dc`.
2. Clicking it opens the 340 px right sidebar: accent "Plan" badge, close
   button, collapsible **"Cache header fix"** row; the toggle flips to
   **"Hide Plan details"** — `ss_2162e9e0`.
3. Expanding the row renders the full plan markdown in a bordered box —
   `ss_5318f85e`.
4. "Implement in a new thread" navigates **immediately** to
   "Implement Cache header fix" while the turn is still running
   (`Working for 10s`), the sidebar auto-opens showing the SOURCE plan, and
   the toggle reads "Hide Plan details" — `ss_445ac564`. (Pre-fix run:
   navigation only happened after the turn settled and the sidebar showed
   the upstream empty state — `ss_b6723bd7`.)
5. sqlite: `proposed_plan_implemented` recorded on the source thread
   (`0c0ad648` seq 28, plan `plan:…:turn:ef056a1e…`) and
   `proposed_plan_source` at seq 1 on the new impl thread `ab761b4e…`,
   ahead of `prompt_started` — the ordering the pending-slot design requires.

## Next tick candidates

- Sidebar "Steps" section once task lists exist (`turn.tasks.updated` port).
- `followups` excision → busy-queue parity; `direct_model_controls` review;
  "Loading" pre-takeover label; terminal-count avatar badge.
