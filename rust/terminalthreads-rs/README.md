# terminalthreads-rs

FILE: terminalThreads.ts Purpose: Shared terminal identity helpers for naming, provider attribution, and run state. Layer: Shared terminal metadata utilities Exports: command parsing plus resolved terminal presentation metadata for web/server consumers.

napi-rs bridge for `packages/shared/src/terminalThreads.ts`.

## Functions

- `dockTerminalScopeId` (`packages/shared/src/terminalThreads.ts:9`)
- `terminalScopeIdsForThread` (`packages/shared/src/terminalThreads.ts:13`)
- `isGenericTerminalThreadTitle` (`packages/shared/src/terminalThreads.ts:40`)
- `defaultTerminalTitleForCliKind` (`packages/shared/src/terminalThreads.ts:270`)
- `managedTerminalCommandNameForCliKind` (`packages/shared/src/terminalThreads.ts:278`)
- `terminalCliKindFromValue` (`packages/shared/src/terminalThreads.ts:282`)
- `deriveTerminalProcessIdentity` (`packages/shared/src/terminalThreads.ts:292`)
- `deriveTerminalCommandIdentity` (`packages/shared/src/terminalThreads.ts:339`)
- `deriveTerminalTitleFromCommand` (`packages/shared/src/terminalThreads.ts:387`)
- `consumeTerminalIdentityInput` (`packages/shared/src/terminalThreads.ts:392`)
- `consumeTerminalTitleInput` (`packages/shared/src/terminalThreads.ts:432`)
- `resolveTerminalVisualIdentity` (`packages/shared/src/terminalThreads.ts:444`)