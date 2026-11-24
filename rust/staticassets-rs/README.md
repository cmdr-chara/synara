# staticassets-rs

Serving policy for the built web bundle (GET * static route): precompressed sidecar negotiation and cache headers. Sidecars (.br/.gz) are emitted at build time by apps/web's Vite precompress plugin — the server never compresses on the request path.

napi-rs bridge for `apps/server/src/staticAssets.ts`.

## Functions

- `staticCacheControl` (`apps/server/src/staticAssets.ts:19`)
- `negotiateStaticEncodingPreference` (`apps/server/src/staticAssets.ts:84`)
- `isSidecarRequestPath` (`apps/server/src/staticAssets.ts:129`)
- `ifNoneMatchSatisfies` (`apps/server/src/staticAssets.ts:143`)
- `staticEtag` (`apps/server/src/staticAssets.ts:154`)