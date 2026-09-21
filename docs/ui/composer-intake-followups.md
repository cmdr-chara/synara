# Native attachments and saved follow-ups

Date: 2026-09-21. Starting candidate: `0bb20db16da361fa52ef8726d9e98cba4b98a050`.
Product upstream reviewed: `e7cd15281e6d16cf8fc55a91496dcff035475e54`, unchanged.
This is an implementation receipt. Native compilation and interaction acceptance
remain pending, not implied by publication or source-level checks.

## Product behavior

The existing Synara/Hubs composer now accepts local files from a system picker,
file drop, and structured file clipboard entries. PNG/JPEG image clipboard entries
have their own native paste path. Ordinary text paste, editor text input and terminal
paste continue through their existing owners. IME composition defers binary paste.
Directories are not walked. URLs and network-share paths are not attachment sources.

Intake validates the complete selection before storing any of it. Supported inputs
are still PNG/JPEG and UTF-8 text/code. Empty, oversized, damaged, animated PNG and
unsupported binary inputs are rejected visibly. PDF, office documents, audio, video,
GIF and WebP attachment support are not claimed by this batch.

Pending snapshots are scoped to a task and restored from SQLite without rereading
or depending on the original files. Source paths are not stored. Each snapshot has
a generated identity, filename, media kind, byte count and, for images, dimensions.
The user can inspect an image or raw-text preview, remove pending attachments, and
explicitly reattach a retained recent snapshot. Each explicit add/reuse has a fresh
pending identity, so an older acknowledgement cannot consume a new selection. A failed import retains its inputs
in memory with Retry/Discard, and orderly close guards those unresolved operations.
Imports completed after navigation still belong to the original task. Recovery
controls expose pending imports from other conversations, including deleted tasks.

The compact paperclip and follow-up controls sit beside the existing session/send
controls. A bounded inline tray appears only when it has content, progress, errors
or an opened follow-up list. Previews and queued text do not introduce a dashboard,
new window or another permanent workspace panel. Synara's owned glyphs, semantic
colors, Glass material and Environment remain in use. No competitor code or assets
were imported. This continues the MonoCode-informed density/progressive disclosure,
while Zeron remains a Zen-only interaction reference.

## Delivery and consent

Send is still explicit and requires prompt text. The new controller entry point
loads exactly the displayed attachment revision and converts snapshots into the
existing protocol-independent Image/Context prompt parts. The existing ACP encoder
checks negotiated image/embedded-context capabilities and total payload limits.
Known unsupported capability combinations are explained and disabled in the
composer, and are rejected again by the backend. Agent/provider names confer no
capability. Unconnected agents negotiate on the existing explicit send path.

The original text-only Controller::submit remains the path for Kanban and other
callers. They do not silently acquire a conversation's pending attachments.
Cancellation ownership is reserved before attachment preparation. Stop can cancel
the preparation wait and prevent a later agent launch. The decoder worker retains
its concurrency permit until its actual work finishes, even if the caller cancels.

Transcript text records filenames and attachment markers, not image bytes or local
source paths. A matching durable local user event moves only that send's snapshot
IDs into Recent, leaving newer pending files alone. Successful prompt completion
provides the same ID-scoped fallback if an event broadcast was missed. Preparation
or capability errors unlock editing and retain any still-pending snapshots. Local transcript recording is
NOT proof that the provider accepted or completed the request. Recent snapshots are
a bounded retry cache, not a complete permanent multimodal transcript. Retries never
automatically send again. Binary replay, media-aware conversation export and deep
attachment display inside historical message rows remain D1/D11/D12 work.

The draft acknowledgement compares the richer transcript projection separately
from the user's original text and edit revision. A local echo cannot erase newer
text typed into the composer while a response is starting.

## Saved follow-up drafts

The clock control opens a persistent, task-local list of text drafts. Queue text
stores the current draft first, then clears it only if its text and local edit revision remain
unchanged. Users can edit, reorder, remove or append a saved draft to the current
composer. Append preserves existing text and retains the queued copy until explicit
removal. Edit saves reject stale revisions and retain the user's edits on failure.
Unfinished editing and writes guard navigation, Hub/project switching and orderly
close. A delayed workspace-opening response cannot replace an unfinished editor.

This is a MANUAL follow-up queue, not automatic dispatch or provider steering.
Pending attachments must be resolved before queuing text so they cannot be silently
left behind. No queue operation runs an agent, cancels work, promotes permission
consent or executes saved text on restart. Automatic queue/steer remains under D4/D8.

## Bounds and privacy

- Up to eight pending attachments, 2 MiB combined, with at most eight Recent entries.
  Combined pending/Recent raw data is capped at 3 MiB. Oldest Recent entries may be
  evicted to fit that cache, never pending entries.
- Images are limited to 8192 pixels per side and 16 megapixels. Header, animation,
  decoder and allocation checks precede preview or provider delivery. Decode work
  is serialized off the render thread with a bounded admission wait.
- Follow-ups are limited to 16 entries of 64 KiB each. Previews/lists have explicit
  display bounds. The text composer and ACP encoder retain their existing limits.

Snapshots and follow-up drafts live in the existing application database. This is
not an encrypted secrets vault. Filenames and selected content are sent to the agent
on explicit Send. Database backups may retain this private content. Clear/Remove
and task deletion do not promise secure erasure from SQLite pages, old backups or
providers. An abrupt crash before an import commits can lose that uncommitted import.
Source files and the clipboard themselves are not modified by intake.

## Compatibility and ownership

New task-attachments and task-followups preferences are versioned, bounded JSON.
The current backup key validator recognizes them and the already implemented Hub
profile keys. Permanent task deletion removes both new records in its existing
transaction. Invalid/future metadata is never replaced with an empty default.
No SQL schema migration, provider profile, permission policy or dependency change
is introduced. Existing task/thread/session IDs, drafts and serialized Studio
compatibility are unchanged. Older applications ignore the new preferences during
ordinary use, but their older backup validation does not understand these keys.
Keep the existing backup/version checks instead of claiming universal rollback.

## Verification ledger

Exact baseline blob identities for all modified source files were checked against
the published candidate before editing. Changed Rust source passes a lexer-aware
delimiter/reference review and whitespace checks. These are NOT compiler checks.
The pinned GPUI clipboard API was inspected, including public Image fields and
structured ExternalPaths entries, rather than assuming current docs match the pin.

Five focused Rust regressions are prepared: attachment encoding/input rejection,
atomic snapshots/capabilities/reopen, ID-scoped acknowledgement/future-data
preservation, saved follow-up ordering/stale edits, and richer draft echo handling.
They have not run. No cargo/rustc or built native app is available in this sandbox,
and the pinned toolchain host does not resolve. No native screenshots, real-agent
attachment exchange, pointer/keyboard/IME journeys or restart acceptance are claimed.
No GitHub test workflow was dispatched and workflow configuration is unchanged.

Remaining acceptance includes real PNG/JPEG/text delivery with negotiated agents,
unsupported capability/error/cancellation paths, image/file clipboard variants on
each OS, file drops, multiple windows and concurrent revisions, native tray sizing,
keyboard/IME, restart, backup/restore, deletion and privacy review. D4/D8/D11/D12,
F2/F4 and I10 remain open for their full scope. Side chats, PRs, Automations, voice,
MCP management and Browser are not implemented by this batch.
