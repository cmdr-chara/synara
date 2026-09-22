# Native conversation and review workflows

The feature-closure sprint adds Debug, persistent goals, recap, PR Fix, inline file
comments and an in-app version/notes surface. These reuse the existing task,
composer, workspace store, Git/process and ACP/direct-runtime owners. Native
verification uses Linux/X11 fixtures. It does not prove live account interoperability,
macOS/Windows/Wayland interaction, accessibility or production release readiness.

## Debug mode

Choose Debug from the native mode/command controls. Synara owns the mode, independently
of provider names and advertised provider modes. Work through Observe, Reproduce,
Investigate, Fix and Verify. Save concrete evidence before advancing each phase.
Preparing the next step creates editable, visible, unsent composer text including
the existing request. Send and Stop remain the normal conversation controls.

The workspace record is task-local and revision-checked. Each phase permits 16 KiB
of evidence. Editing earlier evidence invalidates later evidence. Completion is an
explicit verification action, not an inference from a successful model response.
Disable, back and restart preserve the ordinary transcript and runtime permissions.
Restart restores the workflow without sending anything. Owners are
`shell/debug_workflow.rs` and `storage/debug_workflow.rs`.

## Persistent thread goals

Open the goal control to create or edit an objective, then save. Resume prepares a
visible draft and arms a transient pursuit lease. It does not send the first turn.
Review and Send explicitly. Each arm permits at most two automatic follow-ups, with
a visible five-second countdown and a ten-minute pursuit ceiling. Normal completion
and a bounded final `SYNARA_GOAL_STATUS` response are required before continuation.
Missing or invalid status stops the pursuit rather than guessing success or retrying.

User draft edits take priority and disarm continuation. Questions, approvals,
interruption, failures, event gaps, navigation and task changes stop continuation.
Pause stops future pursuit. Stop also cancels the current response through the
existing controller. Resuming is explicit after a blocker. A model claim of success
opens review, while achievement requires the user's verification evidence.

Objective, pause/blocker/review state, elapsed pursuit time and achievement history
persist. The arm, countdown and running authority never persist. Restart cannot
start a stored prompt. Editing or clearing the goal does not rewrite messages.
Bounds are 4 KiB per objective, 2 KiB per status/verification note and 16 achievements.
The controller checks task/root/draft/sequence identity immediately before dispatch
and cancellation fences prevent a revoked preparation from launching later.
Owners are `shell/goals.rs`, `storage/goals.rs` and the existing submission controller.

## Thread recap

Open Recap on the original conversation and choose Generate (or Regenerate).
Review and edit the request, then create its independent unsent related conversation.
Use normal Send/Stop there. After completion, choose Save recap to cache the result
on the original conversation. Refresh always starts with another reviewed request.
No hidden summarization runtime or implicit send is added.

Context is limited to the newest 128 visible user/assistant messages and 256 KiB.
Omissions are shown. Hidden tool state, approvals, secrets and historical binaries
are not copied. The cache is at most 64 KiB and pins the source/generation identity
and transcript sequence. Stale source, failed generation or an invalid destination
refuses replacement. The original task, transcript, draft and session remain intact.
The source cache survives restart. Owners are `shell/recap.rs` and the existing
`storage/conversation_tools/related` handoff/recap implementation.

## PR Fix

In Pull Requests, select the intended project/repository/PR and collect unresolved
review context. Inspect its PR, head, thread, comment, file and line identities, edit
the instruction, then append to the reviewed unsent destination draft. Normal Send
is a separate action. Cancel abandons preparation without executing a prompt.

Collection is deterministic and read-only, using the existing GitHub provider and
host/process owner. It bounds discovery to 100 threads and 20 comments per thread.
Incomplete pagination is rejected rather than presented as complete. Combined review
context is capped at 96 KiB, the instruction at 16 KiB and the resulting draft at 1 MiB.
Head and review contents are checked again before append. Selection, working root,
task and draft revisions fence late responses. No remote comment/review/merge write
or local checkout/staging is performed by PR Fix. See [Pull Requests](pull-requests.md).

## Inline file comments

Open a saved file in the existing editor, select a line/range and add review text.
Synara retains the exact file/version, line/range and numbered surrounding context.
Review several comments in the task-local queue, then append the combined context
to the unsent composer. Existing draft text is preserved and Send remains explicit.

Bounds are 80 selected lines, two surrounding lines on either side, 16 KiB context,
4 KiB comment, 12 comments per task, 128 KiB stored queue and 64 KiB prompt context.
Unsaved editor contents cannot silently claim a saved-file version. Revalidation
uses the existing filesystem owner, not a separate path/SSH implementation.
Changed, deleted or renamed paths fail closed and retain the queue and draft.
Remove the old comment and review the current file rather than silently rebinding
by filename similarity. Owner modules are `shell/inline_comments.rs` and
`storage/inline_comments.rs`. This is not a remote PR review-comment publisher.

## Version, What's New and local history (partial release workflow)

The in-app surface shows the actual compiled package version and bundled native
development notes. Local version observations and read/dismiss state persist.
A newly observed version becomes unread, and notes can be opened again deliberately.
The local observation history is bounded to 32 entries. It is not a fabricated
remote release catalog, and development notes are not a published release.

A production release feed, signing policy and verified installer are not configured.
The UI says so, without claiming an available update, silently opening a website,
installing anything or inventing endpoints/signatures/release data. Existing updater
foundations remain unchanged. Owners are `shell/releases.rs`, `storage/releases.rs`
and [bundled build notes](native-build-notes.md). Releases remains Partial.

## Persistence, trust and acceptance

The new task-owned records participate in existing backup-key validation and task
deletion. Malformed records and stale edits are rejected, not silently normalized.
No feature clones provider sessions, grants extra permissions, copies approval state,
auto-retries ambiguous writes or turns persisted text into execution authority.
Standalone chats remain the default, Hubs optional and Zen presentation-only.
See the [verification receipt](../verification/feature-closure-sprint.md) for exact
candidates, positive/negative tests and remaining acceptance gaps.
