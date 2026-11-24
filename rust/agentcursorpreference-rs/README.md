# agentcursorpreference-rs

FILE: agentCursorPreference.ts Purpose: Persist the agent cursor colors mirrored from the renderer, so the          Cua driver host can push them at every session open. Layer: Desktop main process Depends on: filesystem.

napi-rs bridge for `apps/desktop/src/agentCursorPreference.ts`.

## Functions

- `normalizeAgentCursorStylePreference` (`apps/desktop/src/agentCursorPreference.ts:37`)
- `parseAgentCursorPreference` (`apps/desktop/src/agentCursorPreference.ts:53`)
- `readAgentCursorPreference` (`apps/desktop/src/agentCursorPreference.ts:61`)
- `writeAgentCursorPreference` (`apps/desktop/src/agentCursorPreference.ts:74`)