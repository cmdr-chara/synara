# September 24 batch 4: documents, Library export and inline import

Native base: `4c9240c395cd08d677a344b2721624dcc6148006`. Upstream main was rechecked
at `eaa61eded31b6755d4f30ba8eabc5d905cf817cb`. The previous 34 slices are preserved.
The relevant upstream sources are `PdfFilePreview.tsx`/PDF navigation hooks and
`onboarding/steps/ProjectStep.tsx`, which embeds its shared import panel.

## Delivered boundaries

- Local Hub Library PDF viewing uses a bounded immutable file snapshot, previous/
  next page, fit, raster zoom and explicit reload. Native task/file/generation
  ownership and cancellation fence asynchronous results. The existing process
  supervisor owns fixed system Poppler helpers, with private temporary storage,
  no credential inheritance and resource limits. This is not an OS sandbox.
- Reviewed Library export captures the current original file, checks it again
  before writing, and reuses private staging plus no-overwrite publication.
  Source changes, destination collisions, symlinks, cancellation and stale owner
  reviews are refused. Preview conversion never changes exported source bytes.
- Onboarding's project step embeds the existing history discovery/preview/
  destination-confirmation/import flow. Pending reads or review block setup
  navigation and hiding the panel. Preparing/importing history never launches an
  agent, and the guide remains available after completion of the import.

No new renderer crate or lockfile is introduced. The already-locked tempfile
crate moves from runtime dev-only use into normal runtime use for private helper
storage. This required dependency-scope change is confined to the PDF owner.

## Validation contract

Targeted tests cover PDF metadata/argument/output bounds, pre-cancellation,
real two-page Poppler rendering, immutable paging across disk changes, invalid
reload/path/symlink refusal, original export bytes and failure paths. The native
journey covers real GPUI page/zoom/reload, export review, inert reopening and
inline import with navigation guards. The integrated pass is limited to
runtime/workspace/app, formatting, strict Clippy and roadmap checks.

Actual successful workflow evidence is appended before publication. A receipt
existing in an unpublished candidate does not mean its checks passed.

## Remaining work and unavailable acceptance

All 21 parity gates stay OPEN. The native save chooser is not exercised by this
Xvfb journey. PDF availability requires Linux system packages, and macOS/Windows,
remote PDF viewing, binary PDF attachments, PDF text/links/forms and historical
Studio content capture remain open. No provider or hardware acceptance is claimed.
Simulator input still needs an implemented target-scoped input backend rather
than merely another simctl argument, and is not part of this batch. Provider
telemetry and signed update trust cannot be manufactured from local fixtures.

## Observed validation

129 app, 90 runtime and 310 workspace unit tests and enabled integration tests passed in run 35994731513. The real-PDF test exposed a helper waiting for stdin EOF. Explicitly dropping stdin fixed that boundary. Focused PDF/runtime/Studio tests, the real-Poppler test and strict Clippy passed in run 35995404125.
Rust sources and manifests match that final passing Rust candidate exactly: Git-index manifest SHA-256 `48b9984789dcc05805dd34d861044895ab8b25b61fe862a3d6c3af0eaec4f512`. The native scroll helper now targets the settings page margin instead of a text input.
Final native PDF/import, formatting and roadmap evidence: https://github.com/cmdr-chara/synara/actions/runs/35996339413
Run attempt: 1. Publication base: `063c07e63a3fe1b6def5009486b1a4c41142c3fb`.
Linux/X11 evidence covers two-page rendering, raster zoom, source-snapshot isolation, damaged-file reload, inert export review, restart and inline history import with pending-review navigation guards. Export service fixtures cover exact bytes, stale source, cancellation, symlinks and no-overwrite publication. The native save chooser, real providers and other platforms were not exercised.
