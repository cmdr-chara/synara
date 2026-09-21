# Pull Requests, Automations and Browser verification

## Proven native candidate

Candidate: `ed114ca62689b081f10c288c44ded0830dd80465`.
Run: https://github.com/cmdr-chara/synara/actions/runs/35662029685
Environment: Ubuntu 24.04, Rust 1.98.1, Linux/X11 on private Xvfb, WebKitGTK 4.1,
Wry 0.57.0, the repository-pinned GPUI, owned loopback pages and fixture agents.
The exported source had a clean working tree. No vendor account was used.

| Evidence | Result |
| --- | --- |
| Browser library, native feature enabled | 40 passed, 1 ignored |
| Ignored real WebKit integration test run explicitly | 1 passed |
| Workspace browser HTTP/MCP tests | 4 passed |
| Pull Requests service tests | 9 passed |
| Durable Automation tests | 10 passed |
| Native application and ACP fixture build | Passed |
| Real GPUI child rendering and input journey | Passed |

The native journey used actual display pixels and pointer events to prove a real
HTTP-served page rendered inside the GPUI Browser pane, a page click changed the
DOM and pixels, native command-palette overlays hid/restored the child, history
navigation and reload made the expected requests, tabs could be created/selected/
closed without ghost content, and the app exited cleanly.

The real WebKit test separately proved manual load, cookie partition isolation,
one-shot consent, document reading, fill and click, rejection of a cross-origin
redirect before its target received a request, cancellation and teardown. It did
not replace browser rendering with a mock. The loopback MCP tests are separate
transport evidence, not an end-to-end live model demonstration.

## Integration candidate

The integration branch subsequently advanced to
`a77409783d0d63f732761aba9ad26e507ccc678c` with side chats and message revisions.
The local three-way merge is conflict-free. The native child also needs to hide
behind the new revision overlay. The session workflow now checks that merged
candidate and publishes it to the same session branch only after all checks pass.
Integrated acceptance and final integration-ref verification remain pending until
this receipt is updated with the resulting candidate and run.

## Boundaries and warnings

The existing unused `ROW_HEIGHT` warning remains. Private Xvfb emitted a DRI3
acceleration warning. This evidence is not GPU-driver acceptance. No workspace-wide
lint or all-platform suite was run as part of this focused native acceptance.
Windows, macOS and native Wayland browser hosts, IME/accessibility, HiDPI, production
websites, downloads/capture exports, live PR writes and live Automation provider
completion/cancellation remain open. IANA/DST schedules, automatic retry and
history pruning are not implemented. The original 120 roadmap tasks and their
checkbox states are unchanged. This is development-branch evidence, not a release.
