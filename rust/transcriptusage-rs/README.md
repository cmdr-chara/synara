# transcriptusage-rs

FILE: transcriptUsage.ts Purpose: Reads the usage and turns Claude Code and Codex keep in their own session files (~/.claude/projects, ~/.codex/sessions) for Usage → Analytics' "This Mac" scope, with a per-file cache: unchanged files are not opened again and grown files are read from where the last read stopped. Layer: server usage reading (node:fs).

napi-rs bridge for `apps/server/src/transcriptUsage.ts`.

## Functions

- `defaultTranscriptRoots` (`apps/server/src/transcriptUsage.ts:43`)