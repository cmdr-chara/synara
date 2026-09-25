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
records the original 25 implemented, pushed slices and the separate September 24
continuation inventory. This ledger tracks
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
Explorer navigation; entry queries now also use the upstream prefix normalization
and POSIX path-ranking semantics, while older SSH helpers remain file-only.
`D11` now prefers a 274-day UTC heatmap of durable provider-reported turn tokens
when available and falls back to persisted local turn starts; account usage
remains separate. `N3` gained durable message update times in ZIP export from the
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

## September 24 batch 1: local execution, goal commands and Simulator apps

Three bounded additions bring the delivered-slice inventory to 28. **All 21 gates
remain OPEN**, with the same 16 material-depth and five near-parity areas.
The [batch receipt](parity-2026-09-24-batch1.md) records verification and limits.

- M2 gains explicit local ACP Run/Stop, status/recent-transcript refresh, duplicate
  admission rejection, bounded cancellation, draft-conflict protection and owned
  server shutdown. Credentials, permissions and interactive sign-in are not
  fabricated. Direct-model tasks and remote workspaces are rejected.
- D9 gains goal resume, clear and edit command forms. Resume replaces only the
  exact command with a reviewable draft and arms the existing bounded pursuit.
  It never sends the first prompt. Edit remains unsaved until Save, and refused
  actions retain the command. Clear requires a paused, loaded, unedited goal.
- D3 gains two-step installation of a user-selected local .app directory and
  explicit bundle termination. The helper is cancellation-owned and arguments
  are literal. Synthetic CLI tests do not constitute real Simulator acceptance.

Exact deferred blockers: Apple touch/keyboard input requires the upstream native
CoreSimulator helper and its macOS/private-framework integration, which this
Rust branch does not package. `simctl` URL/app operations are not a substitute.
The web product still lacks interactive sign-in/approval and remote TLS/deployment
acceptance. None of those blockers prevents the delivered local workflows.

## September 24 editor and Studio depth batch

Three further bounded slices bring the delivered inventory to **31** (25 earlier
plus six continuation additions). All **21 broad gates remain OPEN**, with
16 material-depth gaps and five near-parity lanes.

`D4` now has opt-in, per-buffer local auto-save after one second idle. It uses
the existing version-checked writer, preserves later keystrokes and undo, skips
IME/modal/other-save ownership, and stops auto-save on conflict or write failure
without overwriting either version. Closed/reopened buffers and application
restart do not retain auto-save permission. Remote buffers still require Save.

`D14` gains still-WebP previews with decoded-pixel and output caps and a single
shared preview worker permit. Images remain read-only and animated/damaged WebP
is refused. It also gains reporting-turn metadata and a turn filter, reconstructed
from durable tool-output replacement events rather than inferred from recent
chat activity. Reused tool IDs and status-only updates do not relabel old outputs.
Displayed bytes are explicitly the current file, not a historical turn snapshot.
Full long-running output/version organization and cross-platform acceptance
remain open. See `docs/verification/parity-2026-09-24-batch2.md` for checks.


## September 24 setup/worktree/control batch

The delivered inventory is **34 bounded slices**. All **21 gates remain OPEN**:
16 material-depth gaps and five near-parity lanes. The original task-less
onboarding blocker is addressed by an explicit, durable, unsent setup chat, not
by an extra provider backend or inferred account state.

D1 has explicit generic ACP Connect and advertised authentication with the
existing connection-question UI. Native setup can open the task terminal, but
provider-specific CLI login automation and live account acceptance remain open.
D5/D12 have reviewed new local worktree forks with generated branch names,
commit-pinned checkout, persisted cwd and existing assigned-worktree guards.
Failed task insertion retains the created worktree for explicit recovery, and
an abrupt crash can leave a discoverable Git worktree without a task. Automatic
cleanup/reconciliation, SSH creation and orchestration-wide lifecycle are not
implemented. D6/D9 gain composer-only model shortcuts and exact qualified
next/previous command forms, not direct-model cycling or arbitrary keybindings.

The [batch receipt](parity-2026-09-24-batch3.md) distinguishes native/fixture
validation from live providers and other-platform acceptance. PDF remains
blocked by binary intake and the absence of a packaged native document renderer.
Simulator input still needs its macOS helper and platform integration. Neither
is replaced by a misleading external-open or simctl-only parity claim.

## September 24 document/export/import checkpoint

Batch 4 advances D13/D14 with local Hub Library PDF snapshots, bounded native
page navigation/zoom/reload and reviewed original-file export. D1 gains inline
history discovery/preview/import instead of a detour to another Settings section,
while preserving its existing storage and consent owner. The delivered inventory
is 37 slices, distinct from the 21 broad gates, which all remain OPEN.

Linux Poppler packages are required. Platform save-picker acceptance, non-Linux
rendering, PDF prompt intake, richer PDF interaction, Studio historical contents
and live-provider fresh-install acceptance are not closed by this batch.
See the [batch verification receipt](parity-2026-09-24-batch4.md).

## September 24 committed-file history checkpoint

`D4` gains commit-pinned local exact-path history, read-only revision inspection
and explicit text copy, with task/root/tab ownership and retained editor buffers.
This is one additional delivered slice (38 total), not completion of syntax,
worktree blame or deeper comparison/editing parity. Worktree blame/diff currently
needs a filter-execution policy or filter-free implementation because Git's
`--no-textconv` does not prevent configured clean-filter execution. The new
committed-object path does not use that conversion boundary.
See [the receipt](parity-2026-09-24-batch5.md). All 21 gates remain OPEN.


## September 24 web interaction and live search checkpoint

Batch 6 adds two bounded slices, bringing the separate delivered inventory to
40. All 21 broad gates remain OPEN (16 material-depth and five near-parity).

M2 now presents Session-scoped permission choices and structured task questions
from the existing InteractionBroker. Only AllowOnce/DenyOnce are offered, never
persistent grants. Fresh application receipts, exact task/thread lookup, schema
validation, expiry, response-channel closure and turn cancellation fence replies.
There is no restart replay. Repeated polls retain the same field controls and
values. Connection/URL sign-in, rich command/diff context, durable question-draft
recovery, direct models and remote deployment remain open.

N2 gains a 350 ms query debounce, at most one local/SSH traversal and replacement
coalescing. Retired query, task, root or directory results cannot populate current
search. SSH setup errors release the owned request slot. Search controls retain
fixed space and the narrow pane expands while search is open, so result rows
remain inside their actual hit-test container. The clipping encountered by the
batch5 test is addressed as product code rather than treating its workaround as
acceptance. Exact ignored/generated-file behavior, native SSH and wider
platform acceptance remain open. See
[batch6 scope and evidence](parity-2026-09-24-batch6.md) and
[batch14 scope](parity-2026-09-25-batch14.md).
