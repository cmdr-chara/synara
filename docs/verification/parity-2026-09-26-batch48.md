# Parity batch 48 — Access picker lists real session modes (2026-09-26)

Upstream reference: `a33435c18` (unchanged).

## Change

Upstream's access trigger is a working picker over the thread's runtime
modes (`RUNTIME_MODE_PRESENTATION` copy: "Ask for approval" / "Approve for
me" / "Full access"), not a read-only row. The port had a dead
`ControlAction::AccessInfo` row that did nothing on click while provider
modes were only reachable through a separate Mode control elsewhere.

- `control_choices(Access)` now returns the provider's advertised session
  modes (`ControlKind::Mode` choices → `ControlAction::Mode` dispatch,
  unchanged). Rows whose mode id matches an upstream runtime mode
  (`approval-required` / `auto` / `full-access`) are relabeled with the
  upstream presentation copy verbatim (`access_mode_copy`); other provider
  modes keep their advertised name/description.
- When the provider advertises no modes, the menu keeps the single
  "Ask for approval" info row (upstream shows the picker only where a mode
  exists).
- The composer footer trigger now labels itself with the currently
  selected mode instead of a static "Ask for approval".

## Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no warnings in touched
  files (a `collapsible_if` on the new code was collapsed).
- `cargo test --locked --workspace`: 355 pass; sole failure is the
  documented env-only
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`.

## Live test

Acceptance gesture: on a task connected to an agent that advertises
session modes, the access ⌄ lists the modes (relabeled where ids match
upstream's three runtime modes) and selecting one calls `set_mode`;
on a box with no agent CLIs the menu keeps the read-only info row.
Deferred to next tick's live drive — requires an agent that advertises
modes; recorded here so the gap is explicit rather than implied.

## Known parity gaps recorded

- Upstream normalizes modes per provider
  (`normalizeRuntimeModeForProvider`, `providerSupportsAutoRuntimeMode`,
  `supportsAutoMode` per model); the port lists advertised modes without
  provider-specific normalization.
- Upstream persists runtime mode per thread on the server model
  (`runtimeMode` in contracts); the port's selection lives on the session
  configuration only.
