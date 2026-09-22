# Synara delivery roadmap

This roadmap has two layers:

1. **Current product roadmap** — what users can already do, what is only partial,
   and which substantial product capabilities are genuinely still missing.
2. **Engineering and acceptance ledger (A-Q)** — detailed implementation,
   interoperability, security, platform, performance and release gates.

The A-Q checkbox count is **not** an application-completion percentage. A broad
gate can remain unchecked after substantial feature code has landed because the
same task also requires provider, hardware, accessibility or cross-platform proof.
The original 120 task bodies and checkbox states remain below as the detailed
ledger and historical evidence.

Current comparison snapshot, September 22, 2026:

- Electron reference: `Emanuele-web04/synara@f04341a67bc4941d1b2e91e0b23bbe782dfbc727`
  (Synara 0.9.0-era main).
- Native Rust reference: `cmdr-chara/synara@bd30c42b395f361b0384b744c22039485cca290f`.
- Electron moved **342 commits** beyond the `e7cd152` revision used by the previous
  audit. Computer Use and project-history import are therefore new parity inputs.
- The current audit uses 48 top-level user-visible Electron capabilities: the 46
  named entries in Emanuele's feature overview plus Computer Use and Project Import.

See the current [Electron-to-native feature-gap audit](docs/ui/electron-vs-gpui-feature-gap.md)
for source evidence and the preserved
[pre-Hubs roadmap](docs/history/roadmap-before-hubs-2026-09-21.md) for older checkpoints.

## Current product status

"Present" means the corresponding user capability is substantially implemented in
the Rust product architecture. It does **not** mean every platform or production
acceptance gate is closed.

| Product status | Count | Meaning |
| --- | ---: | --- |
| Present | **8 / 48** | Substantial native user capability exists; remaining work is mainly acceptance or narrower depth |
| Partial | **22 / 48** | Real native functionality exists, but Electron still has material user-facing depth not yet present |
| Missing | **18 / 48** | No equivalent complete user workflow exists yet, or the current architecture does not satisfy the stated Synara product requirement |

### Substantially present

| Capability | Native state |
| --- | --- |
| Side threads | Independent Side chats with separate drafts, tasks, provider sessions and permissions |
| Project Spaces | Native Spaces organization, ordering, assignment and persistence |
| Plan mode | Uses the generic agent's advertised session mode rather than provider-name assumptions |
| Pull Requests workspace | GitHub discovery, list/detail/diffs/checks/reviews and explicit confirmed remote actions |
| Git actions | Branches, remotes, fetch/pull/push, worktrees, stashes and guarded local commit workflows |
| Integrated terminals | Multiple owned terminals, tabs, splits, search, copy and local/validated remote execution |
| Pins & notes | Durable project/thread/message pins plus task-local notes/checklists |
| Local-first data model | Tasks, transcripts, drafts, preferences and recovery live in the native local workspace store |

### Partial capabilities

The following are **not missing from scratch**. They already have meaningful Rust
implementation but still trail the Electron workflow in depth:

- **Models/runtime:** model quick-switching, reasoning/effort controls, context
  management and usage/limit presentation.
- **Parallel/task workflow:** per-task worktree isolation, thread forks,
  Automations and Studio/Hubs output workflows.
- **Review:** diff review depth and code-review-on-demand.
- **Composer/context:** attachments and replies/quoted output as structured context.
- **Workspace tools:** browser preview/dev-server workflow, editor workspace,
  file/source search and iOS Simulator/device depth.
- **Navigation/export:** keyboard-first navigation, thread export and app-owned
  slash-command coverage.
- **Personalization:** complete keybinding coverage, theme/density controls and
  provider/model-rich profile activity statistics.

### Genuinely missing feature development

