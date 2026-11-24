# codexhomepaths-rs

FILE: codexHomePaths.ts Purpose: Pure helpers that mirror how codexAppServerManager.ts decides which          CODEX_HOME directory the codex app-server child process runs against. Centralizing this lets consumers outside the manager (the local image          allowlist, image-path predictions, etc.) stay in sync with the actual          runtime so they don't 404 paths Codex legitimately wrote.

napi-rs bridge for `apps/server/src/codexHomePaths.ts`.

## Functions

- `resolveBaseCodexHomePath` (`apps/server/src/codexHomePaths.ts:26`)
- `resolveSynaraCodexHomeOverlayPath` (`apps/server/src/codexHomePaths.ts:35`)
- `resolveCodexHomeOverlayAccountSegment` (`apps/server/src/codexHomePaths.ts:48`)
- `resolveActiveCodexHomeWritePath` (`apps/server/src/codexHomePaths.ts:74`)
- `resolveCodexHomeAllowlistCandidates` (`apps/server/src/codexHomePaths.ts:99`)