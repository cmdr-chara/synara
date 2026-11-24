# fixedshortcuts-rs

FILE: fixedShortcuts.ts Purpose: The chords no keybinding can take: text editing in every field, the system's   own, and the few Synara handles itself before any binding is looked up. The handlers   for Synara's chords match through this table, and the shortcut editor refuses every   entry, so what the editor allows and what actually fires cannot drift apart.

napi-rs bridge for `apps/web/src/fixedShortcuts.ts`.

## Functions

- `fixedShortcutsForPlatform` (`apps/web/src/fixedShortcuts.ts:104`)
- `matchesFixedShortcut` (`apps/web/src/fixedShortcuts.ts:112`)