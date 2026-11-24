# session-logic-rs

Session logic.

napi-rs bridge for `apps/web/src/session-logic.ts`.

## Functions

- `formatClockDuration` (`apps/web/src/session-logic.ts:105`)
- `formatClockElapsed` (`apps/web/src/session-logic.ts:119`)
- `formatElapsed` (`apps/web/src/session-logic.ts:129`)
- `isSessionOrLatestTurnRunning` (`apps/web/src/session-logic.ts:153`)
- `isSessionOrLatestTurnStartingOrRunning` (`apps/web/src/session-logic.ts:161`)
- `isSessionOpenOrLatestTurnRunning` (`apps/web/src/session-logic.ts:175`)
- `runningSessionActiveTurnId` (`apps/web/src/session-logic.ts:188`)
- `isLatestTurnSettled` (`apps/web/src/session-logic.ts:194`)
- `hasLiveLatestTurn` (`apps/web/src/session-logic.ts:208`)
- `canSessionAnswerPendingRequests` (`apps/web/src/session-logic.ts:226`)
- `isSessionRuntimeRetired` (`apps/web/src/session-logic.ts:240`)
- `isSessionAliveForSubagents` (`apps/web/src/session-logic.ts:247`)
- `isSessionRunningTurn` (`apps/web/src/session-logic.ts:272`)
- `deriveActiveWorkStartedAt` (`apps/web/src/session-logic.ts:278`)
- `deriveActiveTaskListState` (`apps/web/src/session-logic.ts:315`)
- `deriveActiveBackgroundTasksState` (`apps/web/src/session-logic.ts:462`)
- `derivePendingBackgroundWork` (`apps/web/src/session-logic.ts:481`)
- `deriveOutstandingBackgroundTaskIds` (`apps/web/src/session-logic.ts:511`)
- `countOutstandingBackgroundWork` (`apps/web/src/session-logic.ts:519`)
- `deriveRunningSubagentTasks` (`apps/web/src/session-logic.ts:570`)
- `hasLiveTurnTailWork` (`apps/web/src/session-logic.ts:639`)
- `findLatestProposedPlan` (`apps/web/src/session-logic.ts:674`)
- `findSidebarProposedPlan` (`apps/web/src/session-logic.ts:704`)
- `hasActionableProposedPlan` (`apps/web/src/session-logic.ts:731`)
- `buildSourceProposedPlanReference` (`apps/web/src/session-logic.ts:737`)
- `inferCheckpointTurnCountByTurnId` (`apps/web/src/session-logic.ts:762`)
- `derivePhase` (`apps/web/src/session-logic.ts:775`)