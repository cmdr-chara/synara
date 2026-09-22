# Task-local Debug workflow

Open **Debug mode** in the composer's Add menu or **Debug workflow** in the
command palette. The composer shows the current phase while a run exists.

Synara owns the persisted Observation, Reproduction, Investigation, Fix and
Verification lifecycle. Every transition requires user-recorded evidence for
that exact phase visit. Marking a run complete is a user assertion backed by
recorded verification evidence, not independent certification by Synara.
Reinvestigate returns to Investigation and requires fresh evidence. Pause stops
phase progression and instruction insertion. It does not cancel a separately
running agent response, which still uses the normal Stop control.

**Add phase instructions to draft** appends bounded visible instructions and the
last four saved evidence records to the existing composer. It neither sends nor
changes an ACP mode, model binding, task, session, filesystem or approval policy.
The user reviews/edits the draft and uses Send normally. Draft insertion does not
include unsaved evidence. Duplicate insertion at the same dialog revision is
suppressed. No debugging step runs automatically, including after restart.

## Storage and ownership

`WorkspaceService::edit_debug_workflow` is the sole mutation owner. The SQLite
`task-debug:<TaskId>` document uses versioning, a writer transaction and an
expected-revision check. Stale or duplicated requests cannot append duplicate
evidence or replace a newer run. Invalid/future documents are not overwritten.
Archived tasks are read-only, and permanent task deletion removes the document.
Backups validate Debug records before accepting them.

A problem is limited to 4096 bytes, each evidence record to 8192 bytes, and a run
to 64 records. A closed run retains a compact summary among the last 16 runs.
Closing a run explicitly warns that detailed evidence is replaced by that
summary. **Copy saved evidence** preserves the current record on the clipboard.
No transcript or file rollback is implied.

The native dialog retains unsaved input on errors, asks before discarding it,
blocks navigation/close during writes and fences callbacks by dialog generation
and task identity. Restart restores saved workflow state and the normal unsent
draft. It does not create a provider session or execute a persisted prompt.

## Verification targets

Focused Rust tests cover phase gates, fresh evidence after reinvestigation,
pause, restart, task isolation, archive/delete cleanup, stale writers, duplicate
mutations, malformed/future input, forged completion and resource bounds.
`native_sprint2_debug_smoke.py` exercises actual GPUI controls with read-only SQL
assertions and an owned Xvfb display. Native evidence and actual run results are
recorded separately in the sprint verification receipt.
