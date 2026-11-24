# threadpullrequestlinks-rs

FILE: threadPullRequestLinks.ts Purpose: Pure rules for the pull requests linked to a thread: identity, provenance precedence,          tombstones, and which link is the thread's current PR (`lastKnownPr`). Layer: Shared domain logic (server decider/projections, web rendering) Exports: link/unlink/sync transforms, current-PR resolution, legacy replay helpers

napi-rs bridge for `packages/shared/src/threadPullRequestLinks.ts`.

## Functions

- `pullRequestLinkKey` (`packages/shared/src/threadPullRequestLinks.ts:29`)
- `isExplicitPullRequestLinkSource` (`packages/shared/src/threadPullRequestLinks.ts:40`)
- `isPullRequestLinkVisible` (`packages/shared/src/threadPullRequestLinks.ts:46`)
- `visiblePullRequestLinks` (`packages/shared/src/threadPullRequestLinks.ts:50`)
- `findPullRequestLink` (`packages/shared/src/threadPullRequestLinks.ts:56`)
- `isPullRequestDismissed` (`packages/shared/src/threadPullRequestLinks.ts:65`)
- `compareLinkedPullRequests` (`packages/shared/src/threadPullRequestLinks.ts:78`)
- `resolveCurrentPullRequestLink` (`packages/shared/src/threadPullRequestLinks.ts:92`)
- `resolveCurrentPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:113`)
- `resolveBranchManagedPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:120`)
- `legacyPullRequestLinks` (`packages/shared/src/threadPullRequestLinks.ts:129`)
- `pullRequestLinksFromEventPayload` (`packages/shared/src/threadPullRequestLinks.ts:141`)
- `linkPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:190`)
- `unlinkPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:216`)
- `syncDiscoveredPullRequest` (`packages/shared/src/threadPullRequestLinks.ts:233`)
- `refreshPullRequestSnapshots` (`packages/shared/src/threadPullRequestLinks.ts:259`)
- `threadPullRequestsEqual` (`packages/shared/src/threadPullRequestLinks.ts:271`)
- `pullRequestLinksEqual` (`packages/shared/src/threadPullRequestLinks.ts:292`)