# updatestate-rs

Owns updater admission and progress policy; package lanes must never replace one another.

napi-rs bridge for `apps/desktop/src/updateState.ts`.

## Functions

- `canCheckForDesktopUpdates` (`apps/desktop/src/updateState.ts:11`)
- `getDownloadStallTimeoutMessage` (`apps/desktop/src/updateState.ts:15`)
- `isExpectedStalledDownloadCancellationError` (`apps/desktop/src/updateState.ts:20`)
- `hasDownloadProgressAdvanced` (`apps/desktop/src/updateState.ts:36`)
- `shouldBroadcastDownloadProgress` (`apps/desktop/src/updateState.ts:64`)
- `isUpdateVersionNewer` (`apps/desktop/src/updateState.ts:106`)
- `isUpdateVersionAllowedForFlavor` (`apps/desktop/src/updateState.ts:155`)
- `nextStatusAfterDownloadFailure` (`apps/desktop/src/updateState.ts:177`)
- `getCanRetryAfterDownloadFailure` (`apps/desktop/src/updateState.ts:183`)
- `shouldCheckForUpdatesOnForeground` (`apps/desktop/src/updateState.ts:187`)
- `getAutoUpdateDisabledReason` (`apps/desktop/src/updateState.ts:217`)
- `updateFilesMatchRuntimeArch` (`apps/desktop/src/updateState.ts:253`)
- `shouldAcceptAvailableUpdate` (`apps/desktop/src/updateState.ts:291`)