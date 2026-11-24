# usageanalyticsstats-rs

FILE: usageAnalyticsStats.ts Purpose: Usage analytics for Settings → Usage → Analytics from Synara's local projection DB: token deltas, turns, and cost at API rates as UTC-hour rows by provider, model, project, and origin. Cost is the provider's own figure when it reports one (Claude's SDK, Cursor, OpenCode); otherwise (Codex) the tokens are priced at API list prices (usagePricing.ts).

napi-rs bridge for `apps/server/src/usageAnalyticsStats.ts`.

## Functions

- `resolveUsageAnalyticsWindow` (`apps/server/src/usageAnalyticsStats.ts:102`)
- `estimateCost` (`apps/server/src/usageAnalyticsStats.ts:154`)
- `reportingProviders` (`apps/server/src/usageAnalyticsStats.ts:187`)
- `needsPrices` (`apps/server/src/usageAnalyticsStats.ts:196`)
- `buildUsageAnalytics` (`apps/server/src/usageAnalyticsStats.ts:205`)
- `buildTranscriptRows` (`apps/server/src/usageAnalyticsStats.ts:412`)