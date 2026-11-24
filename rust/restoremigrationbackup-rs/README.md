# restoremigrationbackup-rs

Restore migration backup.

`output` crosses the bridge as JSON, so its methods are gone: only the `undefined`
marker (the default `console`) has a callable `warn`/`error`/`log`, and every call
on it is a no-op for the return value. `restoreMarkedMigrationBackup` lives in
`persistence/MigrationBackup.ts`, outside this unit; the CLI catches its every
rejection into exit code 1.

napi-rs bridge for `apps/server/src/restoreMigrationBackup.ts`.

## Functions

- `runRestoreMigrationBackupCli` (`apps/server/src/restoreMigrationBackup.ts:22`)