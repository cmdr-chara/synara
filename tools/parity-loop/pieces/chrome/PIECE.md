# Piece: chrome (window chrome, top strip, base theme tokens)

## Scope (what this piece owns)

- macOS window chrome: traffic lights placement, no separate native title bar, window background.
- The top strip above the content (`AppShellTopStrip`): sidebar toggle, back/forward
  (`AppNavigationButtons`), open-thread tabs strip, right-side strip buttons, their icons, sizes,
  spacing, hover/active colors, tooltips and AX labels.
- Base theme tokens for the default dark theme (and light, when listed below): app background,
  surface, sidebar, borders/hairlines, text primary/secondary/muted, accent, focus ring, radii,
  base UI font family, base font size and weight. Other pieces consume these tokens; only this piece
  changes them.

Out of scope: rail icons and sidebar contents (piece `rail-sidebar`), composer and home empty state
(`home-composer`), settings pages (`settings`). Do not edit those files except to consume tokens.

## Upstream source of truth (read these)

`~/parity/upstream/apps/web/src/components/AppShellTopStrip.tsx`, `DesktopWindowControls.tsx`,
`AppNavigationButtons.tsx`, the open-thread tabs component (find it: `openThreadTabs*`,
`OpenThreadTabs*`), `apps/web/src/index.css`, `apps/web/src/theme/theme.logic.ts`,
`theme.seed.generated.ts`, `apps/web/src/surfaceStyles.ts`, `apps/desktop/src/main.ts` (BrowserWindow
options: `titleBarStyle`, `trafficLightPosition`, `vibrancy`, background color).

## Port files (likely)

`crates/synara-app/src/shell/chrome.rs`, `shell/open_threads.rs`, `src/ui/theme.rs`, `src/ui.rs`
(Palette), `src/ui/metrics.rs`, `src/main.rs` (window options), `shell.rs` (layout of the strip only).

## Required states (the critic captures exactly these)

1. `home` — freshly launched, seeded, dark theme, home screen, sidebar open.
2. `sidebar-collapsed` — same, after pressing the top strip's sidebar toggle once.
3. `thread-tab` — after opening the seeded thread "Fix login redirect loop" from the sidebar (top
   strip shows its tab). Skip this state until `~/parity/harness/seed.py` exists.

Judge region: the whole window, with emphasis on the top strip, window chrome and the overall
palette / typography. Ignore the sidebar's list contents and the composer details (other pieces).

## Port-only things to delete (in scope)

Anything in the port's top strip / title area that upstream does not have (for example extra toolbar
buttons such as a commands button, zen-mode button, or an activity bell, unless upstream has the same
button in the same place). Delete the feature behind it too if nothing else in upstream uses it, and
remove its settings, commands and tests. Record every deletion in builder.md.
