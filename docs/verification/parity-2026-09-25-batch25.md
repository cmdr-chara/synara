# Parity batch 25: editor review and environment orchestration closure

## M21 complete

The native editor now has a bounded end-to-end diff review workflow:
- saved-buffer, current-disk and explicit Git-ref comparison scopes;
- full review or Changes only presentation without changing diff indices;
- copy full review and copy one exact changed block;
- restore one exact changed block or restore the full reference into the unsaved buffer;
- Previous/Next changed-block navigation with wrap and selected-block toolbar actions;
- guarded three-way recovery for non-overlapping disk/local conflicts.

All buffer mutations remain unsaved until explicit Save and recheck task/project/root/path/tab
ownership, comparison generation and the exact current diff. Selection is presentation-only
and is cleared by any buffer/comparison refresh.

Partial Git staging is deliberately not included. The available owner does not provide a
safe index-preserving hunk transaction, so Synara does not risk overwriting unrelated staged
content merely to imitate a staging UI.

## M24 complete

The message/fork environment workflow now makes the execution location explicit instead of
implicitly assuming the project root. A reviewed fork can use:
- the current local or SSH workspace;
- an existing linked worktree;
- an exact recoverable Synara-managed worktree;
- a reviewed new local managed worktree;
- a reviewed new SSH managed worktree on the pinned host.

The same orchestration boundary now coexists with same-task provider continuation and
capability-gated ACP provider-native whole-session forks. Task cwd/worktree identity is
persisted, stale source/worktree/route reviews fail closed, and no dirty source files,
approvals, credentials or ambient SSH configuration are copied.

Automatic post-task worktree cleanup/crash reconciliation remains M22. Real SSH/provider
interoperability remains acceptance work under A07/A09 rather than a reason to leave the
product orchestration feature open.
