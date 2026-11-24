# appsnapmanager-rs

FILE: appSnapManager.ts Purpose: Owns the macOS AppSnap helper lifecycle, permission state, and pending captures. Layer: Desktop main-process service Depends on: A signed Swift helper plus narrow filesystem/process adapters.

napi-rs bridge for `apps/desktop/src/appSnapManager.ts`.

## Functions

- `desktopAppSnapPlatform` (`apps/desktop/src/appSnapManager.ts:305`)
- `parseAppSnapHelperMessage` (`apps/desktop/src/appSnapManager.ts:312`)
- `isPathInsideDirectory` (`apps/desktop/src/appSnapManager.ts:480`)