# Local project-history import

**Settings > Project Import** discovers local Codex or Claude JSONL histories,
previews visible messages and provenance, reviews an existing local destination
project and future agent, then explicitly imports and opens a standalone chat.
There is no automatic history scan or import at startup.

The source's recorded working directory is information for review, not filesystem
authority. Destination paths come from the existing workspace/project owner.
Import does not register arbitrary source paths or map them to SSH workspaces.

## What is imported

A new Ready chat contains a bounded text-only snapshot of visible user/assistant
messages, available timestamps and an import notice. Its composer is empty. The
source files, source conversation and all provider processes remain unchanged.
No provider session, approval, secret, reasoning, tool state, image, attachment or
filesystem snapshot is transferred. It is not provider-session resumption.
An ACP agent starts fresh only on explicit Send. Direct-model use requires its
own reviewed model selection.

Codex event/response duplicates are normalized without duplicating visible text.
Claude history is resolved along one reviewed leaf's ancestor chain, not a union
of sibling branches. Unsupported records and omitted content are visible warnings.
The UI offers explicit branch selection where multiple Claude leaves exist.

## Review, duplicates and recovery

A preview is an immutable in-memory snapshot, pinned to the source digest and
identity. Confirmation rechecks the source and destination. Changed history or a
stale destination rejects the operation and requires a new review. A failed or
cancelled attempt does not leave a half-imported task.

The existing SQLite store transaction couples task, transcript, activity, empty
draft and import receipt. Import identity is provider plus source session UUID.
The receipt prevents reimport across destinations and survives application
restart, supported backup/restore, and deletion of the imported conversation.
A changed source or another leaf of the same already-imported session is reported,
not automatically appended or imported again. Open the existing task when present.
The ledger is bounded to 4096 receipts and fails explicitly when full.

Retry is an explicit action following a failed attempt or a fresh review. Restored
receipts and conversations never run a prompt. No automatic sync, source deletion,
source mutation or hidden retry is implemented.

## Filesystem and input bounds

Discovery uses the existing capability-scoped `WorkspaceFs`. Symlinks and
non-regular files are rejected. Discovery, depth, entry count, time and cancellation
are bounded. A batch exposes at most 256 files from at most 10,000 inspected entries.
Parsing permits at most 20,000 records, 4096 visible messages, 4 MiB of text,
1 MiB per line and 256 KiB per message, within the filesystem read limit.
Malformed/oversized input does not create a destination task.

## Acceptance

Feature implementation includes native discovery, preview, review, confirmation,
opening, duplicate handling and recovery. Tests cover stale source, transactional
rollback/retry, restart, backup, duplicate retention, branching and source bytes.
The [sprint receipt](../verification/max-feature-sprint.md) records actual Linux
native checks. Broader real-history format coverage, attachments, source-session
resumption, automatic incremental sync and other-platform native acceptance remain
outside this slice. The original provider histories are always preserved.
