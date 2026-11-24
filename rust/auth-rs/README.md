# auth-rs

napi-rs bridge for `apps/server/src/auth/`.

## Functions

- `makeEffectAuthRequest` (`apps/server/src/auth/effectHttp.ts:5`)
- `authErrorResponse` (`apps/server/src/auth/effectHttp.ts:18`)
- `isTrustedLocalServerToken` (`apps/server/src/auth/remoteAccess.ts:9`)
- `requireRemoteSessionAdmission` (`apps/server/src/auth/remoteAccess.ts:38`)
- `resolveSessionCookieName` (`apps/server/src/auth/utils.ts:6`)
- `base64UrlEncode` (`apps/server/src/auth/utils.ts:13`)
- `base64UrlDecodeUtf8` (`apps/server/src/auth/utils.ts:18`)
- `signPayload` (`apps/server/src/auth/utils.ts:22`)
- `timingSafeEqualBase64Url` (`apps/server/src/auth/utils.ts:26`)
- `deriveAuthClientMetadata` (`apps/server/src/auth/utils.ts:76`)
