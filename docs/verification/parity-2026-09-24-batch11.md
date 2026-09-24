# Parity batch 11: worktree recovery

M22 partial: the existing fork menu identifies an unassigned linked worktree
when its `synara/<uuid>` branch and `worktree-<uuid>` checkout path match the
application scratch parent. It offers recovery through the existing reviewed
selection, task ownership and directory validation path. The unsent branch
context is generated from the chosen source message. No second checkout occurs.

The checkout error and initial review explain how to recover when task storage
fails. The application still never deletes a worktree automatically. Cleanup and
crash recovery beyond a live linked checkout remain open.

M21 partial: a changed block in the editor comparison has a Restore block
action. It reconstructs the buffer from the reference for that block only,
rechecks the owner, generation and current diff before editing, and leaves the
file unsaved for review. Partial restoration is refused for shortened diffs,
CRLF and mismatched final-newline comparisons. Git staging remains separate.

M30 partial: Studio text files now offer bounded committed Git history and
read-only revision previews. The selected task/path/generation and cancellation
token fence asynchronous results; historical content is labeled separately from
the current file and can be copied without changing workspace files. This uses
the existing filter-free GitService history/revision reads. Uncommitted output
snapshots and long-running lifecycle remain open.

M05: the local web run endpoint now admits an existing SSH workspace with a
pinned connection profile. GET status displays the remote host and an opaque
stamp over workspace and profile metadata. Explicit Run includes that stamp;
admission and the worker reject a stale destination before the controller's
existing remote ACP session path. Stop and timeout retain the same cancellation
path. The HTTP server still binds to loopback and SSH setup stays native.

Validation pending with final batch checks.
