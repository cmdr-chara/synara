# threadworkspace-rs

FILE: threadWorkspace.ts Purpose: Share worktree and workspace-root helpers used across web and server flows. Layer: Shared util Exports: associated worktree helpers plus workspace-root comparison helpers

napi-rs bridge for `packages/shared/src/threadWorkspace.ts`.

## Functions

- `normalizeWorkspaceRootForComparison` (`packages/shared/src/threadWorkspace.ts:33`)
- `workspaceRootsEqual` (`packages/shared/src/threadWorkspace.ts:72`)
- `isWorkspaceRootWithin` (`packages/shared/src/threadWorkspace.ts:87`)
- `isScratchWorkspacePath` (`packages/shared/src/threadWorkspace.ts:113`)
- `deriveAssociatedWorktreeMetadata` (`packages/shared/src/threadWorkspace.ts:119`)
- `deriveAssociatedWorktreeMetadataPatch` (`packages/shared/src/threadWorkspace.ts:152`)