# Migration ledger review — data preservation, replay safety, backup/recovery

Review of the Rust migration ledger (`src/migrations.rs`, `src/migrations/`,
146 migrations ported from the upstream TypeScript effect Migrator) against
the three axes AGENTS.md requires. Independence caveat, stated plainly: the
port and this review share an author, so this document is the structured
self-audit — what it claims is written so each claim can be falsified against
the sources it cites. A truly independent pass remains welcome.

## Scope and method

Every one of the 146 migrations was classified by control flow during the
port audit (see commit history around `7ad2dafbec`):

- **Pure additive** (the majority): CREATE TABLE/INDEX IF NOT EXISTS, ALTER
  ADD COLUMN guarded by table-info inspection. No destructive potential.
- **Conditional backfill** (023, 032): data-dependent UPDATE/INSERT logic —
  both hand-ported with the exact upstream control flow.
- **Rebuild migrations** (005, 027, 044, 055, 057, 058, 062, 146 and related):
  table rebuilds with copy-in/copy-out — the only destructive-capable class,
  each audited individually below.

## Data preservation

The destructive rebuilds follow the upstream pattern: create the new table,
copy rows forward with explicit column lists, drop the old table, rename.
The audit verified, migration by migration:

- Column lists in copy statements cover every column the old schema could
  carry at the point the migration runs — nothing the user wrote is dropped
  silently. Where upstream widened columns later, the widened column set is
  what the copy targets.
- 057/058/062/146 carry **pre-state and post-state guards**: the rebuild
  aborts when the incoming schema is not the expected one (columns missing
  or extra), and the post-state marker prevents the destructive half from
  running against a database that has already moved past it. This is the
  exact upstream guard semantics restored after the mechanical translation
  had dropped them.
- 023 (automation identity rebuild) and 032 (repair path) preserve the
  conditional backfill: rows are migrated with their upstream transformation
  (identity resolution fail-closed in 121/122 — rows that cannot resolve are
  not guessed at).
- Known residual risk, documented in the original port commit: guard
  evaluation for ALTER/CREATE INDEX is lenient rather than upstream's exact
  conditional replay. On fresh databases and fully migrated ones — the two
  states that exist in practice, since the Rust backend is the first writer
  of every database it opens — leniency is behaviorally identical. A
  database carried forward from the TypeScript server mid-lineage would
  observe different guard behavior on the minority of statements where the
  guard is data-dependent; the canonical-lineage validation (below) refuses
  to serve divergent databases before that matters.

## Replay safety

- The effect Migrator semantics are reproduced exactly: tracker rows for the
  whole pending batch are inserted inside the single migration transaction
  before any migration body runs, so a second process racing the first
  fails the insert (primary-key conflict = "already running") rather than
  double-applying. A crash mid-batch rolls back the transaction including
  the tracker rows — nothing is half-applied.
- Tracker DDL matches the effect dialect (`migration_id integer PRIMARY KEY
NOT NULL`, `created_at datetime NOT NULL DEFAULT current_timestamp`,
  `name VARCHAR(255) NOT NULL`), pinned by test.
- Idempotence verified live: a second run against a migrated database
  applies zero migrations; the fresh-run path applies 146 through 146 and
  produces the expected 79 tables and 174 indexes.
- The destructive rebuilds are the replay-critical surface: without guards,
  re-running one against a post-rebuild database could copy settled rows
  into a schema they no longer fit. The pre/post-state guards make replay a
  controlled refusal instead of silent corruption.
- `sql.unsafe()` statements that the first mechanical translation silently
  dropped (14 guarded ALTERs across 076/080/109–114, the DDL of 115, the
  repair logic of 032) were restored in `7f1c532aa6` — found because the
  live shell booted against a migrated database and failed projection
  queries on the missing columns. The class of bug (translator blindness to
  non-template SQL) is closed by construction now: no `// MANUAL` markers
  remain and every migration was diffed against its upstream source.

## Lineage and divergence

- `validate_lineage` enforces the canonical private account lineage (the 10
  account-private migrations) before any migration runs: a database whose
  tracker disagrees with the canonical chain is refused, matching upstream's
  fail-closed posture — the user restores a backup rather than booting
  against a schema the application cannot trust.
- Migration ids are contiguous and ordered, pinned by test.

## Backup and recovery

- Before this review, the Rust runner had **no** pre-migration backup — the
  largest honest gap this review surface found, now closed: a consistent
  copy through sqlite's backup API (safe with a live WAL) is written to
  `userdata/migration-backups/` before any pending migration batch runs,
  rotated at upstream's retention of five, skipped when the database is
  already current. Verified live: fresh-home boot produced a backup that
  opens cleanly.
- Recovery path: stop every Synara process, copy the chosen backup over
  `state.sqlite`. Upstream's operator-experience layers — provenance
  documents, free-space preflight, failure bundles, the guided restore CLI —
  are documented in the port commit as not yet ported; they improve the
  recovery experience but do not change the safety invariant, which the
  plain copy satisfies.
- The in-transaction runner means a failed migration never needs the backup
  for consistency — the database is byte-identical to its pre-run state.
  The backup exists for the failure modes transactions cannot cover:
  corruption discovered later, operator error, disk-level damage.

## Verdict

The ledger is safe to boot against real user data within its documented
envelope: databases the Rust server creates and owns. The two caveats a
reviewer must carry forward are (1) lenient guard evaluation diverges from
upstream on mid-lineage databases carried over from the TypeScript server —
refuse or diff such databases rather than trusting the lenient path — and
(2) the missing operator-experience backup layers deserve a follow-up port
if the Rust backend becomes the primary surface.
