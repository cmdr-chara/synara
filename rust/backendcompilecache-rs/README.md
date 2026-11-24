# backendcompilecache-rs

FILE: backendCompileCache.ts Purpose: Node's on-disk compile cache for the desktop backend child, kept under the          flavor's own data home and scoped to one app version. Layer: Desktop process startup helper Exports: cache directory, backend env, and pruning of other versions' caches.

napi-rs bridge for `apps/desktop/src/backendCompileCache.ts`.

## Functions

- `resolveBackendCompileCacheDir` (`apps/desktop/src/backendCompileCache.ts:25`)
- `backendCompileCacheEnv` (`apps/desktop/src/backendCompileCache.ts:31`)
- `pruneBackendCompileCaches` (`apps/desktop/src/backendCompileCache.ts:42`)