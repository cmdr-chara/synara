# groupthreadstate-rs

FILE: groupThreadState.ts Purpose: One derivation of a group thread's live state — "waiting on you",          "working", "ready for review", "idle", "resolved" — used by the web          Overview surface and by the server's `synara_project_list_threads`          tool so both report the same bucket for the same thread. Layer: Shared domain helper (structural inputs accept both the web        SidebarThreadSummary and the server OrchestrationThreadShell) Exports: GroupThreadStateId, GroupThreadSectionId, GROUP_THREAD_STATES,          GROUP_THREAD_STATE_LABELS, groupThreadStateLabel,          canSessionAnswerPendingRequests, isLatestTurnSettled,          hasLiveLatestTurn, isThreadActivelyWorking,          groupThreadNeedsAttention, resolveGroupThreadState,          GroupCoordinatorStatusId, resolveGroupCoordinatorStatus

napi-rs bridge for `packages/shared/src/groupThreadState.ts`.

## Functions

- `groupThreadStateLabel` (`packages/shared/src/groupThreadState.ts:40`)
- `canSessionAnswerPendingRequests` (`packages/shared/src/groupThreadState.ts:88`)
- `isLatestTurnSettled` (`packages/shared/src/groupThreadState.ts:104`)
- `hasLiveLatestTurn` (`packages/shared/src/groupThreadState.ts:118`)
- `isThreadActivelyWorking` (`packages/shared/src/groupThreadState.ts:133`)
- `groupThreadNeedsAttention` (`packages/shared/src/groupThreadState.ts:163`)
- `resolveGroupCoordinatorStatus` (`packages/shared/src/groupThreadState.ts:197`)
- `resolveGroupThreadState` (`packages/shared/src/groupThreadState.ts:242`)