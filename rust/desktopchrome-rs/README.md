# desktopchrome-rs

FILE: desktopChrome.ts Purpose: Single source of truth for the macOS desktop top-bar chrome geometry that          BOTH the Electron main process and the web renderer must agree on. Layer: Shared runtime utilities (no deps; safe to import from main + renderer)  Why this exists:   The native macOS traffic lights are placed by the Electron main process   (apps/desktop), while the chrome bar they must visually sit inside is rendered by   the web app (apps/web).

napi-rs bridge for `packages/shared/src/desktopChrome.ts`.

## Functions

- `getMacTrafficLightPosition` (`packages/shared/src/desktopChrome.ts:43`)
- `normalizeDesktopZoomFactor` (`packages/shared/src/desktopChrome.ts:66`)
- `resolveMacDesktopTopBarTrafficLightGutterCssPx` (`packages/shared/src/desktopChrome.ts:76`)
- `resolveDesktopDipRectFromCssRect` (`packages/shared/src/desktopChrome.ts:101`)