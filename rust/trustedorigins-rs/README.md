# trustedorigins-rs

FILE: trustedOrigins.ts Purpose: Shared origin checks for browser-facing HTTP/WS routes that expose          local machine data only to Synara's own app surfaces. Layer: Server HTTP/security utility Exports: normalizeCorsOrigin, isTrustedAppOrigin,          shouldRejectUntrustedRequestOrigin

napi-rs bridge for `apps/server/src/trustedOrigins.ts`.

## Functions

- `normalizeCorsOrigin` (`apps/server/src/trustedOrigins.ts:27`)
- `isTrustedAppOrigin` (`apps/server/src/trustedOrigins.ts:78`)
- `shouldRejectUntrustedRequestOrigin` (`apps/server/src/trustedOrigins.ts:96`)
- `shouldRejectAuthMutationOrigin` (`apps/server/src/trustedOrigins.ts:115`)
- `requiresWebSocketAuthentication` (`apps/server/src/trustedOrigins.ts:128`)