# providerusage-rs

FILE: providerUsage.ts Purpose: Single source of truth for provider-usage presentation metadata shared by the server (live usage fetchers) and the web app (Settings → Usage, toolbar popover): which providers expose a usage source, their display labels, learn-more URLs, and the read-only "sign in via CLI" hint used when a credential is missing or expired. Layer: cross-cutting (no runtime deps beyond the ProviderKind type).

napi-rs bridge for `packages/shared/src/providerUsage.ts`.

## Functions

- `providerUsageLabel` (`packages/shared/src/providerUsage.ts:27`)
- `providerUsageDisplayName` (`packages/shared/src/providerUsage.ts:32`)
- `providerUsageLearnMoreHref` (`packages/shared/src/providerUsage.ts:36`)
- `providerUsageNeedsAuthDetail` (`packages/shared/src/providerUsage.ts:41`)
- `selectVisibleProviderUsageSnapshots` (`packages/shared/src/providerUsage.ts:54`)