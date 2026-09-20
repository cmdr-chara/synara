# Batched native conversation utilities

Date: September 20, 2026. Delivery: `astra/gpui-clean-rewrite` only.
Base implementation: `ec1c946b7959738e46e38aab2decfd4c226cb47c`.
Published implementation: `2808549852c23bf80d93e851d1b04c995d57ebb0`.
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
   stored credentials, configuration, tool payloads and attachments are excluded.
   Secrets written inside message text are not automatically redacted.
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
resize/save. This targets the observed equal-split save timeout without changing
split ratios, themes or process ownership. Native confirmation remains pending.

## Recovery and verification state

The interrupted transfer contained a complete nine-file prefix but ended inside
`transcript.rs`. The recovered prefix was retained as review commit
`9a6b7770e1ecbd619a2d25cdf9f0004b18f2203e`, not mistaken for a complete build.
The missing storage module, transcript jump implementation, exports, preference
cleanup and test wiring were completed afterward. Recovery-only tooling is removed
from the final source tree and the normal native verification workflow restored.

### Observed build evidence

[Run 35515246665](https://github.com/cmdr-chara/synara/actions/runs/35515246665),
job `106089970972`, Ubuntu 24.04 x64, pinned Rust 1.98.1:

- PASS: Rust formatting for `synara-app` and `synara-workspace`, Python syntax
  parsing of the new/changed scripts, and `git diff --check`.
- PASS: `cargo +1.98.1 check --locked -p synara-app --bin synara-app`.
- PASS: no unstaged source changes during compilation, including no lockfile change.
- DEFERRED: unit tests, Clippy, native UI journeys, screenshot comparison and
  native save-dialog acceptance. These were not run for this batch.

The runner checked an assembled worktree rather than its trigger commit.
Its recorded `git write-tree` was
`efc7671284f22ad81629688a8dd50f2d08d4962b`. The GitHub plugin recreated that
identical tree and published it as source commit
`2808549852c23bf80d93e851d1b04c995d57ebb0`. The subsequent receipt edit is
documentation only. Source patch artifact: `10606817048`, SHA256
`a96573ade7a0a7faae0bb9f4422805f5a7c37d6b2a27df85ec16e62a21c3a483`.

The first assembly run `35514971101` stopped before compilation on a GitHub
Actions token HTTP 403. The follow-up retained all formatted source blobs and
compiled successfully, but its optional review-tree export also received
`Resource not accessible by integration`. That export step was allowed to fail,
so the overall green workflow is not claimed as an all-steps pass. Final tree,
commit and branch publication were completed using the authorized GitHub plugin.

Implementation was batched first as requested. Regression tests and the combined
native chat utility journey are written for subsequent execution. Their existence
is not test evidence, and compilation is not runtime acceptance. No whole roadmap
gate is closed by this batch.

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
