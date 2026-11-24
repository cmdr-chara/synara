# providerupdates-rs

FILE: providerUpdates.ts Purpose: Shared provider-update filtering and refresh cadence for global toasts and settings. Layer: Web settings/notification utility Exports: update candidate helpers, notification keys, and auto-refresh timing.

napi-rs bridge for `apps/web/src/providerUpdates.ts`.

## Functions

- `isProviderUpdateActive` (`apps/web/src/providerUpdates.ts:76`)
- `isProviderLatestVersionKnowable` (`apps/web/src/providerUpdates.ts:83`)
- `shouldOfferProviderUpdateAction` (`apps/web/src/providerUpdates.ts:87`)
- `shouldPromptProviderUpdate` (`apps/web/src/providerUpdates.ts:98`)
- `shouldShowProviderUpdateStatus` (`apps/web/src/providerUpdates.ts:127`)
- `getVisibleProviderUpdateStatuses` (`apps/web/src/providerUpdates.ts:148`)
- `getNotifiableProviderUpdateStatuses` (`apps/web/src/providerUpdates.ts:164`)
- `providerUpdateNotificationKey` (`apps/web/src/providerUpdates.ts:173`)