| Capability | Current requirement |
| --- | --- |
| Direct multi-provider model runtime | **Major requirement.** Synara itself must support an OpenCode-class catalog of roughly **75+ providers**, independent of ACP. Generic ACP remains the coding-agent layer and does not satisfy direct-provider support. |
| Provider handoff | Continue the same task through another provider/model while preserving reviewed conversation/environment/Git context |
| Native subagents & workflows | Child-agent delegation with visible ownership, phases, usage and pause/stop controls |
| Agent Gateway | App-owned, scoped tool surface allowing capable agents to create/steer Synara work under Synara ownership rules |
| External MCP client integration | Let external MCP clients connect *to Synara*. This is distinct from configuring MCP servers that Synara passes *to an agent*. |
| Debug mode | App-owned evidence-first debug workflow rather than an unavailable menu item |
| Persistent thread goals | Durable objectives, pause/resume, bounded continuation, blocker handling and achievement history |
| Checkpoints & revert | Explicit safe state rollback; edit/resend intentionally does not pretend to roll back files/provider state |
| Stacked pull requests | Stack ordering/readiness and explicitly confirmed safe-prefix merge workflow |
| PR Fix workflow | Gather unresolved review comments into one reviewed agent task/prompt |
| Inline file comments | Line-specific review comments carried into the next prompt |
| AppSnap | Permissioned desktop-window capture and composer attachment flow |
| Rich media in transcript | Generated-image presentation/download and PDF/document viewing comparable to Electron |
| Two-task split views | Keep two independent conversations/tasks visible side by side, including cross-project |
| Thread recap | Generated and cached long-thread summary/re-entry workflow |
| Releases in the app | What's New, release history and user-facing verified update lifecycle |
| Computer Use | New Electron 0.9.0 capability: permissioned desktop/app control, preview, interruption/takeover and audit boundaries |
| Project Import | New Electron 0.9.0 capability: discover/import local Codex and Claude project/conversation history with recovery and duplicate protection |

## Architectural invariants

- **ACP remains generic.** Synara must continue to support ACP-compatible coding
  agents without implementing provider-specific agent backends.
- **Direct model providers are a separate product layer.** Add a provider-neutral
  `ModelProvider`/model-runtime boundary rather than routing direct model use through
  ACP or coupling it to coding-agent process/session ownership.
- **75+ provider breadth must scale through normalization**, not 75 unrelated Rust
  backends. Use a provider/model registry, a small number of transport families,
  capability negotiation and custom/OpenAI-compatible endpoints where appropriate.
- Direct-provider auth, model metadata, streaming, tools, structured output,
  multimodal input, reasoning controls and usage must fail honestly when unsupported.
- MCP direction matters: managed MCP passed to an agent and external clients
  connecting to Synara are separate trust/ownership boundaries.
- Standalone chats remain the default. Hubs are optional shared workspaces. Zen is
  presentation only and never changes task/session ownership.

## Current feature-development sequence

| Priority | Workstream | Definition of done for feature development |
| --- | --- | --- |
| P0 | Direct multi-provider runtime | Synara-owned provider/model registry and normalized direct model runtime supporting the broad provider ecosystem without provider-by-provider UI/backend duplication |
| P1 | Electron 0.9.0 delta | Computer Use and Project Import have real native ownership, permission, persistence and recovery paths |
| P2 | Agent orchestration | Provider handoff, native subagents, Agent Gateway and external-MCP-to-Synara contracts are explicit and scoped |
| P3 | Conversation autonomy/recovery | Goals, checkpoints/revert and Debug mode are real workflows with no hidden execution or fake rollback |
| P4 | Review/composer depth | Stacked PRs, Fix workflow, inline comments, AppSnap, rich media, task split views, recap and release UX |
| P5 | Partial-feature closure | Close the material Electron depth gaps in models/context, Automations, Studio/Hubs, browser sessions/dev servers, editor/search, device/iOS, navigation/export and personalization |

## Acceptance and release work

The following remains important, but is **not unfinished feature development by
itself** when the corresponding feature is already present:

- real authenticated provider/agent interoperability;
- production OS credential stores;
- macOS, Windows and Wayland native acceptance;
- real device/hardware and simulator acceptance;
- accessibility, IME, scaling and reduced-motion evidence;
- browser production-site, download/capture and authenticated-session acceptance;
- scheduler restart/shutdown and external-effect recovery evidence;
- packaging, signing, updater, rollback and supported-OS decisions;
- performance/resource budgets, security review and final release evidence.

## Historical implementation checkpoints

### September 22: Sessions 1-4 consolidated native feature implementation

**Feature implementation session status: complete** for all four requested sessions.
This is deliberately narrower than production, provider and cross-platform acceptance.

- **Session 1, conversation depth:** Side chats are independent related tasks with
  their own drafts, sessions and permission/input ownership. Edit/resend adds a new
  turn without rewriting transcript history. Revision branches create explicit new
  unsent conversations from bounded visible context and do not clone provider
  sessions, approvals, attachments or filesystem state.
- **Session 2, Pull Requests / Automations / Browser:** Pull Requests reuse the
  existing Git/process owners for bounded GitHub discovery, review data and explicit
  confirmed remote actions. Automations persist definitions and bounded run history
  separately from transcripts, atomically claim scheduled slots and create owned
  tasks, start disarmed after restart, and never retry implicitly. Browser reuses
  the existing BrowserHost/Session architecture, with real Linux/X11 WebKitGTK
  embedding plus revocable task-isolated, one-shot-approved agent operations.
