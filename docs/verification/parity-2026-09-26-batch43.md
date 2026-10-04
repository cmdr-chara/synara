# Parity continuation verification - batch 43

Date: 2026-09-26

## Upstream-delta audit: `eaa61ed..a33435c18` (v0.9.2)

Twenty-two upstream commits landed after the pinned reference
`Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`. Each commit was
read against the Rust/GPUI workspace; four contained product changes that apply
to this port and are ported below, and the remaining eighteen are confirmed
already-equivalent, architectural non-issues, or repository/CI work with no Rust
counterpart. The upstream reference line in ROADMAP.md advances to
`a33435c18474eb7816582004e45f87382965ac8d`.

## Delivered

### Tool-approval credential redaction (upstream `b8142ebce`)

`reviewed_tool_input` in `crates/synara-acp/src/wire.rs` previously redacted a
narrow substring match set. Upstream's reviewed-input policy is ported
faithfully: an exact normalized alphanumeric-lowercase sensitive-key table, a
camelCase/snake_case tokenizer, terminal-word rules, the qualifier gate for
`*_token(s)` fields, and the `isProviderCredentialKey` environment-key set
(trimmed uppercase env names). Non-sensitive upstream keys such as
`prompt_tokens`, `total_tokens`, `csrf_token` and `github_token` stay visible,
matching upstream byte-for-byte on the shared cases. Rust retains its
additional whole-field redaction for `env`/`environment`/`headers` objects as a
documented strict superset. Dedicated tests pin every upstream table entry and
the non-redaction cases.

### Opt-in managed-worktree removal on Archive (upstream `3bc6468e8`)

Settings → Worktrees gains a "Delete worktree on archive" toggle backed by the
new persisted `general.delete_worktree_on_archive` preference (default off,
matching upstream). When enabled, `archive_task` runs
`cleanup_archived_managed_worktree` after the archive marker commits: the
managed worktree is removed only when it still carries this task's
`synara-managed` marker, still reports the task's branch, is not bare/locked/
prunable, has a clean checkout, and no other task — including other archived
siblings — uses it or has it assigned. Removal goes through the existing gated
`GitOperation::RemoveWorktree` and is followed by a best-effort
`git worktree prune` via the new `GitOperation::PruneWorktrees`. The task's
`synara/*` branch is always retained for recovery. Re-archiving an archived
task is idempotent, so a blocked cleanup can be retried by toggling Archive
again.

Upstream divergences are deliberate: upstream can also discard the archived
task's snapshot and close terminal sessions — this workspace has no snapshot
subsystem and terminals close with the session — and Rust performs the cleanup
synchronously inside `archive_task` instead of a queued effect, re-verifying
the marker at removal time rather than revalidating an archiveSequence.

### Persisted provider model catalog (upstream `c49ad1ad0`)

The direct-model catalog fetched from models.dev is now persisted across
restarts: `WorkspaceService::save_provider_catalog_snapshot` stores the raw
reviewed JSON payload plus a fetch timestamp under the whitelisted
`direct-model-catalog-v1` preference (8 MiB encode cap). Opening the
direct-model section hydrates and re-parses the snapshot through the same
`parse_catalog` path as a live fetch, and the section header shows the
snapshot's age ("snapshot saved … ago"). Only an explicit "Refresh catalog"
contacts models.dev.

Architectural differences are documented rather than forced: upstream persists
a parsed catalog projection under `stateDir/provider-models/catalogs.json` with
a write-coalescing queue and shutdown finalizer; Rust stores the immutable
source payload atomically in SQLite preferences and re-parses on open, so a
stored snapshot can never be authoritative over a schema the parser no longer
accepts. Upstream's `omp` cache bypass is not applicable — Rust has no
server-side per-request model-role re-resolution.

### Oh My Pi agent provider (upstream `48791a304`)

`default_profiles()` gains `omp` — name "Oh My Pi", launching `omp acp`. The
onboarding sign-in guide now recognizes Oh My Pi profiles: it explains that OMP
owns its credentials under `~/.omp` and its model routing, instructs the user
to run `omp` interactively to sign in, then verify `omp models --json` lists at
least one model before reconnecting, and links `trysynara.com/docs/providers/omp`.
`provider_login_command` only emits the `omp` executable for an exact
known-binary match, so a lookalike profile (e.g. "Oh My Pi clone" running `pi`)
gets no guide and no command.

