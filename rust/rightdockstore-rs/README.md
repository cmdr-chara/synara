# rightdockstore-rs

FILE: rightDockStore.logic.ts Purpose: Pure, testable transitions for the right dock (tabbed multi-pane right sidebar). Layer: UI state helpers Exports: dock pane types, default-state factory, and immutable open/close/activate helpers.

napi-rs bridge for `apps/web/src/rightDockStore.logic.ts`.

## Functions

- `isSingletonPaneKind` (`apps/web/src/rightDockStore.logic.ts:72`)
- `createDefaultRightDockState` (`apps/web/src/rightDockStore.logic.ts:76`)
- `isRightDockPaneKind` (`apps/web/src/rightDockStore.logic.ts:84`)
- `sanitizeRightDockThreadState` (`apps/web/src/rightDockStore.logic.ts:128`)
- `sanitizeRightDockStateByThreadId` (`apps/web/src/rightDockStore.logic.ts:164`)
- `openPaneInState` (`apps/web/src/rightDockStore.logic.ts:253`)
- `closePaneInState` (`apps/web/src/rightDockStore.logic.ts:293`)
- `movePaneInState` (`apps/web/src/rightDockStore.logic.ts:318`)
- `setActivePaneInState` (`apps/web/src/rightDockStore.logic.ts:330`)
- `setDockOpenInState` (`apps/web/src/rightDockStore.logic.ts:340`)
- `updatePaneInState` (`apps/web/src/rightDockStore.logic.ts:350`)
- `setSidechatPaneThreadInState` (`apps/web/src/rightDockStore.logic.ts:394`)
- `toggleSingletonPaneInState` (`apps/web/src/rightDockStore.logic.ts:412`)
- `resolveActivePane` (`apps/web/src/rightDockStore.logic.ts:423`)
- `findMissingSidechatPaneIds` (`apps/web/src/rightDockStore.logic.ts:430`)
- `resolveVisibleDockSidechatThreadIds` (`apps/web/src/rightDockStore.logic.ts:444`)