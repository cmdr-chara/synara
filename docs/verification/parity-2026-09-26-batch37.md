# Batch 37 — six product features

Based on Rust main `5a7c8e27644c99a3a122c3e8441ed3ada1793b92`.
Preserves the separate batch 36 A05 acceptance receipt and concurrent release
workflow fixes. This batch prioritizes implementation over expanding test suites.

## Delivered

- **M01:** Web provider Connect/Reconnect, advertised ACP authentication methods,
  bounded cancellable jobs, route/connection/operation checks and exclusive
  task ownership of connection questions. Website opening and completion are
  explicit actions. Authentication drafts are never persisted. Direct-model
  execution uses the native credential owner; connection operations never send
  a prompt. Providers without advertised authentication still need CLI setup.
- **M02:** ACP raw input has a bounded redacted projection and full-input digest.
  Web permission cards refresh tool input, status, before/after diffs and output.
  An Allow reply must carry the exact current tool/context review fingerprint.
  Changed or hidden/truncated input changes invalidate the previous review.
- **M06:** Explicit HTTPS origin policy for a loopback backend behind a local
  TLS proxy, Linux systemd/Caddy configuration, deterministic headless packages,
  per-file integrity checks and serialized immutable version activation/rollback.
  SIGTERM gracefully shuts down the server. These operator-reviewed packages
  are unsigned; this does not complete M28 or A08. See
  [deployment instructions](../ui/headless-deployment.md).
- **M08:** Private provider-request-owned sign-in tabs on the Linux native browser,
  finish/cancel/retry controls and stale/timeout/close cleanup. WebKit defers the
  actual popup policy before child creation or network access. Approval resumes
  the original request into a related view, preserving POST and opener callbacks.
  Unapproved, replaced, stale and expired policies are explicitly denied.
- **M13:** A continuous capture/decoder worker supplies only the newest Simulator
  frame, with a four-frame-per-second ceiling, bounded memory and task/device/
  navigation ownership. Start/Stop live view controls do not create attachments.
- **M15:** Explicit Simulator MOV recording to a new destination, elapsed status,
  Stop/save and Discard. SIGINT finalization precedes atomic no-overwrite publish;
  five-minute/512 MiB bounds and cancellation discard unfinished recordings.

**M27 remains open.** This batch adds X11 screenshot targeting with letterbox-aware
coordinates, pointer/drag/scroll controls, a key palette, text preparation,
window filtering, larger preview and Escape takeover. macOS/Windows/Wayland and
broader full-desktop behavior are still separate unfinished work.

Also fixes stale image-save picker completion after navigation and a zero-match
exact test filter in the browser acceptance workflow. Existing server execution
fixtures were updated for the already-present automation runtime parameter.
The existing WebKit journey now checks the real opener callback and POST;
the existing live Simulator journey now checks recording finalization.

## Verification and limits

- Rust formatting, JavaScript syntax, roadmap counts and whitespace checks pass.
- All non-GPUI backend crates compile through `synara-server` using the repository's
  exact locked dependency versions in a temporary workspace excluding GPUI.
- The two HTTPS authority checks pass. Linux package stage/activate/rollback
  smoke checks pass using a local ELF fixture, without deploying a live service.
- Earlier focused checks in this batch: provider matrix configuration/credential
  tests (3 passed, paid live run ignored), signed-update target compilation and
  existing updater tests (6 passed). These do not establish live acceptance.
  The extracted image-picker ownership guard also passes its two focused checks.
- Full native compilation and the changed WebKit/Simulator behavior require the
  platform CI environments; this local environment lacks GTK/WebKit prerequisites.
- Workspace-wide strict Clippy remains blocked by existing warnings in
  `automations.rs`, `service.rs`, `storage/attachments/intake.rs` and
  `storage/task_creation.rs`, plus the server's existing nine-argument
  `execution::run_task`. The new permission-review lint was fixed. No lint rules
  were relaxed.

The real-account matrix, microphone/transcription, signed installer, production
host TLS and broad cross-platform journeys are not claimed accepted here.
The prior A05/A06 receipts remain evidence for their exact earlier candidates;
they do not prove the new popup bridge or recording paths. All 21 broad gates
remain unchanged. Execution inventory: **83 shipped, 10 major + 5 acceptance
items remaining = 15**.
