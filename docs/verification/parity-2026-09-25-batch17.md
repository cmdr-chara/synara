# Parity batch 17: bounded automation history pruning

S06 partial: retained automation run history can now be explicitly pruned when
the owning automation definition has already been deleted.

The prune operation:
- requires explicit confirmation;
- only removes non-running ledger records whose definition ID is no longer live;
- preserves generated task/conversation records;
- preserves active runs and all history for current definitions;
- does not change current-definition run-limit or failure-limit semantics.

The native Automations panel discloses the exact eligible count and keeps the
prune action separate from definition deletion. History remains retained by
default, and a full ledger still blocks new runs rather than silently evicting
evidence.

Focused workspace tests cover confirmation, deleted-definition pruning,
conversation retention, and the non-pruning of current-definition history.

S06 remains open for out-of-process scheduling and wider live-provider
acceptance.


## M30 partial: explicit durable-version cleanup

Studio durable text-preview history can now be cleared for the selected file
through a two-step native confirmation. The storage operation revalidates Hub
ownership and the visible relative path, removes only entries for that path, and
never reads, writes or removes the workspace file. Histories for other paths
remain intact.

The UI fences the result by task, selected path and preview generation. Changing
files cancels the confirmation, and a stale completion cannot clear another
file's visible history.

Focused workspace coverage verifies confirmation, hostile-path rejection,
other-file retention, source-file preservation and clean recapture after clear.

M30 remains open for broader long-running lifecycle/version organization.


## S06 partial: retained run-history export

The Automations panel can now export the exact retained run ledger through the
system save dialog. The export is a versioned `synara-automation-history-v1`
JSON snapshot containing immutable run-definition snapshots, ownership IDs,
scheduled/manual identity, timestamps, status, generated task ID and bounded
output/error text.

Export:
- reads the validated durable ledger without changing it;
- does not arm schedules, claim slots, launch providers or mutate conversations;
- uses the existing no-overwrite private-file writer;
- refuses an existing destination rather than replacing it;
- warns that instructions and output may contain private data.

Focused workspace coverage verifies the version marker, exact retained evidence
and no-overwrite behavior.

S06 remains open for out-of-process scheduling and wider live-provider
acceptance.

## S06 partial: live-definition history pruning without limit reset

A live automation can now explicitly prune its own terminal run-history records.
The operation is blocked while a run is active and preserves generated
conversations. Before deleting retained evidence, Synara persists a cumulative
task-backed run count on the definition. Legacy definitions derive a safe floor
from their retained history before any prune.

All max-run checks now use the durable effective count. Editing, pausing/resuming
and later pruning cannot reopen an exhausted max-run budget. Failure streak,
schedule state and generated conversations remain unchanged.

Focused coverage proves a two-run automation stays exhausted after each retained
run record is pruned and that the first generated conversation remains
inspectable.

## S06 partial: explicit Hub context policy

Automation definitions now choose between:
- Project instructions only (the legacy/default behavior); and
- Hub shared context + automation instructions.

Hub mode requires the selected project to have an active Hub. Inside the same
SQLite claim transaction, Synara resolves the Hub, snapshots its revision and
user-maintained instructions/knowledge, combines that visible context with the
automation instructions under a 128 KiB bound, creates the owned task in Studio
scope, persists the exact prompt, then allows the scheduler to submit exactly
that snapshot. Later Hub edits cannot rewrite an already claimed run.

Project mode never imports Hub context even when the project has a Hub.
Transcripts, files and hidden provider state are never harvested automatically.

Focused coverage proves Hub revision/prompt snapshotting, Studio task scope,
post-claim Hub-edit isolation, legacy context defaulting and project-mode
non-injection.

S06 remains open for out-of-process scheduling and wider live-provider
acceptance.


## M30 partial: exact durable-version export

A selected durable Studio text version can now be saved through the system save
dialog without reading or changing the current workspace file.

The native flow requires the same Hub, path, preview generation and exact selected
snapshot to remain current until a destination is confirmed. The backend then
reopens the task-owned version ledger and requires the complete
`StudioTextVersion` record to still exist. Evicted, cleared or otherwise stale
snapshots fail closed.

Export uses the existing no-overwrite private-file writer. Existing destinations
are preserved, and source files/history are not mutated. Focused workspace
coverage verifies exact historical content, no-overwrite behavior and rejection
after the durable snapshot is cleared.

M30 remains open for broader long-running lifecycle/version organization.


## M30 partial: pinned durable versions

Durable Studio text versions can now be pinned and unpinned by exact snapshot
identity. Pin state is persisted with the Hub-owned version ledger and shown in
the native version picker.

Automatic retention now removes only unpinned snapshots. If pinned entries consume
the 12-entry or 1 MiB retention budget, a new capture fails with an explicit
message instead of silently evicting pinned history. Explicit Clear durable
previews remains the deliberate destructive action and can still remove pinned
entries after its existing two-step confirmation.

Pin/unpin reopens the durable ledger and requires the exact task/path/text/capture
record to still exist. A cleared or stale snapshot fails closed. Focused storage
coverage verifies pinned retention across repeated captures and stale pin refusal.

M30 remains open for broader long-running lifecycle/version organization.
