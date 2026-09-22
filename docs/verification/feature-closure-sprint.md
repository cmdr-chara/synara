# Feature-closure sprint verification

## Candidate and scope

Repository: `cmdr-chara/synara`. Starting integration and session-branch base:
`fe515833c565d666204f24e5257d85025dede0e9`. Session:
`astra/feature-closure-sprint`. The six implementation commits culminate at
`96ec3439449bac5631c8d8b59c7afd5fd0bc14df`, tree
`249ac93a99a10ef90d4dd8f0fa6e73dc388234ff`.

Feature inventory: 9 Present / 24 Partial / 15 Missing ->
14 Present / 25 Partial / 9 Missing. Debug, goals, recap, PR Fix and inline comments
move Missing -> Present. Releases moves Missing -> Partial because the production
feed, signatures and verified update/install lifecycle remain unconfigured.
No earlier Partial capability is promoted. No release or PR is requested or created.

These are implementation claims backed by scoped fixtures, not authenticated
provider, hardware, accessibility or cross-platform release acceptance.

## Published feature commits and focused evidence

Each run prepares a hash-checked source patch, tests the resulting index and only
then publishes the identical Git tree. Its triggering transport SHA is not itself
the product source. The artifacts record `tested-tree.txt`, candidate source and
publication mapping. Tests use Rust 1.98.1 and Ubuntu 24.04. Native journeys use
private Xvfb/X11, actual GPUI controls and owned ACP/HTTP/GitHub fixtures.

