# Maximum-feature sprint: recovered and validated implementation

Date: 2026-09-22. This receipt proves a bounded source/fixture candidate, not a
release, authenticated-provider acceptance or complete cross-platform readiness.

## Candidate identities

- Original integration and sprint base: `33fa0a42dd5ac1098a7ba5f1e7bd5b240abbf577`.
- Recovery continuation started at session `d1406f26f2f0790876cb5be0187ac5a46cd48ce7`.
- Protected main at recovery start: `b58f27381e7ddd59678c9961500e8e43d3cc19ab`.
- Initially published direct runtime: `f2ad061fe962be889ddd9e717fdcd0a6c30ab2b0`.
- Recovered ordinary import: `468c776b7e8341c1665f8ffc24b58ffa915ce51c`.
- Recovered ordinary direct-model corrections: `7928cff00f38a061f2f752e01042d574448021e9`.
- Native handoff implementation: `e9b730616fd9799cdc4f4fd28de559ee40a8a67b`.
- Accumulated tested ordinary source: **`16789f4b290268bd54f315855cccae479aab8b2f`**.
- Electron main was re-read and remained `f04341a67bc4941d1b2e91e0b23bbe782dfbc727`.

The cleanup/documentation successor preserves every executable input, manifest,
lockfile and test from the accumulated candidate. It removes the sprint-only
write-capable workflow/helper and retains a read-only reusable/manual acceptance
workflow. Final branch publication/integration uses checked non-force ref updates,
not a workflow that can merge future runs. Main, PRs and releases are outside scope.

## Accumulated checks actually run

