# browsershortcuts-rs

FILE: browserShortcuts.ts Purpose: Shared key-chord matching for in-app browser shortcuts so the renderer and   desktop main process agree on the same bindings without duplicating modifier logic. Layer: Shared runtime utility Depends on: nothing

napi-rs bridge for `packages/shared/src/browserShortcuts.ts`.

## Functions

- `isBrowserCopyLinkChord` (`packages/shared/src/browserShortcuts.ts:27`)
- `isKeyboardShortcutsHelpChord` (`packages/shared/src/browserShortcuts.ts:37`)