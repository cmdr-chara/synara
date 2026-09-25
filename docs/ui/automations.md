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
and uses the existing controller cancellation path. Each execution has a saved
maximum runtime from 1 to 3600 seconds (new definitions default to 900 seconds).
A timeout cancels the owned task and records a failure. Stop scheduling stops
future scheduling without pretending that existing provider work has been
undone. No automatic retry is performed. A new Run now action is the explicit
retry mechanism and creates a new task.

## Schedule and resource limits

Supported schedules are `every Nm` (1 through 10080 minutes), `daily HH:MM`,
`weekdays HH:MM`, `weekly mon HH:MM` (any day from sun through sat), and
`cron <minute> <hour> <day-of-month> <month> <day-of-week>`. Cron expressions
are limited to 120 bytes and support numeric values, comma-separated lists,
inclusive ranges, positive steps, and weekday names `sun` through `sat` (or
weekday numbers 0 through 7, with 7 meaning Sunday). Cron search is bounded to
eight years of local calendar days, which covers sparse leap-day schedules.
Impossible calendar combinations return a no-future-slot error. When both
day-of-month and weekday are constrained, either match is sufficient; when
either field is the literal `*`, both field matches are required. This follows
the upstream scheduler's constrained five-field cron vocabulary.

Timezone accepts UTC, a fixed offset such as +02:00, or an IANA zone such as
Europe/Rome. Calendar schedules use local wall time: a nonexistent
spring-forward time skips that occurrence, and a repeated fall-back time runs
at its first occurrence only. The later half of a folded minute cannot claim
the same scheduled slot again. The UI labels next-run times in UTC and also
displays the configured timezone. Resume recalculates the next future slot from
the current time.

Optional total-run and consecutive-failure limits pause a definition durably.
New definitions default to a three-failure limit; existing definitions retain
their saved limits. Raising a reached run limit is required before resuming.
The bounded runtime is persisted in the definition and the immutable run
snapshot. Older definitions that lack it use the existing 15-minute default.

Skip records one missed occurrence when more than 30 seconds overdue, without
launching an agent. CatchUpOnce dispatches at most one run and advances directly
to the next future slot. Interval advancement is bounded arithmetic, not a
catch-up loop. Only one run per scheduler is active at a time.

The ledger supports 64 definitions and 256 retained runs. Full history blocks
new runs rather than silently deleting evidence. Instructions are limited to
16 KiB, stored output to 4096 characters, and persisted values to the existing
8 MiB storage boundary. Delete requires confirmation and retains run history
and generated conversations. A separate confirmed prune can discard terminal
history only after its definition has already been deleted; generated
conversations, active runs, and history for current definitions are preserved.
The existing backup/restore validator accepts and validates this versioned
ledger. Restoring it never arms a scheduler.

## Validation and remaining work

Focused automation tests cover cron parsing and day matching, fixed-offset and
IANA/DST scheduling, sparse leap-day recurrence, persisted and bounded runtime,
paused saves, stale edits, durable claims, restart recovery, concurrent SQLite
claims, skip/catch-up semantics, confirmed deletion, explicit profile failure,
queued cancellation, confirmation revision fencing, run/failure limits and
backup/restore. The automation test run passes, and the native app target compiles.

Live provider completion/cancellation, native click/keyboard/IME/accessibility
journeys, OS shutdown timing and multi-platform acceptance remain OPEN.
Retry delay/backoff policies are not an effective current-head gap because upstream currently
rejects non-none retry policies. Retained run history can be explicitly exported through the
system save dialog. Deleted-definition history can be pruned, and live-definition terminal
history can be pruned only after preserving a durable cumulative run count, so max-run
enforcement cannot be reset by cleanup. Generated conversations are never deleted by pruning.

Definitions now choose Project-only or Hub shared context explicitly. Project-only is the
legacy/default policy. Hub mode requires an active Hub on the selected project, snapshots the
current Hub revision plus its user-maintained instructions/knowledge inside the atomic run
claim, creates the owned conversation in Studio scope, and persists the exact combined prompt
before provider launch. Transcripts and files are never harvested automatically, and later Hub
edits do not rewrite an already claimed run.

Out-of-process scheduling and wider live-provider lifecycle acceptance remain open. This source
slice does not close the broad Automations acceptance gate.
