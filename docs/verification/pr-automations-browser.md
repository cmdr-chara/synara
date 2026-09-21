# Pull Requests, Automations and Browser verification

## Checked integration candidate

Candidate: `10ff2e0d2d29800ac3894f03e2fbcc851af8efa6`.
Run: https://github.com/cmdr-chara/synara/actions/runs/35665877900
Artifact: `native-webkit-evidence` / `10669062076`.
Artifact ZIP SHA-256: `03f1c3191648fdadf670f32d2e53e18425e804b1f373b2400948d9e59f07d60b`.

This is the actual checked merge, not just the workflow trigger SHA. Its parents
are session fix `93cff9363d858e3294e5ff8f0e927992d92cb67b` and integration head
`a77409783d0d63f732761aba9ad26e507ccc678c`. It preserves the concurrent Side Chats
and message Revisions work. The browser child also hides behind the new revision
overlay. The workflow published this merge to the existing session branch only
after every check passed. The exported source had a clean working tree.

Environment: Ubuntu 24.04, Rust 1.98.1, Linux/X11 on private Xvfb, WebKitGTK 4.1,
Wry 0.57.0, repository-pinned GPUI, owned loopback pages and fixture agents.
No live vendor account was used.

| Evidence | Result |
| --- | --- |
| Browser library, native feature enabled | 41 passed, 1 ignored |
| Ignored real WebKit integration test run explicitly | 1 passed |
| Public Session delayed-history regressions | 2 passed |
| Native browser history-control tests | 2 passed |
| Workspace browser HTTP/MCP tests | 4 passed |
| Pull Requests service tests | 9 passed |
| Durable Automation tests | 10 passed |
| Concurrent related-thread storage tests | 3 passed |
| Native application and ACP fixture build | Passed |
| Real GPUI child rendering and input journey | Passed |

The ignored test was not counted as passing in the first row. It ran explicitly
in a separate private-display command and passed. The workspace and app commands
are focused filters, not evidence for the unexecuted remainder of those suites.

## Native evidence and regressions

The actual GPUI journey proves an HTTP-served page renders inside the Browser
pane, pointer input changes the real DOM and pixels, command-palette overlays
hide/restore the child, Back/Forward/Reload work with deliberately delayed HTTP
responses, blank/new/selected/closed tabs do not retain another page's pixels,
and the app exits cleanly. Assertions use display pixels and native events, not
OCR, mocked rendering, or screenshots substituted for browser content.

The real WebKit test separately proves manual load, manual-cookie acceptance
followed by task-cookie isolation, one-shot consent, document reading, fill and
click, private-world inventory isolation, blank-tab/overlay visibility, and a
cross-origin redirect refused before its target receives a request. Queued
cancellation and teardown are covered by focused native/domain tests. Loopback
MCP tests are transport evidence, not an end-to-end live model demonstration.

Earlier candidate `ed114ca62689b081f10c288c44ded0830dd80465` passed run
`35662029685`, but that result did not establish the later merged candidate.
Merged run `35663245421` exposed a history race: the test used HTTP request
arrival before native commit. Fix `e74ba8e` gates UI history actions and changes
the test to wait for committed/rendered state. Run `35665093997` then passed the
delayed history sequence and exposed retained pixels after selecting a blank
tab. Fix `93cff93` maps the outer native surface only for a selected live view,
and resynchronizes that mapping after creation, cancellation and teardown.
The final run above passed both unchanged behavioral requirements.

## Delivery and repeatability

Only `astra/pr-automations-browser` is used for this session. No pull request or
release is part of the delivery. Final cleanup changes documentation and CI
metadata only, leaving runtime source, tests and Cargo.lock identical to the
checked merge. Session and primary integration ref updates are verified
separately after that cleanup and recorded in the delivery handoff.

`.github/workflows/native-webview.yml` retains the focused test commands as a
manual/callable read-only workflow. Session export, patch-application and
source-publishing scaffolding is removed. No broad native CI matrix is rerun
merely to publish the documentation cleanup.

Local supplemental checks: roadmap structure, Python smoke syntax, exact blob
identity, and rustfmt checks on the touched browser files passed. The isolated
offline browser-domain harness also passed 33 library tests and the 2 public
history regressions. That harness is not counted as native or full-lockfile
acceptance. The CI commands above used the repository lockfile.

## Open acceptance and warnings

The existing unused `ROW_HEIGHT` and concurrent `RevisionState::pending` warnings
remain. Private Xvfb emitted a DRI3 acceleration warning. This evidence is not
GPU-driver acceptance. No workspace-wide lint or all-platform suite was run.

Windows, macOS and native Wayland hosts, IME/accessibility, HiDPI, production
websites, download/capture exports, live PR writes and live Automation provider
completion/cancellation remain open. Native visual acceptance here covers the
Browser, not every Pull Requests or Automations UI journey. IANA/DST schedules,
automatic retry and history pruning are not implemented. Browser approval is
not a general same-origin network sandbox. The feature guides describe those
boundaries explicitly.

The original 120 roadmap tasks and their checkbox states are unchanged. This is
development-branch integration evidence, not release readiness.
