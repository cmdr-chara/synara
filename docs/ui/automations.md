# Native automations

## Ownership and explicit execution

The native Automations route manages a versioned SQLite automation ledger,
independent of conversation events. Definitions store title, exact instructions,
explicit agent profile, project, schedule, timezone, enabled/paused state, next
slot and missed-run policy. The UI supports creation, editing, confirmed manual
runs, confirmed pause/resume, deletion, history and opening owned conversations.
Editing saves paused. Newer edits made during a save are retained in the form.
An open form requires save or explicit discard before quitting.

Loading records never executes them. The process-session scheduler always starts
stopped, including after a restart or restore. The user must explicitly confirm
Start enabled schedules. Run now is separately confirmed with the exact saved
instructions and revision. A changed revision invalidates that confirmation.
The configured profile is looked up by its exact ID. Missing profiles/projects
fail closed, without choosing another agent or workspace. Runs use the existing
Controller and permission interactions. No hidden prompt or permission override
is appended by the scheduler.

## Durable claims and restart behavior

Every mutation takes SQLite's immediate writer transaction before reading the
ledger. A claim reserves the scheduled slot, creates the owned task/thread
identity, saves its visible instructions, and advances the schedule atomically.
The reservation is committed before any provider launch. Independent database
connections cannot claim the same active definition or scheduled slot twice.
A task belongs to the chosen project and records the explicit profile. Run
history retains an immutable definition snapshot, owner UUID, scheduled/manual
identity, start/finish timestamps, state, bounded output/error and task ID.

A process crash leaves an unresolved reservation. It is not retried or silently
resumed. The UI allows explicit recovery only after the user verifies the prior
process has stopped. Recovery records Interrupted, labels the external outcome
unknown, and pauses the definition. Late completions from that prior owner are
rejected. This is at-most-once dispatch reservation, not a promise of exactly-once
external effects. Provider failure/cancellation does not undo external effects.

Stop active / queued run invalidates queued futures as well as the active token
and uses the existing controller cancellation path. Each execution has a
15-minute deadline. Stop scheduling stops future scheduling without pretending
that existing provider work has been undone. No automatic retry is performed.
A new Run now action is the explicit retry mechanism and creates a new task.

## Schedule and resource limits

Supported schedules are `every Nm` (1 through 10080 minutes) and `daily HH:MM`.
Timezone accepts UTC or a fixed offset such as +02:00. IANA zones and DST are
explicitly rejected, not silently approximated. The UI labels next-run times in
UTC and also displays the configured timezone. Resume recalculates the next
future slot from the current time.

Skip records one missed occurrence when more than 30 seconds overdue, without
launching an agent. CatchUpOnce dispatches at most one run and advances directly
to the next future slot. Interval advancement is bounded arithmetic, not a
catch-up loop. Only one run per scheduler is active at a time.

The ledger supports 64 definitions and 256 retained runs. Full history blocks
new runs rather than silently deleting evidence. Instructions are limited to
16 KiB, stored output to 4096 characters, and persisted values to the existing
8 MiB storage boundary. Delete requires confirmation and retains run history
and generated conversations. The existing backup/restore validator accepts and
validates this versioned ledger. Restoring it never arms a scheduler.

## Validation and remaining work

Ten focused automation tests pass, covering fixed-offset scheduling, paused
saves, stale edits, durable claims, restart recovery, concurrent SQLite claims,
skip/catch-up semantics, confirmed deletion, explicit profile failure, queued
cancellation, confirmation revision fencing and backup/restore. They are included
in the passing workspace test run. Native app and test targets compile.

Live provider completion/cancellation, native click/keyboard/IME/accessibility
journeys, OS shutdown timing and multi-platform acceptance remain OPEN. IANA/DST
zones, cron/calendar schedules, configurable retry policy, history export/pruning,
Hub-specific context selection and out-of-process scheduling are not implemented.
This source slice does not close the broad Automations acceptance gate.
