# executable-rs

FILE: executable.ts Purpose: Defines how a command name becomes a concrete executable on every platform. Layer: Shared platform runtime Depends on: node:fs and node:path only.

napi-rs bridge for `packages/shared/src/executable.ts`.

## Functions

- `envPathKeyFor` (`packages/shared/src/executable.ts:41`)
- `hasPathSeparator` (`packages/shared/src/executable.ts:56`)
- `windowsPathExtensions` (`packages/shared/src/executable.ts:60`)
- `pathEntries` (`packages/shared/src/executable.ts:73`)
- `executableNameCandidates` (`packages/shared/src/executable.ts:86`)
- `isExecutableFile` (`packages/shared/src/executable.ts:225`)
- `resolveExecutable` (`packages/shared/src/executable.ts:230`)
- `executableIdentity` (`packages/shared/src/executable.ts:322`)