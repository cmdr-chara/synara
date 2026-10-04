# Parity batch 66 — Rail sidebar layout cluster

Date: 2026-09-29
Upstream reference: `Emanuele-web04/synara@ec3b1f6ef9` (audited range
`a33435c18..ec3b1f6ef9`, 33 commits)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Ports the nine-commit rail-layout cluster deferred from batch 65:

- `afebf4264` feat(web): Beta rail sidebar layout (Codex-style shell)
- `dfc7195ad` feat(web): enable rail sidebar layout in Stable
- `fd88943f9` Refine rail layout spacing and shell appearance
- `5ca9eab50` Refine rail layout seam and header divider
- `0309bc96f` Remove rail content shadow
- `a291804b3` Use hairline borders for the rail layout
- `c19d4f8a2` Add sidebar customization popover for rail layout
- `eef105fed` Add independent rail item customization
- `05703a1d6` Match rail idle glyphs to sidebar section labels

The cluster replaces the classic sidebar with a narrow icon rail plus a
collapsible panel column when `settings.general.sidebar_layout == rail`.
The layout is a preference; Classic remains the default.

## Changes

### `synara-core/src/rail.rs` — store-equivalent logic (pure, tested)

Ports `railShellStore` + `Sidebar.logic.ts` rail helpers:

- `SidebarLayout` (`classic`/`rail`, Classic default) — upstream
  `settings.general.sidebarLayout`.
