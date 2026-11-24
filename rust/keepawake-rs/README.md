# keepawake-rs

FILE: keepAwake.ts Purpose: "Keep this computer awake" for remote access — the persisted          preference plus the controller that holds an Electron sleep blocker          only while the preference is on, remote access is allowed, and the          computer runs on AC power. Layer: Desktop main process Depends on: filesystem for the preference file; the power monitor and sleep             blocker are injected so the controller is testable without Electron.

napi-rs bridge for `apps/desktop/src/keepAwake.ts`.

## Functions

- `parseKeepAwakePreference` (`apps/desktop/src/keepAwake.ts:28`)
- `readKeepAwakePreference` (`apps/desktop/src/keepAwake.ts:44`)
- `writeKeepAwakePreference` (`apps/desktop/src/keepAwake.ts:54`)