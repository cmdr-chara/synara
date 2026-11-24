# coordinatorcheckin-rs

FILE: coordinatorCheckin.ts Purpose: Classify coordinator heartbeat ("check-in") turns so the transcript can hide them entirely while the group activity log can summarize what a check-in found. Layer: shared runtime logic (server ingest + web timeline derivation).

napi-rs bridge for `packages/shared/src/coordinatorCheckin.ts`.

## Functions

- `isSilentCoordinatorCheckinReply` (`packages/shared/src/coordinatorCheckin.ts:13`)
- `isCoordinatorCheckinUserMessage` (`packages/shared/src/coordinatorCheckin.ts:31`)
- `coordinatorCheckinTurnReport` (`packages/shared/src/coordinatorCheckin.ts:43`)
- `suppressCoordinatorCheckinMessages` (`packages/shared/src/coordinatorCheckin.ts:73`)