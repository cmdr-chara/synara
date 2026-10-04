# Parity batch 65 — Composer history, middle-click tabs, project look + favicons

Date: 2026-09-29
Upstream reference: `Emanuele-web04/synara@ec3b1f6ef9` (audited range
`a33435c18..ec3b1f6ef9`, 33 commits)
Commit: see `devin/1790450414-upstream-parity-batch43` HEAD

## Scope

Audited the 33 upstream commits in `a33435c18..ec3b1f6ef9`. Four product
changes are ported; the nine-commit rail-layout cluster is staged for a
dedicated batch (tracked below); the rest are confirmed equivalent or not
applicable to the Rust/GPUI surfaces.

## Changes

### Delivered — `d72a93dc6` middle-click tab close

- `shell/browser.rs`: the browser tab row now binds
  `MouseButton::Middle` to `browser_close_tab`, matching the existing
  explicit close action.

### Delivered — `2e41dd03c` prompt-history recall

Ports `resolvePromptHistoryNavigation` / `derivePromptHistoryFromMessages`
(`ChatView.logic.ts`):

- `synara-core/src/prompt_history.rs` — `PROMPT_HISTORY_MAX_ENTRIES = 100`,
  `derive_prompt_history` (role=user, trimmed non-empty, newest-first,
  capped) and `resolve_prompt_history_navigation` (Up: blocked with no
  state + non-empty draft, or when the caret is not on the first line —
  stale state restarts at index 0 keeping the draft. Down: requires
  state, only on the last line — stale or index 0 restores the draft and
  clears state). Covered by unit tests including the mid-line,
  mid-history and draft-restore cases.
- `input.rs`: Up/Down route through `resolve_prompt_history_navigation`
  when history is enabled, applying `entry` + `state` + `handled`.
- `shell.rs` `refresh_prompt_history`: derives the entries from the
  selected task's transcript and pushes them into the composer whenever
  the task or transcript updates (called at every `self.catalog = …`
  site); `composer.rs` keeps the entries and the `history_enabled` gate
  (disabled while composing, connecting, or with a pending approval/
  input — the upstream `shouldHandlePromptHistoryNavigationKey`
  conditions).

### Delivered — `cb9dd8254` + `3861e05a3` project name/appearance + favicons

Ports `projectAppearance.ts`, `projectEmoji.ts`, `ProjectSidebarIcon`,
`EditProjectDialog` + `ProjectAppearancePicker`,
`ProjectFaviconResolver`, and `useProjectName` local-name semantics
(`persistedProjectNamesByCwd` / `persistedProjectAppearanceByCwd`).
Upstream stores two renderer-local maps keyed by `projectCwdKey(cwd)`;
the port stores one `project-ui:{projectId}` preference record holding
`{name?, appearance?}` — a project that reverts to defaults deletes its
row, matching upstream's persist-nothing-for-defaults behavior.

Core (`synara-core`):

- `project_appearance.rs` — `PROJECT_COLORS` (7 Tailwind pairs; dark =
  72% color + 28% white except yellow which keeps the 400 swatch),
  `DEFAULT_PROJECT_ICON = "folder-2"`, `PROJECT_ICON_OPTIONS` (48
  `name/label/keywords` entries verbatim), `ProjectAppearance`
  (`icon{name,color?}` / `emoji`), `normalize_project_appearance`,
  `first_emoji` (grapheme check for Extended_Pictographic /
  Regional_Indicator / U+20E3 via `unicode-segmentation`), and
  `parse_project_appearance` validation.
- `project_emoji.rs` — `PROJECT_EMOJI_OPTIONS`, the 135-entry curated
  emoji/keywords list copied verbatim from `projectEmoji.ts`.
- `unicode-segmentation = "1.13"` added to workspace + synara-core.

Storage (`synara-workspace`):

- `storage/project_ui.rs` — `ProjectUi` decode with the 128KiB bound,
  owner-checked `project_ui` / `project_uis` reads (malformed rows and
  keys read as default), and `set_project_ui` (deletes the row when the
  normalized record is default).
- `storage.rs` — `delete_preference` and the `project-ui:` prefix in
  `valid_preference_key`.
- `favicon.rs` — the full `ProjectFaviconResolver` candidate order:
  `FAVICON_CANDIDATES` (20), icon hrefs declared in `ICON_SOURCE_FILES`
  (7) via `<link rel=…icon…>` and `{rel:"icon",href}` JSX literal forms
  resolved `public/` then project root (must stay within the project),
  `NESTED_FAVICON_CANDIDATES`, then app-like first-level dirs
  (`web|front|dash|app|client|site`, non-dot, sorted, ≤16) with
  `<dir>/public/favicon.{svg,ico,png}`. 8 unit tests cover the ordering
  and path-escape rejection.

App (`synara-app`):

- `assets/icons/central/*.svg` — the 49 Central icons the appearance
  options reference, vendored into `assets/icons/central/` with a
  `manifest.json` entry; `ui/icons.rs` gained the 49 loader arms,
  `central_icon(name)`, and `Glyph::Pencil`.
- `ui.rs` — `Palette.dark` (`DARK`/`LIGHT` + `configure`) feeds
  `ProjectColor::rgb(dark)`; `action_icon` renders a caller-provided
  leading element where `action` requires a `Glyph`.
- `shell/project_ui.rs` — `ProjectUiState` (uis + favicons + dialog),
  `refresh_project_ui` (one job per catalog refresh: `project_uis()` +
  favicon scan over local roots; remote SSH roots are skipped since the
  resolver reads the local filesystem), `project_icon` /
  `project_name` (local override → `project.name`),
  `project_glyph` implementing `ProjectSidebarIcon` exactly: emoji
  wins, else a non-default Central icon tinted by color, else the
  folder — with the favicon as a badge overlay (`Badge` rows) or the
  primary glyph (`Favicon` rows).
