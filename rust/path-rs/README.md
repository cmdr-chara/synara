# path-rs

FILE: path.ts Purpose: Shared path classification and workspace-relative conversion helpers. Layer: Cross-package utility Exports: absolute-path predicates plus safe workspace relative path helpers

napi-rs bridge for `packages/shared/src/path.ts`.

## Functions

- `isWindowsDrivePath` (`packages/shared/src/path.ts:6`)
- `isUncPath` (`packages/shared/src/path.ts:10`)
- `isWindowsAbsolutePath` (`packages/shared/src/path.ts:14`)
- `isLocalAbsolutePath` (`packages/shared/src/path.ts:18`)
- `isExplicitRelativePath` (`packages/shared/src/path.ts:27`)
- `isNormalizedWindowsAbsolutePath` (`packages/shared/src/path.ts:43`)
- `localPathsEqual` (`packages/shared/src/path.ts:47`)
- `workspaceRelativePathOf` (`packages/shared/src/path.ts:74`)
- `joinWorkspaceRelativePath` (`packages/shared/src/path.ts:102`)
- `isWorkspaceRelativePathSafe` (`packages/shared/src/path.ts:111`)