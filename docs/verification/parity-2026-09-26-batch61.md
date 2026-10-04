# Parity batch 61 — Queued turns surface (send-while-busy queue)

Date: 2026-09-26
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Replaces the port's manual "follow-ups draft shelf" (a per-thread editor that
never sent anything) with upstream's queued-turns surface:
`composerDraftStore.queuedTurns` + `enqueueQueuedTurn` /
`ComposerQueuedHeader` + `QueuedComposerActions` +
`shouldAutoDispatchQueuedComposerTurn` / `queuedComposerDrain`.

- **Enqueue** — `send_prompt` on a `busy` or `connecting` thread now stores
  the composer draft as a queued turn (upstream `useChatTurnSubmission`
  `enqueueQueuedTurn`) instead of silently dropping the send. Attachments are
  refused with the composer error row (upstream queued turns are text-only).
- **Stacked rows above the composer** — `followups_view` renders one row per
  queued turn fused directly above the composer (upstream
  `ComposerQueuedHeader` inside `ComposerStackedPanel`): send icon +
  `compact_queued_preview` (ports `compactQueuedComposerPreviewMarkdown` —
  first non-empty trimmed line, heading/quote/checkbox/list/numbering
  prefixes stripped, fenced block → "Code block", 120 chars) + Steer chip +
  Delete (Trash-style close icon) + Edit (compose icon). The old inline
  TextEntry editor, open/closed toggle, "Queue text draft", Append, and Move
  buttons are deleted — none exist upstream.
- **Actions** — Steer removes the row and dispatches immediately; when a turn
  is in flight (ACP exposes no mid-turn steer channel) it maps to upstream's
  non-steerable-provider path: `cancel()` + `pending_steer` redispatch on
  settle (upstream `QueuedSteerGate` interrupt → redispatch). Edit removes
  the row and restores the text into the composer draft
  (`restoreQueuedTurnToComposer`). Delete removes the row.
- **Auto-drain** — `maybe_drain_followups` runs after each queue reply and on
  `Update::PromptDone`: when the thread is selected, not busy/connecting,
  has no pending interaction gate, and has a queue head, the head is removed
  and dispatched with `dispatch_text` (upstream `mode: "queue"`). A pending
  steer wins over a queued head (upstream steer-before-drain ordering).
- **Persistence** — the queue stays on the existing sqlite `FollowupQueue`
  (revision-CAS `FollowupEdit` adds/removes). Upstream holds it in memory;
  persisting is an invisible superset — same UX, durable across restarts.
- **Extras row** — the "Follow-ups" entry in the composer extras menu is
  removed (no upstream counterpart); the navigation blocker keeps only the
  reviewed-recap check.

## Gates

- `cargo fmt --all -- --check` — clean
- `cargo clippy --locked --workspace --all-targets` — 25 warnings, identical
  to the HEAD baseline (count verified by stashing)
- `cargo test --locked --workspace` — 357 pass; only
  `pull_requests::tests::discovery_does_not_change_worktree_or_index` fails
  (documented env failure — gitconfig rewrites github.com remotes)

## Live test — real app

Release binary `target/release/synara-app --data-dir /tmp/synara-e2e/data-queue`
with `stub-agent-queue.py` (ACP stub: logs each `session/prompt`, streams a
chunk, holds the turn 14–40s, answers `session/cancel`). Acceptance gesture:
sending while busy produces a stacked queued row and the row dispatches
itself on settle. Observed:

- Enqueue: during a busy turn, submitting "SECOND QUEUED DRAFT" rendered the
  stacked row (icon + preview + Steer/X/edit) fused above the composer.
- Drain: on settle the head auto-dispatched — repeatedly observed
  (SECOND/FIFTH/SIXTH/SEVENTH/NINTH each landed as the next user turn).
- Steer: clicking Steer on the "EIGHTH" row removed it and dispatched the
  text as the next turn.
- Edit: clicking the pencil on "EDIT ME DRAFT" removed the row and restored
  the text into the composer input.
- Delete: clicking X on "DELETE ME DRAFT" removed the row; the stub log
  confirms the text was never dispatched (`queue-prompts.log` count 0) and
  no transcript bubble appeared.
- The extras menu "Follow-ups" row and the old editor surface are gone.

## Next tick candidates

- `direct_model_controls` review; Hub→Studio rename; env-panel inline Notes;
  auto-mode provider gating; "Loading" pre-takeover label; "Fast mode"
  extras row; terminal-count avatar badge.
- Deferred live verification: batches 48/50/51 (ACP permissions), 49, 54, 55.
