# Manual browser link downloads

September 23, 2026. Native baseline: `d9f66dd601e4ee9c91a09a738d0eb152d98526b7`.
Verified source candidate: `9319d10929c8da82476d18344df0c90c0f00d1b3`.
Upstream reference: `Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`, particularly `apps/desktop/src/browserSessionPolicy.ts` and the browser manager's download ownership checks.

## Implemented slice

On Linux/X11, a manual browser tab offers **Save linked file as...** in the HTTP(S) link context menu. The native asynchronous save dialog requires a new local filename. Progress and completion appear in the page tooltip. **Cancel download** is available in the originating page menu.

This is an explicit GET workflow, not a replay of intercepted form submissions. One transfer per manual profile is admitted. Transfer ownership includes the exact source URL, originating WebKit view and document epoch. The shared Wry profile gate prevents older tab callbacks from stealing a newer tab's destination. Agent and authentication partitions still deny downloads and no agent download capability is advertised.

Transfers have a 256 MiB limit and 120-second deadline. Data is staged privately under the selected parent directory. Successful 2xx responses are published with mode 0600 through an atomic no-overwrite hard link. Existing files and destination symlinks are refused. Files are never opened automatically. Cancellation, tab destruction, Stop and late callbacks release transfer ownership and private staging.

## Verification

[Native validation run 35888419279](https://github.com/cmdr-chara/synara/actions/runs/35888419279) passed:

- Rust 1.98.1 formatting and `cargo fmt --all --check`.
- 43 browser library tests and both existing history-readiness tests.
- Real WebKit download acceptance: exact bytes from two tabs sharing a profile, HTTP failure, cancellation, revoked epochs, no overwrite and staging cleanup.
- Strict browser Clippy, all targets, warnings denied.
- Existing real WebKit consent/input/redirect/isolation and viewport-capture regressions.
- The actual GPUI app and ACP fixture build, followed by the existing native browser smoke journey.

Exactly two new tests were added: filesystem publication boundaries and one real native download journey. No existing test was weakened.

The initial run 35887269033 failed the revoked-epoch case. Diagnostic run 35887947213 established that both successful transfers and HTTP-failure/cancellation cases worked, but Stop before download-started left the rejected ticket occupied until timeout. The fix retires that invalid ticket as well as cancelling the late native object. The same assertions then passed without fixture probes in the integrated source.

## Remaining scope

Browser parity remains a material-depth gap. This does not implement agent downloads, intercepted POST/blob transfers, a download-manager panel, saved-login import, protected credential portability, authentication popups, session restoration or WebMCP. Production authenticated websites and macOS/Windows/Wayland behavior are not accepted by these Linux fixture checks. The chooser menu is wired natively, while the download integration test invokes its owned transfer path directly. No full browser-parity or release-readiness claim follows from this slice.
