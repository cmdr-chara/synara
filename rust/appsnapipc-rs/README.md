# appsnapipc-rs

FILE: appSnapIpc.ts Purpose: Centralizes the desktop AppSnap IPC contract and renderer push events. Layer: Desktop IPC adapter Depends on: Electron IPC and DesktopAppSnapManager.

napi-rs bridge for `apps/desktop/src/appSnapIpc.ts`.

## Functions

- `sendAppSnapState` (`apps/desktop/src/appSnapIpc.ts:55`)
- `sendAppSnapCaptured` (`apps/desktop/src/appSnapIpc.ts:62`)
- `sendAppSnapError` (`apps/desktop/src/appSnapIpc.ts:69`)
- `sendAppSnapPermissionGuideState` (`apps/desktop/src/appSnapIpc.ts:76`)