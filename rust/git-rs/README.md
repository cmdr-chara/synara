# git-rs

Sanitize an arbitrary string into a valid, lowercase git branch fragment. Strips quotes, collapses separators, limits to 64 chars.

napi-rs bridge for `packages/shared/src/git.ts`.

## Functions

- `sanitizeBranchFragment` (`packages/shared/src/git.ts:24`)
- `sanitizeFeatureBranchName` (`packages/shared/src/git.ts:46`)
- `resolveAutoFeatureBranchName` (`packages/shared/src/git.ts:101`)
- `buildSynaraBranchName` (`packages/shared/src/git.ts:112`)
- `resolveUniqueSynaraBranchName` (`packages/shared/src/git.ts:128`)
- `isTemporaryWorktreeBranch` (`packages/shared/src/git.ts:136`)
- `buildTemporaryWorktreeBranchName` (`packages/shared/src/git.ts:145`)
- `resolveThreadBranchRegressionGuard` (`packages/shared/src/git.ts:152`)