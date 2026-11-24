# providerordering-rs

FILE: providerOrdering.ts Purpose: Keeps provider picker ordering stable across settings, search, and menus. Layer: Web settings utility Exports: default order, normalization, and order comparison helpers.

napi-rs bridge for `apps/web/src/providerOrdering.ts`.

## Functions

- `isProviderKind` (`apps/web/src/providerOrdering.ts:15`)
- `normalizeHiddenProviders` (`apps/web/src/providerOrdering.ts:19`)
- `normalizeProviderOrder` (`apps/web/src/providerOrdering.ts:31`)
- `sameProviderOrder` (`apps/web/src/providerOrdering.ts:48`)
- `compareProvidersByOrder` (`apps/web/src/providerOrdering.ts:55`)