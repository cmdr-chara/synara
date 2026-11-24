# workspace-rs

— path containment,
managed-worktree parsing, local preview roots, and out-of-root reference
recovery.

napi-rs bridge for `apps/server/src/workspace/`.

## Functions

- `listLocalPreviewWorktreeRoots` (`apps/server/src/workspace/localPreviewWorktrees.ts:17`)
- `parseManagedWorktreeWorkspaceRoot` (`apps/server/src/workspace/managedWorktree.ts:3`)
- `resolveOutOfRootFileReference` (`apps/server/src/workspace/outOfRootFileReference.ts:57`)
- `isContainedPath` (`apps/server/src/workspace/realPathContainment.ts:4`)
- `resolveRealPathWithinRoot` (`apps/server/src/workspace/realPathContainment.ts:32`)
- `resolveRealPathForCreateWithinRoot` (`apps/server/src/workspace/realPathContainment.ts:48`)
- `prepareRealPathForWriteWithinRoot` (`apps/server/src/workspace/realPathContainment.ts:99`)