- `RailPanelItemId` (Home, Spaces), `RailRouteItemId` (Kanban,
  PullRequests, Automations, Studio, Settings), `RailItemId`,
  `RailOrderableItemId` with upstream's camelCase persisted names
  (`home`, `spaces`, `kanban`, `pullRequests`, `automations`, `studio`;
  settings is never orderable, matching upstream's default-order list).
- `normalize_rail_item_order`, `normalize_hidden_rail_items`,
  `build_rail_item_order` (order list minus hidden, Studio filtered when
  its section is disabled, active item always kept visible);
  `rail_item_can_hide` (Home is locked — upstream `railItemCanHide`).
- `rail_item_for_destination` / `reconcile_active_rail_item` — upstream
  `railItemForPathname` + `reconcileActiveRailItem`: a route item wins
  when the surface is its destination, a Studio surface resolves
  `studio`, otherwise the stored panel item stays active.
- `rail_item_shows_panel` — upstream `railItemHidesPanel` inverted:
  Kanban and Pull requests are full-width and collapse the panel column.
- Shortcut model: `rail_space_shortcut_key` / `rail_project_shortcut_key`
  (`space:*`, `project:*`), `RailShortcut::{Space,Project}`,
  `resolve_rail_shortcuts` (dedupe, Void always resolves, dead spaces /
  projects drop), `toggle_rail_shortcut_key`,
  `resolve_active_rail_shortcut_key` (space shortcuts active on Home
  when their space is selected; project shortcuts active on Spaces when
  their project is drilled in).
- `RailSpacesSection` + `ordered_space_ids_for_picker` +
  `build_rail_spaces_sections` — upstream `orderedSpaceIdsForPicker`
  ([active space, Void, then remaining spaces], deduped) and the panel's
  space-grouped project sections: empty spaces stay listed, an empty
  Void drops unless it is the only section, orphan projects (in deleted
  spaces) trail as their own sections in first-met order.
- `VOID_SPACE_KEY` = `"void"`.

9 unit tests cover order normalization, hidden filtering, active-item
reconciliation, shortcut key round-trips + dedupe + dead-target drops,
active-shortcut resolution and the spaces-section ordering/dedupe/
orphan rules.

### `synara-app/src/shell/rail.rs` — rail UI + navigation

- `RailState` on `Shell` — `activeItem`, `panelView`,
  `spacesProjectId`, plus the rail's own popover state. Upstream keeps
  the first three in `sessionStorage` (`synara:rail-shell:v1`); a native
  window's lifetime is the same scope, so they are in-memory only.
- `rail_enabled` (sidebar_layout == Rail), `rail_destination`
  (`railItemForPathname` over the surface), `reconcile_rail`
  (`railShellStore.reconcile`: re-syncs the active item after every
  render commit and drops the drilled project once it leaves the
  catalog), `rail_drawer_open` (`resolvedSidebarOpen`: open AND a
  panel-owning item), `sync_rail_drawer` (drives the shared sidebar
  `Drawer` so the collapse animates identically to Classic).
- `set_sidebar_open` — upstream `handleSidebarOpenChange`: reopening
  over a full-width item first reselects the current panel item, then
  opens.
- `select_rail_orderable` — upstream `handleRailItemSelect`: panel
  items select + navigate to the thread surface; route items navigate;
  Studio goes through `open_rail_studio` (gated on
  `settings.general.show_studio` like upstream's studio section flag).
- `select_rail_space` / `select_rail_project` /
  `close_rail_spaces_project` — upstream `openSpacesProject` /
  `closeSpacesProject` and space-shortcut behavior (Home panel + select
  the space + thread surface).
- `toggle_rail_shortcut` — `toggleRailShortcutKey` persisted via app
  settings (`general.rail_shortcuts`), along with `rail_item_order` and
  `hidden_rail_items` (upstream stores all three in app settings too).
- `rail_strip` — upstream `AppRail`: a 52px (`--app-rail-width` 3.25rem)
  nav column of ordered item buttons (36px, rounded-md, active bg +
  hairline), the shortcuts divider + shortcut buttons (spaces use the
  space's symbol glyph, Void uses the `black-hole` Central glyph,
  projects use their favicon), the "…" more button (active while Studio
  is open without its own rail button), and the bottom cluster (Help,
  Settings). `aria_label("Primary")` on the nav; `aria_selected` marks
  the active item (gpui has no `aria-current`, so the selected-state
  attribute is used).
- `rail_central_button` swaps the outline Central glyph for its `-fill`
  pair while active (upstream's `variant="fill"` prop); the 9 vendored
  `RAIL_ITEM_GLYPH_NAMES` icons + `dot-grid-1x3-horizontal` (More) +
  `black-hole` (Void) exist in both `assets/icons/central/` and
  `assets/icons/central-fill/` with `ui::central_fill_icon` /
  `ui::central_icon_variant` loaders.
- `rail_automation_badge` — the Automations rail button's attention
  dot: count of runs with `Failed | Cancelled | Interrupted` status
  (upstream `automationAttentionCount` also counts triage runs with
  `result.unread`; the Rust run model has no unread flag, so the status
  subset is ported — see Gaps).
- `rail_panel_content` — picks the panel column content by surface:
  Settings → `settings_sidebar`, Automations destination →
  `rail_automations_panel`, Studio → `hub_sidebar`, Spaces panel view →
  `rail_spaces_panel`, otherwise the classic `sidebar`.
- `rail_automations_panel` — upstream `RailAutomationsPanel`: title +
  "New automation" primary action, active definitions before paused
  ones, each row subtitled `{project} · {schedule}{ · Paused}` with an
  attention dot when flagged; a row opens the automation editor.
- `rail_spaces_panel` — upstream `RailSpacesPanel`: space-grouped
  project sections (header + hover "Add project" -> workspace browse),
  project rows with running/waiting dots, empty-state copy.
- `rail_spaces_project` — upstream `RailSpacesProjectPanel`: back
  button, project name, edit + new-chat chrome buttons, paged thread
  list (5/page) reusing `thread_row`, "No threads yet" empty state.
- `rail_more_overlay` — upstream `AppRailMoreMenu`: Studio entry (when
  its section is enabled), "Spaces in the rail" / "Projects in the
  rail" check rows toggling shortcut keys, divider, "Customize…".
  Anchored under the rail like upstream's popover (`RAIL_WIDTH + 8`,
  `CHROME_HEIGHT + 8`, 240px).
- `rail_customize_overlay` — upstream `SidebarCustomizeList`: visibility
  eye toggles (Home locked), up/down reorder into `rail_item_order`, and
  the Shortcuts block (remove + reorder into `rail_shortcuts`).
- `rail_shell_tone` / `rail_inset_border` — upstream's rail shell tint
  and hairline divider colors (cluster commits `fd88943f9`,
  `5ca9eab50`, `0309bc96f`, `a291804b3`: spacing, seams, no content
  shadow, hairline borders).
- `rail_glyph_button` — `ui::chrome_button` equivalent for
  dynamically-named rows (it takes only static ids).

### Wiring

- `shell/chrome.rs` — runs `reconcile_rail` once per render while the
  rail is enabled (upstream reconciles as a passive effect each commit;
  only real changes notify, so it does not render-loop), renders
  `rail_strip` before `#sidebar-drawer`, swaps the drawer's inner
  content to `rail_panel_content`, keeps the reduced-motion open
  computation on `rail_drawer_open`, and mounts the More/Customize
  overlays with the other shell overlays.
- `shell/navigation.rs` — `toggle_sidebar` routes through
  `set_sidebar_open` under the rail; the destination rows
  (kanban/pull-requests/automations) are hidden in the panel sidebar
  under the rail (upstream `panelSidebarNavIds = ["newThread"]`).
- `shell/settings.rs` — `ChoiceKind::SidebarLayout` + a "Sidebar
  layout" row in the "Sidebar organization" card (Classic/Rail).
- `synara-workspace/src/settings.rs` — `GeneralSettings` gains
  `sidebar_layout`, `rail_shortcuts`, `rail_item_order`,
  `hidden_rail_items` (default `["studio"]`, matching upstream's
  `DEFAULT_HIDDEN_RAIL_ITEMS`).
- `shell/automations.rs` — `ledger()`/`loaded()` read accessors for the
  rail panel/badge.
- `navigation.rs` / `organization.rs` / `shell.rs` — `thread_row`,
  `new_project_chat`, `symbol_glyph`, `select_space`,
  `browse_workspace` widened to `pub(super)` for the rail module.
- `synara-app/src/ui/icons.rs` — `central_fill_icon` +
  `central_icon_variant` and the 9 new Central icon pairs.

## Mapping decisions

- Upstream's route-based `railItemForPathname`/`onStudioSurface` map
  onto the Rust shell's `Panel` enum + `navigation.studio` flag: Kanban,
  Pull requests, Automations and Settings own their surfaces; any other
  surface resolves Studio when in studio mode, else the stored panel
  item.
- The shared `navigation.drawer` animation powers the rail panel
  collapse; `sync_rail_drawer` drives `set_open(visible && allowed)` so
  rail collapse animates exactly like the Classic drawer.
- `sessionStorage` rail state is in-memory: a native window has no
  session-storage boundary, and the values are recomputed from
  navigation state on every render anyway.
- `aria-current` does not exist in gpui; `aria_selected` carries the
  same semantics for rail items.

## Gaps (documented, not ported)

- Pull-requests badge count: upstream queries a dedicated review-count
  GraphQL field (`resolvePullRequestReviewBadge`, `count+` when
  incomplete). The Rust client has no equivalent data source; the rail
  Pull-requests button renders without a badge.
- Automations badge: the unread-triage flag has no Rust run-model
  counterpart; the badge counts failed/cancelled/interrupted runs only.
- Rail layout is desktop-only upstream as well (the shell has no mobile
  form-factor branch), so no responsive fallback was needed.

## Gates

- `cargo fmt --all -- --check` clean.
- `cargo clippy --locked --workspace --all-targets` — no new warnings
  (all remaining warnings are in pre-existing files).
- `cargo test --locked --workspace` — all suites pass except the known
  environment-only `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (passes on CI; local gitconfig lacks a github.com remote).
- `cargo build --locked -p synara-app` clean.
- `synara-core` rail unit tests (9) pass.
