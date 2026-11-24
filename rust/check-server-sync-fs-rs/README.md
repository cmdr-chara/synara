# check-server-sync-fs-rs

Prevent new main-thread filesystem blocking without banning harmless Node builtins.

napi-rs bridge for `scripts/check-server-sync-fs.ts`.

## Functions

- `countSyncFsReferences` (`scripts/check-server-sync-fs.ts:17`)
- `syncFsViolations` (`scripts/check-server-sync-fs.ts:137`)
- `serverSources` (`scripts/check-server-sync-fs.ts:154`)