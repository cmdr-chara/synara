# storenormalization-rs

FILE: storeNormalization.ts Purpose: Normalizes orchestration projects, threads, messages, and activities with stable identity. Exports: Pure normalization and equality helpers consumed by projection and event reduction.

napi-rs bridge for `apps/web/src/storeNormalization.ts`.

## Functions

- `threadSessionsEqual` (`apps/web/src/storeNormalization.ts:116`)
- `resolveCreateBranchFlowCompletedMerge` (`apps/web/src/storeNormalization.ts:134`)
- `threadShellsEqual` (`apps/web/src/storeNormalization.ts:171`)
- `threadTurnStatesEqual` (`apps/web/src/storeNormalization.ts:236`)
- `textSegmentArraysEqual` (`apps/web/src/storeNormalization.ts:268`)
- `providerReferenceArraysEqual` (`apps/web/src/storeNormalization.ts:307`)
- `deepEqualJson` (`apps/web/src/storeNormalization.ts:348`)
- `normalizeModelSelection` (`apps/web/src/storeNormalization.ts:385`)
- `normalizeProject` (`apps/web/src/storeNormalization.ts:405`)
- `normalizeSpace` (`apps/web/src/storeNormalization.ts:492`)
- `mapSpaces` (`apps/web/src/storeNormalization.ts:517`)
- `normalizeChatMessage` (`apps/web/src/storeNormalization.ts:591`)
- `mergeReadModelThreadDetailWithLiveHotPath` (`apps/web/src/storeNormalization.ts:1118`)
- `normalizeProposedPlans` (`apps/web/src/storeNormalization.ts:1203`)
- `normalizeTurnDiffFiles` (`apps/web/src/storeNormalization.ts:1234`)
- `normalizeActivities` (`apps/web/src/storeNormalization.ts:1310`)
- `withOrchestrationEventSequence` (`apps/web/src/storeNormalization.ts:1426`)
- `capThreadActivities` (`apps/web/src/storeNormalization.ts:1504`)
- `dedupeActivitiesByIdAfterAppend` (`apps/web/src/storeNormalization.ts:1643`)
- `dedupeActivitiesById` (`apps/web/src/storeNormalization.ts:1672`)
- `asActivityRecord` (`apps/web/src/storeNormalization.ts:1733`)
- `normalizeThreadErrorMessage` (`apps/web/src/storeNormalization.ts:1759`)
- `normalizeThreadSession` (`apps/web/src/storeNormalization.ts:1763`)
- `normalizeThreadFromReadModel` (`apps/web/src/storeNormalization.ts:1843`)
- `normalizeThreadShellSnapshot` (`apps/web/src/storeNormalization.ts:2128`)
- `mapProjects` (`apps/web/src/storeNormalization.ts:2314`)
- `toLegacyProvider` (`apps/web/src/storeNormalization.ts:2377`)
- `resolveThreadSidebarMetadata` (`apps/web/src/storeNormalization.ts:2402`)