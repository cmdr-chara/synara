# threadretention-rs

FILE: threadRetention.ts Purpose: Runs the server-side retention loop that archives inactive orchestration threads. Layer: Server maintenance Exports: retention constants, archive-root selection, and scoped job startup.

napi-rs bridge for `apps/server/src/threadRetention.ts`.

## Functions

- `getRetentionArchiveRootIds` (`apps/server/src/threadRetention.ts:176`)