# Piece: rail-sidebar (app rail + main sidebar panel)

## Scope

- The left icon rail (`AppRail`): items, icons, order, sizes, active/hover states, tooltips, AX
  labels, the More menu (`AppRailMoreMenu`), usage ring (`AppRailUsage`), Help menu, Settings button,
  update button. Upstream default layout for a fresh install is what counts.
- The sidebar panel next to it (`Sidebar`, `SidebarPanelTitle`, `SidebarPrimaryAction`,
  `SidebarListSection`, `SidebarSectionToolbar`, `SidebarThreadRowContent`, `SidebarMetaChip`,
  `SidebarRowHoverActions`, `ProjectSidebarIcon`, `SpaceSwitcher`): header ("Synara" surface
  switcher, search, activity toggle), "New thread", Projects section (sort / add buttons, project rows,
  thread rows under projects, relative times, status glyphs), Chats section, empty states.
- Sidebar menus: project sort menu, surface switcher menu, rail More menu, row context menu.

Out of scope: top strip and palette tokens (`chrome`), composer and home empty state (`home-composer`),
thread transcript (`thread`), settings pages (`settings-*`).

## Upstream source of truth

`~/parity/upstream/apps/web/src/components/AppRail*.tsx`, `Sidebar*.tsx`, `sidebar*.ts(x)`,
`SpaceSwitcher.tsx`, `ProjectSidebarIcon.tsx`, `ThreadStatusPillChip.tsx`, `ThreadRunningSpinner.tsx`,
`appRail.logic.ts`, `railShellStore.ts`, `sidebarNavOrdering.ts`, `sidebarRowStyles.ts`,
`timestampFormat.ts`.

## Port files (likely)

`crates/synara-app/src/shell/rail.rs`, `shell/navigation.rs`, `shell/project_ui*.rs`, `shell/activity.rs`.

## Required states

1. `home-seeded` — fresh seeded launch, home screen, sidebar open with both projects and the chat.
2. `project-expanded` — project `atlas-web` expanded showing its two threads (if not expanded by default).
3. `rail-more-menu` — the rail's More menu open.
4. `project-sort-menu` — the Projects "Sort projects" menu open.
5. `surface-switcher` — the sidebar header "Synara" surface switcher menu open.
6. `thread-row-context-menu` — right-click on thread row "Fix login redirect loop".

Judge region: rail + sidebar panel and any menu/popover they open.

## Port-only things to delete (in scope)

Rail items, sidebar sections, row actions, menu items that upstream does not have.
