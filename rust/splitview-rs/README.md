# splitview-rs

FILE: splitView.logic.ts Purpose: Pure helpers for the split-view pane tree (find/replace/collapse leaves, depth caps, panel resets). Layer: UI state helpers Exports: tree traversal/mutation utilities and migration helpers consumed by the store and route surfaces

napi-rs bridge for `apps/web/src/splitView.logic.ts`.

## Functions

- `findPaneById` (`apps/web/src/splitView.logic.ts:18`)
- `findLeafPaneById` (`apps/web/src/splitView.logic.ts:28`)
- `findSplitNodeById` (`apps/web/src/splitView.logic.ts:33`)
- `findParentSplitNode` (`apps/web/src/splitView.logic.ts:39`)
- `findPaneDepth` (`apps/web/src/splitView.logic.ts:49`)
- `collectLeaves` (`apps/web/src/splitView.logic.ts:64`)
- `layoutSplitPanes` (`apps/web/src/splitView.logic.ts:91`)
- `replacePaneInTree` (`apps/web/src/splitView.logic.ts:124`)
- `removeLeafByThreadId` (`apps/web/src/splitView.logic.ts:146`)
- `removeLeafByPaneId` (`apps/web/src/splitView.logic.ts:179`)
- `canSubdivide` (`apps/web/src/splitView.logic.ts:215`)
- `canSubdividePane` (`apps/web/src/splitView.logic.ts:225`)
- `resolveDefaultFocusLeafId` (`apps/web/src/splitView.logic.ts:242`)
- `isLegacySplitViewLike` (`apps/web/src/splitView.logic.ts:263`)