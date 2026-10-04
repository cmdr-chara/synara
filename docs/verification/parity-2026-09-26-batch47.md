# Parity batch 47 — Editor-mode fields no longer collapse (2026-09-26)

Upstream reference: `a33435c18` (unchanged).

## Defect

`EntryMode::Editor` `TextEntry` fields (Notes dialog, automation
instructions, goal editor, handoff forms, PR fix box, PDF fields, theme
editor, follow-up editor, revision editor, recap) rendered as thin empty
rectangles: the editor applied `h_full()`, which resolves to ~0 inside
auto-height dialog parents (e.g. `saved_context.rs` wraps the notes entry in
`div().relative()` with no height), collapsing the box to padding and culling
every shaped line in the paint loop.

## Fix

`input.rs`: Editor mode keeps `flex_1` + `h_full` (fills fixed-height
parents like the Hub editor's 160px wrapper) and adds
`min_h(visible_height())`, so in auto-height parents the configured height
(`self.height`, 60–480px across call sites) becomes the floor instead of the
box collapsing to ~20px.

## Verification

- `cargo fmt --all -- --check`: clean.
- `cargo clippy --locked -p synara-app --all-targets`: no new warnings in
  `input.rs` (pre-existing warnings in other crates unchanged).
- `cargo test --locked --workspace`: 355 pass; the only failure is the
  documented env-only
  `pull_requests::tests::discovery_does_not_change_worktree_or_index`
  (`~/.gitconfig` rewrites github.com remotes through git-manager.devin.ai).
- Root cause identified during live verification of batch 45/46: fields held
  text (Copy button returned full contents, ledger JSON intact) but painted
  nothing.

## Live test (verified after fix, recording `rec-77278a3a`)

- Notes dialog field paints stored text at full height (~115px rendered)
  instead of the previous thin empty rect (`ss_b463000c`); typing appends
  visibly and flips the status to "Unsaved changes" (`ss_79c0254d`).
- Hub settings Instructions field still fills its fixed-height (160px)
  parent and paints text — no regression (`ss_99deefc6`).
- Automation editor's instructions box now paints stored text
  (`ss_12a2b6a7`).
- Discard-unsaved confirm strip still works (`ss_735e8b67`).
