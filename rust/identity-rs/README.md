# identity-rs

napi-rs bridge for `apps/api/src/identity/`.

## Functions

- `hostEnvironmentLockKey` (`apps/api/src/identity/environmentLock.ts:5`)
- `createHostGrantIssuer` (`apps/api/src/identity/grantIssuer.ts:10`)
- `hostRecordFromRow` (`apps/api/src/identity/hostKeyRegistry.ts:53`)
- `isHostProofAuthorization` (`apps/api/src/identity/hostKeyRegistry.ts:69`)
- `toAccountHost` (`apps/api/src/identity/hostRecords.ts:7`)
- `deleteHostSecretRows` (`apps/api/src/identity/hostSecretStore.ts:46`)
- `createHostSecretStore` (`apps/api/src/identity/hostSecretStore.ts:71`)
- `clearOrgCache` (`apps/api/src/identity/orgProvisioning.ts:62`)
- `invalidateOrgCacheForOrganization` (`apps/api/src/identity/orgProvisioning.ts:79`)
- `personalOrgName` (`apps/api/src/identity/orgProvisioning.ts:88`)
- `writeRevocationEvents` (`apps/api/src/identity/revocationLog.ts:16`)
- `verifyBoundedJwt` (`apps/api/src/identity/signing.ts:66`)
- `verifyJwtWithEmbeddedJwk` (`apps/api/src/identity/signing.ts:90`)
- `publicJwkThumbprint` (`apps/api/src/identity/signing.ts:105`)
- `createApiSigningService` (`apps/api/src/identity/signing.ts:168`)
- `clearUserCache` (`apps/api/src/identity/userCache.ts:111`)
- `invalidateUserCache` (`apps/api/src/identity/userCache.ts:119`)
- `userCacheSize` (`apps/api/src/identity/userCache.ts:128`)
- `classifyMagicAuthFailure` (`apps/api/src/identity/workos.ts:242`)
- `classifyAuthorizationCodeFailure` (`apps/api/src/identity/workos.ts:296`)
- `parseWorkosIdentityMap` (`apps/api/src/identity/workosIdentityMap.ts:15`)
