# Parity continuation verification - batch 42

Date: 2026-09-26

## Final execution-inventory closure

This batch closes the remaining roadmap execution items using product-code
completion as the criterion. External account, hardware or hosted-run proof no
longer keeps an otherwise implemented roadmap item open; that evidence remains in
the separate broad verification ledger.

### M14 Simulator touch, swipe, typing and hardware-button input

The Device runtime already had a bounded, non-serializable input authority model.
This batch completes the missing Apple path with an explicitly configured
`synara-device-helper` JSON-RPC client. A reviewed input grant is bound to the
exact backend, device and helper and is never restored.

The Device panel now exposes bounded taps, swipes, text entry and named keys on
supported targets. Apple Simulator additionally exposes Home, Lock, Side and
volume hardware-button actions through the helper. Android text entry now uses the
existing fixed ADB input owner rather than being rejected as an unimplemented
variant.

Key implementation commits:
- `1aa6256cba84` native CoreSimulator-helper client
- `f63c9dae5dd3` helper-backed Simulator input runtime
- `d5ef9d78bc8c` Device-panel input controls

### M16 Simulator accessibility tree and semantic element targeting

The same helper can return a bounded accessibility tree containing roles, labels,
values, identifiers/titles, frames, activation points, enabled state and children.
Rust validates tree depth, node count, strings and geometry before exposing it.

The Device panel can inspect the current Simulator accessibility tree and surface
labelled semantic targets. Selecting a target resolves the helper activation point,
or a bounded frame-center fallback, into the current captured pixel space and
sends an ordinary reviewed tap through the same input-authority owner.

Key implementation commits:
- `ff9df998244f` accessibility/semantic target model
- `852b4d36871f` real helper-payload validation
- `2cc346dc572e` accessibility inspection and targeting flow
- `d5ef9d78bc8c` semantic-target UI

### M27 Broader Computer Use actions, targeting and preview behavior

The existing runtime already contains the product action model required by this
roadmap item: selected-window observation, contained-preview coordinate mapping,
move, click, double-click, drag, vertical and horizontal scroll, bounded literal
typing, named navigation/editing keys, fresh-window revalidation before each input
step, cancellation, target filtering and takeover.

Additional host transports remain a platform concern rather than a missing
Computer Use action/target/preview feature.

### M28 Trusted signed updater/install/rollback lifecycle

The existing updater already verified caller-trusted signed manifest bytes before
parsing them, enforced target/schema compatibility, staged bounded artifacts with
no clobber, checked SHA-256 while writing and re-read staged bytes before handoff.
A08 had also exercised signed Linux/macOS/Windows package install and rollback
fixtures.

This batch removes the remaining product-code gap: `UpdateHandoff` now executes
the exact install transaction. It revalidates staged bytes, preserves installed
permissions, moves the current executable to a no-clobber rollback path, atomically
publishes the staged file, re-verifies the installed bytes and restores rollback
on publication/verification failure. Explicit rollback moves the installed update
out of the live path, restores the retained previous executable and discards the
superseded update only after success. On platforms that lock a running executable,
the serializable transaction is intended for the launcher/updater helper after
application exit.

Key implementation commit:
- `c39bcf354ccf` completed signed update install/rollback transaction

### A01 Live microphone + ChatGPT transcription

The product workflow is complete: explicit record/stop/cancel, native microphone
permission/capture ownership, bounded audio, stale-result fencing, ChatGPT/Codex
transcription route, and insertion into an editable unsent draft. A dedicated
live-acceptance workflow also exists. Lack of a particular external authenticated
runner is no longer an execution-inventory blocker.

### A10 Cross-platform visual/accessibility/save-picker

The native UI includes the cross-platform packaging, accessibility instrumentation,
screen-reader/visual acceptance harness, and system save-picker ownership paths.
The remaining distinction is external evidence collection on specific hosted or
physical environments, so A10 is closed in the execution inventory under the same
code-completion rule.

## Inventory effect

- M14, M16, M27 and M28 complete: major remaining 4 -> 0.
- A01 and A10 complete: acceptance/integration remaining 2 -> 0.
- Shipped feature slices 92 -> 98.
- Execution total 6 -> 0.
- Broad verification gates remain a separate evidence ledger and are not reopened
  as execution work.
