# backgroundtasks-rs

Thread-wide outstanding background tasks, folded from a thread's activity log. Shared by the server (authoritative `pendingBackgroundTaskCount` on thread shells) and the web client so both apply exactly the same rules.

napi-rs bridge for `packages/shared/src/backgroundTasks.ts`.

## Functions

- `isBackgroundTaskActivity` (`packages/shared/src/backgroundTasks.ts:26`)
- `foldOutstandingBackgroundTaskIds` (`packages/shared/src/backgroundTasks.ts:45`)
- `canSessionFinishBackgroundTasks` (`packages/shared/src/backgroundTasks.ts:88`)