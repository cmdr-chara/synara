# Piece: pull-requests (Pull requests / Code review)

## Scope

Code review route (rail "Code review"): list, filters, empty state (no GitHub auth / no PRs), detail panel when available.

Out of scope: palette tokens (`chrome`), rail/sidebar (`rail-sidebar`), composer (`home-composer`),
settings pages. Consume shared building blocks; do not restyle them here.

## Upstream source of truth (under `~/parity/upstream/apps/web/src/`)

routes/_chat.pull-requests*.tsx, components/pullRequest/*, components/PullRequest*

## Port files (likely, under `crates/synara-app/src/`)

shell/pull_requests.rs, shell/pull_requests/*.rs, shell/review*.rs

## Required states (start from a fresh seeded launch unless stated)

1. `pull-requests` — open Code review from the rail
2. `filter-menu` — open its main filter/menu control

If upstream cannot reach a state with the seeded data (for example it needs a GitHub login), capture
what upstream really shows there (its empty / signed-out state) and make the port show the same.

Judge region: main content area.

## Port-only things to delete (in scope)

Anything in this area that upstream does not have: extra panels, tabs, buttons, menu rows, settings,
commands. Delete the code, settings keys and tests behind them, and list them in builder.md.
