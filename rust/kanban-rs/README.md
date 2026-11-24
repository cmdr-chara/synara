# kanban-rs

Shared kanban vocabulary and v2 column derivation (Draft / In Progress / Awaiting you / Done), one derivation serving both the web board and the server read tool. Staleness consults an injected epoch-ms `now`; no wall-clock calls here.

napi-rs bridge for `packages/shared/src/kanban.ts`.

## Functions

- `isKanbanTurnSettled` (`packages/shared/src/kanban.ts:96`)
- `hasKanbanLiveLatestTurn` (`packages/shared/src/kanban.ts:119`)
- `hasKanbanLiveWork` (`packages/shared/src/kanban.ts:129`)
- `kanbanHeartbeatTimestampMs` (`packages/shared/src/kanban.ts:152`)
- `deriveKanbanAwaitingYouReason` (`packages/shared/src/kanban.ts:197`)
- `deriveKanbanColumnV2` (`packages/shared/src/kanban.ts:229`)
- `deriveKanbanAttention` (`packages/shared/src/kanban.ts:252`)