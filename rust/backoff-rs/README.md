# backoff-rs

FILE: backoff.ts Purpose: Retry spacing shared by the account-API pollers, so each one          doubles, caps and spreads its delays the same way. Layer: Server utility (no IO)

napi-rs bridge for `apps/server/src/backoff.ts`.

## Functions

- `exponentialBackoffMs` (`apps/server/src/backoff.ts:7`)