- `shell/project_ui/dialog.rs` — `EditProjectDialog` with the inline
  picker: Emoji/Icons tabs (switching clears the query), search
  (every word prefix-matches an option word), the typed-emoji
  prepended match, the `[default, …PROJECT_COLORS]` swatch row
  (picking a color while an emoji is set previews only, upstream
  `pickColor` semantics), the 8-column ~4-row scrollable grid with the
  selected state, both empty states, and `ProjectSidebarIcon` preview
  beside the name field (placeholder = folder name). Save emits the
  trimmed name + appearance; Escape/outside-click dismisses; focus is
  restored on close like the other dialogs.
- `shell/navigation.rs` — the project row renders `project_glyph` +
  `project_name` (sort key uses the effective name) plus the hover
  `project-edit` pencil that opens `EditProjectDialog`; `ProjectTip`
  renders the same glyph.
- `shell/organization/dialog.rs` — managed-project rows use the
  `Favicon` presentation + local name override, as upstream's
  activity/compact rows do.
- `controls.rs`, `chat_tools.rs`, `zen.rs`, `kanban.rs`,
  `command_palette.rs` — every renderer-side `project.name` display
  site now goes through `project_name` (the upstream
  `useProjectName`-equivalent).
- `chrome.rs` — dialog overlay, composer-focus + `capture_key_down`
  guards, and `restore_project_ui_focus` in the render loop, matching
  the other dialogs.
- All eight `self.catalog = catalog` sites now call
  `refresh_project_ui`, plus `Shell::new` — upstream reloads these
  maps whenever projects change.

Scope choices: `menu::Choice` only carries `Glyph`, so project entries
in the command-palette/choice menus show the effective name but not the
custom glyph (noted for follow-up); favicon resolution reads only local
roots (remote SSH roots cannot be read from the renderer).

## Upstream audit dispositions

Delivered:

- `d72a93dc6` middle-click tab close
- `2e41dd03c` prompt-history recall (preserve typed drafts on Up)
- `cb9dd8254` edit project name/emoji/icon+color
- `3861e05a3` project favicons in recent sidebar rows

Staged for a dedicated batch (nine-commit rail-layout cluster —
`Beta rail sidebar layout` + Stable enable + its refinement and
customization commits; the port's sidebar differs structurally and the
cluster needs its own layout pass):

- `afebf4264`, `dfc7195ad`, `fd88943f9`, `5ca9eab50`, `0309bc96f`,
  `a291804b3`, `c19d4f8a2`, `eef105fed`, `05703a1d6`

Confirmed equivalent (the port already implements the fixed behavior or
the upstream surface has a structurally different port counterpart):

- `f7450ffe9` preserve composer focus during navigation — the port
  keeps `focus_composer`/`previous_focus` state across panel switches.
- `682ea4b9f` address suggestions above native content — the GPUI
  address popover is already drawn as an overlay above page content.

Confirmed not applicable (upstream areas with no Rust product-code
counterpart):

- `ec3b1f6ef` Wait for Claude init before native compaction — the
  port's provider sessions are ACP-negotiated; compaction is
  provider-owned.
- `e84c5e2c4`, `cef13339e`, `4a68e5180`, `4994f2038`, `aec49c0e8`,
  `eae36fed4`, `617f6c598`, `66555215c` — Beta/Stable flavor, Electron
  app-icon, data-sharing copy, badge/logo, and updater-move fixes; the
  port ships one app with none of those surfaces.
- `7e09b9bea` unused sidebar cookie writes — the port never writes
  them.
- `c835066ba` pinned DNS callback timing — Electron network path; the
  GPUI port has no pinned-DNS TLS path.
- `55d649bbc` Cursor/Droid OAuth reopen on session start — web/server
  provider detection; ACP auth is negotiated per provider.
- `600d7b4e9` win32 synara-capture fallback JSON escaping — the
  Electron helper script; the port's `device_capture` is a separate
  implementation.
- `3e11b16a6` cached-grok-login health check — web server provider
  health probe; agent availability is surfaced via profiles.
- `54809cefb` disabled toast-dismiss styling — web CSS fix with no
  GPUI analogue.
- `75ca4e5d8` notification object lifetime — the port does not own
  OS-notification callbacks.
- `7e8d97a19` provider-update hang on Windows PATH/stdin — server-side
  update code path.
- `6a579a6e1` keep implementation plans local — operator chore, no
  product code.

## Gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no new warnings in
  touched files (remaining warnings pre-existing in untouched files).
- `cargo test --locked --workspace`: pass except
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (known environment failure — gitconfig remote rewrite).
- `cargo build --locked -p synara-app`: clean.

## Live test

Deferred to the next live pass (a live-verification debt list is
already queued; this batch adds the project-row glyph/dialog and
history recall to it).

## Upstream divergence deliberately kept

- One `project-ui:{projectId}` preference record replaces upstream's
  two renderer-local maps (name + appearance); the semantics are the
  same — nothing persists for the default look, and the local name is
  renderer-local (it does not rename the project server-side).
- The favicon is resolved to `Arc<gpui::Image>` at refresh time and
  cached per project id; upstream caches only presence per cwd and
  serves bytes via `/api/project-favicon`.

## What remains divergent (queued)

- The nine-commit rail-layout cluster staged above.
- `menu::Choice`-based project pickers show the effective name but not
  the custom glyph (Choice carries `Glyph` only).
- Live-verify deferred: batches 48/50/51 (ACP permissions), 49
  (AppSnap), 54, 55, plus this batch's dialog/glyph/history surface.
