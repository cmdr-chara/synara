# Parity batch 51 — Thread-row trailing status matches upstream's priority ladder (2026-09-26)

Upstream reference: `a33435c18` (unchanged).

## Change

Upstream's trailing slot (`resolveThreadStatusPill` +
`SidebarStatusTrailingGlyph`) ranks per-thread status: Pending Approval
(amber dot) > Awaiting Input (indigo dot) > Working/Connecting (pulsing
spinner) — Completed and Plan Ready fill in when their own signals
exist. The port only ever showed a single focus-colored busy dot.

- `thread_status_dot(task)` now resolves in upstream's priority order:
  pending `UiInteraction::Permission` → amber dot (`palette().pending`),
  pending `UiInteraction::Input` → indigo dot (new `palette().awaiting`),
  otherwise the existing busy/connecting dot (`palette().focus`).
- Same `pending` map + `is_active()` sweep as the conversation surface,
  so dots appear/dismiss with the prompt lifecycle.

## Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no warnings in
  touched files.
- `cargo test --locked --workspace`: 355 pass; sole failure is the
  documented env-only
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`.

## Live test

Acceptance gesture: a task with an unanswered permission prompt shows
the amber dot (alongside the batch-50 "Pending" text); an unanswered
input prompt shows indigo; a running task keeps the focus dot. Same
constraint as batch 50 — no provider CLI on this box raises ACP
permissions on demand, so live-dot proof is deferred; the glyph is
driven by the same map that renders the in-thread prompt card, verified
previously. The busy dot path is unchanged and was observed live on
every prior run.

## Known parity gaps recorded

- Completed-dot (unread completion) needs a `lastVisitedAt` the port
  does not track on `Task`; Plan Ready needs `proposedPlans`. Both legs
  are open until those fields exist — recorded, not faked.
- Upstream's Working glyph is an animated spinner; the port keeps its
  static focus dot (no spinner primitive exists yet).
