# Piece: dialogs-palette (Search palette, shortcuts sheet, project dialogs)

## Scope

Sidebar search palette (Search button / Cmd+K), shortcuts dialog (Cmd+/ or Help menu), Create project dialog, Edit project dialog, Rename thread dialog, Help menu.

Out of scope: palette tokens (`chrome`), rail/sidebar (`rail-sidebar`), composer (`home-composer`),
settings pages. Consume shared building blocks; do not restyle them here.

## Upstream source of truth (under `~/parity/upstream/apps/web/src/`)

components/SidebarSearchPalette.tsx, components/WorkspaceSearchPalette.tsx, components/ShortcutsDialog.tsx, shortcutsSheet.ts, components/CreateProjectDialog.tsx, components/EditProjectDialog.tsx, components/RenameThreadDialog.tsx, components/ui/*

## Port files (likely, under `crates/synara-app/src/`)

shell/command_palette.rs, ui/task_dialog.rs, shell/project_ui*.rs, shell/organization/dialog.rs

## Required states (start from a fresh seeded launch unless stated)

1. `search-palette` — open the sidebar search palette
2. `search-typed` — type "login" in it
3. `shortcuts` — open the keyboard shortcuts dialog
4. `help-menu` — open the rail Help menu
5. `add-project` — open the Add project dialog

If upstream cannot reach a state with the seeded data (for example it needs a GitHub login), capture
what upstream really shows there (its empty / signed-out state) and make the port show the same.

Judge region: the dialog / menu and its backdrop.

## Port-only things to delete (in scope)

Anything in this area that upstream does not have: extra panels, tabs, buttons, menu rows, settings,
commands. Delete the code, settings keys and tests behind them, and list them in builder.md.
