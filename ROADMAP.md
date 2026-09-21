# Synara delivery roadmap

This is the implementation and verification backlog for the native Rust/GPUI
Synara application. It covers the whole agreed product, not just the next demo.
It is a living engineering document, not a release announcement.

## Current checkpoint

### September 21: Zen, personal appearance and active tasks

Zen is an optional presentation for Synara and Studio, not a third chat scope.
It provides a compact header and reversible navigation/Environment disclosure
while retaining the existing composer, draft, editor, process and approval owners.
Appearance adds six light/dark colorways, custom accents, native transparency/blur
requests, a glass-style composer, local wallpapers with fit/dimming/reload controls,
density, text sizes, chat width, motion choices and appearance-profile import/export.
A shared Active tasks dialog exposes running chats and live pending decisions.

[Design, implementation and limitations](docs/ui/zen-personalization.md) records
source work separately from native acceptance. Local structural/whitespace checks
passed. Native compilation/screenshots are unavailable in this sandbox. No GitHub
tests are dispatched. I7/I10 and D5/D6/D10 remain open for their complete acceptance.
Upstream reviewed: `e7cd15281e6d16cf8fc55a91496dcff035475e54`, unchanged.

### September 21: native multi-terminal workspace

The existing Environment Terminal tab now contains project/root-scoped terminal
tabs, independent process and viewport ownership, named tabs, guarded Stop/Restart/
Close actions and a two-pane workspace. Side-by-side and stacked presentation,
narrow-panel adaptation and explicit split-size controls are implemented together
with visible-row search, copy, live-output navigation and terminal-to-unsent-draft
context. Tab metadata is revision-checked and persisted without storing or replaying
processes, commands, output or credentials. App close accounts for all owned shells,
including background and pending-start terminals. Local and pinned-SSH launches
retain their existing backend boundaries.

Source implementation is distinct from native acceptance. Broad/native testing is
deferred under the user's feature-first instruction. A8/G7/I10 remain unchecked.
See [terminal delivery and limits](docs/ui/native-terminal-workspaces.md).
Upstream `e7cd15281e6d16cf8fc55a91496dcff035475e54` was rechecked with no new
revision relative to the preceding batch. Its delegated-delivery and macOS icon
fixes remain deferred rather than silently claimed adopted.


### September 20: native repository operations

Changes now exposes a Repository panel with branch creation/switch/rename and
merged-only deletion, named remote add/edit/remove and explicit fetch/fast-forward
pull/push, worktree list/create/remove, and stash save/apply-with-retention. All
commands use the existing bounded local/SSH GitOperations API. Forms remain tied
to their originating project/root across navigation. Unfinished forms and active
operations block app close. Repository execution, configured credentials and SSH
network transport require explicit per-operation consent. No force operation,
hard reset, stash pop/clear or credential storage is introduced.

The batch preserves the concurrent multi-editor and command-palette work in
`48bf3f9`. The existing Markdown renderer receives its pinned formatter correction.
The UI uses the established Environment split, semantic palette and Synara icons,
with wrapping actions, search and narrow-panel composition. H2/H3/H4/H5/G7/I10
remain partial until native, remote and platform integration is verified.
[Source delivery and remaining verification](docs/ui/workspace-productivity.md).

Upstream main was reviewed at `e7cd15281e6d16cf8fc55a91496dcff035475e54`.
Its new passive delegated-result delivery, human-send reservation and replay
fingerprints add an F11/D12 gap. The macOS icon-after-quit fix is recorded for the
native packaging audit. Neither is claimed adopted by this repository batch.

### September 20: native Explorer actions and cross-surface context

The native Explorer now exposes new-file/new-folder creation, version-checked
rename and explicit permanent file deletion, literal content search, relative-path
copying, file references and selected-text quotation into the unsent chat draft.
All file mutation stays in the existing contained local/SSH workspace services.
Dirty files, open mutation dialogs and external-version conflicts are guarded.
The [Studio and Explorer receipt](docs/ui/parity-studio-files.md) records the
combined source checks and remaining native acceptance. G1/G2/G7/G8/D8/I10 stay
open for their full scope. Multiple editor tabs, recursive directory deletion,
inline search highlighting and full Git review are not claimed by this batch.


### September 20: recovered notes and native Studio file browser

Saved notes/checklists are now wired directly into the checked-in application,
not assembled by CI. The old source-rewriting step is removed while the Jev
orchestrator and conservative routing remain unchanged. Studio adds a contained
file browser, filename/image/output filters, UTF-8/Markdown and bounded PNG/JPEG
previews, copy actions, explicit draft references and opening text in the existing
editor. Markdown can switch to raw source, and image previews have bounded zoom,
100% and Fit controls. Completed structured tool diffs provide output attribution. Other files
remain honestly labelled workspace files. No content is executed or sent merely
by browsing. See [continuation evidence and limits](docs/ui/parity-studio-files.md).
G1/G2/F9/D10 are partial until the corresponding native and platform evidence is
recorded. Browser, multimedia intake, side chats and full output attribution remain open.

### September 20: saved chat notes and checklists

Each conversation now has user-owned notes and an ordered checklist with add,
edit, complete, hide-completed, reorder and remove controls. Explicit Save uses a
revision check to reject stale edits. Copy and Add to draft preserve current chat
text and never send automatically. Close/reload guards protect unfinished edits,
and permanent task deletion removes its context in the same transaction.
D10/F2/I10 remain partial. The [organization/context receipt](docs/ui/parity-organization-context.md)
records the focused combined checks separately from remaining visual/platform work.

### September 20: Spaces and organization batch

Space tabs, create/rename/icon editing, ordering, project assignment and guarded
Space deletion are implemented as a new candidate. Deleting a Space returns its
projects to Void without deleting files or chats. Project and thread pins are
persisted and ordered ahead of unpinned entries. Switching Spaces filters the
sidebar without stopping tools or changing the active conversation. Keyboard
switching/reordering and activity indicators use the existing native controls.
The [organization receipt](docs/ui/parity-organization-context.md) separates source
implementation from targeted and native acceptance. No F7/F8/I10 gate is closed
until its evidence exists. Original theme defaults remain unchanged.

### September 20: batched native chat utilities

The coherent chat batch adds literal transcript search with work-log navigation,
durable message pins, explicit text conversation copy/export, non-destructive
prompt reuse/quotation, advertised slash-command insertion and cross-scope thread
finding. All actions preserve existing drafts, event history and agent authority.
The [chat utility receipt](docs/ui/parity-chat-tools.md) distinguishes text-only
export and draft reuse from still-missing multimodal sharing, rollback and resend.
Regression tests are prepared but deferred until the batch is assembled, as requested.
Compilation is tracked separately from native interaction acceptance in the receipt.
D8/D9/D12/F2/F8/I10 remain open for their full scope. Theme defaults are unchanged.

### September 20: Environment workspace interaction

The native Environment now has real Terminal/Explorer/Changes tabs, a shared Add
menu, pointer/keyboard split resizing, maximize/restore, and versioned saved
layout. General exposes Open by default and Reset layout. Explicit open/hide
updates the preference, while restored tools do not launch shells or agents.
Theme defaults are unchanged. Dracula remains optional.
Individual tool-tab closing and keyboard reordering preserve tool identity and
persist across restart. Dirty files and running shells require explicit resolution
before their tab closes. Closing the final tab leaves the launcher available.
The Environment receipt separates exact-candidate verification from the retained
harness failures. No theme, dependency or approval-policy defaults were changed.

