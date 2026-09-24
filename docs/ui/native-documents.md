# Native Library documents and export

The Hub Library now renders local PDF files as a read-only, single-page native
image with previous/next, fit, zoom and explicit reload. The document is an
immutable snapshot: changing the file on disk does not mix versions while paging.
Reload starts again at page one and rejects a newly damaged or missing file.
Changing the selected file/task, leaving the Library or closing the app cancels
an outstanding PDF request and rejects any stale result.

## Operating boundary

The initial renderer is Linux-only and requires OS-managed `poppler-utils`
(`/usr/bin/pdfinfo` and `/usr/bin/pdftoppm`) and `util-linux`
(`/usr/bin/prlimit`). Synara does not install or download a helper, search a
project's PATH, launch a browser, or send document bytes to a provider. Missing
helpers and other platforms report unsupported. Keep these OS packages updated.

WorkspaceFs opens only regular contained non-symlink files. Input is at most
8 MiB and 2000 pages. One preview decoder is admitted at a time. The selected
page is rendered within a 1600-pixel square, with bounded stdout/stderr, a
15-second request deadline, 10-second CPU limit, 768 MiB address-space limit and
16 MiB per-file limit. Poppler receives a snapshot on stdin. Its temporary
spool/font-cache files use a private temporary directory, and its environment
contains no inherited credential or display variables. The existing process-tree
owner stops and reaps helpers on errors, cancellation and dropped callers.
These controls are **not an OS security sandbox** and do not prove immunity to
renderer vulnerabilities. The native surface does not enable PDF links, scripts,
forms, attachments, editing, text selection or password entry. Encrypted,
malformed and over-limit inputs can be refused rather than partly represented.

The renderer is currently a Hub Library surface, not binary prompt intake or a
remote workspace/Explorer PDF implementation. Raster zoom enlarges the bounded
page image rather than claiming vector/text-layer parity with upstream pdf.js.

## Save current file as

Library files, including types without a preview, can be copied through an
explicit review showing the selected path and snapshot byte count, followed by
the native new-file destination chooser. The 8 MiB containment limit applies.
This exports the **current original file**, not a thumbnail or previously loaded
PDF snapshot. A changed source, changed task directory, expired five-minute
review, invalid destination, existing file or symlink is refused. The shared
private-staging/no-overwrite export writer publishes only complete bytes.

Cancelling review or the chooser writes nothing. Changing selection before
confirming the destination cancels. After confirmation, the owned write reports
its result independently of subsequent preview navigation, and app close waits
for that result. The native save picker still requires platform acceptance.

Upstream comparison: `apps/web/src/components/PdfFilePreview.tsx` and its page
navigation/zoom hooks at `eaa61eded31b6755d4f30ba8eabc5d905cf817cb`. This is a
bounded addition, not full PDF, attachment, Studio or platform parity.
