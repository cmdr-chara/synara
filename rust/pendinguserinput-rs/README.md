# pendinguserinput-rs

FILE: pendingUserInput.ts Purpose: Normalize draft answers and progress for pending user input prompts. Layer: Web chat state utility Exports: Draft answer helpers and progress derivation used by ChatView/composer panels.

napi-rs bridge for `apps/web/src/pendingUserInput.ts`.

## Functions

- `resolvePendingUserInputAnswer` (`apps/web/src/pendingUserInput.ts:50`)
- `setPendingUserInputCustomAnswer` (`apps/web/src/pendingUserInput.ts:67`)
- `togglePendingUserInputOptionSelection` (`apps/web/src/pendingUserInput.ts:83`)
- `buildPendingUserInputAnswers` (`apps/web/src/pendingUserInput.ts:108`)
- `hasCompletePendingUserInputAnswers` (`apps/web/src/pendingUserInput.ts:125`)
- `omitNullPendingUserInputAnswers` (`apps/web/src/pendingUserInput.ts:144`)
- `derivePendingUserInputProgress` (`apps/web/src/pendingUserInput.ts:161`)
- `splitRecommendedOptionLabel` (`apps/web/src/pendingUserInput.ts:197`)