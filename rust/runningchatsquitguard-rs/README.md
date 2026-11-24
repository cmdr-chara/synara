# runningchatsquitguard-rs

FILE: runningChatsQuitGuard.ts Purpose: Coordinates the renderer Stay/Quit handshake before a user-initiated desktop quit. Layer: Desktop quit policy Depends on: Quit confirmation IPC payloads from the renderer.

napi-rs bridge for `apps/desktop/src/runningChatsQuitGuard.ts`.

## Functions

- `shouldPromptForRunningChatsBeforeQuit` (`apps/desktop/src/runningChatsQuitGuard.ts:15`)
- `quitConfirmationPresentationForPlatform` (`apps/desktop/src/runningChatsQuitGuard.ts:19`)
- `parseQuitConfirmationRequest` (`apps/desktop/src/runningChatsQuitGuard.ts:23`)
- `parseQuitConfirmationResponse` (`apps/desktop/src/runningChatsQuitGuard.ts:62`)