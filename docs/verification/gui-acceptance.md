# GUI and measurement acceptance matrix

This is a test plan for the agreed Zen/Synaric product, not a claim that those
layouts, browser adapters or platform journeys are implemented. The behavioral
contract is [Zen/Synaric architecture](../architecture/zen-synaric.md), and its
implementation backlog is [the R/S/T supplement](../roadmap/zen-synaric.md).

## How to record evidence

Record the exact source SHA, executable hash, build profile, operating system,
architecture and test identifier. Separate automated fixture, native interaction,
manual visual and screen-reader results. A compile or a screenshot of an idle
window cannot pass an interaction check. Mark an unavailable journey BLOCKED or
NOT RUN, not PASS. Keep recordings and raw local engineering notes private unless
separately reviewed and explicitly shared. They may contain personal information.

The platform order is macOS Apple Silicon, Windows x64, then Linux. Existing
Linux smoke remains useful but does not substitute for Apple/Windows evidence.
Run destructive or browser-login tests only with disposable projects and test
accounts. No real user desktop, keychain, account or repository is a test fixture.

## Shared GUI and mode switching

| ID | Journey | Expected evidence |
| --- | --- | --- |
| UI-01 | Start with no saved interface preference | Synaric opens, no Synara login required, no agent auto-starts |
| UI-02 | Select Zen, quit normally, restart | Global preference restored without creating separate history or settings stores |
| UI-03 | Switch while an agent streams | Same task/session/model and exactly one active prompt, no duplicate event subscription |
| UI-04 | Switch with a pending permission or question | Same request survives, no implicit allow, keyboard focus reaches the response |
| UI-05 | Switch with an unsent multiline draft and active IME composition | No accidental send, text/caret/selection preserved, composition handled deliberately |
| UI-06 | Scroll into history, then switch while new output arrives | Same logical message anchor, no jump to bottom, explicit return-to-live works |
| UI-07 | Switch with unsaved editor changes and an active tool panel | No data loss or process restart, panels can be reopened with their state |
| UI-08 | Use only the keyboard in both modes | Sidebar, composer, model picker and consent actions are reachable and escapable |

## Shared model picker

The reference is the description and screenshots of
[upstream PR #1252](https://github.com/Emanuele-web04/synara/pull/1252), not its
legacy implementation. Both views share one picker/controller and preset store.

| ID | Journey | Expected evidence |
| --- | --- | --- |
| PICK-01 | Open from either GUI | Same current model, preset and negotiated traits, placement may differ |
| PICK-02 | Search a synthetic large catalog | Accurate filtered rows, bounded work, no per-keystroke full UI rebuild |
| PICK-03 | Use provider tabs and hidden/unavailable providers | Respect ordering and visibility, disabled entries explain why, no invented catalog |
| PICK-04 | Use modifier+1 through modifier+9 while open | Select only the matching visible result, do not also trigger sidebar/task shortcuts |
| PICK-05 | Enter in search, Tab and Shift+Tab | Top enabled result and tab traversal match the reference behavior |
| PICK-06 | Choose effort from the hover side panel, then without a pointer | Same supported effort options and commit behavior, keyboard equivalent exists |
| PICK-07 | Click a model row with an existing effort | Preserve only a compatible effort, visibly reconcile unsupported values |
| PICK-08 | Change supported footer traits repeatedly | Picker stays open, agent accepts or rejects each update explicitly |
| PICK-09 | Star one model with two different trait combinations | Distinct presets, full supported configuration restored, stable identities not display names |
| PICK-10 | Apply a preset after capability/catalog change | Unsupported or missing traits are not silently sent or falsely shown as accepted |
| PICK-11 | Switch layout with a preset selected | Same provider/model/effort/speed/thinking state, no second preset database |
| PICK-12 | Select a locked-session preset from another provider | Explain the real negotiated restriction rather than silently creating or replacing a session |
| PICK-13 | Force the second of several config requests to fail | Reconcile actual agent state, no false atomic-success message or hidden partial application |
| PICK-14 | Enable the effort slider preference | Supported steps, reset and speed control have accessible labels and keyboard behavior |

## Browser use and privacy

| ID | Journey | Expected evidence |
| --- | --- | --- |
| WEB-01 | Manually open a tab, navigate/back/forward/reload and close it | Real embedded browser surface, expected focus/history, clean helper teardown |
| WEB-02 | Agent requests input/read/capture/download/upload | Explicit task/tab/document/action-bound consent before dispatch, exact approved payload used |
| WEB-03 | Navigate or redirect after approval but before dispatch | Stale grant rejected, including same-origin document replacement |
| WEB-04 | Crash, close or reconnect the browser helper | Old grants cannot be replayed, unrelated tabs/tasks retain correct ownership |
| WEB-05 | Agent attempts to access a manual or authentication tab | Denied at the service boundary, not just hidden by the GUI |
| WEB-06 | Inspect diagnostic preview before opting in | No report is sent, no automatic network sender or raw browser/transcript payload |
| WEB-07 | Opt in with canary paths, prompts, URLs, tokens and screenshots in local state | Only reviewed non-personal fields eligible, private values absent from outgoing preview |

No standalone Rust browser policy test closes WEB-01 or proves engine sandboxing.
Actual native embedding, origin parsing, payload binding, cookies and helper IPC
must be exercised together before browser support is advertised.

## Accessibility and performance

Run UI-01 to UI-08 and PICK-01 to PICK-14 on Apple Silicon with VoiceOver, then
repeat with a supported Windows screen reader and relevant Linux accessibility
stack. Record keyboard order, announced names/roles/states, contrast, reduced
motion, display scaling and focus after closing menus. Do not infer accessibility
from visible labels alone.

Use reproducible synthetic workloads for cold/warm startup, composer input,
streaming transcript, concurrent tasks and large diffs. Record build flags,
hardware class, power state, cache state and workload parameters in a local test
record before comparing runs. Separate startup-to-first-frame from process-exit
duration. Idle CPU and resident-memory measurements need their own units and
collectors, not a millisecond timing report. Set budgets only after collecting a
baseline. No unmeasured speed or memory claim is accepted.

## Local sample aggregation

`scripts/ekop/support/aggregate_samples.py` complements the existing process
measurement harness. It validates timing samples already collected by native or
microbenchmark instrumentation and calculates median and nearest-rank p95. It
does not execute the app, collect samples, certify the supplied candidate SHA,
measure accessibility or send diagnostics. Its output is a local engineering
artifact, not a production telemetry payload.

The accepted input has exactly `case`, `build`, `synthetic`,
`operations_per_sample` and `samples_ms`. Cases and build labels are enumerated.
Samples must be 3-1000 finite nonnegative numbers no larger than one hour.
Duplicate JSON keys, unknown/private fields, oversized input and existing output
files are rejected. The output retains the operation count so a batch duration is
not confused with a single-operation latency. Cross-machine comparisons still
require the local hardware/build/workload record described above.

```sh
python3 -m unittest discover -s scripts/ekop/support -p 'test_aggregate_samples.py' -v
python3 scripts/ekop/support/aggregate_samples.py \
  --input /absolute/path/to/reviewed-samples.json \
  --candidate <exact-source-commit> \
  --output /absolute/path/to/new-local-report.json
```

The support CI validates this tool on the three target operating systems. Its
test timings are synthetic fixtures and are not Synara performance measurements.
