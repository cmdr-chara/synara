# usagepricing-rs

FILE: usagePricing.ts Purpose: API list prices per model, to estimate what Synara usage would cost for providers that report no cost of their own (Codex). Rates come from LiteLLM's public model price table; it is downloaded at most once a day and kept in the state folder so prices still load offline.

napi-rs bridge for `apps/server/src/usagePricing.ts`.

## Functions

- `parseRateTable` (`apps/server/src/usagePricing.ts:126`)
- `lookupRate` (`apps/server/src/usagePricing.ts:151`)
- `priceTokens` (`apps/server/src/usagePricing.ts:162`)
- `priceTranscriptUsage` (`apps/server/src/usagePricing.ts:178`)