| Feature | Published source commit | Passing Actions run | Focused test executions | Native assertion groups |
| --- | --- | ---: | ---: | ---: |
| Debug | `dcfcb1a93025fd7a799c50061ccc257cf5508f1b` | [35727674241](https://github.com/cmdr-chara/synara/actions/runs/35727674241) | 4 (`debug_`, including one existing Git diagnostic regression) | 5 |
| Recap | `4f45119cae50984a3dc2c3200f9f7102863b0cf9` | [35729256352](https://github.com/cmdr-chara/synara/actions/runs/35729256352) | 3 `recap_` + 7 `handoff_` | 4 |
| PR Fix | `4f617190aadf40c8fe937acaf0ea6f0fa5948a80` | [35731338373](https://github.com/cmdr-chara/synara/actions/runs/35731338373), attempt 2 | 4 `pr_fix_` + 9 `pull_requests::tests` | 4 |
| Inline comments | `0b4a9524bc2173255805f8398f58c8fa2fbd52d8` | [35735100062](https://github.com/cmdr-chara/synara/actions/runs/35735100062) | 3 `inline_` | 4 |
| Releases | `e17c4fad5e80fb1babd4d4476d39188f36f65048` | [35739790127](https://github.com/cmdr-chara/synara/actions/runs/35739790127) | 3 `releases_` + 4 `update::tests` | 3 |
| Goals | `96ec3439449bac5631c8d8b59c7afd5fd0bc14df` | [35748053330](https://github.com/cmdr-chara/synara/actions/runs/35748053330) | 4 `goals_` | 5 |

Totals above are 41 selected test executions and 25 native assertion groups, not
25 individual Python assertions. The focused runs are intermediate evidence, not
a substitute for the accumulated final campaign.

Debug proves evidence gating, five phases, explicit preparation/verification and
task/restart isolation. Recap proves reviewed unsent generation, cancellation,
explicit cache ownership, refresh and restart. PR Fix proves read-only collection,
stale-head refusal and exact destination preservation. Inline proves range/context
identity, multiple comments, persistence, changed/deleted/renamed rejection and
explicit append. Releases proves real compiled version, read/dismiss/reopen and
narrow-window persistence without an invented updater. Goals proves the two-turn
follow-up budget, draft priority, approval/question blocks, navigation disarming,
Stop, restart inertia and human-reviewed achievement/clear.

## Diagnosed failed attempts (retained)

- Debug run `35726331265`: native close control could not be revealed after four
  groups. Evidence height and the close-control journey were corrected before the
  passing candidate. Unit/build success was not described as complete native proof.
- PR Fix run `35731338373`, attempt 1: the private Xvfb server failed to start before
  the app/journey began. The 13 focused tests and build passed. The preserved error
  was `Xvfb did not provide a private display`. Attempt 2 passed the actual journey.
- Inline run `35733631071`: the competing transport candidate failed native build.
  The advanced branch was inspected and reconciled non-forcibly with the persisted
  task-owned implementation before run `35735100062`. No remote work was force-reset.
- Releases run `35736748375`: the clock function referred to the wrong crate.
  The workspace clock reference was fixed. Run `35738542235` then exposed a test
  resize call using the scenario rather than its owned desktop. The corrected
  native journey passed at run `35739790127`.
- Goals run `35741600111`: the ordinary Send path reused a navigation guard and
  paused the armed goal before sending. Send now retains the reviewed goal lease
  while preserving unsaved-editor guards. Actual navigation still disarms it.
- Goals run `35746391454`: the initial hello stayed unsent before a goal existed.
  Send readiness now includes asynchronous thread/draft/goal restoration, with
  opt-in boolean-only diagnostics. The native test waits for an enabled Send
  control instead of sending during startup. Run `35748053330` passed all groups.

## Final accumulated validation

The final campaign [35755789769](https://github.com/cmdr-chara/synara/actions/runs/35755789769)
uses transport `10dd2027d1d3f3066b59e63e243543c6466b4376`. Its fully checked product
source is published as `9637069b2e8cc7e210803b62daae8d017ccd04db`, tree
`fbea34609ee1f452b238a0419a136e52ecf59cc7`. No part of a failed native journey is counted as a complete pass.

| Check | Result |
| --- | --- |
| `cargo +1.98.1 test --locked --workspace --exclude synara-app` | 542 passed, 0 failed, 21 ignored, 0 filtered |
| `cargo +1.98.1 test --locked -p synara-app` | 79 passed, 0 failed, 0 ignored, 0 filtered |
| Native app and ACP fixture build | Passed on Ubuntu 24.04 / Rust 1.98.1 |
| Python structure/router/source/native-scope and roadmap regression tests | 60 passed |
| Original A-Q task bodies and checkbox states | 120 unchanged |
| Thirteen native journeys | 67 assertion groups passed, no failed journeys |

The 21 ignored backend tests are explicitly excluded from pass totals. They
comprise opt-in SSH/vendor acceptance and child-entry fixtures. This campaign is
not evidence that all ignored environments were exercised.

| Native journey | Passing assertion groups |
| --- | ---: |
| debug | 5 |
| goals | 5 |
| recap | 4 |
| pr_fix | 4 |
| inline_comments | 4 |
| releases | 3 |
| chat_behavior | 5 |
| model_draft | 7 |
| handoff | 6 |
| project_import | 6 |
| direct_models | 7 |
| integrations | 6 |
| browser_webview | 5 |

All journeys used real native controls in private Xvfb/X11 sessions, with owned
ACP, HTTP, filesystem and GitHub fixtures. Assertions cover negative paths,
independent draft/task identity, explicit sending, cancellation and restart
inertia. Images are observations of those fixtures, not live-provider proof.

### Accumulated failures and corrections

The first accumulated run `35750279926` passed 542 backend tests and all six new
feature journeys (25 groups), then exposed a pre-existing legacy chat test that
expected two preference fields instead of the existing three-field schema. Both
the schema and the stale expectation were already present at the starting ref.
The repair retains exact full-schema equality, including `show_recent_attachments`.

Run `35751883654` confirmed that repair, passed all 79 app tests and 60 Python
regression tests, and completed the shared journeys rather than stopping after
one failure. It found two distinct issues:

- A real integration regression: after selecting a Hub/chat, read-only goal and
  inline-comment restoration incorrectly blocked the following panel transition.
  Restoration is now distinguished from pending writes. Writes, validation and
  unsaved edits still block departure. Goal loads also carry a generation fence,
  preventing a late load from a previous visit to the same task from applying.
  Read-only loading no longer prevents orderly close. The final model/draft and
  goal/inline journeys exercise the corrected shared navigation and safeguards.
- A test-driver defect: the shared X11 fill helper omitted the underscore in the
  `project_import` fixture directory. It now dispatches Shift+minus for `_`, and
  the import test still requires exact input and source/receipt identity.

Run `35753088953` confirmed the import driver correction and passed the backend,
app and Python tests, but exposed the same read-only restoration issue in Recap.
Recap now distinguishes reads from pending reviews/writes while generation and
cache operations still wait for all loading to complete. The Debug journey also
clicked stale startup geometry while the layout changed. Debug opening is now
fenced until its thread/state are ready, and the journey waits for the enabled
control instead of relying on an old position. The two failed journeys remain
recorded, not relabeled as acceptance.

Run `35754525925` passed twelve of thirteen native journeys and all Rust/Python
checks. The remaining goal journey attempted Resume before post-turn readiness
had settled. The UI now derives the Resume enabled state from the same predicate
as the handler, preserving every safety check. The test waits for that state at
each resume and still requires the exact task-scoped draft and bounded dispatch
counts. No action is silently retried or accepted while its prerequisites fail.

The final full campaign above reruns all affected and shared native journeys,
backend tests and app tests after these corrections. Earlier partial successes
are retained as diagnostic history, not substituted for the final candidate.

### Cleanup identity and integration

The cleanup successor removes the 19 session-only transport files: 16 numbered
patch parts, the request, the transfer/publisher helper and its write-enabled
workflow. It updates this receipt only. The complete retained product, build,
fixture and test inputs are byte-identical to the checked tree above. No source
code, dependency, native build notes or test is changed after final validation.
Useful permanent tests and all historical commits remain. No new release or PR
is created and the session branch is retained.

The integration target is re-read before a non-force update. The completion
report records the final remote SHAs and verified containment. `main` must remain
`b58f27381e7ddd59678c9961500e8e43d3cc19ab`. This receipt alone is not a claim that a
future ref update succeeded: the subsequent GitHub ref/compare reads establish it.

### Upstream comparison boundary

The sprint started against Electron `f04341a67bc4941d1b2e91e0b23bbe782dfbc727`.
A later ref read found `03fd183c2430d1f6f4a95c9495467cfc2bb2425d`, three commits
ahead. Its file delta includes existing runtime/provider/device and build/release
work. That file-delta inspection is not a new exhaustive Electron audit. The
48-capability inventory remains pinned to the stated comparison snapshot rather
than silently claiming parity with every subsequent upstream change.

## Baseline and acceptance boundaries

The structural audit still rejects the pre-existing isolated Browser
`crates/synara-browser/src/native/actions.js: non-Rust core source`. Its exact
single-error baseline is preserved. Browser functionality and the checker are
not weakened to obtain a green result. The roadmap retains all original A-Q task
bodies and checkbox states: 17 lanes, 120 tasks, 16 checked.

Two pre-existing app warnings (`RevisionState::pending`, `ui::ROW_HEIGHT`) remain
outside this scope. The validated source aligns PR Fix type visibility and clears
only the obsolete recap navigation warning, not unrelated errors.

Real provider/GitHub interoperability, macOS/Windows/Wayland, accessibility/IME,
production scheduler/restart environments, signed packaging and verified updater
acceptance remain open. No credentials or historical binary data are fabricated.
The next missing products are rich transcript media, two-task split views, stacked
PRs, AppSnap, checkpoints/revert, native subagents, Agent Gateway, inbound MCP and
Computer Use. Relative cost depends on native platform and ownership constraints.
