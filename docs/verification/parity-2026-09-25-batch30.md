# Parity batch 30: managed worktree lifecycle closure

## M22 complete

Synara-managed worktree tasks now carry a durable versioned ownership record that
is committed in the same SQLite transaction as the new task and unsent draft.

The record binds:
- task, project and workspace identity;
- exact managed repository checkout root;
- exact task working directory inside that checkout;
- generated `synara/<uuid>` branch;
- local vs pinned-SSH workspace ownership.

The marker validates absolute non-traversing paths and requires the generated
branch UUID to match the `worktree-<uuid>` checkout directory. Tasks created in
an existing user-managed worktree do not receive this marker.

### Automatic task-deletion cleanup

After an archived managed task is durably deleted, Synara reopens the ownership
record under the existing lifecycle lock, verifies that no remaining task uses
the checkout, revalidates the project/workspace, linked-worktree path, branch and
live Git metadata, and runs ordinary non-force `git worktree remove` through the
same local or pinned-SSH Git owner.

Dirty, locked, prunable, reassigned, stale or otherwise unsafe checkouts are never
forced away. A cleanup failure does not resurrect the deleted conversation and
does not delete the ownership record.

Generated branches are deliberately retained.

### Durable crash/dirty reconciliation

Managed ownership records intentionally outlive task deletion until cleanup is
confirmed. Project and workspace deletion refuse to erase metadata while such a
record remains.

An explicit project/workspace deletion retries every exact pending marker first.
If a previously dirty checkout has been made clean, the retry removes it and
forgets the marker. If any checkout is still unsafe, deletion fails with the
bounded retained-worktree report and ownership stays recoverable.

This also covers a process interruption after task deletion but before Git
cleanup: the durable marker survives restart and participates in the next
project/workspace lifecycle retry.

M22 is complete at the product-feature level. Real pinned-host SSH cleanup and
cross-platform lifecycle evidence remains under A07/A10.
