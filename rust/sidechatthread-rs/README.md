# sidechatthread-rs

FILE: sidechatThread.ts Purpose: The one answer to "is this thread a sidechat?", shared by server and web. A sidechat          either forks a source thread (`sidechatSourceThreadId`) or stands alone with a          `sidechatContext` (a GitHub item asked about from the inbox).

napi-rs bridge for `packages/shared/src/sidechatThread.ts`.

## Functions

- `isSidechatThread` (`packages/shared/src/sidechatThread.ts:17`)
- `isStandaloneSidechatThread` (`packages/shared/src/sidechatThread.ts:22`)
- `sidechatContextMatchesGitHubItem` (`packages/shared/src/sidechatThread.ts:27`)