# ZIP and PNG export integration receipt

## Exact source and publication outcome

Target baseline: `e4a740f11450e03f26b0435f0d3e442dc1325e7b` on
`astra/gpui-clean-rewrite`.

[Run 35896910604](https://github.com/cmdr-chara/synara/actions/runs/35896910604)
validated input `deca98cda22b2a4eaccf38c8f19808a36503a7ec` and produced commit
`1279a2a78f324f3e3d933a83856d8080e4fae700`, tree
`d31ed6e3f42b9140b00efa55acae918973bd35e6`.
All compilation, formatting, focused tests, strict Clippy, structural checks and
both native application journeys passed. **The workflow as a whole is not green.**
Its subsequent Git push was rejected because the runner's GitHub App token did
not have `workflows` permission for `.github/workflows/native-webview.yml`.
That rejection is not a test failure or evidence that the connected GitHub
account cannot publish changes.

The retained commit object and exact tree were recovered through the authorized
GitHub connector. Integration uses that tree directly, without regenerating or
changing the verified Rust, manifests, lockfile, Python journeys or permanent
workflow. It removes only these run-scoped transport files and adds this receipt:

- `.github/parity-next.patch.gz`
- `.github/validate_zip.py`
- `.github/finish_exports.py`
- `.github/workflows/parity-downloads.yml`

The final integration commit is parented directly on the target baseline, not on
the temporary validation history. No unrelated branch is reset or force-pushed.

## Measured verification

The successful validation steps executed 57 Rust tests:

| Check | Passing tests |
| --- | ---: |
| Browser unit/domain checks | 43 |
| Browser history readiness | 2 |
| Conversation tools, including ZIP export | 8 |
| Native command parser | 1 |
| Real WebKit PNG export | 1 |
| Real WebKit clipboard capture | 1 |
| Real WebKit manual downloads | 1 |

Four display-dependent tests were initially ignored by the normal browser unit
invocation. Three were then explicitly run under isolated Xvfb as listed above.
The existing real navigation/redirect test is not claimed as executed in this
combined run. The permanent native-webview workflow continues to run it.

The existing native chat-tools journey passed all seven recorded workflow checks,
including ZIP menu discovery, draft/event preservation, pinning, copy/reuse,
command handling and restart. The existing GPUI embedded-browser journey passed
all five recorded checks, including real network/rendering, pointer input,
delayed-commit history, tab lifecycle and clean shutdown.

`cargo +1.98.1 fmt --all --check` passed. Strict Clippy passed for browser,
workspace and app with all targets and warnings denied. Exact lockfile validation
allowed only direct links to already-locked zip 8.6.0 and gdk-pixbuf 0.18.5,
without adding or upgrading package versions. Structural checks confirmed nine
crates. Roadmap checks confirmed all 120 historical tasks and 16 checked states
are preserved, not reclassified as product-completion evidence.

Only two new Rust regression tests were added for these two export slices.
Existing native journeys were extended, not replaced. Earlier failed attempts
and their diagnosed corrections remain in the
[ZIP verification record](conversation-zip-export.md) and
[PNG verification record](browser-png-export.md). None was deleted, relabeled
as flaky or made green by relaxing an assertion.

The retained artifact is `parity-zip-evidence`, ID `10766659509`, SHA-256
`8b8192ad94c79c2c203ad11b4b0f79f1487ffa30dd6983840ae4684d7ea8f50a`.
Its chat and browser result files both report `passed`. It includes native
screenshots and test logs. The artifact has a three-day retention period, so
this receipt and the commit source retain the lasting record.

## Scope, privacy and remaining work

ZIP export writes `thread.json` and `transcript.md` from one completed durable
snapshot, with no unsent draft or stored credential/session configuration.
Transcript text is not a secret-redaction service and may itself contain private
information supplied during the conversation. Export is explicit and never
starts an agent. Existing destinations are not overwritten.

PNG export is a human-triggered save of the visible manual-browser viewport.
It shares existing capture ownership, cancellation and no-overwrite publication.
No agent screenshot/download authority, automatic attachment or file launch is
added. Full-page capture is not implemented by this slice.

The system save pickers themselves were not automated. ZIP bytes/privacy and
publication were exercised at the service layer. PNG pixels/encoding and
publication were exercised against real WebKitGTK. This is Linux/X11 fixture
acceptance, not macOS/Windows/Wayland, hardware, live-provider or authenticated
production-site acceptance. No production release or general ship approval is
implied.

The browser lane remains a material depth gap. Thread export advances within
near parity. Voice input, first-run onboarding and the headless/web workspace
remain missing. The other parity lanes and historical acceptance gates remain
open as recorded in the roadmap.
