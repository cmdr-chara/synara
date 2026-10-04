# Parity continuation verification - batch 45

Date: 2026-09-26

## 1:1-parity correction: Hubs overlay slimmed to upstream studio-project shape

Upstream has no new commits past the `a33435c18` (v0.9.2) reference, so this
batch works priority item (c): Rust-only inventions that diverge from upstream.

### What upstream does (reference: `Emanuele-web04/synara@a33435c18`)

- `OrchestrationProject` is the grouping primitive
  (`ProjectKind = ["project","chat","studio"]` in
  `packages/contracts/src/project.ts`); a project is just `{id, kind, name}`
  plus joins to threads.
- Project instructions live in a web-local zustand store
  (`projectInstructionsStore.ts`), edited in the environment panel's
  `EnvironmentProjectInstructionsSection` and **seeded into a new thread's
  `notes`** by `mergeProjectInstructionsIntoThreadNotes` when a turn executes
  (`apps/web/src/...`). They are never merged into composer drafts.
- Thread notes are `Schema.optional(ThreadNotes)` on `OrchestrationThread`
  capped at `THREAD_NOTES_MAX_CHARS = 16_384`
  (`packages/contracts/src/orchestration.ts`), set via
  `ThreadMetaUpdateCommand`.
- `automation.ts` has **no context policy**: an automation run's prompt is the
  stored instructions verbatim in the ordinary project scope.

### What the port had (removed)

The `HubProfile` overlay carried invented fields with no upstream counterpart:
a `description`, a curated `memory` ("shared Hub knowledge"), an
`include_in_new_threads` toggle that injected `## Hub instructions` into
composer drafts, an `archived` flag with archive UI, a
promote-message-to-knowledge composer action, an
`AutomationContextPolicy` (project-vs-hub run scope with `hub_revision`
snapshots and Studio-scope tasks), and `main_task` wiring. None of these exist
upstream, so they are deleted.

### What the port now does (kept, upstream-shaped)

- `HubProfile` is `{version, revision, project, name, instructions}` — the
  project's name plus the instructions text that seeds each new thread's saved
  notes (`task-context:{task}` `TaskContext.notes`), matching upstream's
  instructions → thread-notes merge. Legacy v1 rows decode through a lenient
  fallback that folds `memory` into `instructions` so no user text is lost.
- `create_hub_thread` creates an empty-draft Studio-scope thread;
  `create_hub_task` creates one carrying the user's own reviewed draft text;
  both write `TaskContext.notes = instructions` inside the same transaction.
- The editor is two fields — Name and Instructions — labelled "Hub settings",
  with copy matching upstream ("Project instructions are saved to each new
  thread's notes").
- Automations: `AutomationDefinition.context` and `AutomationRun.hub_revision`
  are retained only as `Option<serde_json::Value>` tombstones
  (`deny_unknown_fields` structs must still decode rows written by older
  versions); the claimed run's prompt is the instructions verbatim in
  `TaskScope::Project`.

### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: 14 unique warning sites,
  byte-identical to committed HEAD (verified against a `git worktree` check of
  HEAD; the one `large_enum_variant` that moved — `AutomationRun` grew past the
  lint's diff threshold — is resolved by boxing `Pending::Recover`).
- `cargo test --locked --workspace` (skipping the known env-only
  `discovery_does_not_change_worktree_or_index`): green. During one heavily
  loaded parallel run three `integrations::probe` tests and one `synara-acp`
  process-integration test timed out on loopback-socket `WouldBlock` while a
  second full build ran concurrently; all pass in isolation and in a clean
  solo suite run (355/0 workspace lib), and the touched code is unrelated to
  those fixtures.
- Storage tests cover: verbatim-prompt claim under legacy `context:"hub"`
  rows, `task-context` seeding, draft validation, and v1→v2 decode.
- Live UI check: verified in `cargo run -p synara-app` on an isolated
  `--data-dir` (recording `rec-0b0c512f`). Hub detail shows threads sorted by
  recency + "Instructions · {bytes} · Revision {N}" caption with no archive /
  "Use in new threads" / description / "Shared context" UI (`ss_799254cc`).
  Hub settings editor has only Name + Instructions and saves with the toast
  "Hub settings saved. New threads inherit the instructions in their notes."
  (`ss_3855e56f`). New hub threads open with an empty composer; instructions
  land in `task-context:{id}` (`ss_f2bf32ba`). Automations: no Run context
  row / Context segment / Context policy anywhere (`ss_d77346f4`,
  `ss_82b98637`, `ss_99739017`). Kanban: enabled Run/Stop, no archived banner,
  empty-hub copy "Select a Hub before creating a task." (`ss_27060c8e`,
  `ss_f8bb0d1e`). Studio message actions are Copy/side-chat/Pin/Reply only —
  no hub-knowledge button (`ss_74e9218d`, `ss_zoom_badc84e1`).

### Known parity gaps recorded

- `TaskContext.notes` still caps at 128 KiB vs upstream's 16,384-char thread
  notes cap; deferred with the checklist-removal/env-panel reshape.
- Upstream edits instructions in the environment panel; the port's Hub
  settings editor is the equivalent surface.