Scope notes: OMP ships in the Beta lane upstream but is a normal default
profile here because this port has no Beta/Stable channel split; existing
saved `agent_profiles` lists are user-owned and are not rewritten — users opt
in by adding the profile in settings; and OMP renders with the generic agent
glyph since there is no vendored logo asset and only known identities get
provider branding.

## Confirmed not applicable / already equivalent

- `d28bdddc0` (scope Claude session-grant widening): the Rust permission
  surface offers only Allow once / Deny — there is no `acceptForSession`-style
  widening for any request kind, so nothing can over-widen.
- `5a5d7ceb2` (transcript scroll ownership reset on thread switch): React
  effect-ordering fix; the GPUI transcript's `TranscriptState::sync` already
  re-anchors to the tail on every thread switch
  (`following = !same_thread || self.is_following()`).
- `f86e8f078` (onboarding "Not installed" detection race): upstream added async
  provider detection state to avoid flashing not-installed; Rust probes the
  configured command synchronously at render (`command_found`), so there is no
  async gap to race.
- `0a6d1f506` (settle restart-orphaned turns before reactors start): upstream
  reordered startup because background reactors could read a stale running turn
  before reconciliation; Rust's `recover_interrupted` runs synchronously inside
  `WorkspaceService::open`/`main` bootstrap before any session can start —
  both entry points already enforce this ordering and the pending-permission
  row lifecycle is covered by `interrupted_permissions_do_not_reappear_as_actionable_after_restart`.
- `be86899e1` (consent card before Computer foreground takeover): upstream's
  agent could raise the user's desktop based on message text, needing phrase
  matching plus an approval card. Rust Computer Use has no message-driven
  foreground path: control exists only through an explicit per-task user window
  selection (15-minute lease, revocable, Synara's own window refused), and each
  input requires a fresh byte-identical observation.
- `54740148f` (updater button latch): the Rust updater is the runtime
  transaction engine (`UpdateHandoff`) driven by a launcher/helper; there is no
  updater sidebar/button surface to latch.
- `82c246b49` (blank provider path crashes project import): upstream's
  server-settings `binaryPath` fields have no Rust counterpart — agent profiles
  are validated named profiles, and import paths go through the canonicalized
  `WorkspaceFs`.
- `ca406d7f1` (mixed `\\?\` verbatim path forms on Windows): Rust canonicalizes
  the workspace root once at `WorkspaceFs::open` and derives every comparison
  from that canonical form; mixed verbatim/normal forms cannot coexist.
- `b307957c5` (harden stdio/frame transports): upstream's Codex app-server
  framer drops invalid-UTF-8 lines and resyncs because subprocesses share that
  pipe. Rust ACP owns its spawned-agent stdout exclusively and fails closed on
  malformed or oversized frames — a tested contract
  (`stdout_contamination_malformed_json_and_invalid_utf8_fail_closed`) kept as
  the deliberate security boundary. The `ByteAccumulator` change is a
  quadratic-concat fix; Rust's `Frames` is a single amortized `Vec<u8>`.
  The `FrameTransport` keyframe-classifier injection has no counterpart — the
  Rust port has no frame transport or keyframe priming surface.
- `c140f797f` (editor detection blocking the Windows event loop):
  single-threaded Electron main fix; Rust runs detection-free tokio workers.
- `847c03307` (Pi SDK pin alignment): no Pi provider in the Rust port.
- `533832287`, `85a1717fa`, `db4cadb05`, `765f5d3fe`, `afc1d351d`,
  `708c513a7`, `a33435c18`: upstream repository/CI/release-lane/icon/version
  changes with no Rust product-code counterpart.

## Inventory effect

- Upstream-delta backlog after the pin: 22 -> 0 (4 ported, 18 confirmed
  not applicable or already equivalent).
- Shipped feature slices 98 -> 102.
- The pinned upstream reference in ROADMAP.md advances to
  `a33435c18474eb7816582004e45f87382965ac8d` (v0.9.2).
