# threadexport-rs

FILE: threadExport.ts Purpose: Single source of truth for when a thread transcript may be exported. Exports must not capture in-flight output: a partial assistant          response would be serialized as if it were a completed message.

napi-rs bridge for `packages/shared/src/threadExport.ts`.

## Functions

- `threadExportBlockedReason` (`packages/shared/src/threadExport.ts:14`)