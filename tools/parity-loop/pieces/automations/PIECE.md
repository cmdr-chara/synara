# Piece: automations (Automations)

## Scope

Automations route (rail Automations): list, empty state, about panel, new automation dialog with every menu (template, target, schedule, run mode, workspace mode, permissions, model).

Out of scope: palette tokens (`chrome`), rail/sidebar (`rail-sidebar`), composer (`home-composer`),
settings pages. Consume shared building blocks; do not restyle them here.

## Upstream source of truth (under `~/parity/upstream/apps/web/src/`)

routes/_chat.automations*.tsx, routes/-automations.*.tsx, components/automation/*, components/RailAutomationsPanel.tsx

## Port files (likely, under `crates/synara-app/src/`)

shell/automations.rs, shell/automations/*.rs

## Required states (start from a fresh seeded launch unless stated)

1. `automations` — open Automations from the rail
2. `new-automation` — open the new automation dialog
3. `schedule-menu` — schedule menu open in that dialog
4. `template-menu` — template menu open

If upstream cannot reach a state with the seeded data (for example it needs a GitHub login), capture
what upstream really shows there (its empty / signed-out state) and make the port show the same.

Judge region: main content area and dialogs.

## Port-only things to delete (in scope)

Anything in this area that upstream does not have: extra panels, tabs, buttons, menu rows, settings,
commands. Delete the code, settings keys and tests behind them, and list them in builder.md.
