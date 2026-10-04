# Piece: studio-hubs (Studio and Hubs)

## Scope

Studio route and Hubs route reachable from the rail More menu / surface switcher: landing, empty states, create flows.

Out of scope: palette tokens (`chrome`), rail/sidebar (`rail-sidebar`), composer (`home-composer`),
settings pages. Consume shared building blocks; do not restyle them here.

## Upstream source of truth (under `~/parity/upstream/apps/web/src/`)

routes/_chat.studio.index.tsx, routes/_chat.hubs.index.tsx, components/studio/*, components/hub*/*

## Port files (likely, under `crates/synara-app/src/`)

shell/studio*.rs, shell/hubs*.rs

## Required states (start from a fresh seeded launch unless stated)

1. `studio` — open Studio
2. `hubs` — open Hubs

If upstream cannot reach a state with the seeded data (for example it needs a GitHub login), capture
what upstream really shows there (its empty / signed-out state) and make the port show the same.

Judge region: main content area.

## Port-only things to delete (in scope)

Anything in this area that upstream does not have: extra panels, tabs, buttons, menu rows, settings,
commands. Delete the code, settings keys and tests behind them, and list them in builder.md.
