# composerdraftmodels-rs

FILE: composerDraftModels.ts Purpose: Normalizes provider-scoped model selections and resolves effective composer models. Exports: Model state helpers used by persistence, actions, and the public facade.

napi-rs bridge for `apps/web/src/composerDraftModels.ts`.

## Functions

- `normalizeProviderKind` (`apps/web/src/composerDraftModels.ts:154`)
- `normalizeProviderInstanceId` (`apps/web/src/composerDraftModels.ts:172`)
- `providerInstanceModelSelectionKey` (`apps/web/src/composerDraftModels.ts:177`)
- `modelSelectionStorageKey` (`apps/web/src/composerDraftModels.ts:184`)
- `readModelSelectionForProviderInstance` (`apps/web/src/composerDraftModels.ts:188`)
- `normalizeModelSelectionMapByInstance` (`apps/web/src/composerDraftModels.ts:196`)
- `makeModelSelection` (`apps/web/src/composerDraftModels.ts:216`)
- `normalizeProviderModelOptions` (`apps/web/src/composerDraftModels.ts:323`)
- `normalizeModelSelection` (`apps/web/src/composerDraftModels.ts:529`)
- `reconcileProviderScopedModelSelection` (`apps/web/src/composerDraftModels.ts:612`)
- `stripNonStickyModelOptions` (`apps/web/src/composerDraftModels.ts:674`)
- `sanitizeStickyModelSelectionMap` (`apps/web/src/composerDraftModels.ts:695`)
- `legacySyncModelSelectionOptions` (`apps/web/src/composerDraftModels.ts:711`)
- `legacyMergeModelSelectionIntoProviderModelOptions` (`apps/web/src/composerDraftModels.ts:731`)
- `legacyToModelSelectionByProvider` (`apps/web/src/composerDraftModels.ts:763`)
- `deriveEffectiveComposerModelState` (`apps/web/src/composerDraftModels.ts:792`)