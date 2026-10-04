# Batch 52 — unread-completed sidebar dot (`lastVisitedAt` parity)

## Upstream behavior

`apps/web/src/store.ts` persists a client-side `lastVisitedAt` ISO stamp per
thread (`markThreadVisited` stamps `now` on visit; `markThreadUnread` rewinds
it). `Sidebar` shows the Completed glyph — a small accent dot — when a
non-active row's `latestTurn.completedAt > lastVisitedAt`.

## Port changes

- `crates/synara-workspace/src/storage/chat_preferences.rs`:
  `Store::task_visits()` / `Store::save_task_visit(id, at_ms)` and the
  `WorkspaceService` wrappers persist visit stamps as `task-visited:{id}`
  keys in the `preferences` KV table (mirrors `task-draft:{id}`; the write
  checks task existence inside the immediate transaction). Client-side
  preference data, not an event — matching upstream's store-local stamp.
- `crates/synara-workspace/src/service.rs`: `Catalog` gains
  `visited: HashMap<TaskId, i64>`, populated on every catalog build, so the
  badge rides every `Update::Catalog` re-render.
- `crates/synara-app/src/shell.rs`: `select_task` stamps
  `catalog.visited[id] = now_ms()` and fire-and-forget persists it via a new
  `Update::Noop` job variant.
- `crates/synara-app/src/shell/navigation.rs`: `thread_status_dot` gains the
  lowest-priority leg — `TaskState::Completed`, row not selected, and
  `task.updated_at_ms > visited.get(id)` yields the 5px `palette().focus`
  accent dot. `updated_at_ms` stands in for `latestTurn.completedAt` (the
  completion event is the last envelope to stamp the task).
- Pending-approval and pending-input legs still outrank it; a currently
  selected row never shows the dot (upstream's inactive-row gate).

## Gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no new warnings (the
  `service.rs:582` too-many-arguments warning predates this batch — same
  count on committed HEAD).
- `cargo test --locked --workspace`: green except the documented
  environment failure
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (`~/.gitconfig` rewrites github.com → git-manager.devin.ai; never fix).

## Live verification

Acceptance gesture: with a Completed task in the sidebar whose last event
postdates its visit stamp, the row shows a 5px accent dot; selecting the
task clears it (stamp rewritten to now); navigating away does not bring it
back.

Verified on this box: the app launches, and a completed task row rendered
the accent dot; clicking the row cleared it and a later visit did not
restore it. (No provider CLI here can drive a fresh completion on demand —
the leg was exercised against tasks already completed in the workspace DB,
which is the same state upstream keys off.)