- **Session 3, Plugins / Skills / MCP:** Native ownership-aware integration
  inventory, reviewed local Markdown skills and task/agent-scoped HTTP MCP
  configuration remain integrated. Secret material stays behind reference-based
  credential boundaries and negotiated capabilities, not provider-name guesses.
- **Session 4, Device / Settings:** Bounded ADB/simctl discovery/capture/lifecycle
  support, explicit device-input authority and the implemented Settings/privacy/
  notification/navigation functionality remain integrated.

The consolidation review found no unique feature commits left to replay from the
temporary branches at its starting ref. The Session 2 final head matched the
integration head, Sessions 3 and 4 were strict ancestors, and the Session 1 branch
had already been retired after its conversation commits landed. Current cross-session
ownership keeps manual saved follow-ups separate from automation scheduling, browser
task credentials separate from manual cookies and general MCP credentials, related
conversation state separate from automation records, and PR writes on the existing
Git/provider boundary.

Broad acceptance remains evidence-driven. In particular macOS/Windows acceptance,
native Wayland browser support, production packaging/updater, accessibility/IME,
live authenticated GitHub/provider journeys, production scheduler/restart behavior,
real hardware/device acceptance and a production OS credential store remain open
where the receipts below do not prove them. Historical task bodies and older
checkpoint statements remain evidence for the revisions they describe.

### September 21: Pull Requests, durable Automations and native Browser

The single `astra/pr-automations-browser` session adds GitHub provider discovery,
bounded list/search/detail/activity/checks, the existing native diff presentation,
and explicit confirmed create/comment/review/state-change/merge actions. Loaded
project identity and reviewed head SHA constrain writes. It does not check out
branches, stage files or create a second Git implementation.

Automations has a SQLite definition/run ledger, atomic scheduled-slot claims and
owned task creation, revisions, fixed-offset schedules, history, cancellation and
explicit scheduler arming. Restart never replays saved instructions by implication.
IANA/DST, automatic retries, history pruning, direct Hub association and live-provider
acceptance remain open rather than being silently approximated.

Browser uses the existing Session/BrowserHost with a real Linux/X11 WebKitGTK child
surface and a task-isolated, revocable MCP endpoint. Agent navigate/read/click/fill
operations require one-shot native approval. DOM references remain in an isolated
script world. No arbitrary script execution, general page IPC, cookie export or
unbounded transfer interface is exposed. Native Wayland, Windows, macOS, download/
capture exports, accessibility, IME and production-account acceptance remain open.

[Pull Requests](docs/ui/pull-requests.md), [Automations](docs/ui/automations.md) and
[Browser](docs/ui/native-browser.md) document their operational limits. The
[candidate-specific receipt](docs/verification/pr-automations-browser.md) records
actual checks and remaining acceptance. This checkpoint does not close any broad
historical lane gate or change the original task bodies and checkbox states.

### September 21: native Plugins, Skills and scoped MCP management

The session from `980d86b` adds compact native ownership-aware Plugins inventory,
reviewed local Markdown skill import/update/remove with origin receipts and
explicit unsent draft insertion, and task/agent-scoped HTTP MCP configuration.
MCP tests perform bounded modern discovery or legacy negotiation and list only
advertised tools. Saved credentials are references through the existing secret
boundary. Agent sessions receive only explicitly enabled, capability-checked
configuration and must retire before reconfiguration or local removal.

Provider-owned extension lifecycle, remote skill catalogs, production OS credential
stores, OAuth and unsupported transports remain visible limitations, not switches
or successful-install claims. [Feature behavior](docs/integrations.md) and the
[focused implementation receipt](docs/verification/plugins-skills-mcp.md) separate
source, fixture and native evidence. E8/I9 and platform acceptance remain open.
The one authorized session branch is integrated only after focused verification.

### September 21: Device and functional Settings source

`astra/device-settings` continues exact base `980d86b`. Real bounded ADB/simctl
adapters now back native discovery, screenshot presentation, supported lifecycle
commands and explicit probed Android input. The Device Environment tab fences
late replies, retains stale targets, revokes transient authority, handles resized
viewports and stops capture work when hidden without changing chat/session state.

