# Batch 53 — task notes reshaped to upstream `ThreadNotes`

## Upstream behavior

Upstream stores per-thread notes as a single freeform string
(`packages/contracts/src/orchestration.ts`: `ThreadNotes =
Schema.String.check(Schema.isMaxLength(THREAD_NOTES_MAX_CHARS))` with
`THREAD_NOTES_MAX_CHARS = 16_384`), rendered as a "Notepad" textarea in the
Environment panel (placeholder "Type here"). There is no per-task checklist
inside notes — upstream's checklist is the *pinned-messages* checklist, a
separate projection — and no saved folder-path list; notes never inject into
prompts (no prompt-assembly reference in `apps/server`).

## Port changes

- `crates/synara-workspace/src/storage/task_context.rs`: `TaskContext` is now
  `{version, revision, notes}` — the invented `checklist`/`ChecklistItem` and
  `folder_references` fields (plus `validate_folder_reference`,
  `as_prompt_context`, the folder/checklist prompt sections) are deleted.
  Notes cap becomes `MAX_NOTE_CHARS = 16_384` (chars, matching upstream's
  `maxLength`; JS counts UTF-16 units — surrogate pairs count 2 there, 1
  here — recorded divergence). `deny_unknown_fields` removed so stored v1/v2
  blobs still decode with the dropped keys ignored; `CURRENT_VERSION = 3`.
- `crates/synara-app/src/shell/saved_context.rs`: the dialog is notes-only —
  checklist rows/item editor/hide-done, folder picker, and the invented
  "Copy"/"Add to draft" prompt-insert buttons are gone. Placeholder matches
  upstream's "Type here"; header/button labels are "Notes".
- `crates/synara-app/src/shell/checkpoints.rs` + workspace checkpoints: copy
  and counts now describe draft + notes only.
- `crates/synara-app/src/shell/command_palette.rs`: "Chat notes and
  checklist" → "Chat notes".

## Decisions recorded

- Notes still save via the existing revision-checked write (optimistic
  concurrency is internal plumbing, not a user-visible invention).
- Upstream renders notes inside the Environment panel with debounced
  autosave; the port's notes surface remains its existing modal dialog with
  explicit Save. An Environment-panel port is its own epic — deferred, not
  part of this increment.
- `MAX_CONTEXT_BYTES` (256 KiB KV guard) is retained as storage plumbing.

## Gates

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked --workspace --all-targets`: no new warnings.
- `cargo test --locked --workspace`: green except the documented
  environment failure
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`.

## Live verification

Acceptance gesture: the task "Notes" dialog shows a single freeform notes
editor (placeholder "Type here") with Save/Reload — no checklist, no folder
picker, no insert/copy buttons; typed notes persist across reopen and the
stored blob has no checklist/folder_references keys.

Verified live on this box: opened the dialog on a chat, typed and saved
notes, reopened — notes restored, UI is notes-only, and
`task-context:<id>` in `preferences` re-encoded as
`{"version":3,"revision":N,"notes":"..."}` with the legacy keys dropped.
