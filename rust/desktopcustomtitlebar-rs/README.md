# desktopcustomtitlebar-rs

FILE: desktopCustomTitleBar.ts Purpose: Persist the Windows/Linux custom title bar preference for Electron boot. Layer: Desktop main process Depends on: filesystem; preference must be readable before BrowserWindow creation.

napi-rs bridge for `apps/desktop/src/desktopCustomTitleBar.ts`.

## Functions

- `parseCustomTitleBarPreference` (`apps/desktop/src/desktopCustomTitleBar.ts:20`)
- `readCustomTitleBarPreference` (`apps/desktop/src/desktopCustomTitleBar.ts:33`)
- `writeCustomTitleBarPreference` (`apps/desktop/src/desktopCustomTitleBar.ts:42`)
- `resolveDesktopCustomTitleBarState` (`apps/desktop/src/desktopCustomTitleBar.ts:48`)
- `resolveDesktopTitleBarFrameOptions` (`apps/desktop/src/desktopCustomTitleBar.ts:71`)