# keybindingeditor-rs

FILE: keybindingEditor.ts Purpose: Model behind Settings → Keybindings: the rows it lists, what a recorded          shortcut would collide with, and the server edits each action sends. Layer: UI helper Depends on: the shortcut definitions, the keybinding matcher, and key capture.

napi-rs bridge for `apps/web/src/keybindingEditor.ts`.

## Functions

- `whenConditionsCanHoldTogether` (`apps/web/src/keybindingEditor.ts:117`)
- `buildShortcutEditorRows` (`apps/web/src/keybindingEditor.ts:249`)
- `filterShortcutEditorRows` (`apps/web/src/keybindingEditor.ts:269`)
- `shortcutModifierHint` (`apps/web/src/keybindingEditor.ts:393`)
- `evaluateRecordedShortcut` (`apps/web/src/keybindingEditor.ts:403`)
- `shortcutSaveEdits` (`apps/web/src/keybindingEditor.ts:469`)
- `shortcutRemoveEdits` (`apps/web/src/keybindingEditor.ts:488`)
- `shortcutResetTakeovers` (`apps/web/src/keybindingEditor.ts:498`)
- `shortcutResetEdits` (`apps/web/src/keybindingEditor.ts:512`)