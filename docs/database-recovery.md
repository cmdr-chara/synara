# Database backup and recovery

## Explicit operations

The UI/controller can call `WorkspaceService::backup_to(destination, options)` and
`WorkspaceService::restore_to(backup, destination, options)`. Both execute on a
blocking storage worker, not the rendering thread. The corresponding synchronous
`Store` APIs are for worker-side use only.

Backup uses the SQLite online backup API, including committed WAL contents. It is
not a copy of only the main database file. A successful artifact is self-contained
and needs no WAL/SHM sidecars. A concurrent writer may advance the original after
the snapshot. The snapshot itself must remain internally consistent.

Restore requires a new absolute destination path whose parent already exists.
It never overwrites the active database, an inaccessible/corrupt database, an
existing backup or an unrelated file. Keep the original for diagnosis. Opening the
new database is a separate user action. Creating the artifact does not open a
workspace, start an agent, send a saved prompt or resume a protocol session.

Recovery returns a receipt containing the resolved destination, byte count,
SHA-256 fingerprint and schema version. The fingerprint detects byte changes when
compared with a retained receipt. It is not authentication, a signature or proof
of origin. Database content can include private conversations and paths. Export is
local and explicit. Do not upload artifacts or include them in diagnostic bundles
without separate, informed authorization.

## Validation and compatibility

Restore accepts the application's known SQLite schemas 1, 2 and 3. Older accepted
schemas are migrated in the private staging copy. The source is opened read-only
and is never migrated in place. A newer schema is rejected rather than guessed or
downgraded. Schema 0, foreign databases, unrecognized schema objects, triggers,
views and altered table/index definitions are rejected. This intentionally rejects
manually modified databases, even when they are otherwise valid SQLite files.

Validation includes `quick_check`, `foreign_key_check`, bounded typed JSON,
workspace/project/task ownership, task/thread identity, ordered event IDs and
sequences, per-thread byte/event limits, head totals and activity/catalog metadata.
Import does not grant permission to execute a stored command profile or trust a
remote host. Ordinary profile, connection and host-trust checks still apply after
an explicit open.

OS directory aliases are resolved before opening the database. The database leaf
itself is never canonicalized or followed as a symlink. Selected workspace-root
aliases likewise remain bound to the opened directory handle, not to a later
replacement of the alias. This fixes macOS `/var` aliases without weakening leaf
or contained-directory protections.

## Direct models and imported history

The known preference validators include direct-provider metadata, task-scoped
model bindings, related-conversation origins and the bounded history-import ledger.
Backups retain references, reviewed schemas and visible imported text, never
resolved OS key bytes. Import receipts continue preventing duplicates after
restore, including receipts whose imported conversation was deleted. An imported
history snapshot or a reviewed continuation does not acquire a provider session,
permission decision or automatic send through restoration.

See [Project Import](ui/project-import.md), [provider continuation](ui/provider-handoff.md)
and [direct models](ui/direct-models.md) for their narrower contracts.

## Limits and failure behavior

Default maximum artifact size is 1 GiB and default operation deadline is 60 seconds.
Callers can lower either limit, with a maximum deadline of five minutes. Incremental
copy, validation queries and file-copy hashing check a cancellation token. Query
progress hooks interrupt long-running validation. A completed publish is not
retroactively reported as cancelled. Filesystem/kernel calls cannot provide a hard
real-time deadline on a failed or stalled storage device.

A private same-filesystem staging directory owns intermediate SQLite sidecars.
The final artifact is synchronized and atomically published without replacing an
existing destination. Unix parent-directory synchronization follows publication.
Before publication, error/cancellation removes staging data. A filesystem error
reported after publication can leave a complete destination file. Inspect the
file rather than retrying with an overwrite. Existing source/destination files are
never deleted to make a retry succeed.

SQLite-full tests use `max_page_count` on a disposable database, not exhaustion of
the developer's disk. They prove event/head/activity/task updates roll back together.
Corrupt or inaccessible database opens continue returning an error, never an empty
replacement catalog.

## Evidence

Focused command with the repository's pinned Rust toolchain:

```sh
cargo +1.98.1 test --locked -p synara-workspace storage::recovery
cargo +1.98.1 test --locked -p synara-runtime root_alias_tests
```

Portable tests cover snapshot/reopen, live WAL, exact event identity, stored session
references/settings, concurrent writes, cancellation/deadline/size rejection,
old-schema migration without source modification, no-clobber collisions, malformed
schemas/JSON, symlink refusal, inaccessible input and SQLite-full rollback. The
service test verifies that restore returns a file receipt, not a running workspace.
Native backend CI must pass separately on Linux, macOS and Windows before claiming
cross-platform acceptance. This is not GUI, keychain, device or release acceptance.
