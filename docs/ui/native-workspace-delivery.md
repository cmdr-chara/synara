# Native workspace continuation receipt

Starting branch: `astra/gpui-clean-rewrite` at
`156e578aa7cfcdf71e70ad9a1b481ae473743246`. No unrelated working changes existed in
the isolated checkout. No PR, release, default-branch or protected-ref change is
part of this task.

## Completion ledger

| Outcome | Check | State / evidence |
| --- | --- | --- |
| Extend the existing shell without replacing Environment | Source review and retained Environment native journey | OPEN |
| Native per-file Git review and explicit scoped actions | Parser/state/storage tests and real temporary-repository desktop journey | OPEN |
| Commit drafts and review choices survive navigation/restart | SQLite round-trip, stale-save refusal and native restart | OPEN |
| Current upstream behavior reviewed and relevant gaps recorded | Exact upstream comparison at batch boundaries and before final push | OPEN |
| A second independent roadmap batch implemented | Live transcript work-status behavior and focused verification | OPEN |
| Full/narrow native evidence inspected | Actual GPUI screenshot artifacts, not mockups | OPEN |
| Coherent source pushed and CI checked | Exact branch head and candidate-specific check results | OPEN |

## Current upstream review

Start and first-batch checks: main is
`b58f27381e7ddd59678c9961500e8e43d3cc19ab`, 24 commits after the last recorded
product reference `948875954f432978eab7dd5fa44c3028b8d99a81`.
Further classification and final boundary check are recorded below when complete.

## Verification record

Local environment: Debian container, no installed Rust toolchain or resolvable
Rust distribution host. Authenticated GitHub plugin source artifacts provided the
exact checkout. Native build and desktop checks run in the repository's Linux CI.
No local compiler result or local native screenshot is claimed.

## Batch 1: native Git review (source implemented, runtime pending)

- Replaces the legacy raw whole-repository Git surface inside the existing Changes
  tab. No new panel framework, default theme or navigation destination is added.
- Worktree/staged filtering, selected-file and all-file review, line-numbered
  virtualized unified preview, raw mode, exact diff copy, explicit file stage and
  unstage, rename/binary/untracked/conflict states, native keyboard navigation and
  a stacked narrow-panel navigator are implemented. Copy is not the clipped view.
- Explicit diff-to-chat appends bounded fenced context to the existing durable
  unsent draft. Opening a selected file uses the existing dirty-editor guards.
- Commit messages and review choices are saved per project ID and current
  Environment root, with bounded records, serialized debounced writes, optimistic
  revisions, preserved invalid/future data, two-step reload and a close guard.
  Commit completion clears only the exact unchanged submitted draft. This does
  not add automatic managed-task-worktree discovery or change existing root
  selection semantics.
- Git actions remain on the selected local/SSH service. No push, reset-hard,
  discard, branch switch, new network permission, provider capability or shell
  execution is introduced. Whole-file staging is labelled as such, not hunk staging.
- Focused route tests: 21 passed locally. Python journey syntax check passed.
  Rust compilation, Rust tests, strict Clippy and actual desktop screenshots are
  pending candidate-specific CI. The immutable candidate is never formatted or
  assembled in-place by CI. Formatter diagnostics, when needed, use a disposable
  copy and cannot make the candidate's formatting check pass.

## Upstream changes since 9488759

Reviewed current main `b58f27381e7ddd59678c9961500e8e43d3cc19ab` and its 24-commit
file comparison. Current provider documentation and model trigger were read at
that exact revision. Classification:

| Upstream change | Native disposition |
| --- | --- |
| Latest live tool description remains visible as later tool calls arrive | Adopt independently in batch 2, preserve durable transcript/approval ownership |
| GitHub Markdown alerts | New D1 rendering gap, candidate for independent bounded follow-up |
| Claude context target versus measured budget, pending target labels, idle-only process replacement | B6/C3/D1/D8 gap. Do not invent runtime provenance or pretend generic ACP exposes these vendor SDK controls |
| Claude Artifacts opt-in and warnings for /design and /slides | E6/I1/D8 gap. Keep provider-owned availability and agent-advertised command boundaries |
| Composer model trigger freezes its open-time size, retains compact accessible labels | Existing native popover/release-based controls cover part of the interaction. Measured-budget suffix and full parity remain open |
| Provider discovery/settings cache changes | Generic native ACP discovery already exists. Vendor-specific discovery/configuration parity remains deferred |
| Environment PR section changes | H6/D10 remain open. Local Git review is not remote PR integration |
| Import/announcement layout and arbitration changes | F8/I4 remain deferred. Do not present absent import UI as available |
| New macOS icon-source assets and packaging resources | Preserve the currently bundled Synara mark/icons. Native platform icon packaging review remains P/I10 work |

No Zeron or MonoCode implementation or asset was copied. Their screenshots were
used only for hierarchy, density, consistency and panel-composition research.