Settings adds restore-last-chat behavior, effective editable navigation bindings,
recent-attachment visibility, higher contrast without flattening Glass, actual
agent/configuration entry points, desktop notification preference/test and privacy
controls. Archived deletion reserves the task, closes its session, and checks the
archive state inside the SQLite writer transaction. No quotas or unsupported
platform capabilities are invented. Existing appearance profiles, usage and SSH
boundaries remain intact.

[Device/Settings inventory and unsupported targets](docs/ui/device-settings.md) and
[implementation receipt](docs/verification/device-settings-session.md) separate
source checks from Rust validation and hardware/native acceptance. A session-scoped
validation is prepared. Hardware, macOS, Windows, native capture/input, screen-reader
and notification-delivery acceptance remain open. No historical task body or
checkbox state is changed, and no whole L/I/P gate is closed by these source slices.

### September 21: native attachment intake and saved follow-ups

Continuing `0bb20db`, the composer now implements local PNG/JPEG and UTF-8 file
selection, image/file clipboard paste, drop, persistent per-task snapshots, bounded
previews, removal, recent-snapshot reuse and capability-checked agent delivery.
File paths are not stored or silently sent. Intake failures retain recoverable inputs,
late results keep their original task identity, and cancelled preparation cannot
turn into a delayed agent launch. The text-only submission path stays unchanged for
other callers. Local transcript acknowledgement is not provider delivery proof.

A separate manual follow-up list persists text drafts with edit, reorder, remove
and append-to-composer actions. Queuing clears only unchanged text/edit revisions
once saved. Nothing is auto-sent, steered or executed after restart. New preference
records participate in task deletion and the current backup key validator, including
the existing Hub metadata keys. The compact tray uses existing native controls and
opens details progressively rather than adding another permanent panel.

[Scope, privacy bounds and acceptance](docs/ui/composer-intake-followups.md) records
source checks and five prepared, unrun regressions. Native compilation, clipboard/
drop/IME behavior, real-agent delivery and restart remain unverified. No GitHub test
workflow is dispatched. D4/D8/D11/D12/F2/F4/I10 remain open for their full scope.
Upstream was reviewed at `e7cd15281e6d16cf8fc55a91496dcff035475e54`, unchanged.

### September 21: compact workspaces, Hub tasks and editor management

The continuation from `e418fff` adds a Hub-scoped native task view with literal
search, activity/attention filters, counts, pinning and explicit Run/Stop. Creation
uses the existing atomic Hub task service and captures scope when the composer
opens. Normal Kanban remains separate. Task state is not simulated by dragging a
card, and shared context is never silently added to a Create-and-run request.

Editor workspace commands add guarded Save all, stop-between-files, Close saved,
Close other saved, retained-buffer reopening and tab reordering. Save all preserves
newer edits, uses existing local/pinned-SSH guarded writes and stops on the first
failure. Closed buffers retain in-process undo/selection only, not restart recovery.
Terminal tab ordering now persists through its existing layout owner without
restarting or moving processes.

MonoCode's workspace captures inform compact pane-local controls, flat tabs and
row-based Hub navigation. No competitor implementation/assets/tokens are copied.
The existing Synara shell, glass materials and Zen domain boundaries remain intact.
See [workspace direction, features and acceptance](docs/ui/monocode-workspace-adaptation.md).
Source checks do not establish native compilation or behavior. No GitHub test
workflow is dispatched. F10, A8, G2/G8 and I10 remain open for their full acceptance.

### Previous recovery checkpoint

September 21 recovery continues published `bf67608` and Glass correction `ae64983`.
Transparent Glass uses one continuous window tint, native compositor blur requests,
non-opaque editor/terminal/Git roots and bounded local wallpaper preparation. Zen
retains the existing Environment, focus/IME guards and explicit narrow-window exit.
Assistant-message branching saves quoted context into a new unsent draft, without
cloning a provider session, permission decisions or filesystem state.

The Hubs foundation replaces the Studio-facing workspace navigation with optional
shared-context workspaces. It adds creation with a managed or chosen local folder,
Main and child threads, editable instructions and curated knowledge, revision-checked
saving, archive/restore, source-message promotion and a Library with reporting-thread
links. Normal Synara chats remain available without Hub setup. Existing Studio task,
draft, session and file identities are retained through a compatibility projection.
The serialized `Studio` scope remains until a separately verified migration can
remove it. Hubs are not a cloud runner, automatic memory system or new agent backend.

At the previous recovery checkpoint, Hub-specific Kanban was still missing. The
new task view above now implements a bounded source slice of it. Multi-repository/
source membership, semantic retrieval, automations, parallel task orchestration and
Settings compatibility labels remain open. Library previews require a local workspace.

