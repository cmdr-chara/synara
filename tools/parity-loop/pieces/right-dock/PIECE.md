# Piece: right-dock (Right dock: diff, terminal, files, browser)

## Scope

Panels opened from the thread header / top strip right side: diff panel, terminal drawer, file explorer/preview, browser panel; their toolbars, tabs, empty states.

Out of scope: palette tokens (`chrome`), rail/sidebar (`rail-sidebar`), composer (`home-composer`),
settings pages. Consume shared building blocks; do not restyle them here.

## Upstream source of truth (under `~/parity/upstream/apps/web/src/`)

components/DiffPanel*.tsx, components/ThreadTerminalDrawer.tsx, components/terminal/*, components/BrowserPanel.tsx, components/browser/*, components/WorkspaceFilePreview.tsx, components/ReviewFileTreePanel.tsx, rightDockStore*.ts

## Port files (likely, under `crates/synara-app/src/`)

shell/dock.rs, shell/panels.rs, shell/review*.rs, shell/terminal*.rs, shell/browser*.rs, shell/explorer.rs, shell/editors*.rs

## Required states (start from a fresh seeded launch unless stated)

1. `diff` — in seeded thread "Fix login redirect loop", open the diff panel
2. `terminal` — open the terminal
3. `files` — open the file explorer
4. `browser` — open the browser panel

If upstream cannot reach a state with the seeded data (for example it needs a GitHub login), capture
what upstream really shows there (its empty / signed-out state) and make the port show the same.

Judge region: the dock panel region.

## Port-only things to delete (in scope)

Anything in this area that upstream does not have: extra panels, tabs, buttons, menu rows, settings,
commands. Delete the code, settings keys and tests behind them, and list them in builder.md.
