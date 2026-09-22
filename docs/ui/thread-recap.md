# Thread recap

The command palette's **Thread recap** opens a task-owned review of visible
conversation text and any cached recap. Reading, copying and reopening a saved
recap never contacts a provider. Generating is a separate, explicit direct-model
request. It does not send a normal chat prompt, bind the task to that model,
retire an ACP session, create a task, inherit approvals or alter the normal draft.

## Review and generation

Inspect exact source to send shows the bounded selection. It contains at most
64 whole visible User/Assistant messages within 48 KiB, in chronological order.
Oversized messages and older messages that do not fit are counted as omitted.
Reasoning, tool output, approvals, structured questions, attachments and filesystem
contents are not included. There is no historical binary reconstruction.

Filter configured providers/models and select a destination. The endpoint is
shown before Generate. The user is told that this sends the reviewed text and
may incur provider charges. No configured model means an explicit unavailable
state, not fallback to an arbitrary provider. Generic ACP remains independent.

Generation is capped at 1024 output tokens, at most 32 KiB of returned text and
120 seconds. The transport families and scoped secret references are reused
from the direct model layer. Tools are never requested or executed. Incomplete,
empty, oversized, tool-proposing or abnormally terminated responses cannot
replace a cache. Stop cancels the owned request. Remote work already performed
cannot be undone. Closing during generation requests Stop and waits for the
operation to finish. There is no automatic retry.

## Cache and freshness

The cache is separate metadata under `task-recap:<task UUID>`. It records model,
provider, reviewed endpoint/profile fingerprint, generated time, task/project/
thread/folder identity, source sequence/fingerprint, included/omitted counts and
a monotonic revision. It is model-generated assistance, not authoritative
transcript history.

Before requesting and before saving, the source is reconstructed and compared
with the reviewed snapshot. Archived, active or interaction-waiting tasks cannot
generate. Changed source/task ownership or another saved recap prevents a stale
write. Cancellation before the atomic save preserves the old cache. Once a
completed save has committed, later cancellation does not roll it back.

A changed conversation is labelled stale when the review is opened or reloaded.
Regenerate explicitly replaces the cached result only on success. Failure leaves
the old recap intact. Restart restores only the completed cache, not in-flight
prompts or authority. Malformed/oversized cache data is reported, not silently
reset. Recovered backup entries validate both the document and task-key identity.

## Evidence scope

Focused workspace tests target persistence/reopen, source/task/revision fencing,
cancelled and duplicate writes, whole-message bounds and hidden-role exclusion.
Controller tests exercise output completion, truncation, tool proposals and
duplicate events. The native fixture exercises real loopback inference from an
ACP conversation, explicit destination review, copy, Stop, rejected tools,
restart, stale display and refresh without transcript mutation.

These fixtures do not establish live authenticated provider or non-Linux native
acceptance. Recap correctness still requires reading the underlying conversation.