The initial source tree was reconstructed exactly before recovery. Local structural
checks do not establish compilation. Cargo/rustc are unavailable and the pinned
Rust host cannot be resolved in this sandbox. Native build, focus/restart journeys,
platform transparency and native screenshots remain unverified. No GitHub test
workflow is dispatched and workflow configuration is unchanged. Feature commits
use `[skip ci]`; the complete acceptance contract below is deferred, not removed.

At that recovery checkpoint, upstream `Emanuele-web04/synara` main was reviewed at
`e7cd15281e6d16cf8fc55a91496dcff035475e54`. The current product-status section
above supersedes that snapshot. The previously recorded passive delegated-result
and macOS icon fixes remain historical evidence for that revision.

See [Glass and branching](docs/ui/zen-personalization.md),
[Hubs implementation and migration](docs/ui/hubs.md), and the
[feature-gap audit](docs/ui/electron-vs-gpui-feature-gap.md).
Zeron is a Zen-only UX reference. No competitor code, assets or theme values are
copied, and normal Synara navigation is not replaced by a Zeron shell.

## Completion and ownership

This A-Q section is the **engineering and acceptance ledger**, not the product
feature-count dashboard. Integrated means published source. Partial means only part
of a gate is implemented or verified. Check a gate only with its required evidence.
Fixtures are not vendor agents, browser mockups are not GPUI screenshots, and one
platform is not all targets. Do not derive an app-completion percentage from the
checkbox count.

The current direct-provider requirement is broader than the original wording of I5:
Synara must eventually provide a separate, provider-neutral direct model runtime with
roughly 75+ provider breadth while preserving generic ACP as an independent
coding-agent architecture. The original I5 task body remains unchanged below.

F9's former Studio output requirements now belong to the optional Hub Library.
F1/F2/F8 own Hub organization, context persistence and compatibility. D9/D10/D12 own
message promotion, shared context and related execution threads. F10/F11/H6 still
own real Kanban, scheduling and PR integration. These mappings do not close gates.

Only publish `astra/gpui-clean-rewrite`. Keep `main`,
`archive/pre-rewrite-main-2026-09-17`, releases and default-branch configuration
unchanged. Do not open a PR without an explicit request. Preserve the independent
root `43b1fb89bf19dadc388d18008f9ceb21b8215716` and never graft another history.

## Delivery order

| Milestone | User-visible outcome | Current state | Completion gate |
| --- | --- | --- | --- |
| M1: native workspace + generic ACP | Durable chats/tasks, ACP agents, files/Git/terminal, permissions and recovery | Substantial implementation present | Finish remaining interoperability and lifecycle acceptance without provider-specific agent duplication |
| M2: direct provider platform | Synara itself can use a broad OpenCode-class provider/model ecosystem | **Major feature development missing** | Provider-neutral runtime, secret-backed auth, dynamic models/capabilities and representative provider-family proof |
| M3: Electron capability closure | Missing/partial user workflows above reach intended native depth | In progress | No current "missing" capability remains without an explicit owner-approved exclusion |
| M4: platform/provider/hardware acceptance | Implemented features behave on supported OS/provider/device targets | Open by area | Platform-specific evidence, accessibility/security and real-service/hardware proof |
| M5: production readiness | Installable, recoverable, secure and supportable builds | Open | Packaging/signing/updater/rollback, performance budgets, final documentation and release-owner decisions |

## A. Recover and complete the native terminal

Status: **Integrated and verified on Linux; cross-platform acceptance remains open**.

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

## B. Complete generic agent and connection lifecycle

Status: **Generic ACP foundations integrated; lifecycle/interoperability acceptance remains partial**.

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

## C. Prove real-agent interoperability

Status: **Partial**, with real-agent/custom-profile proof integrated; broader ACP interoperability remains open.

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

## D. Conversation, permissions and structured questions

Status: **Feature-rich partial**: Side chats, additive edit/resend, search, pins and export are integrated; handoff/goals/checkpoints/debug and richer interaction depth remain open.

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

## E. Registry, installation and custom agents

Status: **Partial**: secure agent registry/custom-profile foundations and Synara-owned Skills/Plugins inventory are integrated; provider-owned lifecycle depth remains open.

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

## F. Workspaces, projects, tasks and persistence

Status: **Feature-rich partial**: durable tasks/Hubs/Kanban/Automations and recovery foundations are integrated; Project Import and remaining Studio/organization depth remain open.

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

