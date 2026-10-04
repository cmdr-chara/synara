# Piece: settings-usage (Settings > Usage & limits page)

## Scope

Everything inside the Settings "Usage & limits" page: header (eyebrow, title, badge, description), every
card, row, label, description, control, default value, order, empty state, inline help, and the
dialogs/menus the page opens directly. The settings navigation and shared panel primitives belong to
piece `settings-general`; only consume them here.

## Upstream source of truth

`~/parity/upstream/apps/web/src/routes/_chat.settings.tsx` (find the `usage` section and the panel
component it renders), `apps/web/src/settingsNavigation.ts` (id `usage`), `settingsSearchIndex.ts`,
`components/settings/*` used by that panel, `appSettings.ts` and `packages/contracts/src` for defaults.

## Port files (likely)

`crates/synara-app/src/shell/settings.rs` and the `shell/settings/*.rs` file that renders this page.

## Required states

1. `page` — from the seeded home screen open Settings, then the "Usage & limits" page (top of page).
2. `page-bottom` — the same page scrolled to the bottom (skip if it does not scroll in either app).
3. Up to 2 more states that open this page's own menus or dialogs (choose the most used ones;
   name them `menu-<name>` / `dialog-<name>`, same in both apps).

Judge region: the settings page content (right of the settings navigation), including its menus.

## Port-only things to delete (in scope)

Rows, cards, controls and options on this page that upstream does not have, with their settings keys,
code and tests.
