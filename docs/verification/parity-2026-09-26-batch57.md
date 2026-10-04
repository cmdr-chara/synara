# Parity batch 57 — Plan Ready leg (`proposedPlans`, "Plan ready" follow-up)

Date: 2026-09-26
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Ports upstream's Plan Ready follow-up: a plan-mode turn that ends with a
`<proposed_plan>` block becomes a per-thread plan record, the composer shows a
"Plan ready" banner, and an empty submit sends the implementation prompt.

- **`synara_core::proposed_plan`** — upstream
  `packages/shared/src/proposedPlan.ts` helpers: `ProposedPlan`
  (`{id, turn_id, plan_markdown, implemented_at_ms, implementation_thread_id,
  created_at_ms, updated_at_ms}` — upstream `OrchestrationProposedPlan`),
  `extract_proposed_plan_markdown` (first `<proposed_plan>` block,
  case-insensitive, trimmed; `PROPOSED_PLAN_BLOCK_REGEX`),
  `strip_proposed_plan_blocks_from_text` (all blocks + trim),
  `proposed_plan_title` (first `#`-heading), `strip_displayed_plan_markdown`
  (drop the plan title heading + a following "summary" heading),
  `collapsed_proposed_plan_preview` (first N lines + `"..."`),
  `resolve_plan_follow_up_submission` (empty draft → `Implement` with
  `"PLEASE IMPLEMENT THIS PLAN:\n{markdown}"`; non-empty → `Refine`),
  `build_plan_implementation_thread_title` (`"Implement {title}"`),
  `build_proposed_plan_markdown_filename` + `normalize_plan_markdown_for_export`
  (`{sanitized title ?? "plan"}.md`, `trimEnd()+"\n"`),
  `has_actionable_proposed_plan`, `find_latest_proposed_plan` (prefers the
  latest turn's plans, then latest overall). No `regex` crate in the port —
  extraction is hand-rolled via `to_lowercase()` + `find`.
- **New thread events** — `ThreadEvent::ProposedPlan { plan }` (upstream
  `thread.proposed-plan.upsert`: replace by id, else append and push a
  `TranscriptItem::Plan` timeline entry; strips the plan blocks out of the last
  assistant message — upstream strips at display time in
  `deriveTimelineEntries`, the port strips at apply time because it has no
  message→turn linkage) and `ThreadEvent::ProposedPlanImplemented { plan_id,
  implementation_thread_id }` (upstream `thread.proposed-plan.implemented`,
  recorded when the implementing turn starts).
- **ACP extraction** — after `session.prompt()` resolves in
  `submit_prompt_owned`, when the task's `InteractionMode` is `Plan` (upstream
  OpenCodeAdapter's `activeInteractionMode === "plan"` gate) the last assistant
  message is scanned for a plan block and a `ProposedPlan` event is recorded.
  The plan id is `plan:{threadId}:turn:{turnId}` with the turn id taken from
  `thread.turns.last()` — `session.prompt()` resolves with the **stopReason**
  (`"end_turn"`), not a turn id. Direct-model submits extract unconditionally
  (upstream CodexAdapter parity) with the real turn id.
- **Composer "Plan ready" banner** — `plan_follow_up_banner()`: `px_5 py_4`
  row with muted "Plan ready" + the plan title (truncating, medium weight),
  mirroring upstream `ComposerPlanFollowUpBanner` (`px-5 pt-4 pb-4`, `text-ui`
  label + `text-ui-lg` title). Shows only when upstream's
  `showPlanFollowUpPrompt` conditions hold: no pending inputs, interaction mode
  is plan, latest turn settled, and an actionable plan exists
  (`hasActionableProposedPlan` on `findLatestProposedPlan`). While visible the
  composer placeholder becomes "Add feedback to refine the plan, or leave this
  blank to implement it" and an empty submit stays enabled.
- **Implement submit** — `send_prompt` runs
  `resolve_plan_follow_up_submission`: a non-empty draft is sent as a plan-mode
  refinement; an empty draft sends `"PLEASE IMPLEMENT THIS PLAN:\n{markdown}"`,
  flips the task to `InteractionMode::Default` **before** the submit reads the
  persisted mode (so the implementation prompt carries no plan shim), and
  records `ProposedPlanImplemented` on the thread (upstream
  `markSourceProposedPlanImplemented` on `turn.started`).
- **Transcript plan card** — `TranscriptItem::Plan` renders upstream's
  `ProposedPlanCard`: "Plan" badge + title (`proposed_plan_title`, fallback
  "Proposed plan"), markdown body of `strip_displayed_plan_markdown`,
  collapsible when `len > 900 || lines > 20` (10-line preview + `...` +
  "Expand plan"/"Collapse plan"), and copy/download buttons (toast
  "Plan copied as markdown"; download writes `{slug}.md` into the task's
  working directory).

## Fixed while verifying

- **Draft ack never matched shim'd prompts** — batch 56's mode prefixes meant
  the user `TextDelta` echoed the *wire* text (`PREFIX\n\nUser request:\n…`)
  while `DraftState.sent` stored the raw draft, so the composer draft survived
  every plan/debug send (observed live: a sent draft re-appearing after
  restart). `DraftState` now records the echo text **and** the raw draft
  separately (`submitted(id, draft, echo)`); `acknowledge_draft` matches the
  echo and clears only when the draft is unchanged since send. Applied to all
  three send paths (composer, side chats, kanban).

## Gates

- `cargo fmt --all -- --check` — clean.
- `cargo clippy --locked --workspace --all-targets` — 25 warnings, identical
  count and set as committed HEAD (verified by stash/recount).
- `cargo test --locked --workspace` — all green except the documented
  environment failure `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (`~/.gitconfig` rewrites github.com remotes); `integrations::probe` flake
  also observed once, passes on retry.
- New unit tests cover every `proposed_plan` helper (extract/strip/title/
  preview/filename/submission resolution/latest-plan selection).

## Live test — real app

`target/release/synara-app --data-dir /tmp/synara-e2e/data-plan` with the
`opencode` profile pointed at `/tmp/synara-e2e/stub-agent-plan.py` (extended
ACP stub: emits an `agent_message_chunk` containing a real
`<proposed_plan>` block then resolves `session/prompt` with
`stopReason: "end_turn"` so the turn settles).

Acceptance gestures (all observed):

1. Extras row "Plan mode · Turn plan mode on" → "Plan" chip appears.
2. Send "plan the retry backoff work" → wire prompt carries the verbatim plan
   prefix + `User request:` (last-prompt.txt); assistant replies
   "Here is the plan." with the `<proposed_plan>` block **stripped** from the
   visible message.
3. Transcript shows a **Plan card**: "Plan" badge, "Cache header fix" title,
   numbered steps — and the composer shows the **"Plan ready  Cache header
   fix"** banner with the follow-up placeholder.
4. Composer draft cleared after the send (draft-ack fix verified on screen).
5. A second send of a non-empty draft → stays in plan mode (refinement path);
   a third send with an **empty** composer → user bubble is exactly
   `PLEASE IMPLEMENT THIS PLAN:` + plan markdown, `task-interaction-mode` pref
   flips to `"default"` (sqlite), banner + Plan chip disappear, and a
   `proposed_plan_implemented` event is recorded on the thread.
6. sqlite `events` table shows `proposed_plan` with a real turn id
   (`plan:…:turn:2872087d-…`) — the earlier `turn:end_turn` defect fixed.

Plan card actions (copy/download) and expand/collapse are wired but the card
exercise above used the default expanded state; the download button's write
target is `task.working_directory.join(filename)`.

## Deferred (upstream-parity queue)

- `buildPlanImplementationThreadTitle` / `sourceProposedPlan` /
  `onImplementPlanInNewThread` — "implement in a new thread" affordance.
- Plan sidebar (`PlanSidebar.tsx`, `findSidebarProposedPlan`, "Plan
  details"/"Tasks" label swap).
- Terminal-count avatar badge; Hub→Studio surface rename; env-panel inline
  Notes; auto-mode provider gating; "Loading" pre-takeover label; "Fast mode"
  extras row; followups excision→busy-queue; `direct_model_controls` review.
