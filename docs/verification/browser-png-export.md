# Manual PNG export verification

September 23, 2026. Source baseline `e4a740f11450e03f26b0435f0d3e442dc1325e7b`.
Candidate input `deca98cda22b2a4eaccf38c8f19808a36503a7ec`. The exact formatted source is committed by
[validation run 35896910604](https://github.com/cmdr-chara/synara/actions/runs/35896910604)
after the deciding checks.

## Scope and ownership

Upstream `Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`
was compared through `browserManager.ts` captureScreenshot and its native bridge.
The native slice adds an explicit PNG save action, not full upstream screenshot
parity. It reuses the current manual viewport capture owner and private
no-overwrite download publisher. Both publication and capture authority are
revoked by the document epoch. Agent capture capability is unchanged.

The chooser accepts only a user-selected local destination. Encoding uses the
existing locked gdk-pixbuf with its async stream API. Snapshot and encode errors,
hidden/loading or replaced pages, cancellation and timeout publish no output.
The selected file is never overwritten, followed as a symlink or auto-opened.

## Actual checks

- Rust 1.98.1 formatting and fmt check.
- Strict browser, workspace and app Clippy for all targets with warnings denied.
- Focused conversation and native command regressions, browser unit and history tests.
- One new real WebKit regression: PNG signature and decoded viewport dimensions,
  unchanged clipboard, existing-file preservation, revoked-epoch refusal and
  private staging cleanup. Marker: `PNG_EXPORT_ACCEPTANCE`.
- Existing real clipboard-capture and manual-download regressions.
- Actual GPUI/ACP fixture build and existing chat-tools and embedded-browser journeys.
- Structural and roadmap checks preserving the historical 120 tasks and 16 checks.

The system file chooser interaction itself is not automated by this regression.
The snapshot, async encoder and publisher run against real WebKitGTK, not mocks.
This is Linux/X11 fixture acceptance only. No macOS/Windows/Wayland or real
authenticated-site proof is claimed. Exactly one Rust regression was added for
this PNG slice, with no new general test harness.

## Preserved dependency failure

Run 35892272307 failed dependency resolution before Rust tests because the
proposed `v2_36` feature is not exposed by pinned gdk-pixbuf 0.18.5. Inspection
of gtk-rs-core tag 0.18.5 confirmed that `save_to_streamv_async` is available
unconditionally and the only opt-in version features are v2_40/v2_42. The
unsupported flag was removed without changing package versions or encoder
behavior. This is a diagnosed manifest correction, not an ignored test.

## Remaining parity

Browser/WebMCP remains a material depth gap. Full-page capture, automatic
composer attachment, saved-session import, auth popups and WebMCP are not closed.
This batch adds neither a production release nor a cross-platform support claim.
