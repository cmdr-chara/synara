# Feature-closure sprint 2: native workflow boundaries

See the [verification receipt](../verification/feature-closure-sprint-2.md) for exact
commits and test runs. These are bounded user workflows, not a release certification.
The roadmap's Present classification describes the supported workflow below, not
complete platform coverage or every conceivable state dimension.

## Stacked pull requests

Open a PR in the existing Pull Requests workspace, inspect the ordered stack and
readiness, then review the selected root-to-current prefix before confirming merge.
Relationships use exact repository/head/base identities and stable sibling ordering.
Incomplete, ambiguous, duplicate, cyclic and oversized graphs fail closed. Limits
are 100 open PRs and 16 stack members. Reviews expire after two minutes.

The existing PR client, repository scope, cancellation and process owners remain
responsible. All selected identities and checks are refreshed before any write.
Forks can be inspected, but merging supports same-repository stacks only. Merge is
root first with reviewed head SHAs. Descendants are explicitly retargeted to the root
base and rechecked. There is no force merge, hidden checkout or unrelated local Git
mutation. Stop reports confirmed progress. Ambiguous writes are not automatically
retried or rolled back. Refresh and review before deciding on another write.

## Transcript images

PNG/JPEG originals live in ordered task/thread events rather than the evictable
Recent shelf. Uploaded and agent-returned images have distinct provenance. An
agent-returned image is not proof of generation, and the UI labels creation as
unverified. Device and AppSnap provenance survives attachment reuse. A browser
capture source tag does not imply that browser capture export is implemented.

Images render inline, expand into bounded previews and explicitly export exact
original bytes without overwriting an existing file. Encoded size, counts, headers,
dimensions and decode allocations are bounded. The original image limit is 2 MiB.
Missing, corrupt or unsupported images produce a safe notice instead of fabricated
history. Clearing Recent preserves transcript originals. Restart never sends media.
PDF/document viewing, broader media types and direct-model multimodal parity remain
unsupported, so transcript media remains Partial.

## Two-task conversation view

Use View two tasks to choose an existing same- or cross-project task. The secondary
view reuses SideChatState and existing Controller/session/draft owners. It is not an
Environment split or a second task system. Sessions stream independently, and focus
routes Enter/Send and Stop. The secondary composer is text-only. Pending attachments
refuse Send rather than being silently dropped, and full tools remain in the first
view. The secondary transcript shows its latest 200 messages with that limit visible.

Narrow windows display one selected task at a time with explicit switching. Closing
or replacing a pane changes presentation only, without cancelling or replaying work.
Drafts and events persist. The pair itself is deliberately transient across restart.
Broader IME, accessibility and cross-platform interaction acceptance remains open.

## AppSnap

Open AppSnap, explicitly discover visible windows, review and select one window,
then capture that target. Linux/X11 is the supported platform implementation. It
uses the existing bounded process owner, checks PID/class/title/geometry identity,
and refuses desktop/root/dock, hidden, stale or oversized targets. Image dimensions
must match the reviewed target. There is no whole-desktop fallback.

The image enters existing durable pending attachments without Send. Image bytes
survive restart, but window selection and capture consent do not survive restart or
task replacement. Command success is not an inferred OS permission grant. macOS,
Windows and Wayland are explicitly unsupported by this first path. AppSnap captures
images only and does not provide Computer Use authority.

## Bounded checkpoints and revert

Open Checkpoints, save a snapshot, review a historical snapshot, then choose Restore
draft + notes. The supported state is exactly the unsent text draft and saved task
notes/checklist. Files, Git working tree/index, transcripts, attachments, execution
metadata, provider sessions and approvals are not restored. This is actual rollback
of that subset, not additive edit/resend or revision branching.

Capture and restore reserve the existing task slot and refuse active prompt, setup
or shutdown work. Native UI waits for pending draft saves and refuses conflicting
editors, IME composition and approval/control operations. An armed goal must be
disarmed explicitly. A brief input shield prevents new local input during the write.
No agent connection, inherited approval or secret is needed.

One immediate SQLite transaction rechecks task/project/thread/root identity,
snapshot/history revisions, current draft and notes revision. It creates a
pre-revert recovery snapshot and restores draft plus notes together. Failure rolls
back all writes. Reviews expire after five minutes. Intervening changes require a
fresh review. The recovery snapshot is itself reviewable and restorable.

History is bounded to eight snapshots per task, 512 KiB encoded history and 128 KiB
per snapshot. Oldest entries are pruned. Oversized state is refused, not truncated.
Malformed or future-version history is preserved. Permanent task deletion removes
owned checkpoints. Restart retains history but not an open confirmation or consent,
and never starts a saved prompt. The roadmap counts this explicitly bounded first
workflow as Present, not as full workspace or provider rollback.

## Remaining development

Native subagent/workflow delegation, Agent Gateway, external MCP clients connecting
to Synara and Computer Use remain Missing. Related tasks are not delegated workflows,
outbound MCP is not inbound MCP, and capture is not permission to control a computer.
Existing task, session, Git, filesystem and process owners must remain authoritative.
