# backendnodeoptions-rs

FILE: backendNodeOptions.ts Purpose: Builds Node runtime arguments for the packaged desktop backend child. Layer: Desktop process startup helper Exports: backend heap limit and process-local Node argument helpers.

napi-rs bridge for `apps/desktop/src/backendNodeOptions.ts`.

## Functions

- `resolveBackendMaxOldSpaceMb` (`apps/desktop/src/backendNodeOptions.ts:27`)
- `withBackendHeapLimitArg` (`apps/desktop/src/backendNodeOptions.ts:42`)
- `resolveBackendNodeArgs` (`apps/desktop/src/backendNodeOptions.ts:54`)