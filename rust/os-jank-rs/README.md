# os-jank-rs

FILE: os-jank.ts Purpose: Smooths over shell/path differences between packaged app launches and login shells. Exports: PATH hydration plus base-dir helpers used by server startup.

napi-rs bridge for `apps/server/src/os-jank.ts`.

## Functions

- `expandHomePath` (`apps/server/src/os-jank.ts:85`)
- `resolveBaseDir` (`apps/server/src/os-jank.ts:88`)