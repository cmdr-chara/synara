# Current-upstream native parity gates

This is a completion ledger for the continuing GPUI parity effort, not a release
claim or a percentage. Baseline: native `ef30c3db931dbf179d0a26657c891fb1ff89dbe9` and
upstream `eaa61eded31b6755d4f30ba8eabc5d905cf817cb` on 2026-09-23. The
[current feature-gap audit](../ui/electron-vs-gpui-feature-gap.md) supplies the
evidence and product scope behind each row. A gate remains **OPEN** until its
workflow, failure path, and relevant platform/provider acceptance have been
observed; a green compile or narrow unit test alone does not close it.

**OPEN does not mean no feature was delivered.** The
[roadmap's delivered-slice inventory](../../ROADMAP.md#delivered-feature-slices-in-the-september-23-sprint)
records 25 implemented, pushed slices across three commits. This ledger tracks
larger end-to-end parity outcomes, so all 21 gates can remain open while those
additions are available.

| Gate | Outcome needed to close the current-head gap | Deciding evidence | State |
| --- | --- | --- | --- |
| M1 Voice recording/transcription | Record, cancel and transcribe bounded microphone audio into an editable unsent draft with explicit permissions and stale-result fencing | Native capture/transcription journey, refusal/cancel/error tests, provider and platform acceptance | OPEN |
| M2 Headless/web workspace | Run the workspace without GPUI and use its authenticated web product remotely under a documented bind/TLS/update policy | Server/API/browser journeys, auth/Origin tests, readiness and shutdown, remote deployment acceptance | OPEN |
| D1 Onboarding | Complete provider enable/sign-in, project creation and replay through the native setup flow | Fresh-install and replay journeys with real provider/project state | OPEN |
| D2 Browser/WebMCP | Restore sessions/logins and popup auth, support agent file transfer and diagnostics, and expose page WebMCP under explicit authority | Authenticated browser journeys, privacy and partition tests, platform acceptance | OPEN |
| D3 iOS device | Present live simulator frames and scoped input, recording, app/URL operations and accessibility targeting | Hardware/simulator journeys and permission/lifecycle tests | OPEN |
| D4 Editor/diff | Match upstream syntax, autosave/conflict, comparison, blame and editing behavior | Multi-file editing and conflict journeys plus native interaction acceptance | OPEN |
| D5 Managed worktrees | Own per-task isolated worktrees and environment-aware fork/orchestration choices | Task/fork lifecycle, cleanup and conflict journeys | OPEN |
| D6 Provider/model/context | Support ACP model+effort presets, ordering/cycling, richer context controls and real provider/account telemetry | Representative provider journeys, persistence and transport evidence | OPEN |
| D7 Computer Use | Match upstream action, input, targeting and preview behavior across supported platforms | Permission, kill-switch, native interaction and platform journeys | OPEN |
| D8 Automations | Support upstream recurrence, effective no-retry policy and orchestration semantics over durable claims | Schedule/DST/restart tests and live run/failure journeys | OPEN |
| D9 Commands/keybindings | Support needed argument forms and context-aware keybindings without stealing provider commands | Command parser, key routing and native interaction journeys | OPEN |
| D10 Releases/updater | Verify trusted native artifacts and provide signed install/update/rollback | Feed provenance/signature, upgrade and rollback tests on release packages | OPEN |
| D11 Profile/analytics | Show provider/model mix, token heatmap and account/usage statistics from actual sources | Real telemetry fixture and account integration journeys | OPEN |
| D12 Handoff/forks | Continue or fork same-task/provider sessions with explicit environment choice | Provider and worktree continuation journeys | OPEN |
| D13 Attachments/media | Preserve folder references, broaden formats and view PDF/documents in app | Persistence, preview, export and failure journeys | OPEN |
| D14 Studio | Match current upstream long-running output lifecycle and organization | Output/task lifecycle and recovery journeys | OPEN |
| N1 Theme/density | Finish visual, accessibility and platform acceptance for native appearance controls | Screenshot, keyboard, screen-reader and platform matrix | OPEN |
| N2 File/source search | Match ranking, ignored-file and platform behavior of project search | Local/SSH fixtures plus native navigation/search acceptance | OPEN |
| N3 Thread export | Complete structured metadata and save-picker/platform behavior | Round-trip content/privacy and native save-picker journeys | OPEN |
| N4 Replies/context reuse | Match upstream selection-to-current/side/new-task interaction | Context and branch placement journeys | OPEN |
| N5 Multi-provider workspace | Prove representative upstream runtime breadth and provider-specific UX | ACP/direct-provider interoperability matrix | OPEN |

`M` denotes a baseline missing surface, `D` a material depth gap and `N` a
near-parity lane. Each row is a separately auditable outcome. When a slice lands,
record its exact verification in the relevant feature documentation and retain
the commit in Git history, then reassess the whole gate before changing
**OPEN** to **PASS**. New upstream behavior or contradictory evidence reopens a gate.

## September 23 implementation checkpoint

The native tree now has source surfaces for both former `M` lanes: voice capture,
transcription and draft insertion, and a local headless server with an
authenticated catalog preview. They remain OPEN because live device/provider
acceptance and the full remote web workspace are outstanding. This changes the
product lane count to **0 wholly missing, 16 material depth, 5 near parity**,
while all 21 completion gates remain OPEN.

`D8` gained five-field cron, DST-aware schedule claims and a configurable
1–3600-second execution limit. Upstream currently rejects fixed/exponential
retry policies at create/update despite contract shapes, so those policies are
not treated as an effective-current-head difference. `N2` gained fuzzy name
ranking, generated-directory filtering and typed file/directory results with
Explorer navigation; older SSH helpers remain file-only. `D11` gained a
274-day UTC heatmap of persisted local turn starts, distinct from provider or
account usage. `N3` gained durable message update times in ZIP export from the
same SQLite snapshot. Cross-platform GUI, microphone hardware and live ChatGPT
transcription journeys have not been exercised here.
`D9` gained exact `/synara/automation list` and `new` forms that open the
existing review or unsaved form without arming the scheduler.

The next native batch adds a token-protected, paged task-message API and a
read-only task browser to `M2`; one-level project-folder creation and existing
folder registration inside onboarding to `D1`; advertised-order ACP model
cycling buttons to `D6`; and still WebP import/preview with animated rejection
and a 2 MiB combined converted-media cap for PNG prompt delivery to `D13`. `D11` gains a labeled snapshot of
latest persisted session-model choices per local thread, grouped by agent;
that snapshot is not per-turn model use or provider billing. `D4` adds an
explicit two-step disk reload and a fresh-version compare-and-save overwrite
after an editor conflict, preserving later buffer edits. `D9` adds exact
`/synara/goal pause` with active-goal and pending-edit guards; resume remains
in the Goal panel. The web transcript excludes reasoning-role messages in
line with the native transcript.
All affected gates remain OPEN pending their listed full journeys.

Integrated Rust evidence: 105 app tests passed; 292 workspace tests passed with
one process-fixture test intentionally ignored; 81 runtime tests passed; and
15 server library tests plus one server binary test passed. `cargo fmt --all
--check`, `git diff --check` and strict Clippy across the four affected crates
passed. These checks do not substitute for live microphone, provider, remote
deployment or multi-platform acceptance.

Latest integrated batch evidence: 119 app tests, 296 workspace unit tests
(one process fixture ignored), three workspace settings integration tests,
16 server library tests and one server binary test passed. Six `ssh_live`
tests remain intentionally ignored without their isolated SSH fixture.
`cargo fmt --all --check`, `git diff --check`, strict Clippy on app/workspace/server,
and the embedded browser JavaScript syntax check passed. The new web API has
auth, paging, Unicode response-bound and reasoning-exclusion regression tests;
WebP has still/animated, persistence, preview, conversion and cap checks.
Native GUI, provider, remote deployment and platform acceptance remain open.

## September 23 feature batch checkpoint

This batch advances ten existing gates without closing them. `M2` now has
authenticated local workspace registration, agent-profile discovery, unsent task
creation and draft editing in the headless browser. It still cannot execute an
agent from the web UI or serve a remote deployment. `D2` can optionally restore
the URLs of manually owned browser tabs from a private local file; query strings
and fragments are stripped, and authenticated/agent tabs are excluded. URL paths
may still contain sensitive data, so this is an explicit opt-in. `D3` adds
user-triggered HTTP(S) URL opening and launch of an installed bundle in the
selected booted iOS Simulator, without a live stream or input controls.

`D5` and `D12` gain assistant-message forks into a selected existing linked Git
worktree. The task working directory is persisted, nested project directories
are checked for symlink escape, and assigned worktrees are protected from
explicit removal. Creation, task moves, release and cleanup remain open. `D6`
gains persisted ACP model and effort presets plus provider order, with each
preset checked against the connected session's currently advertised options.
`D7` gains bounded literal Unicode typing through the existing Computer Use
action. `D9` gains exact `/synara/automation edit <id>`, opening a saved
definition for review without arming it. `D10` rechecks staged update artifact
identity, size and digest before a prospective install handoff; signed feeds,
publisher identity, installation and rollback are still absent. `D14` can
reopen a selected Studio output in its source chat's Library while preserving
output and task attribution.

`D1` provider sign-in remains blocked by the task-scoped authentication API:
fresh onboarding has no selected task to authenticate. `D13` PDF viewing remains
open because the current intake rejects binary PDFs and no native renderer is
wired. All 21 gates remain **OPEN** until the deciding journeys above are met.
Focused implementation checks covered the changed worktree, command, browser,
Simulator, update, model and Studio paths; live macOS Simulator, X11 typing,
provider login, remote deployment and signed update acceptance were not run.
