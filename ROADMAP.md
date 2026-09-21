# Synara delivery roadmap

This is the implementation and acceptance backlog for native Rust/GPUI Synara.
Code present on the feature branch is not the same as verified native behavior.
The complete earlier checkpoints, per-lane explanations and failed/passed receipts
are preserved in [the pre-Hubs roadmap](docs/history/roadmap-before-hubs-2026-09-21.md).
All 120 original task bodies and checkbox states remain below without alteration.

## Current checkpoint

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

Upstream `Emanuele-web04/synara` main was reviewed at
`e7cd15281e6d16cf8fc55a91496dcff035475e54`, unchanged. The previously recorded
passive delegated-result and macOS icon fixes remain deferred.

See [Glass and branching](docs/ui/zen-personalization.md),
[Hubs implementation and migration](docs/ui/hubs.md), and the
[feature-gap audit](docs/ui/electron-vs-gpui-feature-gap.md).
Zeron is a Zen-only UX reference. No competitor code, assets or theme values are
copied, and normal Synara navigation is not replaced by a Zeron shell.

## Completion and ownership

Integrated means published source. Partial means only part of a gate is implemented
or verified. Check a gate only with its required evidence. Fixtures are not vendor
agents, browser mockups are not GPUI screenshots, and one platform is not all targets.
Do not derive an app-completion percentage from the checkbox count.

F9's former Studio output requirements now belong to the optional Hub Library.
F1/F2/F8 own Hub organization, context persistence and compatibility. D9/D10/D12 own
message promotion, shared context and related execution threads. F10/F11/H6 still
own real Kanban, scheduling and PR integration. These mappings do not close gates.

Only publish `astra/gpui-clean-rewrite`. Keep `main`,
`archive/pre-rewrite-main-2026-09-17`, releases and default-branch configuration
unchanged. Do not open a PR without an explicit request. Preserve the independent
root `43b1fb89bf19dadc388d18008f9ceb21b8215716` and never graft another history.

## Delivery order

| Milestone | User-visible outcome | Depends on | Completion gate |
| --- | --- | --- | --- |
| M1: reliable local agent loop | Open workspace, choose agent, work in a durable task, use terminal, recover safely | Existing baseline, A, B, C, D | Real agent proof plus local interaction/recovery acceptance |
| M2: complete local workspace | Everyday files/editor/diff/Git, agent management and settings | M1, E, F, G, H, I | Complete local journey and failure-path matrix |
| M3: remote workspace | Run an agent and development tools over verified SSH | B, C, F, J | Controlled SSH integration plus remote UI smoke |
| M4: browser and device tools | Browser-assisted workflows and supported device/simulator integration | K, L, M | Isolation, lifecycle and platform-specific proof |
| M5: production readiness | Installable, recoverable, secure and responsive supported-platform builds | All applicable lanes, N, O, P, Q | Per-platform evidence and owner decisions, not an automatic release |

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

Status: **Integrated foundations; acceptance remains partial**.

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

Status: **Partial**, reviewed-release and custom-profile proof integrated.

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

Status: **Partial, with scoped interactions and virtual transcript state integrated**.

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

Status: **Partial, registry recovery/integrity hardening integrated**.

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

Status: **Partial, persistence hardening integrated**.

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

## H. Git workflows

Status: **Partial, literal unstage hardening integrated**.

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

Status: **Foundation integrated; native browser embedding remains open**.

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

Status: **Open**. Ownership: Rust orchestration and a narrow Apple-native helper where required.

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

1. Complete the Hub transition and integration without duplicating runtime owners:
   finish Settings labels, Hub task-board depth, Library intake/provenance and explicit
   project context. Preserve old Studio data until migration acceptance is recorded.
2. Continue chat depth: native attachment acceptance, richer mentions, supported
   automatic queue/steer, edit/resend and Side chats. Never infer provider
   capabilities from their names or silently send imported/shared context.
3. Deliver Pull Requests and Automations with actual backend authorization, scoped
   state, cancellation and durable history. Continue Plugins/Skills/MCP, then the
   real Browser host and platform tools, rather than adding visual-only controls.
4. Keep the Synara identity, flat row-based Hubs and a restrained Zen layout. Record
   native full/narrow, menu, approval and tool captures when a native build is
   available. Run focused local checks while developing, not repetitive broad CI.
5. Before final delivery, run the applicable Q/platform gates on the final candidate,
   synchronize documentation, verify protected refs and publish coherent checkpoints.
