# searchquery-rs

FILE: searchQuery.ts Purpose: Canonical normalization for workspace ENTRY (file/directory name)          search queries. The server ranker and the web match highlighter must          agree on what counts as the query — when they drift, a result can          match server-side while the client renders it with no emphasis          (e.g. typing "./comp" for Composer.tsx).

napi-rs bridge for `packages/shared/src/searchQuery.ts`.

## Functions

- `normalizeWorkspaceEntrySearchQuery` (`packages/shared/src/searchQuery.ts:18`)