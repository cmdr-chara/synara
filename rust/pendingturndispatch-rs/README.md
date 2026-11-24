# pendingturndispatch-rs

FILE: pendingTurnDispatch.ts Purpose: Cross-component signal that a thread has a composer turn dispatch          the thread event stream has not yet acknowledged. Layer: Web subscription utility Exports: mark/clear/has helpers consumed by ChatView and the EventRouter          catch-up watchdog.

napi-rs bridge for `apps/web/src/pendingTurnDispatch.ts`.

## Functions

- `markPendingTurnDispatch` (`apps/web/src/pendingTurnDispatch.ts:31`)
- `clearPendingTurnDispatch` (`apps/web/src/pendingTurnDispatch.ts:35`)
- `hasPendingTurnDispatch` (`apps/web/src/pendingTurnDispatch.ts:39`)