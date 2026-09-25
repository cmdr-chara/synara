# Parity batch 22: reviewed bulk worktree cleanup

## M22 partial: clean all reviewed recoverable worktrees

The source-message environment chooser now offers one bulk cleanup action when
more than one unassigned Synara-managed scratch worktree is recoverable.

The confirmation captures the exact reviewed path/branch set. Execution never
discovers or removes a newly appearing checkout that was not in that review.
Each item is routed through the existing single-worktree owner, which rechecks:
- source-task identity and idle state;
- local workspace ownership;
- exact project worktree path and unassigned status;
- canonical Synara scratch parent;
- matching `synara/<uuid>` branch and `worktree-<uuid>` directory;
- live Git worktree metadata and non-overlap with the source repository.

Removal remains ordinary non-force `git worktree remove`. Dirty, assigned,
locked, prunable, stale, cancelled or otherwise unsafe worktrees are retained and
reported individually. Generated branches are never deleted.

Focused regressions cover a mixed reviewed set containing one clean orphan, one
dirty orphan and one now-assigned worktree. Only the clean checkout is removed,
the dirty file survives byte-for-byte, the assigned task remains mapped, and all
three generated branches remain. A separate test verifies that missing mutation
consent refuses the entire bulk operation before any removal.

M22 remains open for automatic post-task lifecycle cleanup and broader crash
reconciliation.
