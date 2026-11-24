# windowsshellico-rs

FILE: windowsShellIco.ts Purpose: Build BMP-in-ICO bytes that Explorer's taskbar can extract. Layer: Desktop-native Windows shell integration PNG-compressed ICO entries update window chrome but resolve to the generic blank-page glyph on the Win11 taskbar.

napi-rs bridge for `apps/desktop/src/windowsShellIco.ts`.

## Functions

- `inspectIcoEntries` (`apps/desktop/src/windowsShellIco.ts:27`)
- `extractIcoPngImages` (`apps/desktop/src/windowsShellIco.ts:35`)
- `encodeWindowsShellIco` (`apps/desktop/src/windowsShellIco.ts:45`)