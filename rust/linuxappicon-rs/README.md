# linuxappicon-rs

FILE: linuxAppIcon.ts Purpose: Apply the app-icon preference on Linux desktops that draw app and window icons          from the XDG icon theme. Layer: Desktop-native preference utility  Wayland has no per-window icon in the protocols Electron uses (`setIcon` only reaches X11/XWayland windows).

napi-rs bridge for `apps/desktop/src/linuxAppIcon.ts`.

## Functions

- `desktopEntryIconName` (`apps/desktop/src/linuxAppIcon.ts:27`)
- `resolveLinuxDesktopIconName` (`apps/desktop/src/linuxAppIcon.ts:47`)
- `linuxDataHome` (`apps/desktop/src/linuxAppIcon.ts:72`)