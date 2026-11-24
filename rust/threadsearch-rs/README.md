# threadsearch-rs

FILE: threadSearch.ts Purpose: Message-content search over persisted chat history for the sidebar palette. The web client only keeps message bodies for recently opened threads, so this read covers every live thread regardless of what a client has hydrated.

napi-rs bridge for `apps/server/src/threadSearch.ts`.

## Functions

- `tokenizeThreadSearchQuery` (`apps/server/src/threadSearch.ts:35`)
- `buildThreadSearchExcerpt` (`apps/server/src/threadSearch.ts:44`)