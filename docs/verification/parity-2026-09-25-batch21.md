# Parity batch 21: automation process and continuation orchestration

Implementation commits:
- `46b78fad26f226e34424d5cdefd31a15c73e8a0e` — explicit headless scheduler owner;
- `f1282a27aa9a3f1dcebce75e7eb4d30054ce01de` — Standalone, Heartbeat and Dedicated execution modes;
- `2692b82cc685ca6c5fa95a61d38c70b3b0ec2aca` — continuation cooldown and scheduled deferral.

## Headless scheduling

`synara-server --automations` explicitly arms the same durable
`AutomationScheduler` used by GPUI. Ordinary headless startup remains disarmed.
The server starts the scheduler only after workspace recovery and acquisition of
the workspace owner lock. Shutdown cancels/stops automation ownership before
controller teardown. Because the server and desktop both use the same process
ownership lock, they cannot concurrently schedule one database.

## Execution modes

Definitions now persist the pinned-upstream execution model:

- **Standalone** creates a fresh owned conversation for every run.
- **Heartbeat** continues one user-selected existing target conversation.
- **Dedicated** creates one automation-owned conversation on the first run and
  durably reuses that same task on later runs.

Legacy definitions decode as Standalone with no target. A new Dedicated
definition cannot import another task as its owned conversation, while an
existing Dedicated definition keeps its system-assigned target across edits.

Continuation target identity is rechecked at save and run time. Targets must
remain in the automation project, use the selected ACP agent and have no
direct-model binding. At run time they must additionally be idle, have no
unsent draft or pending attachments, and not already be used by another running
automation. User-owned composer state is never overwritten.

## Cooldown and deferral

Continuation cooldown is persisted and bounded from 0 through 86400 seconds,
defaulting to 60. Recent external target activity temporarily blocks Heartbeat
or Dedicated continuation. The automation's own last completed run is exempt,
so a short Dedicated schedule is not silently stretched to the cooldown period.

For scheduled runs, temporary target activity, draft/attachment ownership,
cross-automation ownership or cooldown returns no claim and rolls back the
schedule advancement, preserving the due occurrence for a later poll. Manual
Run surfaces the same condition as a reviewable error instead of silently
pretending a run occurred.

## Focused verification

Source-level audit confirms legacy defaults, target persistence, scheduled
rollback/deferral, own-run cooldown exemption, the explicit headless opt-in and
shutdown ordering. Focused workspace regressions cover:
- Heartbeat target reuse and user-draft refusal;
- scheduled heartbeat cooldown deferral without consuming the slot;
- Dedicated first-task creation and immediate reuse of its own completed task;
- Standalone fresh-task-per-run behavior.

No workflow status is attached to these commits, and this execution environment
does not expose a Rust toolchain/checkout. This receipt therefore does not claim
a fresh cargo, rustfmt or Clippy pass.

S06 remains open only for the pinned upstream AI-evaluated completion/stop
policy and wider live-provider lifecycle acceptance. D8 remains OPEN.
