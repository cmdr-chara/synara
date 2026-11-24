# providerinstances-rs

FILE: providerInstances.ts Purpose: Shared provider-instance resolution from legacy and generic settings. Layer: Shared runtime utility Exports: provider instance derivation and start-option helpers

napi-rs bridge for `packages/shared/src/providerInstances.ts`.

## Functions

- `unresolvedAutomationInstanceId` (`packages/shared/src/providerInstances.ts:59`)
- `isUnresolvedAutomationInstanceId` (`packages/shared/src/providerInstances.ts:63`)
- `isProviderKind` (`packages/shared/src/providerInstances.ts:131`)
- `defaultInstanceIdForProvider` (`packages/shared/src/providerInstances.ts:135`)
- `providerProfileDirectoryEnvironment` (`packages/shared/src/providerInstances.ts:139`)
- `providerImportedDirectoryConfig` (`packages/shared/src/providerInstances.ts:157`)
- `inferLegacyProviderKindFromInstanceId` (`packages/shared/src/providerInstances.ts:167`)
- `inferLegacyProviderKindFromModel` (`packages/shared/src/providerInstances.ts:196`)
- `inferLegacyProviderKindFromModelSelection` (`packages/shared/src/providerInstances.ts:220`)
- `resolveModelSelectionInstanceId` (`packages/shared/src/providerInstances.ts:229`)
- `resolveProviderStatusInstanceId` (`packages/shared/src/providerInstances.ts:235`)
- `codexAccountInstanceId` (`packages/shared/src/providerInstances.ts:260`)
- `deriveProviderInstanceConfigMap` (`packages/shared/src/providerInstances.ts:375`)
- `deriveProviderInstances` (`packages/shared/src/providerInstances.ts:435`)
- `deriveUnsupportedProviderInstances` (`packages/shared/src/providerInstances.ts:465`)
- `resolveProviderInstance` (`packages/shared/src/providerInstances.ts:494`)
- `providerStartOptionsFromInstance` (`packages/shared/src/providerInstances.ts:525`)
- `mergeProviderStartOptions` (`packages/shared/src/providerInstances.ts:659`)