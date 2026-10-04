# Parity batch 49 — Extras "Attach window" picks a window inside the menu (2026-09-26)

Upstream reference: `a33435c18` (unchanged).

## Change

Upstream's extras menu (`ComposerExtrasPanel.tsx`) is a two-view menu:
`view === "root"` lists Files / Attach window / Goal (+ Plan/Debug/Fast
variants), and choosing "Attach window" navigates to `view === "windows"`
— the same menu repopulated with a "Back" row and one row per capturable
window from `appSnap.windows`. Picking a row captures that window as the
draft attachment.

The port instead jumped straight to its standalone AppSnap card — an
invented second surface the upstream web UI does not have on this
gesture. Now the extras menu owns the flow:

- `ControlState::extras_windows` marks the extras menu as showing its
  windows view; `control_choices(ControlKind::Extras)` returns
  `[Back]` + one row per `SnapWindow` (title-or-class label,
  `"{class} · {w}×{h}"` detail) when the flag is set. While discovery
  runs the view shows a single disabled "Finding windows…" row; empty
  results show "No capturable windows". Dismiss clears the flag.
- `ControlAction::AttachWindow` sets `extras_windows`, runs a menu-mode
  AppSnap discovery (`discover_appsnap_menu` — same SnapTools discovery,
  card stays closed), and reopens the extras menu.
- `ChoiceMenu::set_choices` swaps a menu's rows in place so the
  discovery reply can populate the already-open menu
  (`refresh_extras_menu`).
- `ControlAction::AttachWindowIndex(i)` selects + captures that window
  via the existing `capture_appsnap` path (draft attachment, unchanged).
- `ControlAction::ExtrasBack` reopens the extras menu at its root view.
- `SnapView::in_menu` marks menu-driven discovery/capture so replies are
  still accepted while the AppSnap card itself is closed
  (`appsnap_reply` now requires `open || in_menu`); discovery results
  refresh the open menu in place, errors surface through the shell error
  strip ("Attach window failed: …") and leave the menu showing the empty
  state.
- The old `toggle_appsnap` entry point is removed (dead once the menu
  took over); `open_appsnap_from_settings` still opens the standalone
  card from its settings surface.

## Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no warnings in
  touched files.
- `cargo test --locked --workspace`: 354 pass; failures are the
  documented env-only
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  and the parallel-build-only
  `integrations::probe` `WouldBlock` flake (passes when run solo:
  7/7 probe tests green).

## Live test

Acceptance gesture: with a task open, `+` → "Attach window" swaps the
extras menu to its windows view (Back row + per-window rows or an empty
state), and "Back" returns to the root view. Verified in the running app
(`cargo run -p synara-app` build `bca718daa`):

- `+` → "Attach window": menu repopulated in place to `Back` +
  "No capturable windows · No supported visible application windows were
  found" — the standalone AppSnap card never opened.
- The menu-mode discovery failure surfaced through the shell error
  strip: "Attach window failed: operation is not supported: AppSnap
  currently supports Linux/X11 only…" — this box is macOS, where the
  port's SnapTools has no capture backend (pre-existing capability gap,
  same error the standalone card would show; upstream desktop captures
  via native APIs on each platform).
- "Back" returned the menu to the root view
  (Files and folders / Attach window / Goal / Plan mode / Debug mode /
  Follow-ups).

The per-window-row → draft-attachment capture gesture cannot be
exercised on macOS because discovery itself is unsupported; on
Linux/X11 the same code path lists `SnapWindow` rows and captures the
selected index through the existing `capture_appsnap` path.

## Known parity gaps recorded

- Upstream's windows view also renders a per-row focus/hover state and a
  header; the port uses the standard ChoiceMenu chrome (same rows).
- The standalone AppSnap card remains reachable from its settings
  surface; upstream web has no such card — the card predates this port's
  UI work and is retained only for the settings entry point.
