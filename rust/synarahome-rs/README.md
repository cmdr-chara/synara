# synarahome-rs

FILE: synaraHome.ts Purpose: Resolves the user-level Synara base directory without Effect, so the backend server and the Electron main process agree on one location during early startup. Exports: expandHomePath, resolveSynaraHomeDirectory, SYNARA_HOME_ENV_NAME.

napi-rs bridge for `packages/shared/src/synaraHome.ts`.

## Functions

- `expandHomePath` (`packages/shared/src/synaraHome.ts:13`)
- `resolveSynaraHomeDirectory` (`packages/shared/src/synaraHome.ts:30`)