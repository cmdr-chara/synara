# Parity batch 58 — Implement plan in a new thread (`onImplementPlanInNewThread`)

Date: 2026-09-26
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Ports upstream's split-button plan-follow-up footer and the
"Implement in a new thread" path (`useChatTurnFollowUps` /
`ChatComposerFooter`).

- **Composer footer while `showPlanFollowUp` holds** — upstream renders a
  "Refine" pill when a draft is typed and a split **"Implement"** button +
  chevron menu when the composer is empty (`ChatComposerFooter.tsx`). Ported
  as `plan_submit_buttons`: `hasPrompt` → "Refine" (submits the plan-mode
  refinement path from batch 57); empty → "Implement" (same-thread implement)
  + chevron that opens a `ControlKind::PlanImplement` menu carrying
  "Implement in a new thread". The menu reuses the Extras anchoring
  (`composer_bounds`) so it pops above the composer.
- **`implement_plan_in_new_thread`** — upstream `onImplementPlanInNewThread`:
  creates a same-project thread titled
  `truncateTitle(buildPlanImplementationThreadTitle(markdown), 50)` ("Implement
  Cache header fix"), dispatches `PLEASE IMPLEMENT THIS PLAN:\n{markdown}`
  verbatim in Default interaction mode
  (`planImplementationDispatchSettings` = `{interactionMode: "default"}`), then
  navigates to the new thread. On the source thread it records
  `ThreadEvent::ProposedPlanImplemented{plan_id, implementation_thread_id}` —
  upstream's `sourceProposedPlan` reference on `thread.turn.start`, which
  marks the source plan implemented. Failure after thread creation deletes
  the new thread (upstream `thread.delete` rollback) and surfaces "Could not
  start implementation thread".
- Port-shape divergence recorded: upstream copies `envMode`/`worktreePath`/
  `workingDirectory` onto the new thread; the port's one-task-per-linked-
  worktree rule (`create_scoped_task_in_worktree` rejects an already-assigned
  worktree) means the new thread lands in the project directory instead.

## Gates

- `cargo fmt --all -- --check` — clean.
- `cargo clippy --locked --workspace --all-targets` — 25 warnings, identical
  to committed HEAD baseline.
- `cargo test --locked --workspace` — green except the documented
  `discovery_does_not_change_worktree_or_index` env failure.
- `cargo check -p synara-app` clean after the fix-ups (unclosed delimiter,
  `rgba` path, two `clone_on_copy` on `Copy` `ThreadId`).

## Live test — real app

`target/release/synara-app --data-dir /tmp/synara-e2e/data-plan`,
`opencode` profile → `/tmp/synara-e2e/stub-agent-plan.py` (ACP stub; emits a
real `<proposed_plan>` chunk then resolves `session/prompt`).

Acceptance gestures (all observed, screenshots in
`/Users/devin/screenshots/`):

1. Plan-mode thread with an actionable plan and an empty composer shows the
   split **Implement** button + chevron instead of the send icon
   (ss_85b04c53).
2. Chevron opens a menu containing "Implement in a new thread"
   (ss_fb590c87).
3. Clicking it creates a new task **"Implement Cache header fix"** in the
   same project, navigates to it, and the user bubble is exactly
   `PLEASE IMPLEMENT THIS PLAN:` + plan markdown (ss_051caf49). Wire check:
   `/tmp/synara-e2e/last-prompt.txt` holds the verbatim prompt with **no**
   plan-mode shim — upstream `interactionMode: "default"`.
4. sqlite `events` on the source thread:
   `proposed_plan_implemented` with `implementation_thread_id` pointing at the
   new thread (`738487a8-…`).
5. Revisiting the source thread: the "Plan ready" banner is gone (plan no
   longer actionable) and the Plan card remains (ss_bbda85b1).
6. A second plan-mode turn (after fixing the stub's reused `sessionId`
   collision) produced a new plan, and typing a draft flips the button to
   **"Refine"** (ss_effada11); clearing restores the Implement split.

Test-infra note: `stub-agent-plan.py` now mints a fresh `sessionId` per
`session/new` (was hardcoded `stub-1`, which tripped the app's
already-owned-session guard on the second turn after a restart — app
behaviour verified correct: draft preserved, error banner shown).

## Deferred (upstream-parity queue)

- Plan sidebar (`PlanSidebar.tsx`, `planSidebarOpenOnNextThreadRef` — upstream
  opens the plan sidebar on the new implementation thread; the port has no
  plan sidebar surface yet).
- Terminal-count avatar badge; Hub→Studio surface rename; env-panel inline
  Notes; auto-mode provider gating; "Loading" pre-takeover label; "Fast mode"
  extras row; followups excision→busy-queue; `direct_model_controls` review.
