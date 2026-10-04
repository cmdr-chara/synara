# Piece: settings-general (settings shell + General page)

## Scope

- How Settings opens (from the rail / sidebar Settings button) and the settings screen layout:
  the settings navigation (groups, items, icons, order, labels, badges, search box, back control),
  the page header (eyebrow, title, description), panel/card primitives (`SettingsPanelPrimitives`,
  `SettingControls`), row layout, toggles, selects, inputs.
- The General page: every row, label, description, control, default value and order.
- The settings search box behavior on typing (results list / filtering) — one state below.

Out of scope: content of other settings pages (each is its own piece), base palette tokens (piece
`chrome`), rail and app sidebar outside settings (piece `rail-sidebar`).

## Upstream source of truth

`~/parity/upstream/apps/web/src/routes/_chat.settings.tsx`, `settingsNavigation.ts`,
`settingsSearchIndex.ts`, `settingsPanelStyles.ts`, `settingsSidebarNavStyles.ts`,
`components/settings/SettingsPanelPrimitives.tsx`, `components/settings/SettingControls.tsx`,
the General panel component (find it from `_chat.settings.tsx`), `appSettings.ts` (defaults).

## Port files (likely)

`crates/synara-app/src/shell/settings.rs`, `shell/settings/*.rs` (general/native/personalization),
settings parts of `shell/navigation.rs`.

## Required states

1. `settings-general` — from the seeded home screen, open Settings; General page shown.
2. `settings-general-scrolled` — scroll the General page to its bottom.
3. `settings-search` — type `theme` into the settings search box.

Judge region: the whole settings screen (navigation + page).

## Port-only things to delete (in scope)

Settings sections or rows that upstream does not have. Known example: the port's "Getting started"
settings section (upstream has no such section; onboarding is piece `onboarding`) and any settings
group/section names that do not exist in `settingsNavigation.ts`. Also the port's hidden extra sections
(Direct models, Device / capture, Privacy & security, Plugins & integrations, Subagents & workflows,
Project import) if upstream has no equivalent settings section: check upstream first, list what you
delete and why in builder.md. Remove their code, settings keys, commands and tests, not just the nav item.
