# Parity batch 50 — Sidebar thread rows show upstream's "Pending" approval badge (2026-09-26)

Upstream reference: `a33435c18` (unchanged).

## Change

Upstream sidebar thread rows (`SidebarThreadRowContent.tsx` +
`resolveThreadStatusPill` in `Sidebar.logic.ts`) render a small
`text-ui-xs font-medium` "Pending" label — amber
(`text-amber-600` light / `text-amber-300/90` dark) — at the right end of
the title row whenever `resolveThreadStatusForSidebar` reports
"Pending Approval" (a pending approval interaction on a session that can
still answer). Pending user-input gets its own status label, not the row
text.

The port already tracks the same per-task pending set
(`HashMap<InteractionKey, UiInteraction>`) and drops stale entries via the
`is_active()` sweep, so the equivalent of upstream's
"can still answer" gate is already in place — it just never surfaced on
rows.

- `Palette::pending`: amber-300/90-equivalent dark (`0xe7c24a`),
  amber-600 light (`0xd97706`).
- `thread_row` appends a medium-weight `text_xs` "Pending" span when the
  task has a pending `UiInteraction::Permission` — hidden automatically
  once answered, cancelled, or swept inactive.

## Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no warnings in
  touched files.
- `cargo test --locked --workspace`: 354 pass; failures are the
  documented env-only
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  and the parallel-build-only `integrations::probe` `WouldBlock` flake
  (probe module passes when run solo).

## Live test

Acceptance gesture: while an agent task has an unanswered permission
prompt, its sidebar row shows amber "Pending" at the title's right end;
once answered, the badge disappears. Requires an agent that raises an
ACP permission request — no provider CLI on this box advertises one
reliably on demand, so live-badge proof is deferred and recorded here.
Code path is driven entirely by the same `pending` map the conversation
view uses to render the prompt card itself.

## Known parity gaps recorded

- Upstream's row also carries a trailing status glyph (Working spinner,
  Completed dot, Awaiting Input indigo pill, Plan Ready) driven by the
  shared `resolveThreadStatusTrailingIndicator`; the port only shows a
  busy dot — the fuller status-glyph ladder remains open.
- Upstream's "Awaiting Input" state does not put text on the title row
  either; behavior already matches for input requests.
