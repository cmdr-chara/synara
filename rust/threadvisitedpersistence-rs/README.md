# threadvisitedpersistence-rs

FILE: threadVisitedPersistence.ts Purpose: Persists which threads have an unseen completion, plus a watermark, so unread          state survives reloads. Exports: Initial lastVisitedAt resolution for hydration plus the store's writer.

napi-rs bridge for `apps/web/src/threadVisitedPersistence.ts`.

## Functions

- `resolveInitialLastVisitedAt` (`apps/web/src/threadVisitedPersistence.ts:62`)
- `persistThreadVisitedState` (`apps/web/src/threadVisitedPersistence.ts:113`)