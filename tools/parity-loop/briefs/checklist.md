# Brief: measurable parity checklist from upstream source

First read `~/parity/CONTEXT.md` in full and follow its hard rules.

## Goal

Build the measurable half of the parity program: a checklist generated from upstream source
(`~/parity/upstream`, current main) of every route, component, setting and keybinding, each mapped to
the port (`~/Projects/synara-gpui-pr11`, read only) with a status, plus a scorer.

## Deliverables (owned files: everything under `~/parity/checklist/`)

1. `~/parity/checklist/extract.py` — deterministic extraction from upstream source into
   `~/parity/checklist/items.json`. One item per:
   - route: every file in `apps/web/src/routes` that defines a route (skip `-*` helper files and tests),
     using the TanStack route path.
   - component: every non-test, non-story `.tsx` file under `apps/web/src/components` (and each exported
     React component in it when a file exports more than one). Exclude `*.browser.tsx`, `*.test.tsx`.
   - setting: every user-facing setting key. Source of truth: the settings schemas in
     `packages/contracts/src` (search `settings`), `apps/web/src/appSettings.ts`, and the rows indexed in
     `apps/web/src/settingsSearchIndex.ts`. Include section, label, default value.
   - keybinding: every command/default shortcut (`packages/contracts/src/keybindings.ts`,
     `apps/web/src/keybindings.ts`, `apps/web/src/fixedShortcuts.ts`, `KEYBINDINGS.md`). Include default keys.
   Each item: `id` (stable, e.g. `component:Sidebar/SidebarThreadRow`), `kind`, `upstream_ref` (path:line),
   `area` (one of the areas below), plus kind-specific fields.
2. `~/parity/checklist/status.json` — your first-pass mapping: for every item, `port_ref` (path:line in the
   port, or null), `status` in {`done`, `partial`, `missing`, `na`}, and `note`. `na` is allowed ONLY for
   items that cannot exist in a native desktop app (e.g. web-only browser-test helpers, mobile-only layout,
   Electron-preload plumbing with no UI), and needs a one-line reason. Be strict: `done` only when the port
   has the same UI element with the same strings and behavior you can point to. Grep the port for the
   upstream strings to decide; when unsure, mark `partial`.
   Also list port-only UI (sections, settings, commands, menu items whose strings never appear upstream)
   as `kind: "port-only"` items with status `delete` — these are deletion candidates.
3. `~/parity/checklist/score.py` — prints per-kind and per-area counts and the overall percentage
   (`done` + `na`) / total, and exits non-zero unless it is 100% and there are zero `port-only` items.
   `score.py --area <area>` limits to one area. `score.py --list <area> [--status missing]` lists items.
4. `~/parity/checklist/README.md` — how items are extracted, how status is decided, how to update.

Areas (use exactly these ids): `chrome` (window chrome, top strip, tabs, theme tokens), `rail-sidebar`,
`home-composer`, `thread` (transcript, messages, work log, plans, approvals), `settings`, `kanban`,
`pull-requests`, `automations`, `studio-hubs`, `inbox-tasks-groups-plugins`, `onboarding`,
`right-dock` (diff, terminal, browser, files, editor, computer, device), `dialogs-menus-palette`, `other`.

## Acceptance checklist

1. `python3 ~/parity/checklist/extract.py && python3 ~/parity/checklist/extract.py` produces byte-identical
   `items.json` both times (`shasum` both).
2. Item counts: routes >= 15, components >= 400, settings >= 60, keybindings >= 30. Print them.
3. Every item in `items.json` has a status entry; `score.py` runs and prints a table.
4. Spot-check: 10 random `done` items, open the port ref and confirm the upstream string or element is there.
   Record the 10 in the report.
5. No edits outside `~/parity/checklist/`.

## Output

`~/parity/checklist/REPORT.md` (write it first, update as you go): counts, score table output, the 10
spot checks, the port-only deletion list, open questions. Max 700 words.
