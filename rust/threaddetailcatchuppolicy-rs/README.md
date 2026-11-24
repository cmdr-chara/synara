# threaddetailcatchuppolicy-rs

FILE: threadDetailCatchupPolicy.ts Purpose: Decide when a subscribed thread's periodic projection reconcile can be skipped. Layer: Web subscription policy (pure) Exports: ThreadDetailSyncEvidence, isThreadDetailVerifiedInSync, isLiveThreadStreamDelivering Why: The root route polls every subscribed running thread on a fixed cadence: a cheap      `replayEvents` catch-up (returns the thread's events past the client cursor) and a      full `getThreadDetailSnapshot` projection reconcile that re-ships and re-normalizes      the entire transcript.

napi-rs bridge for `apps/web/src/threadDetailCatchupPolicy.ts`.

## Functions

- `isThreadDetailVerifiedInSync` (`apps/web/src/threadDetailCatchupPolicy.ts:34`)
- `isLiveThreadStreamDelivering` (`apps/web/src/threadDetailCatchupPolicy.ts:48`)