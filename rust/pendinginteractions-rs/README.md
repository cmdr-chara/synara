# pendinginteractions-rs

FILE: pendingInteractions.ts Purpose: Shared policy for claiming responses and recognizing invalidated approval/user-input callbacks. Layer: Cross-package orchestration utility Exports: stale-callback matcher, reclaim timing, and response-status predicates.

napi-rs bridge for `packages/shared/src/pendingInteractions.ts`.

## Functions

- `respondingInteractionReclaimCutoff` (`packages/shared/src/pendingInteractions.ts:91`)
- `respondingInteractionReclaimAt` (`packages/shared/src/pendingInteractions.ts:98`)
- `isPendingInteractionAwaitingResponse` (`packages/shared/src/pendingInteractions.ts:109`)
- `isPendingInteractionResponseClaimable` (`packages/shared/src/pendingInteractions.ts:115`)