# Parity continuation verification - batch 44

Date: 2026-09-26

## 1:1-parity correction: Debug interaction mode replaces invented evidence journal

Upstream has no new commits past the `a33435c18` (v0.9.2) reference, so this
batch works priority item (c): Rust-only inventions that diverge from upstream.

### What upstream does (reference: `Emanuele-web04/synara@a33435c18`)

- `ProviderInteractionMode = Schema.Literals(["default","plan","debug"])` lives
  on `OrchestrationThread.interactionMode`
  (`packages/contracts/src/orchestration.ts`).
- `withProviderDebugModePrompt` in `apps/server/src/provider/debugMode.ts`
  prepends a verbatim `<synara_debug_mode>...</synara_debug_mode>` instruction
  block (observe → reproduce → investigate → fix → verify) to provider-bound
  prompts when the thread's interaction mode is `debug`.
- `/debug` and `/default` are both advertised slash commands
  (`packages/shared/src/composerSlashCommands.ts`).
- The composer extras panel shows a "Debug mode" toggle row with a check state
  (`ComposerExtrasPanel.tsx`); the mode persists across drafts, turns, forks and
  restarts and grants no extra permissions.

### What the port had (removed)

`crates/synara-workspace/src/storage/debug_workflow.rs` and
`crates/synara-app/src/shell/debug_workflow.rs` implemented an invented
evidence-first journal: a `DebugWorkflow` record with `DebugPhase`
(Observe/Reproduce/Fix/Verify) transitions, `DebugEdit` ledgers, phase prompts,
an accordion debug bar, and ~15 call sites gating checkpoints, follow-ups,
goals and the recap. No upstream counterpart exists — no `debugWorkflow`
model, no evidence-phase API — so the journal is deleted, not kept.

### What the port now does (added)

- `crates/synara-workspace/src/storage/interaction_mode.rs`:
  `InteractionMode { Default, Debug }` persisted per task under the
  `task-interaction-mode:{task}` preference key; `DEBUG_MODE_PROMPT_PREFIX`
  reproduces the upstream `<synara_debug_mode>` block verbatim;
  `with_debug_prompt` prepends it for Debug-mode submissions (idempotent).
  The task-delete cascade and `valid_preference_key` allowlist cover the new
  key (and keep `task-debug:` for legacy row cleanup).
- `WorkspaceController::submit_prompt_owned` applies `with_debug_prompt` after
  input validation, so every submit path — ACP or direct-model — carries the
  prefix while transcript text stays clean.
- Shell: `Update::DebugMode`, `debug_tasks: HashSet<TaskId>` hydrated per
  selected task, `load_debug_mode`/`set_debug_mode`/`debug_mode_reply`,
  `/debug` and `/default` slash entries, a "Debug mode" row in the composer
  mode menu with the persisted check state, a command-palette action, and a
  "Debug" badge chip on the workflow strip whose click returns the mode to
  Default — matching upstream's badge toggle.

### Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: back to baseline (8
  app-shell + 4 workspace pre-existing warnings; none new).
- `cargo test --locked --workspace`: green (3 new storage tests cover
  default/persistence/cross-task isolation, unknown/archived-task rejection,
  and byte-exact prefix). One flaky `synara-runtime` owner-lock test failed
  once on timing, passed 96/0 on rerun; the crate is untouched by this batch.
  The known gitconfig-proxy failure
  (`discovery_does_not_change_worktree_or_index`) remains env-only.
- Live UI check (real GPUI app, isolated data dir, debug build): the port
  namespaces native commands — the live syntax is `/synara/debug` and
  `/synara/default`; bare `/debug` is deliberately not intercepted and sends
  as a normal prompt (verified: no command menu, no badge). With the native
  prefix: `/synara/debug` renders the "Debug" badge chip and writes
  `task-interaction-mode:{task}` = `"debug"`; the extras menu "Debug mode"
  row shows its checkmark; the badge survives a task switch away and back;
  `/synara/default` and clicking the chip both clear it; a normal prompt
  submits with no badge; the deleted phases accordion no longer renders.
  Recording `rec-1445eb6b-…-edited.mp4` and screenshots under
  `~/screenshots/` capture each gesture.

### Known parity gaps recorded

- Upstream persists interaction mode on the orchestration thread and copies it
  through forks/handoffs; the port keys it per task and has no fork-copy step —
  a task fork starts in Default until the user re-enables Debug.
- Upstream `/default` also returns the provider session to its advertised
  default session mode; the port now mirrors that where the session advertises
  a "default" mode choice.
