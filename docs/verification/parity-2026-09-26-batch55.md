# Parity batch 55 — in-flight turn transcript affordances

Date: 2026-09-26
Upstream reference: `a33435c18` (v0.9.2)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Ports upstream's in-flight-turn transcript affordances (the rows that exist only
while a turn is running):

- **`working-header` row** — upstream `MessagesTimeline` shows a non-collapsible
  twin of the settled "Worked for …" activity row reading
  `Working for {formatClockElapsed}` while the turn is live. Ported in
  `shell/activity.rs`: `finished_at_ms == None` now renders
  `Working for {format_clock_duration(now_ms - started)}`; a settled turn keeps
  `Worked for {format_duration(...)}`. Both upstream formatters
  (`formatClockDuration`, `formatDuration`) are ported verbatim.
- **`working` tail row** — upstream `resolveWorkingLabel`
  (`ChatView.logic.ts`): connecting → `Starting {provider}…`, otherwise busy →
  `Thinking`. Ported as `RenderRow::Working` appended by `projected_rows` after
  the plan row; rendered as a muted label with a 1400 ms triangle-pulse opacity
  animation (upstream shimmer), static under `reduce_motion`. Resolves the
  display name from `self.profiles` by `task.agent_id`, falling back to the raw
  agent id (upstream uses the provider display name the same way).
- Resync: `busy`/`connecting` are shell state, not thread events, so
  `sync_working_row()` re-projects (event `None` ⇒ full topology rebuild) on
  every insert/remove of both sets, plus both durable `transcript.sync`
  callsites now pass `working: bool` so hydration drops the tail row for a
  settled thread.
- Live counter: `Update::Tick` (500 ms) now calls `cx.notify()` while the last
  turn is unfinished so the "Working for Ns" label advances without a message.

## Deferred upstream detail

`resolveWorkingLabel`'s `Loading` arm (isSendBusy && !takeover) is not
distinguishable in the port — `busy` covers send + in-flight turn as one set.
Sending shows `Thinking` immediately rather than `Loading` for the brief
pre-first-chunk window. Noted, not ported.

## Verification

Live test — real app (`cargo run --release -p synara-app --data-dir
/tmp/synara-e2e/data`), seeded `agent_profiles` preference pointing `opencode` at
a real ACP stub script (`/tmp/synara-e2e/stub-agent.py`) that handshakes
(`initialize`, `session/new`), emits one `agent_message_chunk` on
`session/prompt`, and deliberately never resolves the prompt so the turn stays
in-flight. Screenshots `ss_a276e9ee`, `ss_ca142f4f`, `ss_d8d05ac7`:

- Sent "working row probe" → transcript shows user message, then
  `Working for 1s` header row (24s → 52s across later shots — the Tick-driven
  counter advances), the stub's streamed chunk, and a `Thinking` tail row.
- Sidebar task row shows the busy accent dot during the turn.

The turn-end flip (`Working for` → `Worked for`, tail row removal) exercises the
same `busy.remove` → `sync_working_row()` path used by `connecting`; Escape did
not cancel the stub session on this build, so the flip was not observed live —
path is verified by the sync-code sharing, not by screenshot.

## Gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no new warnings.
- `cargo test --locked --workspace`: green except the documented env-only
  `pull_requests::tests::discovery_does_not_change_worktree_or_index` failure
  (gitconfig remote rewrite).

## Next tick candidates

- Terminal-count avatar badge (needs per-thread terminal scope).
- Plan Ready leg (`proposedPlans`).
- Upstream `Loading` pre-takeover label (needs send-busy vs turn-busy split).
