# Reviewed provider continuation

**Hand off** in the conversation header (next to Checkpoints), **Continue
with...** in the conversation tools, or the command palette entry open a
compact Electron-style target menu. Picking **Handoff to X** creates the
**unsent related conversation** immediately with the generated context. The
original stays intact and nothing is sent automatically. **Review before
creating...** at the menu foot opens the full picker where the working
folder and bounded visible context can be edited before explicit creation.

The full review also offers **Continue here**. That keeps the existing TaskId,
ThreadId, working folder, task scope, files, Git state and transcript, but
explicitly replaces the task's ACP/direct provider route and stores the reviewed
context as its visible unsent draft. It never sends automatically. Continue here
is unavailable while the source composer contains an unsent draft, attachments
are pending/loading, or the conversation is active.

Creating a related conversation still gives the destination fresh task/session
authority. Same-task continuation invalidates the old saved provider session and
best-effort retires any old live session only after the route transaction commits.
Neither form copies approvals, secrets, hidden reasoning or tool state.

## Review and creation

The review records source identity, transcript sequence, workspace/project/root
identity, current ACP/direct route and target configuration. Both child creation
and same-task continuation recheck them under the existing task reservation and
SQLite transaction. Changing the source, current route, destination authority,
agent profile or direct-model settings invalidates review.
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

Child creation writes the task, unsent draft, origin and optional direct binding
atomically through the existing related-conversation storage owner. Same-task
continuation atomically rewrites only the existing task's provider agent/direct
binding, saved-session row and reviewed draft after independently confirming that
the persisted source draft is empty and no attachments are pending. It starts no
provider, subagent, process, Git action or external write. Only a later explicit
Send uses the chosen ACP/direct runtime. Restart restores the child and draft without sending it.
Selection-generation guards prevent a slow review/create response from replacing
an unrelated newly selected conversation. Escape respects composing text and does
not silently discard changed review content.

The native related-continuation and same-task route-continuation workflows are
implemented. Product handoff remains **partial** for provider-native session
fork/transfer capabilities and their live interoperability. Synara does not infer
those capabilities from provider names. There is no filesystem rollback,
checkpoint restore, worktree clone or autonomous delegation in this feature.

The [sprint receipt](../verification/max-feature-sprint.md) records seven backend
handoff tests and the native edited-context/ACP/direct/restart journey, including
the diagnosed native character-input failure and its successful correction.
