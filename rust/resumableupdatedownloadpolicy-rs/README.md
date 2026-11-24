# resumableupdatedownloadpolicy-rs

FILE: resumableUpdateDownloadPolicy.ts Purpose: Defines the synchronous HTTP, retry, progress, and checksum policy for resumable updates. Layer: Desktop updater policy Depends on: URL and plain data only; no filesystem, socket, Electron, or updater lifecycle.

napi-rs bridge for `apps/desktop/src/resumableUpdateDownloadPolicy.ts`.

## Functions

- `computeProgressInfo` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:42`)
- `parseContentRangeTotal` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:59`)
- `selectSha512Encoding` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:73`)
- `classifyDownloadResponse` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:94`)
- `computeRetryDelayMs` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:119`)
- `shouldGiveUp` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:131`)
- `isCrossOrigin` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:159`)
- `buildDownloadHeaders` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:170`)