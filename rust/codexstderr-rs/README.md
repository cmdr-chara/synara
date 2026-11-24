# codexstderr-rs

FILE: codexStderr.ts Purpose: Classify Codex text/JSON diagnostics consistently at ingestion and historical rendering. Layer: Shared runtime utility; no logging or process ownership.

napi-rs bridge for `packages/shared/src/codexStderr.ts`.

## Functions

- `normalizeCodexProcessLine` (`packages/shared/src/codexStderr.ts:19`)
- `isIgnorableCodexProcessLine` (`packages/shared/src/codexStderr.ts:23`)
- `isCodexUnstableFeaturesWarning` (`packages/shared/src/codexStderr.ts:28`)
- `isCodexMultiAgentDeprecationWarning` (`packages/shared/src/codexStderr.ts:36`)
- `isCodexUserMcpOAuthStartupError` (`packages/shared/src/codexStderr.ts:47`)
- `readCodexMcpSubscriptionError` (`packages/shared/src/codexStderr.ts:56`)
- `normalizeCodexUserVisibleErrorMessage` (`packages/shared/src/codexStderr.ts:100`)
- `classifyCodexStderr` (`packages/shared/src/codexStderr.ts:113`)