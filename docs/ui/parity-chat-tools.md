# Batched native conversation utilities

Date: September 20, 2026. Delivery: `astra/gpui-clean-rewrite` only.
Base implementation: `ec1c946b7959738e46e38aab2decfd4c226cb47c`.
Electron reference: `948875954f432978eab7dd5fa44c3028b8d99a81` and the supplied
73-image UI/UX reference. Theme defaults are unchanged. Dracula remains optional.
Roadmap ownership: D8, D9, D12, F2, F8 and I10, all partial.

## Implementation batch

1. Find in conversation: literal, case-insensitive search over reconstructed
   saved message text, bounded results, next/previous navigation, Ctrl/Command+F
   and F3/Shift+F3. Search can reveal a message inside collapsed work details.
   Explicit jumps suspend automatic tail following until the user resumes it.
2. Message pins: role-aware durable anchors, pin/unpin actions and a searchable
   pinned-message navigator. Pin writes are idempotent SQLite transactions.
   Unknown/future pin data is preserved instead of silently reset. Task deletion
   removes its pin preference in the same deletion transaction.
3. Prompt reuse and quotation: reuse the last user prompt, reuse an individual
   prompt, or quote a response into the current draft. Existing text is retained.
   None of these actions sends a prompt or performs turn rollback.
4. Copy text conversation: copy saved user, assistant and reasoning text as
   Markdown. This is not a complete multimedia/session export. Unsent drafts,
   credentials, configuration, tool payloads and attachments are not exported.
5. Export text conversation: the native save dialog selects a local destination.
   A private staging file is completed first, then published without overwriting
   any existing destination or following a destination symlink. Unsupported
   filesystem operations fail visibly. No upload or external sharing is performed.
6. Advertised command discovery: search the current thread's advertised slash
   commands and insert a selected command into the draft. Unsupported commands
   are not invented, and selecting one does not automatically submit it.
7. Cross-project thread finder: Ctrl/Command+K searches active thread titles,
   project names, providers and scopes, retaining the existing guarded navigation.
   Archived threads and hidden Studio entries are excluded.

The existing Synara icon assets, semantic palette, shared menu/input components,
controller, permission handling and draft writer remain authoritative. Search
results are generation-scoped so late worker responses cannot replace a newer
query or a different conversation. New menu actions retain selected-task identity.

The batch also releases an armed Environment divider gesture before a keyboard
resize/save. This addresses the observed equal-split save timeout without changing
split ratios, themes or process ownership.

## Recovery and verification state

The interrupted transfer contained a complete nine-file prefix but ended inside
`transcript.rs`. The recovered prefix was retained as review commit
`9a6b7770e1ecbd619a2d25cdf9f0004b18f2203e`, not mistaken for a complete build.
The missing storage module, transcript jump implementation, exports, preference
cleanup and test wiring were completed afterward. Recovery-only tooling is removed
from the final source tree and the normal native verification workflow restored.

At this checkpoint the implementation is assembled first, as requested by the
user. The new regression tests and the combined native chat utility journey are
written for subsequent execution. Do not count them as passed merely because they
exist. Compilation/formatting results, when obtained, are separate from runtime
acceptance. No whole roadmap gate is closed by this batch.

Prepared checks cover role identity, streamed-chunk search, literal wildcard
handling, unknown IDs, pin persistence/concurrent openers, stale/corrupt pins,
deletion cleanup, private no-clobber exports, Unicode/whitespace, prompt reuse,
command-name validation, hidden-message navigation and an owned native UI journey.
The focused batch lane is selected only for known changed paths. Unknown or
backend/dependency changes keep the broader fallback.

Retained prior failure: Environment run `35511918838` passed tab selection,
close/reorder safeguards and the existing chat, Studio, guarded-close and Kanban
journeys, but failed at keyboard equal-split persistence. That run is not green.
The additional Environment gesture correction in this batch still needs its
native journey rerun during the combined verification phase.

## Limits

Search returns at most 500 matching messages and uses a bounded, consistent
SQLite replay snapshot on a worker. Text exports are limited to 8 MiB. Pins are
limited to 256 per task. These are not full-text indexing or long-session
performance acceptance claims. Exported conversation text may be private and may
contain secrets that the user themselves included in messages.

Native save-dialog success/failure, all new UI journeys, real IME, screen readers,
macOS/Windows/Wayland, authenticated vendor behavior and the complete 73-state
visual comparison remain unverified for this batch. Edit/resend, file-affecting
rollback, message forks, attachments/voice, rich media sharing, browser embedding,
side chats and the other open product routes are not delivered by these utilities.
