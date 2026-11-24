# filesystemplatform-rs

FILE: filesystemPlatform.ts Purpose: Centralizes filesystem operations whose durability or identity semantics differ by OS. Layer: Shared platform runtime

napi-rs bridge for `packages/shared/src/filesystemPlatform.ts`.

## Functions

- `supportsPosixPermissions` (`packages/shared/src/filesystemPlatform.ts:11`)
- `syncDirectoryEntry` (`packages/shared/src/filesystemPlatform.ts:16`)
- `syncRegularFile` (`packages/shared/src/filesystemPlatform.ts:40`)
- `sameFileIdentity` (`packages/shared/src/filesystemPlatform.ts:56`)
- `isMacAppBundlePath` (`packages/shared/src/filesystemPlatform.ts:69`)