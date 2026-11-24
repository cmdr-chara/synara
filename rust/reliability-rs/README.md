# reliability-rs

Both functions are Cloudflare-D1-bound: every JSON-representable input ends
in the same TypeError the original raises before reaching the database.
wasm-bindgen bridge for `apps/analytics/reliability.ts`.

## Functions

- `storeReliabilityRecords` (`apps/analytics/reliability.ts:15`)
- `readReliabilityTotals` (`apps/analytics/reliability.ts:50`)