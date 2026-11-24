# computergrants-rs

The macOS privacy grants desktop control needs, in the words the user reads. Three surfaces have to agree on this copy — the server backend's availability message, the desktop app's send-time preflight, and the web card and settings panel — and they used to spell it out separately, which is how "Screen Recording and Accessibility" and "Accessibility and Screen Recording" ended up describing the same state in the same session.

napi-rs bridge for `packages/shared/src/computerGrants.ts`.

## Functions

- `missingComputerAppSnapPermissions` (`packages/shared/src/computerGrants.ts:43`)
- `computerGrantsBlockControl` (`packages/shared/src/computerGrants.ts:75`)
- `sortComputerPermissions` (`packages/shared/src/computerGrants.ts:103`)
- `listComputerPermissions` (`packages/shared/src/computerGrants.ts:110`)
- `computerStaleGrantAdvice` (`packages/shared/src/computerGrants.ts:145`)
- `computerPermissionSetupMessage` (`packages/shared/src/computerGrants.ts:169`)