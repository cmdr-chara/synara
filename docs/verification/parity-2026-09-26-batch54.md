# Parity batch 54 — 2026-09-26 — per-task runtime-mode persistence (`thread.runtimeMode`)

Upstream reference: kartikkabadi/synara @ `a33435c18` (v0.9.2).

## Upstream behavior being matched

`thread.runtimeMode` (packages/contracts/src/orchestration.ts:269-271):
`RuntimeMode = "approval-required" | "auto" | "full-access"`,
`DEFAULT_RUNTIME_MODE = "full-access"`. The choice rides the thread:

- the composer Access button shows the persisted (or default) mode label even
  before any session exists (`RUNTIME_MODE_PRESENTATION` verbatim labels in
  `apps/web/src/lib/runtimeMode.ts`);
- the Access picker always lists the three runtime modes — it is draft state,
  not session state — and the selection is applied on dispatch/session start
  (`adoptTurnModes` in storeEventReducer.ts);
- re-creating a session for a thread re-applies the stored mode.

## What changed

- `storage/chat_preferences.rs` — `task-runtime-mode:{TaskId}` preference:
  `task_runtime_mode`, `task_runtime_modes` (bulk LIKE map), and
  `save_task_runtime_mode` (immediate-tx upsert, gated on task existence).
  Plus `runtime_mode_round_trips_per_task_and_dies_with_it` — writes, reads,
  deletes with the task.
- `service.rs` — `Catalog.runtime_modes: HashMap<TaskId, String>` populated by
  the same bulk read.
- `controller.rs` (`session_for`) — after `save_session`, before `slot.live`:
  re-applies the stored mode when the new session advertises it; on failure
  records a `Notice` in the transcript. Stored modes the provider does not
  advertise are skipped (upstream normalizes per-provider).
- `controls.rs` —
  - Access picker with no provider-advertised modes now lists the three
    canonical modes (verbatim upstream label + description via
    `access_mode_copy`), with the stored — or `full-access` default — row
    checked, dispatching `ControlAction::Mode` (the no-task composer keeps the
    informational row).
  - `ControlAction::Mode` only calls `set_mode` when a live session actually
    advertises that mode; the choice is always persisted. (This also fixes a
    latent bug where picking a mode spawned a provider session.)
  - `ControlState.pending_mode` folds the picked mode into
    `catalog.runtime_modes` on `ControlFinished`, so the composer button
    label refreshes immediately without waiting for the next catalog reload.
  - Button fallback label is upstream's default ("Full access").

## Gates

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --locked --workspace --all-targets`: clean.
- `cargo test --locked -p synara-workspace -p synara-app`: green except the
  documented environment failure
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (`~/.gitconfig` github.com rewrite + spawned-git env whitelist; fails on
  committed HEAD too — never fix).

## Live verification (real app)

`cargo build -p synara-app` + `--data-dir /tmp/synara-e2e/data`, pre-seeded
`task-runtime-mode:c6c92451-…`=`full-access` in `native-workspace.sqlite3`:

1. Composer Access button rendered **"Full access"** (persisted label).
2. Opening the picker showed all three rows verbatim — "Ask for approval /
   Always ask to edit external files and use the internet", "Approve for me /
   Only ask for actions detected as potentially unsafe", "Full access /
   Unrestricted access to the internet…" — under the "Agent permissions"
   header, with "Full access" checked.
3. Clicking "Ask for approval" flipped the button label immediately.
4. `sqlite3`: `task-runtime-mode:c6c92451-…` → `approval-required` — persist
   only, no provider session spawned.

Deferred (no provider CLI on this box): `set_mode` into a live session and the
`session_for` re-apply path against a real agent — the code path exists and is
covered by the storage test, but end-to-end needs an agent that advertises
`session/modes`. Provider-specific "auto" gating (upstream restricts it to
codex + claudeAgent) remains in the deferred parity queue.
