# keybindingrules-rs

FILE: keybindingRules.ts Purpose: The keybinding rule grammar both the server and the web read and write: parsing   `key` and `when` strings, writing them back, rule identity, and the unassigned marker. Layer: Shared runtime utility Depends on: keybinding contracts only

napi-rs bridge for `packages/shared/src/keybindingRules.ts`.

## Functions

- `parseKeybindingShortcut` (`packages/shared/src/keybindingRules.ts:31`)
- `parseKeybindingWhenExpression` (`packages/shared/src/keybindingRules.ts:145`)
- `compileKeybindingRule` (`packages/shared/src/keybindingRules.ts:231`)
- `encodeKeybindingShortcut` (`packages/shared/src/keybindingRules.ts:241`)
- `encodeKeybindingWhen` (`packages/shared/src/keybindingRules.ts:266`)
- `encodeKeybindingRule` (`packages/shared/src/keybindingRules.ts:286`)
- `resolvedKeybindingRuleIdentity` (`packages/shared/src/keybindingRules.ts:302`)
- `keybindingRuleIdentity` (`packages/shared/src/keybindingRules.ts:312`)
- `isUnassignedKeybindingShortcut` (`packages/shared/src/keybindingRules.ts:322`)
- `isUnassignedKeybindingRule` (`packages/shared/src/keybindingRules.ts:327`)