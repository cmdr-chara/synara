# transcriptusageparsers-rs

FILE: transcriptUsageParsers.ts Purpose: Pure line parsers for the usage Claude Code and Codex write to their own session files, so Usage → Analytics can count sessions run anywhere on this Mac (the "This Mac" scope), not only Synara's chats. Layer: server usage parsing (pure, no filesystem).

napi-rs bridge for `apps/server/src/transcriptUsageParsers.ts`.

## Functions

- `transcriptTokenTotal` (`apps/server/src/transcriptUsageParsers.ts:57`)
- `transcriptPromptTokens` (`apps/server/src/transcriptUsageParsers.ts:68`)
- `claudeLineMightMatter` (`apps/server/src/transcriptUsageParsers.ts:107`)
- `parseClaudeTranscriptLine` (`apps/server/src/transcriptUsageParsers.ts:127`)
- `initialCodexTranscriptState` (`apps/server/src/transcriptUsageParsers.ts:203`)
- `codexLineMightMatter` (`apps/server/src/transcriptUsageParsers.ts:221`)
- `parseCodexTranscriptLine` (`apps/server/src/transcriptUsageParsers.ts:241`)