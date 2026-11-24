# sitefaviconcache-rs

FILE: siteFaviconCache.ts Purpose: Resolve and cache website favicons by domain so the UI can render a          real site icon (instead of a generic globe) for link chips and          markdown source links. Deduplicates by hostname — every URL on a          given site shares one outbound fetch and one cached blob.

napi-rs bridge for `apps/server/src/siteFaviconCache.ts`.

## Functions

- `tryParseHost` (`apps/server/src/siteFaviconCache.ts:41`)
- `resolveFavicon` (`apps/server/src/siteFaviconCache.ts:149`)
- `clearSiteFaviconCache` (`apps/server/src/siteFaviconCache.ts:177`)