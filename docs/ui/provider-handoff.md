# Reviewed provider continuation

**Continue with...** in the conversation header or tools opens a native target
picker. Choose a saved ACP agent or configured direct model, review the working
folder and bounded visible context, edit that context, then explicitly create an
**unsent related conversation**. Opening the source through **Open original**
returns to the original transcript and draft.

This is not same-session migration. When provider session transfer has no safe
contract, Synara creates a fresh TaskId and ThreadId. The UI says so. The original
task, draft, persisted/live provider session and approvals are not mutated or
retired to simulate a handoff. The new task shares the same existing project,
working folder and task scope, not a new Git checkout or a copied worktree.
It has fresh task authority and no inherited permission decisions.

## Review and creation

The review records source identity, transcript sequence, workspace/project/root
identity and target configuration. Creating the continuation rechecks them all
inside the existing task reservation and SQLite transaction. Changing the source,
destination authority, agent profile or direct-model settings invalidates review.
Active tasks and shutdown refuse creation. Repeated confirmation cannot create
another child from the same review. A rolled-back creation can be explicitly
retried without leaving a partial task or origin record.

Context includes only bounded visible user/assistant text, up to the latest 128
messages and 256 KiB, with included/omitted counts. A message too large for safe
review is rejected. The editable resulting draft is limited to 1 MiB. No hidden
reasoning, tool state, attachments or automatic Hub transcript expansion occurs.
Draft edits require explicit discard rather than being erased by navigation.

No source approvals, secrets, environment grants or provider sessions are copied.
Choosing a direct model explicitly creates a fresh binding to the reviewed saved
profile. The shared OS credential owner may resolve that profile's existing key
only on a subsequent authorized request. This is not copying the source task's key.

## Lifecycle and limits

Creation writes the task, unsent draft, origin and optional direct binding atomically
through the existing related-conversation storage owner. It starts no provider,
subagent, process, Git action or external write. Only a later explicit Send uses the
chosen ACP/direct runtime. Restart restores the child and draft without sending it.
Selection-generation guards prevent a slow review/create response from replacing
an unrelated newly selected conversation. Escape respects composing text and does
not silently discard changed review content.

The native related-continuation workflow is implemented. Product handoff remains
**partial** relative to in-place same-task provider continuation or session transfer.
Those need an explicit interoperable protocol contract, not provider-name guesses.
There is no filesystem rollback, checkpoint restore, worktree clone or autonomous
delegation in this feature.

The [sprint receipt](../verification/max-feature-sprint.md) records seven backend
handoff tests and the native edited-context/ACP/direct/restart journey, including
the diagnosed native character-input failure and its successful correction.
