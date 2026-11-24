# editorappdiscovery-rs

FILE: editorAppDiscovery.ts Purpose: Shared helpers for resolving installed editor apps/packages without          duplicating platform-specific search rules across launch and icons. Layer: Server runtime utility Exports: app/package search helpers used by open.ts and editorAppIcons.ts Depends on: EDITORS metadata plus filesystem stat checks.

napi-rs bridge for `apps/server/src/editorAppDiscovery.ts`.

## Functions

- `getEditorMacApplications` (`apps/server/src/editorAppDiscovery.ts:47`)
- `getEditorWindowsUriScheme` (`apps/server/src/editorAppDiscovery.ts:51`)
- `getEditorWindowsStorePackages` (`apps/server/src/editorAppDiscovery.ts:55`)
- `normalizeMacApplicationBundleName` (`apps/server/src/editorAppDiscovery.ts:61`)
- `resolveMacApplicationSearchPaths` (`apps/server/src/editorAppDiscovery.ts:66`)
- `resolveMacApplicationBundlePath` (`apps/server/src/editorAppDiscovery.ts:89`)
- `resolveAvailableMacApplication` (`apps/server/src/editorAppDiscovery.ts:109`)
- `clearWindowsStorePackageDiscoveryCache` (`apps/server/src/editorAppDiscovery.ts:178`)