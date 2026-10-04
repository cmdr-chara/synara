# Piece: kanban (Kanban)

## Scope

Kanban route (rail Kanban item): board, columns, cards from seeded threads, empty states, project selector, card hover/menu, new card flow.

Out of scope: palette tokens (`chrome`), rail/sidebar (`rail-sidebar`), composer (`home-composer`),
settings pages. Consume shared building blocks; do not restyle them here.

## Upstream source of truth (under `~/parity/upstream/apps/web/src/`)

routes/_chat.kanban.*.tsx, components/kanban/*, kanbanUiStore.ts

## Port files (likely, under `crates/synara-app/src/`)

shell/kanban.rs, shell/kanban/*.rs

## Required states (start from a fresh seeded launch unless stated)

1. `kanban` — open Kanban from the rail
2. `kanban-project` — Kanban for project atlas-web
3. `card-menu` — open the menu of the first card

If upstream cannot reach a state with the seeded data (for example it needs a GitHub login), capture
what upstream really shows there (its empty / signed-out state) and make the port show the same.

Judge region: main content area.

## Port-only things to delete (in scope)

Anything in this area that upstream does not have: extra panels, tabs, buttons, menu rows, settings,
commands. Delete the code, settings keys and tests behind them, and list them in builder.md.
