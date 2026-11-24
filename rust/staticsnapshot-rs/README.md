# staticsnapshot-rs

FILE: staticSnapshot.ts Purpose: Materializes a real-on-disk snapshot of static assets that live inside          an asar archive, so serving survives the archive being replaced on disk. Layer: Shared runtime utility (desktop main + server startup)  Electron caches an asar's header per process.

napi-rs bridge for `packages/shared/src/staticSnapshot.ts`.

## Functions

- `findAsarArchivePath` (`packages/shared/src/staticSnapshot.ts:29`)
- `snapshotDirectoryName` (`packages/shared/src/staticSnapshot.ts:39`)
- `ensureStaticSnapshot` (`packages/shared/src/staticSnapshot.ts:101`)