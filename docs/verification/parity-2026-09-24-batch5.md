# September 24: committed-file history in the native editor

This is one bounded editor-depth addition, not closure of the editor/diff parity
gate. It builds on the PDF/Library/onboarding batch and preserves earlier slices.
Upstream reference remains `eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.
Relevant upstream surfaces: `apps/web/src/components/DiffLineBlamePopover.tsx`,
`DiffPanel.tsx`, and their base-blob and blame queries. This addition supplies
safe committed-content inspection, not a claim of matching their full workflows.

## Delivered workflow

With a local file open, **File history** lists up to 50 commits touching that exact
path at a captured HEAD. Choose a commit to inspect its immutable UTF-8 blob in a
read-only, line-numbered preview and explicitly copy the complete revision.
**Back to buffer** returns to the same editor with unsaved text and Undo preserved.
There is no checkout, restore, write, provider prompt, or automatic clipboard copy.

The file list is exact-path history, without following renames. Commit author,
date and subject are repository metadata, not verified authorship. An unborn
repository, absent/deleted path, non-regular tree entry, binary/non-UTF-8 content,
missing object, helper failure or size limit gives an error rather than fallback.
Revision text is capped at 1 MiB. The display is limited to 6000 lines and 2000
characters per line, with a visible truncation label and full-text copy retained.

Existing Git process ownership supplies deadlines, bounded stdout/stderr and
cancellation on future drop. Task, project, root, file, tab and request generation
fence asynchronous results. Changing the selection, leaving Files, closing the
history panel or quitting cancels its read. Neither restart nor opening history
runs an agent or changes the file/index/HEAD.

The commands read commits/trees/blobs only. Replacement objects, lazy fetching,
network protocols, credential helpers, signature display, external diff and
worktree conversion are not used. This requires a Git version supporting
`--no-lazy-fetch`; older Git errors instead of silently enabling network access.
SSH file history is not implemented and never falls back to a local repository.

## Explicit blocker retained

Worktree-based line blame and HEAD-vs-working-file diff were not added. A local
Git fixture with a configured `filter.<name>.clean` showed that both `git blame
--no-textconv` and `git diff --no-ext-diff --no-textconv HEAD` execute that filter.
Those flags only exclude text conversion/external diff, not clean-filter input
conversion. Adding those actions as supposedly inert reads would broaden process
execution authority. Full blame/diff needs an explicit filter-execution policy or
a filter-free snapshot/mapping implementation. This batch instead uses immutable
committed objects. Existing Git actions were not refactored or reclassified.

## Verification contract

Focused service tests cover literal nested paths, actual commit order/content,
malicious clean/textconv configuration without execution, unchanged disk edits,
stale HEAD with immutable old revisions, cancelled reads, traversal, unknown
revision, binary and oversized content. Parser tests cover framing, bounds and
unknown dates. The native fixture exercises commit selection, exact revision
copy, unsaved buffer/Undo preservation, unchanged disk/HEAD and inert restart.
The integrated check covers the changed workspace/app crates, formatting and
strict Clippy. Results are appended only after actual validation.

All 21 broad parity gates remain OPEN. Native syntax highlighting, worktree line
blame, full comparison scopes, remote/other-platform acceptance and richer diff
editing remain outside this bounded delivered workflow.

## Observed validation

129 app tests, 312 workspace unit tests and three settings integration tests passed in run 35997592162. Two workspace fixtures and six isolated SSH tests were intentionally ignored. Focused history tests and strict Clippy passed after parser and Git-notes corrections in run 35998559065.
Rust sources/manifests are byte-identical to the passing candidate: Git-index manifest SHA-256 `293d67af71371bcb994206bf39b951bb1c62de03b82f0914cd191805075d6d6b`.
The earlier native attempts encountered display-startup delay and a clipped Explorer content-search result. This history journey uses a bounded 30-second display-startup timeout and opens the actual Explorer file row directly. Search-result mouse/layout acceptance remains a separate N2 issue, not a claimed pass here.
The native file-history journey, formatting and roadmap checks passed before publication. Evidence: https://github.com/cmdr-chara/synara/actions/runs/36002141219
Run attempt: 1. Publication base: `d7eecbebeeb1dd3bc97583dc7f90cd78c9a57c8b`.
Linux/X11 evidence covers commit selection, exact old/current revision copy, read-only focus, unsaved buffer and Undo preservation, unchanged disk/HEAD and inert restart. Local Git fixtures cover filter non-execution, unknown revisions, traversal, cancellation, binary/size refusal and immutable reads after HEAD moves. Real providers, SSH and other platforms were not exercised.
