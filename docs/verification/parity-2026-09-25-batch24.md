# Parity batch 24: SSH managed worktree creation

## M23 complete

The reviewed new-worktree fork workflow now supports enrolled SSH workspaces in
addition to local repositories.

For an SSH source, Synara:
- runs `git worktree list --porcelain -z` through the existing typed Git owner
  on the pinned SSH host;
- identifies the canonical source worktree and committed HEAD;
- derives the new destination itself as `worktree-<uuid>` beside that canonical
  repository root, rejecting repositories directly under `/`;
- reviews the exact remote repository, base object, generated branch and
  destination before mutation;
- runs the existing typed `AddNewWorktree` operation on that same pinned host;
- keeps hooks, signing, credentials and network helpers disabled;
- rechecks worktree identity and HEAD immediately before checkout;
- validates the new project directory through the existing remote filesystem
  helper before saving the task.

No arbitrary remote destination is accepted, ambient SSH configuration is not
used, and source dirty files are not copied because checkout is pinned to the
reviewed committed object.

If checkout succeeds but task persistence fails, the exact unassigned
`synara/<uuid>` branch plus `worktree-<uuid>` checkout is recoverable from the
same source-message environment chooser. Recovery uses the pinned SSH Git owner,
rechecks branch/path/source-repository overlap and creates no new checkout.

Focused pure validation covers deterministic remote sibling derivation, root-level
refusal and exact reviewed branch/destination matching. The existing SSH Git and
remote-filesystem owners remain responsible for transport/path validation.

M23 is complete as a product feature. A07 remains open for real pinned-host SSH
worktree/search acceptance.