[Run 35717821611](https://github.com/cmdr-chara/synara/actions/runs/35717821611)
validated the SHA256-pinned source delta on staging commit `1f6e397b6d100a372df597d8f2dec66c4a46e713`,
then published the identical ordinary source as `16789f4b290268bd54f315855cccae479aab8b2f`.
All 13 source/test paths were rehashed locally and matched that published commit.
The runner was Ubuntu 24.04, Rust 1.98.1, with actual GPUI/WebKitGTK windows on private Xvfb.

| Check | Observed result |
| --- | --- |
| `cargo +1.98.1 test --locked --workspace --exclude synara-app` | **522 passed, 0 failed, 21 ignored**, no filtered tests |
| Native application and ACP fixture build | Passed with two existing unused-code warnings |
| `rustfmt +1.98.1 --check --edition 2024 --config skip_children=true` on new Google/schema Rust modules | Passed |
| Google native setup/discovery/stream/schema/Stop/restart | **5 checks passed** |
| Existing direct runtime native review/stream/usage/Stop/restart/ACP return | **7 checks passed** |
| Project Import native stale-source/atomic import/duplicates/source/restart/Claude chain | **6 checks passed** |
| Provider continuation native review/edited draft/ACP/direct/source/restart | **6 checks passed** |
| Corrected optional-Hub/model-draft regression, including independent restart drafts | **7 checks passed** |
| Existing real-WebKit Browser/native overlay/task scope regression | **5 checks passed** |
| Existing Plugins/Skills/MCP ownership, credential refusal and restart regression | **6 checks passed** |
| `git diff --check` | Passed |

Native total: **42 assertion groups across seven executed journeys**, not 42
independent OS/provider acceptance gates. The 28 model tests include the new third
family, key-header isolation, path injection rejection, bounded pagination,
unknown-capability refusal, schema validation and mismatch-without-success. The
workspace suite includes seven handoff tests, import rollback/retry/backup tests,
active model/session controls and direct cancellation/shutdown ownership tests.

The 21 ignored cases comprise isolated-SSH/vendor opt-in checks and child-fixture
entry points, some exercised through parent tests. They are not counted as passing.
The native build still warns about existing `RevisionState::pending` and
`ui::ROW_HEIGHT`. No unrelated baseline formatting was changed to suppress warnings
or make a global style gate green. The new native journey checks are not a global
accessibility, IME, Glass compositor or supported-OS matrix.

Artifact **10690128976**, `max-feature-evidence`, SHA256:
`0feed5420b1c7f9ec4eafcf2930f8fbf5ad74efdb7f0f6b467b07adb02c39f27`.
It contains the exact revision, clean status, source bundle, build/test log,
per-journey result files and 26 native screenshots. The archive hash was verified
before inspection. Representative Google schema review/mismatch, continuation
review/result, import, Browser and integrations layouts were visually inspected.

## Documentation and legacy structural checks

The roadmap validator and its nine tests pass. All 120 original A-Q task bodies
and checkbox states are preserved, with only current D/F/I summaries and the
execution queue updated. Changed Markdown local links were checked. The structural
audit's 12 unit tests, native-scope routing's 22 tests, CI routing's 11 tests and
source-publisher's six negative/positive tests pass. These are 60 Python unit tests,
separate from Rust/native results.

The legacy workspace structural audit itself still rejects
`crates/synara-browser/src/native/actions.js` as non-Rust core source. The isolated
Browser helper is an unchanged Session 2 file with the same blob at the pre-sprint
baseline. There are no other structural errors in the recovered tree. This
pre-existing checker mismatch was neither hidden nor fixed by deleting the Browser
implementation or weakening its checker. It is not a failure of the new direct
runtime or continuation path. No global formatting acceptance is claimed.

## Recovery evidence and diagnosed failures

The previous acknowledged `d1406f...` was not an accepted final source head.
Run **35678424404** failed on the optional-Hub journey's missing save-control probe.
At recovery start, Project Import/corrections still existed in a checked transfer
package rather than ordinary source. Nothing was discarded or assumed integrated.

[Run 35712587706](https://github.com/cmdr-chara/synara/actions/runs/35712587706)
added the missing geometry probes to existing real Hub controls, retained explicit
Hub creation and draft-isolation assertions, and passed import/direct/Hub journeys.
It published the ordinary import and correction commits listed above. It did not
restore the obsolete behavior of automatically selecting/creating Studio on entry.

The prior uncommitted handoff draft was not present in the supplied artifacts.
The replacement uses existing authoritative related-conversation storage and task
reservations, not the nonexistent `WorkspaceService::save_task` call from that draft.
[Run 35714973606](https://github.com/cmdr-chara/synara/actions/runs/35714973606)
passed seven backend handoff tests and built the app, then exposed a real native
input defect: the modal stopped ordinary key events before GPUI's character-input
fallback. The query stayed empty. That failure was not retried unchanged or hidden.

The correction stops only the appropriate Escape event, respects composition,
and activates the real header continuation action.
[Run 35716036176](https://github.com/cmdr-chara/synara/actions/runs/35716036176)
then passed all six handoff and seven Hub/model-draft native groups and published
`e9b7306...`. Those journeys passed again against the accumulated Google/schema
candidate. The earlier runs/artifacts remain historical evidence.

## Boundary review

Direct inference remains outside ACP. The Controller reuses task reservations,
configuration gates and prompt-owned event consumption. Restored bindings, imported
text and continuation drafts never start requests. No retry follows an ambiguous
external result. Native errors redact credentials. Keys are scoped to canonical
endpoint plus protocol and are never copied into task metadata or history.

Import reads capability-scoped regular files, rejects stale source and changed
destination reviews, and commits transcript plus duplicate receipt atomically.
Handoff revalidates source sequence/authority/target metadata, preserves the original
session and creates an unsent child without cloned permissions, secrets or Git
state. Google rejects path injection, unsupported function/signature round trips,
non-text output and incomplete stops. Structured-output validation gates the final
success event and never executes code, resolves references or fetches schema URLs.

The all-backend run and existing Browser/integrations native journeys retain
Sessions 1-4 owners. No feature copied upstream code, assets, layout or theme values.
Source-module and callback changes remain scoped to the requested capabilities.

## Remaining product work and acceptance

P0 is real but partial: three shared families and metadata normalization do not
prove roughly 75+ provider interoperability. OAuth/cloud signing, additional
protocol requirements, native multimodal conversations, approved tool execution
and broader structured-output parity remain implementation work. Google effort and
signature-dependent tool history are explicitly unsupported. Handoff remains a
reviewed related conversation, not in-place session migration.

Live paid-provider accounts, real OS credential-store interaction and macOS/Windows
native acceptance were unavailable. Tests used owned loopback HTTP and an ACP
fixture, not real authenticated provider services. Physical devices, production
websites, scheduler operational acceptance and broad release/accessibility gates
remain open. No unavailable credential/hardware gate was labeled complete.

Computer Use, native subagents, Agent Gateway, external MCP clients connecting to
Synara, goals/checkpoints/Debug and the remaining review/composer capabilities are
still roadmap work. This session added neither a placeholder owner nor a misleading
UI control for them.