The [Environment receipt](docs/ui/parity-environment.md) records the exact check
states, native captures and remaining limits. Storage/state regressions prove
bounded preferences, ordered saving and preservation of malformed/future data.
This is bounded progress in G7/I6/I10/F2, not completion of those gates. Per-thread
layouts, multiple terminals, Browser and Side chats remain separate work.

### September 20: original UI inspection and native Kanban

The recovered 73-image Electron reference set is now indexed and reviewed, with
full-resolution inspection of the new-task dialog and populated board. Earlier
screenshot-access failures below are historical. See
[original UI analysis and Kanban evidence](docs/ui/parity-kanban.md) and the
[dated image register](docs/ui/reference-9488759-index.json).

This candidate adds the original two-level project overview/board structure,
shared toolbar, isolated task composer, project/provider selection, atomic
creation with an unsent draft and explicit controller-backed Run/Stop actions.
Pending launch cancellation and unfinished-form close/discard protection preserve
user text. The previously failing dialog now owns one primary-button hover style
and retains explicit modal/confirmation focus cycles and trigger-relative pickers.
The corrected source `770eae1c4e134b8dd285ecd46b9563fb79736199` passes
[focused native run 35505674865](https://github.com/cmdr-chara/synara/actions/runs/35505674865)
(presentation job `106064975562`): changed-package formatting, targeted
Rust regressions, strict changed-package Clippy, application/fixture build,
Kanban creation/lifecycle, Studio restoration, guarded-close journeys and the
no-source-mutation check. Static run `35505674726` also passes.
See the linked receipt for the retained failed runs and final artifact. No broad
suite was rerun to refresh this record. F1/F2/F10/D4/I10 remain open for their
complete scope, including drag/drop, preconnection model/effort, follow-up drafts
and the full cross-platform visual matrix.

### Earlier September 20 provider and draft checkpoint

Provider favorites, durable drafts and editable Chat behavior (September 20, 2026):
provider-icon/Starred tabs, explicitly acknowledged model selection, persisted model
favorites, per-chat draft restoration, ordered close-time saving and Enter/timestamp
preferences are implemented in the checked source delta. Targeted storage/input/UI
tests, strict changed-package Clippy and native fixture journeys provide bounded
acceptance. See [the exact candidate and evidence](docs/ui/parity-model-drafts.md).
The first combined run had a legacy input-targeting failure, retained in the
record; the focused follow-up validates the correction without weakening assertions.
D4/D8/F2/I6/I10 remain open for their full scope. The 73-image reference archive
still could not be opened after renewed transport/file-access attempts, so this is
not a complete visual-parity or cross-platform acceptance claim.


### September 20: searchable choices and rich native chat

Source checkpoint: `917f7a0a96ff41c6a026ff0ed9a749f2556cdda8`, continuing
`386e1b2a309b947bec1ddd9fe0e70e4f80a6a2a7` on `astra/gpui-clean-rewrite`.
The product reference for this continuation is Electron Synara
`948875954f432978eab7dd5fa44c3028b8d99a81`, not the older audit revision.

D1/D8 now include searchable native model/agent choices, stable action identity
after filtering, safe empty results, release-only selection and draft-preserving
dismissal. Native chat adds aligned Markdown tables, read-only task checkboxes,
fenced-code language labels and exact code copying using Synara's bundled icons.
The existing controller, durable events, compact Add menu and reduced-motion
boundary remain authoritative. Search does not restart the menu's entrance.

The [checkpoint and verification receipt](docs/ui/parity-9488759.md) records the
exact candidates, focused Rust and native X11 results, failure corrections and
remaining limitations. The focused lane covers 20 Rust tests, seven test-selector
regressions and 24 native interaction checks, including actual ACP model changes,
code clipboard contents, narrow layouts and restart. Unknown/backend/dependency
changes still select full native verification; documentation-only changes do not
rebuild the application.

This is a bounded D1/D8 continuation, not completion of either gate. Provider
tabs/starred presets, the remaining composer controls, browser, Studio outputs,
approval-policy parity and broader settings workflows remain open. The supplied
73-screen archive still needs full native visual comparison: local image and
execution tools failed in this continuation, so CI interaction evidence is not
claimed as pixel-level acceptance. Existing chat creation, Studio scope and basic
settings were already implemented before this slice. I10 and final delivery
remain open. The earlier checkpoints below are retained as historical evidence.

### Earlier integrated checkpoints

Conversation/composer continuation from `1548e3ade5a0ea292b560727af579ae0a5a16ca0`:
compact native session pickers, explicit backend-acknowledged configuration,
capped composer and message hierarchy are implemented as a new verification
candidate. The existing native input, durable transcript and backend ownership
remain intact. Native interaction and visual acceptance must be recorded on this
candidate before treating it as delivered product parity. See
[conversation/session controls](docs/ui/session-controls.md). This does not close
the full D/I gates or supersede the complete parity gaps below.


The four isolated implementation sessions have now been consolidated into the
delivery branch. Current integrated candidate:
`084a31db3df39440caf1523290ea721fdcbac7ca`.

Integrated session heads:

- A/J/M: `7d22346c6664700859c14550fc861de5b9b3660f`
- B/C/D: `f1e021905c59f081ba4aeee1f7dcb152d5f94aaa`
- E/K/O/P: `fc4e27d41f96f65258ab7471b6871e9830ce057f`
- F/G/H: `c835ee23d6fa43577bb3c7c50419cca5af2a301a`

The integration commits are `0750080f81a5684f6fe22afd91082cbea98b9777`,
`7a88d1709492af8e87bee85205c3854f8cdc227a` and
`3aac06a75020df78c74492232d071e0e79699d21`, followed by formatting correction
`084a31db3df39440caf1523290ea721fdcbac7ca`. All four session heads are ancestors
of the integrated candidate.

Candidate-specific verification on September 18, 2026:

- [Roadmap and formatting run 35341346630](https://github.com/cmdr-chara/synara/actions/runs/35341346630): PASS.
- [SSH transport verification run 35341346636](https://github.com/cmdr-chara/synara/actions/runs/35341346636): PASS.
- [Native verification run 35341346711](https://github.com/cmdr-chara/synara/actions/runs/35341346711): PASS, including formatting, workspace check, strict Clippy, workspace tests, native application/ACP fixture build and isolated GPUI desktop interaction smoke.

The parentless rewrite root remains
`43b1fb89bf19dadc388d18008f9ceb21b8215716`. Protected refs remained unchanged
during integration: `main` at `657389cc86f345bcb7b11c843670fcdee326e91d`
and `archive/pre-rewrite-main-2026-09-17` at
`29b826b8d8e73cc270e311a4c0031b629316b2ec`. No PR was created.

Important integrated progress since the earlier baseline:

- the recovered native terminal surface, guarded paste, scrollback, selection/IME
  handling and Linux PTY/process cleanup are now published;
- pinned SSH workspaces now include remote files/saves, remote PTY, Git,
  reconnect handling and native Remote-panel coverage;
- ACP connection/session ownership, scoped permissions/questions, bounded input
  validation, custom command profiles and virtual transcript state are integrated;
- registry recovery/integrity work, browser-host policy foundations, measurement
  tooling and cross-platform compile groundwork are integrated;
- SQLite migration/concurrent-open hardening, deterministic task ordering and
  literal Git unstage behavior are integrated.

Session completion does not mean every roadmap acceptance gate is closed. Real
authenticated vendor journeys, full browser embedding, Apple device tooling,
settings/secrets/platform UX, security/inspector completion, native macOS/Windows
interaction acceptance and final delivery remain open where listed below.

The GPUI application is functional and interaction-tested. The September 19
presentation pass brought its main shell, icons, chat layout, Studio switcher and
basic settings closer to Synara, but functional parity remains open. The
[Electron-to-GPUI feature audit](docs/ui/electron-vs-gpui-feature-gap.md) compares
73 user-visible features against Electron revision `73cd1811a81e4a62b6199a722fd6fac379abf9ba`
and native revision `c8e06e7d57c01b96744989746146fcfd507a3aa0`. It is a
source audit, not runtime acceptance. Continue the native GPUI migration on
`astra/gpui-clean-rewrite`, preserving the Rust backend and independently
implementing Synara's product flows. Zeron remains an architecture/design-quality
reference only; do not copy its source, assets, tokens or compositions.

## How to read and maintain this roadmap

- **Integrated** means code is present in the published branch. It does not imply
  all edge cases, real agents or operating systems have been tested.
- **Partial** means a foundation or a limited user path exists, but the gate below
  remains open.
- **Pending recovery** means work was reported in a local working tree but is not
  in the audited remote checkpoint. Do not count it as delivered or overwrite it.
- **Open** means implementation or sufficient evidence remains.
- Check a task only after its acceptance evidence exists. Record the commit,
  command, platform and result. Reopen it when later changes invalidate evidence.
- A fixture agent is not a vendor agent. Compilation is not an interaction test.
  One working operating system is not cross-platform acceptance.
- Do not derive a completion percentage from checkbox totals. The tasks have
  different costs and risks. Prior informal estimates are not measured progress.

Update this file in each meaningful implementation checkpoint. Keep failed gates,
limitations and untested platforms visible. Complete a coherent unit, validate,
commit, push and continue. A blocked independent subsystem does not block all
other work. Near an execution limit, preserve the current unit and the exact next
step instead of leaving an undocumented working tree.

## Audited baseline

Baseline source: `1994d0346ff389b9c016690d7f77a204f7de3656`.
The table below records the earlier baseline and is retained for comparison.

| Area | Existing foundation | Remaining qualification |
| --- | --- | --- |
| Native desktop | GPUI shell, tabs, composer and platform text input | Accessibility, sustained-use testing and other platforms remain open |
| Agent host | Generic ACP transport, negotiation, sessions and connection ownership | Authenticated vendor interoperability and complete optional-capability coverage remain open |
| Conversation | Normalized events, streaming, tools, permission and question surfaces | Product-level transcript/composer polish and broader long-session evidence remain open |
| Workspaces | SQLite catalog, tasks, durable event delivery and startup restoration | Broader multi-project workflows, backup/recovery and product UX remain open |
| Files and Git | Bounded UTF-8 editing, guarded saves, close confirmation, status/diff/staging/commit | Richer editor and Git operations remain open |
| Registry | Catalog UI, explicit review, pinned package launchers and checked binary installation | Full install/update lifecycle and real-distribution matrix remain open |
| Terminal/runtime | PTY shell, bounded history and process/execution-host services | Cross-platform PTY/process acceptance remains open |
| Verification | Linux build, lints, fixtures and native Xvfb interaction smoke | Native macOS/Windows interaction, Wayland and authenticated vendor acceptance remain open |

Baseline CI:
[Native verification run 35237792208](https://github.com/cmdr-chara/synara/actions/runs/35237792208).

### Integrated delta after the parallel sessions

| Area | Integrated progress | Still open |
| --- | --- | --- |
| Terminal/runtime | Direct GPUI terminal grid, xterm key/mode handling, Unicode/wide cells, selection/copy, IME/preedit, reviewed paste, bounded scrollback, resize and Linux foreground-job cleanup | Windows ConPTY/Job Objects, macOS PTY lifecycle and broader platform acceptance |
| Remote/SSH | Pinned trust, enrollment diagnostics, remote files/saves, PTY, Git, reconnect identity and native remote-workspace smoke | Explicit forwarding/development-server flow and stronger remote descendant-cleanup guarantees |
| Agent/conversation | Connection/session ownership, authentication-required state, stale interaction expiry, scoped permissions/questions, input validation, custom command profiles and virtual transcript state | Authenticated real-agent end-to-end journeys and remaining B/D acceptance matrix |
| Registry/browser/performance | Registry recovery and immutable review hardening, browser consent/lifecycle policy foundation, measurement/aggregation harnesses and GUI acceptance specification | Actual embedded browser, application performance baselines, accessibility runs and updater/distribution work |
| Storage/Git | Atomic migration hardening, concurrent opener coverage, deterministic task ordering and literal unstage behavior including pre-first-commit | Full workspace/editor/Git product workflow matrix |
| Integration | All four session heads consolidated under the independent root with final Linux native and SSH verification green | Product UI overhaul and remaining roadmap lanes below |

Historical session handoff documents remain useful evidence for individual
checkpoints, but statements that terminal or remote work are merely "pending
recovery" are superseded by the integrated candidate above.

## Delivery order

| Milestone | User-visible outcome | Depends on | Completion gate |
| --- | --- | --- | --- |
| M1: reliable local agent loop | Open workspace, choose agent, work in a durable task, use terminal, recover safely | Existing baseline, A, B, C, D | Real agent proof plus local interaction/recovery acceptance |
| M2: complete local workspace | Everyday files/editor/diff/Git, agent management and settings | M1, E, F, G, H, I | Complete local journey and failure-path matrix |
| M3: remote workspace | Run an agent and development tools over verified SSH | B, C, F, J | Controlled SSH integration plus remote UI smoke |
| M4: browser and device tools | Browser-assisted workflows and supported device/simulator integration | K, L, M | Isolation, lifecycle and platform-specific proof |
| M5: production readiness | Installable, recoverable, secure and responsive supported-platform builds | All applicable lanes, N, O, P, Q | Per-platform evidence and owner decisions, not an automatic release |

These milestones are dependency groupings, not stopping points. Security,
performance measurement and platform checks run alongside implementation.

### Electron feature-parity gate

The [feature-gap audit](docs/ui/electron-vs-gpui-feature-gap.md) is the dated
inventory for this migration. Its “Present/partial” entries are not completed
gates. Every missing or partial row must be implemented, explicitly excluded by
an owner decision, or documented as a deliberate capability-driven difference.
Before closing a row, record a native interaction check against the corresponding
Electron flow, including restart/recovery, keyboard operation and supported
platforms where relevant. Do not close a whole route because its sidebar label
or placeholder screen exists.

| Electron surface | Roadmap tasks owning remaining parity |
| --- | --- |
| Sidebar, projects, Spaces, import and search | F1, F7, F8 |
| Transcript, composer, attachments, voice, handoff and Environment | D1–D4, D8–D12 |
| Plugin/skill discovery and provider setup | B6, E6–E8, I6 |
| Studio and Kanban | F9, F10 |
| Pull requests and Git | H2–H6 |
| Automations and scheduler | F11 |
| Files, diff, dock, side chats and terminal tabs | A8, G1–G8 |
| Settings, desktop capture and notifications | I1–I10 |
| Embedded browser and device | K1–K6, L1–L5 |
| Packaging, updater, cross-platform and accessibility | O1–O4, P1–P6 |

## A. Recover and complete the native terminal

Status: **Integrated and verified on Linux; cross-platform acceptance remains open**.
Ownership: `synara-runtime`, `synara-app`.

The previously unpublished terminal work was recovered and is now part of the
delivery branch through A/J/M head
`7d22346c6664700859c14550fc861de5b9b3660f`. The integrated implementation
includes the direct native GPUI terminal surface, cell/style rendering,
Unicode/wide-cell handling, cursor and terminal metadata, application-cursor and
bracketed-paste modes, selection/copy, GPUI text-input/IME handling, guarded
clipboard paste review, user-owned scrollback, resize and retained final output.

Linux PTY regressions cover direct control input, resize ordering, final-output
retention, bounded input, repeated start/stop, partial-start cleanup and foreground
jobs in separate process groups. The final integrated native verification and SSH
runs are recorded in the current checkpoint.

- [x] A1 Recover the previous local diff without resetting, cleaning or replacing
  uncommitted work. Compare every file with the published baseline. Preserve a
  recoverable checkpoint before resolving overlaps.
- [x] A2 Integrate direct keyboard input, native cell rendering, ANSI colors,
  Unicode/wide characters, cursor, title and working-directory updates.
- [x] A3 Exercise interactive shells, control keys, application cursor modes,
  alternate screen, terminal replies, resize, focus, selection/copy and IME.
- [x] A4 Complete bounded scrollback and user-owned scroll position. Retain final
  output after process exit without retaining dead PTY resources.
- [x] A5 Require deliberate review for dangerous pasted control sequences or
  multiline commands. Support bracketed paste without silent execution.
- [x] A6 Prove interrupt, stop, close and restart clean up the shell and foreground
  job/process group. Cover exit-before-stop, concurrent stop and reader teardown.
- [x] A7 Run full desktop regression and focused terminal tests, then publish the
  coherent terminal checkpoint. Extend CI smoke to the accepted behavior.
- [ ] A8 Add multiple terminal tabs and split/resize behavior, search and session
  management. Prove independent PTY ownership and cleanup for each tab.

Acceptance for the Linux implementation is met by the focused PTY/terminal tests
and integrated native smoke. This does not establish Windows ConPTY/Job Object or
macOS terminal acceptance; those platform gates remain under M/P.

## B. Complete generic agent and connection lifecycle

Status: **Integrated foundations; acceptance remains partial**.
Ownership: `synara-agent`, `synara-acp`, `synara-runtime`.

The B/C/D session head
`f1e021905c59f081ba4aeee1f7dcb152d5f94aaa` is now integrated. It adds explicit
authentication-required state, stronger connect/auth/disconnect ownership,
multi-session isolation, stale interaction expiry, bounded input/schema validation
and deterministic shutdown behavior. The final integrated workspace checks and
native smoke pass, superseding the earlier branch-publication blocker. Historical
slice-level evidence remains in [the BCD handoff](docs/bcd-session-handoff.md).

- [ ] B1 Audit ACP method/capability coverage against current primary protocol and
  pinned Rust SDK documentation. Keep stable/unstable behavior explicit.
- [ ] B2 Complete and test connection start, initialize, authenticate, connect,
  fail, exit, restart and disconnect transitions, including crash during requests.
- [ ] B3 Complete capability-driven new/load/resume/close/list/delete session
  behavior where supported, extra directories, titles and concurrent loading.
- [x] B4 Prove a connection owns multiple sessions without duplicate ownership,
  process-per-message behavior or cross-task event leakage.
- [ ] B5 Cover malformed/oversized frames, partial reads, contaminated stdout,
  stderr floods, unknown methods, request-ID correlation, EOF, cancellation,
  timeouts, backpressure and bounded teardown.
- [ ] B6 Complete session modes, model/config selectors, dynamic config changes,
  commands, compaction and MCP configuration where negotiated. Unsupported
  features must be visible or omitted, never assumed by agent name.
- [ ] B7 Keep launch/auth hints and compatibility overrides narrow and isolated.
  Preserve protocol-independent backend/connection/session contracts. Add boundary
  checks so only `synara-acp` depends directly on ACP schema types.

Acceptance remains open because the complete current-protocol capability matrix,
all malformed-transport cases and the full negotiated config/MCP surface have not
been accepted as one final matrix.

Backend cleanup checkpoint: cancellation reaches the external agent before a
blocked/failed event consumer is awaited. All callback terminals receive stop
requests before exit/output waits, and connection failure cleans them before
error delivery. Shared exit/output budgets prevent per-terminal multiplication.
Three regressions fail on the original implementation and pass after the fix.
The follow-up also prevents a prompt from being launched after cancellation
completed during delayed initial-event delivery. FIFO enqueue is serialized with
cancel, while response ownership retains its original deadline and drop cleanup.
Linux backend verification passes 249 tests with strict Clippy and formatting.
See [cleanup backpressure evidence](docs/verification/acp-cleanup-backpressure.md).
B2/B5 remain open for their complete integrated acceptance matrices, not because
this published cleanup slice is missing.

## C. Prove real-agent interoperability

Status: **Partial**, reviewed-release and custom-profile proof integrated.
Ownership: agent integration and QA.

C1/C2 evidence: candidate `a02d6b16468782e7b9d7be9158d3afc80d5e26c1`, Linux x64,
OpenCode 1.18.31, `cargo test --locked -p synara-acp --test vendor_probe -- --ignored
--nocapture --test-threads=1`, successful CI run `35246447427`. Gemini CLI 0.60.0
also passed initialization and explicit auth-required detection in the same run.
See [the compatibility matrix](docs/agent-compatibility.md). Authenticated coding
and the native vendor-agent interaction path remain unverified.

- [x] C1 Install an official OpenCode distribution in an isolated test environment
  with reviewed origin/version and launch it through the generic ACP backend.
- [x] C2 Verify identity, initialize/capabilities and session creation without
  credentials where possible. Distinguish auth-required from protocol failure.
- [ ] C3 With separately supplied authorized credentials, complete prompt,
  streaming, tools, permission allow/deny, cancellation and session restoration.
- [ ] C4 Repeat with a second independently implemented ACP-compatible vendor
  agent without adding another full backend.
- [x] C5 Repeat with a user-defined command/args/environment profile without source
  changes, including paths and arguments containing spaces and Unicode.
- [ ] C6 Record executable/version, OS, supported methods, unsupported features,
  results and redacted diagnostics. Do not fabricate provider/model availability.

The integrated B/C/D branch adds a persisted custom-command fixture covering spaces,
Unicode, literal shell metacharacters and controlled environment-variable names
through the same generic backend, closing C5. C3/C4 remain open because no
user-authorized vendor credentials were supplied for authenticated prompt/tool
journeys.

Acceptance: a compatibility matrix distinguishes fixture, no-credential protocol
and authenticated end-to-end results. Missing credentials leave C3/C4 open but do
not prevent transport, registry, UI or other independent work.

## D. Conversation, permissions and structured questions

Status: **Partial, with scoped interactions and virtual transcript state integrated**.
Ownership: `synara-core`, `synara-agent`, `synara-app`.

The integrated B/C/D work adds bounded typed-input validation, request-scoped
question/permission lifetime handling and virtual transcript scroll ownership.
Final native CI proves that this combined source compiles, tests and opens in the
GPUI desktop, but it does not yet close the long-conversation and complete
interaction acceptance matrix below.

D1/D8 presentation continuation: native GFM tables/task lists and code-copy actions
now render from durable messages. Searchable model/agent choices preserve the
controller's original indices and require a matching key release. The receipt in
[parity-9488759.md](docs/ui/parity-9488759.md) covers positive and rejected search,
actual model acknowledgement, Unicode copy, narrow table layout and restored
transcripts. Full D1/D8 remain unchecked: rich media, structured tool cards,
provider tabs/starred presets, mentions and the other listed controls are not
completed by this slice. No approval policy or backend capability was invented.

- [ ] D1 Complete rendering and durable replay for user/assistant text, thinking,
  tool lifecycle/results/failures, plans, usage, compaction, status and errors.
- [ ] D2 Test stream ordering, duplicate handling, interrupted turns, overlapping
  interactions and restoration failures. Never silently drop hydrated history.
- [ ] D3 Implement user-owned transcript scrolling, explicit follow/resume, long
  transcript virtualization and responsive streaming without activity-driven jumps.
- [ ] D4 Complete draft retention, queued/steered prompts where supported, clear
  send/cancel states and useful recovery after disconnect.
- [ ] D5 Complete allow-once/session/persistent policies only where appropriate,
  denial, cancellation, expiry and restart cleanup. No silent approval or implicit
  promotion from one-time consent to persistent consent.
- [ ] D6 Complete typed question forms, text, choice/multi-choice and URL-based
  interactions where protocol permits. Validate required fields and cancellation.
- [ ] D7 Ensure agent text/links/tool output cannot trigger commands, unsafe URLs,
  credential exposure or UI actions without an explicit trust boundary.
- [ ] D8 Complete the Electron composer interaction set: file/folder/thread/
  agent/skill mentions, slash-command discovery, model/effort presets and
  capability-aware plan, goal, debug and fast controls. Unsupported actions must
  not appear functional.
- [ ] D9 Add edit/resend and safe turn rollback where supported, message fork and
  pin, transcript find/minimap and structured tool/result cards. Preserve durable
  history and require explicit confirmation for file-affecting rollback.
- [ ] D10 Build the per-chat Environment surface: usage, branch/PR/automation
  context, pinned checklist, notes, project instructions, recap, local servers,
  editor targets and side-chat links. Persist user-owned state separately from
  agent-generated text.
- [ ] D11 Add image paste/drop/preview and generated-image presentation, window
  context and voice recording/transcription through explicit platform permission
  and attachment boundaries. Test failure, cancellation and restart paths.
- [ ] D12 Implement provider/worktree handoff, cross-surface context, temporary
  chats and safe thread export/share. Define persistence and deletion semantics
  before exposing these actions.

Acceptance: native UI tests cover both positive and rejected interactions, stale
requests, concurrent requests, keyboard operation and restored conversations.

## E. Registry, installation and custom agents

Status: **Partial, registry recovery/integrity hardening integrated**.
Ownership: `synara-registry`, `synara-workspace`, `synara-app`.

The integrated E/K/O/P work adds offline catalog fallback, immutable review
fingerprints, update classification, path hardening and interrupted-install
recovery tests. macOS arm64, Windows x64 and Linux registry/consumer compile-test
groundwork passed in the isolated session. Full E lifecycle acceptance remains
open as listed below.

- [ ] E1 Keep official registry schema, origin, cache, metadata and platform matching
  current. Validate unsupported schema versions and failed/offline refresh behavior.
- [ ] E2 Exercise binary/archive distributions end to end with digest checks,
  download/expanded limits, traversal/link/device rejection, filename collisions,
  staging cleanup and atomic installation. Never bypass a missing required digest.
- [ ] E3 Exercise pinned npm/uv launchers, runtime discovery and useful missing-tool
  errors. Clearly separate package-manager integrity from verified binary archives.
- [ ] E4 Complete review/approve/install/cancel/remove/update UX, license/origin
  display, rollback to retained versions and refusal to remove in-use installations.
- [ ] E5 Test receipt/executable tampering, symlink substitution, interrupted
  installs, simultaneous operations and environment-based launch substitution.
- [ ] E6 Finish custom profile create/edit/import/export/delete UX and optional
  non-secret defaults. Store secret references, not literal secrets.
- [ ] E7 Validate installed profiles through the same backend on each supported
  target. Show incompatible/unsupported distributions instead of guessing.
- [ ] E8 Provide plugin and skill discovery, search, installed-state display and
  capability-aware enable/disable management. Keep agent-owned extensions and
  Synara-owned installation consent separate.

Acceptance: fixture and real-distribution tests demonstrate consent, integrity,
recoverable failure and no automatic launch merely from browsing/importing.

## F. Workspaces, projects, tasks and persistence

Status: **Partial, persistence hardening integrated**.
Ownership: `synara-core`, `synara-workspace`, `synara-app`.

The F/G/H integration hardens SQLite schema upgrades into one immediate
transaction, rechecks schema state under the writer lock, adds rollback and
concurrent-opener regression coverage and makes equal-recency task ordering
deterministic. Broader workspace/product flows remain open.

- [ ] F1 Complete multi-workspace/project/task create, select, rename, archive,
  recent-work and deletion workflows with clear data-retention behavior.
- [ ] F2 Persist working directories, agent associations, session references,
  selected workspace/task, drafts, settings and appropriate UI state.
- [ ] F3 Prove ordered durable delivery, transaction rollback, idempotent replay,
  task projections and concurrent-writer behavior. Bound long-history memory use.
- [ ] F4 Provide backup/restore, corrupt/inaccessible/disk-full database handling,
  migration tests and explicit forward/backward compatibility behavior.
- [ ] F5 Restore without autostarting agents or executing saved prompts. Surface
  missing directories, missing agents and failed session restoration.
- [ ] F6 Keep local/remote identity separate. Never interpret a remote path through
  local filesystem callbacks or silently substitute a local execution host.
- [ ] F7 Add Synara Spaces, including create/edit/delete, rename/reorder, project
  assignment, activity state and restoration without losing existing projects.
- [ ] F8 Complete project create/import/edit/pin/order and provider-thread import;
  add combined project/thread/command search and the missing sidebar context
  actions. Verify duplicate prevention and safe handling of missing paths.
- [ ] F9 Complete Studio beyond a separate chat scope: generated output tree,
  image previews and attributed files that reopen with their conversation.
- [ ] F10 Complete Kanban overview and project boards with new-task drafting,
  project/provider selection, status movement and clear run/stop semantics.
- [ ] F11 Implement automation definitions, durable scheduling and run history,
  plus create/edit/pause/resume/run/stop/delete UI. Cover time zones, missed runs,
  failure/stop policy, thread ownership and restart without duplicate execution.

Backend recovery checkpoint: `Store::backup_to`, `Store::restore_to` and asynchronous
`WorkspaceService` wrappers now provide bounded online SQLite snapshots and
no-clobber restore into a new path. They validate schema, integrity, foreign keys,
JSON identities and event-head metadata, migrate supported older copies in staging,
and support cancellation/deadlines. Originals and active stores are never replaced.
Linux regression evidence covers WAL snapshots, concurrent writes, corrupt/newer/
hostile inputs, symlinks, SQLite-full rollback and safe service boundaries. Native
macOS/Windows CI is pending for this checkpoint, so F4 remains unchecked until that
matrix is accepted. See [database recovery](docs/database-recovery.md).

Acceptance: restart and interrupted-write tests preserve domain state or show an
actionable failure, never a silently empty replacement database.

## G. Files, editor and diff

Status: **Partial**. Ownership: workspace file services and native editor UI.

- [ ] G1 Complete contained explorer navigation, file creation/rename/delete,
  search and refresh with confirmation for destructive actions.
- [ ] G2 Extend native editing with reliable selection, undo/redo, search/replace,
  multiple files/tabs, dirty indicators and close/save conflict decisions.
- [ ] G3 Handle external modification/deletion, BOMs, line endings, non-UTF-8 text,
  binary files and large files through explicit supported or read-only modes.
- [ ] G4 Preserve atomic/guarded saves and workspace containment. Test traversal,
  symlinks, TOCTOU substitution and permission failures.
- [ ] G5 Complete staged/unstaged diff presentation, navigation, binary/renamed/
  deleted files, large-diff limits and link-back to the corresponding editor file.
- [ ] G6 Keep scans, reads, search and diff computation off the rendering thread.
- [ ] G7 Complete workspace dock tabs, adjustable/maximized splits and side chats;
  persist layout deliberately and test narrow-window keyboard/focus behavior.
- [ ] G8 Add cross-surface editor context and selection-to-chat with reviewed
  references, multi-file tabs and the missing rich diff navigation/review UI.
  Keep file authority and Git mutation in their existing services.

Acceptance: edit/save/reopen and conflict-recovery journeys preserve content and
formatting, and oversized/untrusted inputs do not freeze or escape the workspace.

## H. Git workflows

Status: **Partial, literal unstage hardening integrated**.
Ownership: Rust system-Git service and Changes UI.

The F/G/H integration switches path-only unstage to a literal reset form that also
works before the first commit and adds coverage for spaces, Unicode, bracket
characters, leading dashes, nested paths, detached HEAD and preservation of
unrelated staged/working-tree content. The broader Git workflow matrix remains
open.

- [ ] H1 Harden status, staged/unstaged diff, literal-path stage/unstage and commit
  for unusual filenames, nested directories, detached HEAD and empty repositories.
- [ ] H2 Add branch management, remotes, fetch/pull/push, worktrees and stash with
  progress, cancellation and explicit destructive-operation boundaries.
- [ ] H3 Define hooks/signing and credential policies rather than silently changing
  Git behavior. Never execute repository hooks just by opening a workspace.
- [ ] H4 Surface conflicts, authentication/network failures, missing Git and
  concurrent index changes without discarding user changes.
- [ ] H5 Reuse host-aware services for remote Git. Bound command output and keep
  commands asynchronous to GPUI.
- [ ] H6 Implement the Pull requests route: repository discovery, list/search/
  filters, detail/code/timeline/checks/reviews and authorized create, draft/ready,
  close/reopen and merge actions. Handle auth, unavailable repositories and
  concurrent remote changes without altering unrelated Git state.

Typed backend operations now expose explicit branch/remotes/fetch/fast-forward
pull/non-force push/worktree/stash actions with bounded progress, cancellation,
separate mutation/execution/network consent and redacted error categories.
The stash cleanup and porcelain push rejection regressions are fixed and the
expanded 19-test typed-Git suite passes locally. The full local backend suite,
strict Clippy and formatting also pass. Exact-candidate native and host-aware
acceptance are still pending, so no whole H gate is closed from this checkpoint.
See [typed Git operations](docs/verification/git-operations.md).

Pinned SSH tests now cover typed branches, stash/worktree recovery and explicit
fetch/pull/push, including divergence and changed-key refusal. The existing
`GitService` gains stop-on-drop ownership, a startup-inclusive deadline and
fixed-category diagnostics without changing frontend signatures. Final-candidate
native platform checks remain required for the new cleanup slice.


Acceptance: isolated repository tests cover success/failure for each offered UI
operation. No user repository is used as a destructive test fixture.

## I. Settings, authentication, secrets and platform UX

Status: **Partial / Open**. Ownership: workspace settings and platform boundary.

- [ ] I1 Complete validated, versioned settings and native settings UI, defaults,
  keybindings, theme/font preferences and invalid-config recovery.
- [ ] I2 Host agent-owned protocol/terminal/browser authentication, auth-required
  retry and supported logout without duplicating vendor subscription runtimes.
- [ ] I3 Integrate OS keychain/credential stores for Synara-owned secrets. Handle
  unavailable/locked stores and prohibit plaintext persistence fallback.
- [ ] I4 Complete native menus, file dialogs, notifications, focus traversal,
  accessibility labels, contrast and keyboard-only workflows.
- [ ] I5 Keep any future direct `ModelProvider` separate from coding-agent backends.
  Do not build redundant provider runtimes merely to duplicate agent behavior.
- [ ] I6 Finish all 15 native settings sections against the feature-gap audit.
  In particular, make Chat behavior editable; add editable keybindings, provider
  usage/limits, richer profile insights, models/writing and archived deletion.
  Keep unavailable provider data visibly unavailable rather than invented.
- [ ] I7 Complete Appearance preferences: custom light/dark theme colors and
  import/export, app icon/title bar, density, chat width, typography/terminal
  sizes, time format, contrast and translucent sidebar, with reduced-motion and
  restore-defaults behavior.
- [ ] I8 Add native notification preferences/test delivery and AppSnap/window
  capture setup, permissions, shortcut and attachment flow on supported systems.
  Expose a clear unsupported state elsewhere.
- [ ] I9 Add managed MCP connection pairing/test/revoke and managed-worktree
  inspection/cleanup in Settings, subject to explicit consent and safe secret/
  linked-conversation handling.
- [ ] I10 Compare native main, chat, Studio, settings and dock screenshots and
  interactions with Synara's owned UI at common scales. Finish missing menu,
  popup and work-detail transitions; verify focus, hit testing and reduced motion.

Acceptance: secret-canary tests cover storage, logs, export and screenshots.
Platform-native UX and credential behavior need real per-platform tests.

## J. SSH and remote development

Status: **Integrated Linux remote workflow; forwarding and stronger cleanup gates remain open**.
Ownership: execution host, workspace services and native remote UI.

The recovered A/J/M implementation is integrated. It provides pinned host/identity
configuration, explicit enrollment diagnostics, fail-closed trust behavior,
structured remote execution, guarded remote filesystem access, remote document
save helpers, remote PTY, host-aware Git, persisted remote workspace identity,
disconnect observation/fresh reconnect and a native Remote panel.

Controlled loopback SSH verification passes on the final integrated candidate.
Native remote smoke exercises enrollment, files and terminal interaction. This is
real SSH evidence, not a mock transport. It still does not prove explicit
development-server forwarding or complete cleanup of every remotely detached
descendant.

- [x] J1 Complete host profiles, identity/authentication, known-host verification,
  explicit host-key enrollment and actionable changed-key refusal.
- [x] J2 Prove remote cwd/environment/executable quoting, agent stdio transport and
  remote process ownership through the same backend as local execution.
- [x] J3 Implement remote filesystem read/write/explorer and guarded saves with
  remote containment. Never fall back to local paths on transport failure.
- [x] J4 Add remote PTY input/output/resize, Git and local/remote task association.
- [ ] J5 Implement reconnect, network loss, timeouts, session ownership and cleanup.
  Killing a local SSH client is not proof of remote process cleanup.
- [ ] J6 Add explicit port forwarding, remote development-server discovery and
  consented browser connection with loopback/security defaults.
- [x] J7 Test against a controlled SSH server with pinned host keys, then exercise
  the native remote-workspace journey. Include failure and hostile-input cases.

Acceptance is partial: files, agent transport, terminal and Git run on the selected
host with fail-closed trust and reconnect coverage. J5 remains open for stronger
remote descendant/process ownership guarantees, and J6 remains unimplemented.

## K. Browser host

Status: **Foundation integrated; native browser embedding remains open**.
Ownership: narrow native browser-host boundary and GPUI shell.

The E/K/O/P session contributes a standalone browser consent/lifecycle policy with
one-shot grants, revocation on navigation/close/crash, expiry and negative tests.
No embedded browser engine, tab surface, capture path or helper IPC is wired into
the application yet, so K1-K5 remain open.

- [ ] K1 Select a supported embedding boundary per target and keep the application
  domain in Rust. Document process, rendering and IPC ownership.
- [ ] K2 Implement tabs, navigation, history, focus, redirects, popups, failure and
  crash recovery before exposing richer automation.
- [ ] K3 Isolate cookies/sessions/credentials and agent/application contexts.
  Review OAuth redirects, navigation schemes and origin restrictions.
- [ ] K4 Add controlled screenshots, downloads and automation with explicit
  permissions and bounded IPC, not a general privileged command bridge.
- [ ] K5 Test browser/helper shutdown, downloads, auth isolation and malicious
  content on each implemented platform.
- [ ] K6 Match the Electron browser workflow after the safe host exists: local
  server discovery, tab actions, controlled screenshots/annotations and approved
  agent automation. Keep cookie/vault and download handling inside K3–K5 policy.

Acceptance: embedded content cannot access workspace or application credentials
outside a reviewed permission boundary. Browser delays do not block M1/M2.

## L. Device and iOS tooling

Status: **Open**. Ownership: Rust orchestration plus narrow Apple-native helper if
required by supported APIs.

- [ ] L1 Define device/simulator discovery, lifecycle and failure-state UX.
- [ ] L2 Implement supported simulator capture and input through a documented,
  bounded IPC protocol. Keep Apple API code outside domain logic.
- [ ] L3 Validate helper ownership, stale/disconnected devices, permissions,
  display resize and explicit user-directed input.
- [ ] L4 Test on actual supported Apple hardware/OS. Make unsupported platforms
  explicit. Do not call an untested IPC mock device support.
- [ ] L5 Provide the native device viewer, frame/input controls and agent-visible
  device tools only after L2–L4 prove their supported helper and permission path.

Acceptance: a real supported simulator/device can be displayed and controlled,
with reliable disconnect/cleanup and no undocumented privilege escalation.

## M. Application runtime and service boundaries

Status: **Partial, Linux process/PTY ownership substantially advanced**.
Ownership: `synara-runtime` and platform adapters.

The integrated A/J/M work includes POSIX process-group supervision, aggregate
launch-allocation bounds, PTY lifetime tests, shutdown ordering and host-aware
remote helper boundaries. Linux regression coverage proves foreground-job cleanup,
normal/partial-start teardown, repeated terminal cycles and final-output retention.
The native application now shuts down its owned terminal before exit. Callback
terminal cleanup is now independent of blocked event/error consumers, with
shared teardown budgets and explicit diagnostic failure reporting. See the
[ACP cleanup checkpoint](docs/verification/acp-cleanup-backpressure.md).

- [ ] M1 Complete POSIX process groups, Windows process trees/Job Objects and
  PTY/ConPTY lifetime ownership, including spawn and partial-start failures.
- [ ] M2 Audit executable/environment resolution, shell quoting and WSL behavior
  where offered. Keep arguments structured until the actual shell boundary.
- [ ] M3 Define bounded HTTP/WebSocket/RPC/IPC services only where a product lane
  needs them, with timeouts, cancellation, origin/authentication and error policy.
- [ ] M4 Ensure agents, terminals and browser/device helpers do not become orphaned
  after task close, restart, connection failure or application shutdown.

Linux evidence closes important implementation slices but not the cross-platform
wording of M1, browser/device ownership in M4 or the complete service-boundary
matrix. Windows Job Objects/ConPTY and macOS lifecycle acceptance remain open.

## N. Security, logging and ACP inspector

Status: **Partial**. Ownership: all trust boundaries and native diagnostics UI.

- [ ] N1 Maintain a threat model for repositories, agents, tool output, downloads,
  registry metadata, SSH hosts, browser content and imported/generated files.
- [ ] N2 Audit filesystem containment, privilege/permission prompts, registry
  extraction, network redirects, environment injection and persistent consent.
- [ ] N3 Complete bounded inspector requests/responses/notifications, method/ID,
  connection state, agent version, capabilities and stderr presentation.
- [ ] N4 Add clear, restart and safe copy/export with secret redaction. Document
  that transcript/content traces may still contain sensitive user data.
- [ ] N5 Add dependency/license review, vulnerability checks, log rotation,
  diagnostic bundles and privacy controls. Telemetry, if added, requires explicit
  policy and must not silently capture prompts or credentials.
- [ ] N6 Test hostile inputs, denial paths, redaction, resource exhaustion and
  safe recovery across the implemented services.

Acceptance: negative tests and a focused adversarial pass supplement happy-path
smoke. Callback containment is not described as an OS sandbox.

## O. Performance and accessibility evidence

Status: **Groundwork integrated; product measurements/accessibility evidence open**.
Ownership: native UI, services and QA.

The E/K/O/P integration adds bounded process measurement and sample aggregation
harnesses plus a detailed GUI/performance/accessibility acceptance matrix. These
tools are verification groundwork, not application benchmark results. O1-O4 stay
open until real named-hardware workloads and accessibility journeys are recorded.

- [ ] O1 Establish reproducible startup, idle RAM/CPU, composer, transcript,
  terminal and large-file/diff benchmarks on named hardware/builds.
- [ ] O2 Measure bounded memory and rendering under streaming, large histories,
  terminal floods and concurrent tasks. Set budgets from evidence, not language.
- [ ] O3 Fix measured bottlenecks, then compare before/after under identical input.
- [ ] O4 Audit IME, Unicode/bidirectional text, display scaling, screen readers,
  keyboard-only input, reduced motion and focus restoration per platform.

Acceptance: published measurements and interaction evidence support each claim.
No claim that a Rust/GPUI implementation is automatically faster or accessible.

## P. Cross-platform, distribution and updater

Status: **Cross-platform compile groundwork integrated; native acceptance remains open**.

The E/K/O/P session verified registry/consumer and browser-policy foundations on
macOS arm64, Windows x64 and Linux x64, and compiled the native application on
macOS arm64 and Windows x64. Those results do not constitute native GUI,
terminal, credential, installer or updater acceptance, so P1-P5 remain open.

- [ ] P1 Add native build/test lanes for macOS arm64 and Windows x64 and retain
  Linux x64. Exercise X11 and Wayland separately.
- [ ] P2 Validate text input, dialogs, fonts/GPU behavior, terminal ownership,
  credentials and install paths on actual target environments.
- [ ] P3 Produce development packages/installers with clear prerequisites,
  architecture naming, dependency notices and reproducible build instructions.
- [ ] P4 Implement a consented authenticated/signed update path, download
  integrity, interruption handling, rollback and incompatible-data protection.
- [ ] P5 Obtain owner decisions for project licensing, signing identities,
  distribution/update endpoints and supported OS minimums. Do not invent terms,
  publish packages or create a release without separate authority.
- [ ] P6 Exercise packaged-app desktop journeys on each advertised OS, including
  notifications, file/window capture where supported, restart/restore and update
  failure recovery. Record feature exclusions and platform-specific differences.

Acceptance: each advertised target has its own working artifact and interaction
results. CI cross-compilation alone does not qualify a target as supported.

## Q. Final integration, documentation and delivery

Status: **Open for final delivery; current parallel-session integration completed**.

The four completed session branches are now integrated on the independent rewrite
lineage and the exact combined candidate passed static, SSH and native Linux
verification. Q remains a final-candidate gate because the product UI overhaul and
other open lanes below must be reverified again before delivery.

- [ ] Q1 Run the applicable verification commands below against the exact final
  candidate, not only an earlier green commit.
- [ ] Q2 Map every implemented UI feature to tests and every remaining requested
  feature in the [Electron feature-gap audit](docs/ui/electron-vs-gpui-feature-gap.md)
  to an open task here or an explicit owner-approved exclusion. Remove dead
  scaffolding and stale claims.
- [ ] Q3 Synchronize README, settings/profile examples, architecture boundaries,
  compatibility matrix, security notes, migration/backup and troubleshooting docs.
- [ ] Q4 Check formatting, secrets, dependency notices and generated/vendor
  boundaries. Keep new source independently authored and domain logic in Rust.
- [ ] Q5 Publish only `astra/gpui-clean-rewrite`. Verify expected root and ancestry,
  intended commits and unchanged protected branch refs. Do not create/update a PR,
  merge, create a release, change the default branch or write other branches.
- [ ] Q6 Record exact final commit, tests/results, actual agents/platforms tested,
  residual failures and the next implementation lane in the checkpoint report.

Protected refs: `main` and `archive/pre-rewrite-main-2026-09-17`.
Expected parentless root: `43b1fb89bf19dadc388d18008f9ceb21b8215716`.
Do not merge, rebase or graft other histories into the delivery branch.

## Verification contract

```sh
cargo fmt --check
cargo check --locked --workspace
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
python3 scripts/audit_workspace.py
python3 scripts/test_apply_source.py
python3 scripts/check_roadmap.py --self-test
python3 scripts/check_roadmap.py
cargo build --locked -p synara-app --bin synara-app -p synara-acp --bin synara-acp-fixture
python3 scripts/native_smoke.py --binary target/debug/synara-app \
  --fixture target/debug/synara-acp-fixture --output /tmp/synara-roadmap-smoke
```

Use a fresh smoke output directory and an isolated display/test data directory.
Never control the user's desktop or run test commands inside an unrelated user
project. Install requirements only through an authorized environment path.

SSH acceptance additionally requires `python3 scripts/ssh_smoke.py` on Linux with
OpenSSH installed, running as an ordinary user. It explicitly executes the seven
server-dependent `ssh_live` tests that the ordinary test suite leaves ignored.

Vendor protocol acceptance additionally requires the explicitly opted-in
`vendor_probe` test and reviewed release manifest. See
[agent-compatibility.md](docs/agent-compatibility.md) for isolation, commands,
results and the distinction between an auth-required response and authenticated
coding. Ordinary workspace tests never download or launch vendor releases.

For each accepted task, retain this evidence shape in a checkpoint or test report:

```text
Task ID:
Candidate commit:
Platform / executable versions:
Check / journey:
Expected behavior:
Observed result:
Evidence path or CI job:
State: PASS | FAIL | GAP | BLOCKED
Remaining limitations:
```

### Coverage map

| Requested domain | Roadmap ownership |
| --- | --- |
| Native Rust architecture, GPUI and independent delivery | B, I, M, Q |
| Generic ACP transport, capabilities, connection/session APIs | B |
| OpenCode, second real agent and arbitrary custom commands | C, E |
| Auth, models/config, MCP and optional protocol functionality | B, C, I |
| Normalized conversation, streaming, tools, plans and usage | D, F |
| Permissions, elicitations, filesystem/terminal callbacks | A, B, D, G, N |
| Registry metadata/distributions, secure install and inspector | E, N |
| Workspace/projects/tasks, SQLite, settings and restoration | F, I |
| Spaces, project/thread import, sidebar search and Studio outputs | F7–F9 |
| Kanban task creation/movement and Automations | F10–F11 |
| Editor/explorer, diffs and system Git workflows | G, H |
| Pull request discovery, review and authorized actions | H6 |
| Chat actions, multimodal input, handoff and Environment context | D8–D12 |
| Plugins, skills and complete settings/appearance | E8, I6–I10 |
| Native terminal, PTY/ConPTY and process supervision | A, M |
| SSH, remote files/agents/PTY/Git and forwarding | J |
| Browser, OAuth, downloads and automation | K, I, N |
| Device/simulator presentation and native IPC | L, M |
| Credentials, updater, packaging and owner decisions | I, P |
| Security, performance, platform acceptance and documentation | N, O, P, Q |

## Immediate execution queue

1. Complete the main chat journey first (D8–D12): composer discovery, attachments,
   queue/steer, rich transcript actions and Environment context. Keep the
   independently authored Synara visual/motion pass in I10 aligned with each
   implemented flow.
2. Deliver the missing whole routes and sidebar workflows (F7–F11, H6): Spaces,
   imports/search, Studio outputs, Kanban task creation, Automations and Pull
   requests. Treat scheduler and remote PR actions as backend work with explicit
   authorization and recovery, not just screen reconstruction.
3. Complete workspace and settings depth (A8, E8, G7–G8, I6–I9), then Browser
   and supported Device tools (K/L). Finish I/N security and secrets alongside
   these flows. Continue B/C acceptance, E lifecycle, J5/J6 remote cleanup and
   O/P measurement/platform evidence. C3/C4 require separately authorized vendor
   credentials; L requires supported Apple hardware/OS evidence.
4. For each feature, compare the native interaction to the dated
   [feature-gap audit](docs/ui/electron-vs-gpui-feature-gap.md) and record exact
   candidate/runtime evidence. Run Q against the final integrated candidate,
   synchronize docs, verify the independent root and protected refs, and publish
   only `astra/gpui-clean-rewrite`.

Native navigation continuation (September 19, 2026): the published-source shell has a modular navigation/design foundation, backend-owned project/chat actions and a native keyboard-menu regression in regular CI. This is a partial checkpoint, not recovery of the unpublished local UI or a visual-parity claim. See `docs/ui/native-navigation.md` and the exact-candidate evidence receipt. Existing unfinished roadmap items remain open.
