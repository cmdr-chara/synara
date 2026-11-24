# devicehelpercache-rs

Device helper cache location, shared by the server and the smoke CLI. The helper links private CoreSimulator/SimulatorKit frameworks whose symbols move between Xcode releases, so a compiled binary is only valid for the toolchain that produced it and the cache is keyed on that toolchain.

napi-rs bridge for `packages/shared/src/deviceHelperCache.ts`.

## Functions

- `deviceHelperCacheKey` (`packages/shared/src/deviceHelperCache.ts:44`)
- `deviceHelperSourceRevision` (`packages/shared/src/deviceHelperCache.ts:69`)