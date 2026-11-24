# wstransportevents-rs

FILE: wsTransportEvents.ts Purpose: Publish renderer-local WebSocket transport state changes to UI runtimes. Layer: Web transport utility Exports: event helpers used by wsNativeApi and terminal runtime recovery.

napi-rs bridge for `apps/web/src/wsTransportEvents.ts`.

## Functions

- `readLatestWsTransportState` (`apps/web/src/wsTransportEvents.ts:16`)
- `getWsSettlingThreadIds` (`apps/web/src/wsTransportEvents.ts:24`)
- `emitWsTransportState` (`apps/web/src/wsTransportEvents.ts:64`)
- `readLatestWsCompatibilityIssue` (`apps/web/src/wsTransportEvents.ts:105`)
- `emitWsCompatibilityIssue` (`apps/web/src/wsTransportEvents.ts:109`)