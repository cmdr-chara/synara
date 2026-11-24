# orchestrationeventcoalescing-rs

FILE: orchestrationEventCoalescing.ts Purpose: Collapse a flush window of orchestration domain events before they reach the          store reducer, so N streamed text deltas for one message cost one reducer pass. Layer: Web client event pipeline (pure) Exports: coalesceOrchestrationUiEvents Why: Coalesce text across interleaved threads without moving a thread's state      transitions ahead of its other events.

napi-rs bridge for `apps/web/src/orchestrationEventCoalescing.ts`.

## Functions

- `coalesceOrchestrationUiEvents` (`apps/web/src/orchestrationEventCoalescing.ts:42`)