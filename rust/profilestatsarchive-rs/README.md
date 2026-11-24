# profilestatsarchive-rs

FILE: profileStatsArchive.ts Purpose: Snapshot a thread's profile-stat aggregates into the durable profile_stats_deleted_* tables, then hard-delete every row the thread owns (projections, events, checkpoints, session runtime). This is what lets a delete actually free disk space without shrinking the Profile page numbers.

napi-rs bridge for `apps/server/src/profileStatsArchive.ts`.

## Functions

- `aggregateThreadTurnSnapshotRows` (`apps/server/src/profileStatsArchive.ts:244`)
- `aggregateThreadTokenRows` (`apps/server/src/profileStatsArchive.ts:362`)