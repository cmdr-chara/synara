# groupworkspacescaffold-rs

FILE: groupWorkspaceScaffold.ts Purpose: Owns the managed Group workspace layout — mkdir of the group folder and the          agent-facing instruction files (AGENTS.md/CLAUDE.md). Instructions are only          written when missing so user edits are never clobbered.

napi-rs bridge for `apps/server/src/groupWorkspaceScaffold.ts`.

## Functions

- `cleanupGroupWorkspaceRoot` (`apps/server/src/groupWorkspaceScaffold.ts:85`)