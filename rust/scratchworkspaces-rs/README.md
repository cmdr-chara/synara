# scratchworkspaces-rs

FILE: scratchWorkspaces.ts Purpose: Per-thread scratch working directories for provider sessions that          start before any project workspace exists (e.g. a chat's first          turn). The root stays in the user's cache so it is outside project          ancestry and outside temporary directories writable by agents.

napi-rs bridge for `apps/server/src/scratchWorkspaces.ts`.

## Functions

- `resolveScratchWorkspacesRoot` (`apps/server/src/scratchWorkspaces.ts:32`)
- `ensureIsolatedScratchWorkspace` (`apps/server/src/scratchWorkspaces.ts:176`)
- `resolveScratchWorkspaceCwd` (`apps/server/src/scratchWorkspaces.ts:195`)