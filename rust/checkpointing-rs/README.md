# checkpointing-rs

Checkpointing.

napi-rs bridge for `apps/server/src/checkpointing/`.

## Functions

- `parseCheckpointFilesFromUnifiedDiff` (`apps/server/src/checkpointing/Diffs.ts:60`)
- `parseManagedCheckpointRef` (`apps/server/src/checkpointing/Utils.ts:27`)
- `isManagedCheckpointRefForThread` (`apps/server/src/checkpointing/Utils.ts:42`)
- `checkpointRefForThreadTurn` (`apps/server/src/checkpointing/Utils.ts:47`)
- `checkpointRefForThreadTurnInManagedFamily` (`apps/server/src/checkpointing/Utils.ts:53`)
- `checkpointRefForThreadMessageStart` (`apps/server/src/checkpointing/Utils.ts:63`)
- `checkpointRefForThreadTurnStart` (`apps/server/src/checkpointing/Utils.ts:72`)
- `checkpointRefForThreadTurnStartInManagedFamily` (`apps/server/src/checkpointing/Utils.ts:78`)
- `checkpointRefForThreadTurnLive` (`apps/server/src/checkpointing/Utils.ts:92`)
- `checkpointRefForThreadRevertRescue` (`apps/server/src/checkpointing/Utils.ts:109`)
- `resolveProjectCwdForKind` (`apps/server/src/checkpointing/Utils.ts:131`)
- `resolveThreadWorkspaceCwd` (`apps/server/src/checkpointing/Utils.ts:142`)
- `formatCheckpointHeadTrailer` (`apps/server/src/checkpointing/historyIntegration.ts:14`)
- `checkpointReflogAnchor` (`apps/server/src/checkpointing/historyIntegration.ts:21`)
- `parseCheckpointCommit` (`apps/server/src/checkpointing/historyIntegration.ts:32`)
- `checkpointHeadTransitions` (`apps/server/src/checkpointing/historyIntegration.ts:64`)
- `parseRawDiff` (`apps/server/src/checkpointing/historyIntegration.ts:98`)
- `parseNameOnlyPaths` (`apps/server/src/checkpointing/historyIntegration.ts:113`)
- `chunkArgumentGroups` (`apps/server/src/checkpointing/historyIntegration.ts:121`)