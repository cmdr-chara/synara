# jwtclaims-rs

FILE: jwtClaims.ts Purpose: Read the payload claims of a JWT that a provider CLI stores. Nothing   here checks the signature, so the claims must never decide authorization.

napi-rs bridge for `apps/server/src/jwtClaims.ts`.

## Functions

- `decodeJwtClaims` (`apps/server/src/jwtClaims.ts:9`)