# voicetranscription-rs

FILE: voiceTranscription.ts Purpose: Owns the desktop-specific voice transcription flow for Electron builds. Layer: Desktop IPC + ChatGPT upload bridge Depends on: Codex auth discovery, Electron net uploads, and the shared server voice contract.

napi-rs bridge for `apps/desktop/src/voiceTranscription.ts`.

## Functions

- `requestDesktopVoiceTranscription` (`apps/desktop/src/voiceTranscription.ts:214`)
- `registerDesktopVoiceTranscriptionHandler` (`apps/desktop/src/voiceTranscription.ts:279`)