## G. Files, editor and diff

Status: **Feature-rich partial**. Multi-file editing, search, preview, Explorer and native review integration exist; richer Electron editor/diff depth and task split views remain open.

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

## H. Git workflows

Status: **Feature-rich partial**: repository operations and the Pull Requests workspace are integrated; stacked PR/Fix/review-depth and broader network acceptance remain open.

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

## I. Settings, authentication, secrets and platform UX

Status: **Feature-rich partial**. Native Settings is substantial; the direct 75+ provider runtime, production secret store and remaining platform UX remain major open work.

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

## J. SSH and remote development

Status: **Integrated Linux remote workflow; forwarding and stronger cleanup gates remain open**.

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

## K. Browser host

Status: **Linux/X11 native Browser implementation complete for the current feature slice; cross-platform host, browser-session depth and production acceptance remain open**.

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

## L. Device and iOS tooling

Status: **Device tooling source/UI implemented for bounded ADB/simctl discovery, lifecycle and capture; full iOS input/agent-device parity, hardware proof and platform acceptance remain open**.

- [ ] L1 Define device/simulator discovery, lifecycle and failure-state UX.
- [ ] L2 Implement supported simulator capture and input through a documented,
  bounded IPC protocol. Keep Apple API code outside domain logic.
- [ ] L3 Validate helper ownership, stale/disconnected devices, permissions,
  display resize and explicit user-directed input.
- [ ] L4 Test on actual supported Apple hardware/OS. Make unsupported platforms
  explicit. Do not call an untested IPC mock device support.
- [ ] L5 Provide the native device viewer, frame/input controls and agent-visible
  device tools only after L2–L4 prove their supported helper and permission path.

## M. Application runtime and service boundaries

Status: **Partial, Linux process/PTY ownership substantially advanced**.

- [ ] M1 Complete POSIX process groups, Windows process trees/Job Objects and
  PTY/ConPTY lifetime ownership, including spawn and partial-start failures.
- [ ] M2 Audit executable/environment resolution, shell quoting and WSL behavior
  where offered. Keep arguments structured until the actual shell boundary.
- [ ] M3 Define bounded HTTP/WebSocket/RPC/IPC services only where a product lane
  needs them, with timeouts, cancellation, origin/authentication and error policy.
- [ ] M4 Ensure agents, terminals and browser/device helpers do not become orphaned
  after task close, restart, connection failure or application shutdown.

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

## O. Performance and accessibility evidence

Status: **Groundwork integrated; product measurements/accessibility evidence open**.

- [ ] O1 Establish reproducible startup, idle RAM/CPU, composer, transcript,
  terminal and large-file/diff benchmarks on named hardware/builds.
- [ ] O2 Measure bounded memory and rendering under streaming, large histories,
  terminal floods and concurrent tasks. Set budgets from evidence, not language.
- [ ] O3 Fix measured bottlenecks, then compare before/after under identical input.
- [ ] O4 Audit IME, Unicode/bidirectional text, display scaling, screen readers,
  keyboard-only input, reduced motion and focus restoration per platform.

## P. Cross-platform, distribution and updater

Status: **Cross-platform compile groundwork integrated; native acceptance remains open**.

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

## Q. Final integration, documentation and delivery

Status: **Open for final delivery; current parallel-session integration completed**.

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

1. **Build the direct multi-provider foundation first.** Define the provider-neutral
   model/runtime contract, registry/catalog, secret-backed authentication and the
   minimum transport families needed to scale toward roughly 75+ providers. Keep
   this strictly separate from generic ACP coding-agent sessions.
2. **Absorb the current Electron 0.9.0 delta.** Design and implement Computer Use
   and Project Import with explicit ownership, permissions, recovery and platform
   boundaries instead of visual-only parity.
3. **Close orchestration and conversation gaps.** Provider handoff, native subagents,
   Agent Gateway, external-MCP-to-Synara, goals, checkpoints/revert and Debug mode
   should reuse existing task/session owners and must not inject hidden authority.
4. **Finish high-value depth gaps.** Stacked PR/Fix workflows, inline comments,
   AppSnap/rich media, two-task split views, thread recap, release UX, browser
   sessions/dev servers, complete iOS tooling and remaining editor/navigation depth.
5. **Run acceptance after coherent feature slices.** Use focused verification while
   developing, then close provider/platform/hardware/accessibility/security and final
   Q gates against exact integrated candidates. Do not turn missing acceptance into
   fake feature incompleteness or vice versa.
