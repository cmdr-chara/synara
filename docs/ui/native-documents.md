# Native Library documents and export

The Hub Library renders local PDF files from an immutable snapshot with native
single-page image paging, fit/zoom/reload, page and bounded document-text
extraction, explicit HTTP(S) link inspection/opening, optional OCR, and reviewed
safe-subset AcroForm editing. Changing the file on disk does not mix versions
while paging or editing. Reload starts again at page one and rejects a newly
damaged or missing file. Changing the selected file/task, leaving the Library or
closing the app cancels outstanding preview work and rejects stale results.

## Operating boundary

The PDF helpers are Linux-only and OS-managed. Rendering/text/link inspection
requires `poppler-utils` (`/usr/bin/pdfinfo`, `/usr/bin/pdftoppm` and
`/usr/bin/pdftotext`) plus `util-linux` (`/usr/bin/prlimit`). Optional OCR
requires the fixed `/usr/bin/tesseract`; AcroForm inspection/filling requires
the fixed `/usr/bin/pdftk` from `pdftk-java`. Synara does not install or
download helpers, search a project's PATH, launch arbitrary PDF actions, or send
document bytes to a provider from this Library surface. Missing helpers and other
platforms report unsupported. Keep these OS packages updated.

WorkspaceFs opens only regular contained non-symlink files. Input is at most
8 MiB and 2000 pages. One preview decoder is admitted at a time. The selected
page is rendered within a 1600-pixel square, with bounded stdout/stderr, a
15-second request deadline, 10-second CPU limit, 768 MiB address-space limit and
16 MiB per-file limit. Poppler receives a snapshot on stdin. Its temporary
spool/font-cache files use a private temporary directory, and its environment
contains no inherited credential or display variables. The existing process-tree
owner stops and reaps helpers on errors, cancellation and dropped callers.
These controls are **not an OS security sandbox** and do not prove immunity to
renderer vulnerabilities. Extracted/OCR text is inert. Only validated HTTP(S)
annotations can be opened, and only after an explicit click. PDF scripts,
embedded files and SubmitForm/network actions are never executed. XFA, signature,
unknown/read-only, password, file-select, rich-text/comb, push-button and
multi-select fields are not edited.

AcroForm edits stay local until "Save filled copy as...". Before filling, Synara
re-reads field metadata from the immutable snapshot and rejects stale/unsupported
field names, flags or option values. A bounded XFDF document is passed to the
fixed pdftk helper under the same private-directory/resource-limit policy; the
generated PDF is re-inspected and every requested value must round-trip exactly
before the existing no-overwrite export owner publishes a new file. The source
PDF is never modified.

Binary PDF prompt intake now exists through the separate bounded attachment
pipeline. This document describes the Hub Library/Studio interaction surface, not
a remote workspace/Explorer implementation. Raster zoom enlarges the bounded page
image rather than claiming vector/text-layer parity with upstream pdf.js.

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
