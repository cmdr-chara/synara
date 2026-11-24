# browseripc-rs

FILE: browserIpc.ts Purpose: Centralizes the desktop browser IPC contract and handler wiring. Layer: Desktop IPC adapter Depends on: Electron ipcMain/webContents and DesktopBrowserManager

napi-rs bridge for `apps/desktop/src/browserIpc.ts`.

## Functions

- `sendBrowserState` (`apps/desktop/src/browserIpc.ts:30`)
- `sendBrowserCopyLink` (`apps/desktop/src/browserIpc.ts:39`)
- `sendBrowserAnnotationEvent` (`apps/desktop/src/browserIpc.ts:46`)
- `registerBrowserIpcHandlers` (`apps/desktop/src/browserIpc.ts:54`)