# claudetokenstats-rs

Shared read-time Claude accounting for Profile Stats and deletion snapshots. Old compact modelUsage may be process-cumulative: only versioned results or retained per-turn main-loop usage are safe.

napi-rs bridge for `apps/server/src/claudeTokenStats.ts`.

## Functions

- `tokenStatsThreadFilter` (`apps/server/src/claudeTokenStats.ts:16`)
- `claudeTokenActivityCtes` (`apps/server/src/claudeTokenStats.ts:79`)