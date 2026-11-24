# profilestats-rs

FILE: profileStats.ts Purpose: Compute Profile-page stats from Synara's local projection DB only. The share card never reads provider archives or cloud services for metrics.

napi-rs bridge for `apps/server/src/profileStats.ts`.

## Functions

- `sqliteModifierFromUtcOffsetMinutes` (`apps/server/src/profileStats.ts:116`)
- `num` (`apps/server/src/profileStats.ts:129`)
- `nonEmptyString` (`apps/server/src/profileStats.ts:140`)
- `aggregateProfileSkillUsageRows` (`apps/server/src/profileStats.ts:263`)
- `heatmapIntensity` (`apps/server/src/profileStats.ts:391`)
- `normalizeProviderKind` (`apps/server/src/profileStats.ts:451`)
- `turnModelSelectionCte` (`apps/server/src/profileStats.ts:620`)
- `userPromptEventsQuery` (`apps/server/src/profileStats.ts:666`)
- `tokenDeltaCtes` (`apps/server/src/profileStats.ts:722`)