# Parity batch 12: PDF attachment context

M29 partial: selected PDF files are imported as binary local snapshots after
bounded Poppler validation. The composer labels PDF attachments and previews
extracted text. At send time, up to 12 pages are extracted as labeled text
context, with an explicit omitted-page notice and 512 KiB per-document and
2 MiB combined PDF-text limits. The PDF bytes are not sent to the provider.
Empty/image-only pages cannot be used as text context. Extraction uses the
existing fixed helper path, resource bounds and timeouts. Other binary document
formats, OCR and full PDF interaction remain open.

DOCX addition: only `word/document.xml` is read from a selected archive. The
archive, decompressed XML and extracted text are each bounded. Paragraphs,
line breaks and tabs are projected as plain text; relationships, embedded
objects and external resources are ignored. Empty or malformed main text is
rejected before the attachment is saved. Its extracted content is previewed
and sent as text context under the combined document limit.

Studio Library also extracts bounded DOCX main-document text for a read-only
preview and Copy text. Export still saves the original DOCX. Open in editor
remains available for plain text only, so binary documents are not treated as
editable source. Rich formatting, embedded media and page-level interaction
remain open.

An open Studio PDF snapshot now offers bounded, labeled text extraction for
its first 12 pages with explicit disclosure when more pages exist. The text is
previewed read-only (first 128 KiB on screen) and the full bounded extraction
can be copied. Page changes or file reload cancel the old extraction result.

M02 partial: the web one-time permission review displays up to two bounded
recorded tool text or terminal outputs, including truncation and exit status.
Shortened recorded diffs are also labeled before a one-time grant.
This uses the matching task thread tool snapshot and remains read-only until
an explicit one-time response. The provider's live command request and full
interactive diff review still require additional work.

M30 partial: Studio keeps at most 12 text snapshots of at most 128 KiB each
during an open session. Refreshing a changed file captures a new version;
earlier versions are labeled with capture times and can be previewed/copied
without modifying the file. Session snapshots are cleared when Studio closes,
so durable uncommitted output history remains open.

M30 lifecycle partial: a completed tool event for the selected Studio task
refreshes the open Library. If a list read is already running, a subsequent
completion schedules one more read after it finishes. A different task or a
closed Studio does not start a refresh. Explicit manual Refresh remains available.

M21 partial: the editor's changed-block review can copy a bounded block with
added/removed markers after validating its owner, generation and exact current
comparison. A stale view cannot copy another buffer's content. A new two-step
Restore all action replaces only the unsaved editor buffer with the selected
reference. The confirmation is cleared on any buffer/comparison change, and the
confirming click rechecks task/project/root/path/tab ownership, generation and
the exact current diff. Neither action writes the file or touches the Git index.

M22 recovery: selecting a Synara-shaped unassigned worktree from the existing
fork menu now uses a dedicated recovery operation. It rechecks the source task,
the generated branch and worktree directory UUID, the linked unassigned entry,
and live Git metadata under the lifecycle lock before saving an unsent draft.
Ordinary existing worktrees continue through their existing creation path.
Automatic cleanup and broader crash recovery remain open.

CI prerequisite: the Linux backend workflow installs `libglib2.0-dev` with
its GTK/WebKit dependencies before regression checks, so later always-running
all-features jobs have the native `glib-sys` dependency even when a preceding
structural check fails. The DOCX text helper now has crate visibility matching
its crate-level re-export, fixing the E0364/E0603 compilation failure without
making the helper public outside `synara-workspace`.
The web workspace client asset now lives under `assets/web-workspace`; the
server embeds it from there. The native structural audit keeps its no-JavaScript
in-Rust-crates rule while allowing the added server crate.

Validation pending with final batch checks.
