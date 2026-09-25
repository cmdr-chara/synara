# Parity batch 19: same-task provider handoff

Implementation commit:
`e2b68ed20f70fb09a31883ac75a57c6ee30dcbff`, based on
`7c7a26aaa252dbe3cd5ce1c20b5c8c424415747b`.

## S07 complete

The full provider-handoff review now offers two explicit outcomes:

- **Create unsent continuation** retains the established child TaskId/ThreadId path.
- **Continue here** keeps the existing TaskId, ThreadId, task scope, working
  directory, files, Git state and transcript while replacing only the reviewed
  ACP/direct route and the visible unsent continuation draft.

The in-place storage transaction rechecks source task identity, transcript
sequence, workspace/root authority, current route identity and target
configuration. A route change after review makes the review stale.

Same-task handoff refuses to overwrite user-owned composer state. The persisted
source draft must be empty and pending attachments must be absent. The UI also
keeps Continue here disabled while draft/attachment state is nonempty, loading or
changing. The separate-child path remains available.

On commit, the task agent and direct binding are replaced atomically, the old
saved provider session is invalidated, and the reviewed draft is persisted in
the same transaction. No prompt is sent. Controller cleanup clears the task's
live connection/session references and best-effort closes the old ACP session
only after storage has committed; cleanup failure cannot turn a completed route
mutation into a retryable duplicate handoff.

## Focused verification

Source-level review confirms:
- route identity is part of the single-use handoff review;
- nonempty persisted draft and pending attachments fail closed;
- ACP/direct binding replacement, session deletion, task update and draft write
  are one SQLite transaction;
- the source transcript, task/thread IDs and working directory are not rewritten;
- the full-dialog action is separate from the existing quick child-handoff path;
- no conflict markers or unrelated files are present in the implementation diff.

The focused storage regression covers draft rejection, attachment rejection,
ACP-to-direct same-task continuation, direct-to-ACP clearing, generated task
count stability, saved-session invalidation and stale route rejection.

This environment has no Rust toolchain, so this receipt does not claim a cargo,
rustfmt or Clippy pass. D12 remains OPEN for provider-native fork/session
capabilities and live provider/worktree continuation acceptance.
