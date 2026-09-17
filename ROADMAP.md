# Synara delivery roadmap

This is the implementation and verification backlog for the native Rust/GPUI
Synara application. It covers the whole agreed product, not just the next demo.
It is a living engineering document, not a release announcement.

## Current checkpoint

The roadmap and README navigation are published. The roadmap structure validator
has passed its nine regression tests and validates 17 workstreams with 97 tasks.
This is a coverage/navigation check, not a product-completion score.

SSH transport advanced in `500d6622345366e8e41c152312274e4b10cbf033`.
[SSH verification run 35243900061](https://github.com/cmdr-chara/synara/actions/runs/35243900061)
passed six runtime integration tests and one ACP integration test covering two
fixture-agent profiles. The tests use a real OpenSSH server restricted to loopback
with disposable identity and host keys. They prove literal cwd/argument handling,
binary-transparent stdio, separate stderr, remote exit reporting, rejection of
unknown/changed host keys and wrong identities, and effective forwarding policy.
They do not prove a remote desktop workflow, remote descendant cleanup or a real
vendor service. After formatting corrections, candidate
`98956cf2f9185592985e0ac3a4051009362ee415` passed static, SSH and full Linux native
verification, including desktop interaction smoke, in
[run 35245383218](https://github.com/cmdr-chara/synara/actions/runs/35245383218).

Real vendor interoperability advanced in
`a02d6b16468782e7b9d7be9158d3afc80d5e26c1`.
[Vendor probe run 35246447427](https://github.com/cmdr-chara/synara/actions/runs/35246447427)
installed checksum-pinned official OpenCode 1.18.31 and Gemini CLI 0.60.0 releases
through Synara's registry service and initialized both through the same ACP
backend. OpenCode created a session. Gemini returned an explicit authentication
requirement. Both disconnected successfully. No credentials, authentication calls
or prompts were used. This closes C1/C2 for the tested Linux OpenCode release,
not C3/C4 authenticated workflows. [Compatibility evidence](docs/agent-compatibility.md)
records versions, scope, results and remaining limitations. The first vendor
candidate's separate formatting failure was identified for correction. Each
subsequent candidate still needs its own applicable checks.

The direct terminal-grid work is still pending recovery. Local execution returned
transport timeouts, so this checkpoint deliberately changed independent SSH files
and did not replace the pending terminal implementation.

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
The branch contains these integrated development foundations:

| Area | Existing foundation | Remaining qualification |
| --- | --- | --- |
| Native desktop | GPUI shell, tabs, composer and platform text input | Accessibility, sustained-use testing and other platforms remain open |
| Agent host | Generic ACP transport, negotiation, sessions and connection ownership | Live vendor interoperability and complete optional-capability coverage remain open |
| Conversation | Normalized events, streaming, tools, permission and question surfaces | Long transcripts, follow/queue behavior and edge-case UX need further proof |
| Workspaces | SQLite catalog, tasks, durable event delivery and startup restoration | Broader multi-project workflows, backup/recovery and remote UI remain open |
| Files and Git | Bounded UTF-8 editing, guarded saves, close confirmation, status/diff/staging/commit | Richer editor and Git operations remain open |
| Registry | Catalog UI, explicit review, pinned package launchers and checked binary installation | Real distribution/platform matrix and update lifecycle need further proof |
| Terminal/runtime | PTY shell, bounded history and process/execution-host services | Direct terminal grid and foreground-job cleanup are pending recovery/acceptance |
| Verification | Linux build, lints, fixtures and native Xvfb interaction smoke | No general macOS, Windows, Wayland, SSH or live-provider acceptance claim |

Baseline CI: [Native verification run 35237792208](https://github.com/cmdr-chara/synara/actions/runs/35237792208).
The workflow's source job may create an integration commit. Correlate the actual
checked-out revision, not only the workflow trigger SHA, before reusing evidence.
The table describes that baseline. New SSH and vendor protocol evidence is recorded
in the current checkpoint and its lane below, without rewriting earlier results.

The previous local handoff reported direct terminal input/grid rendering, PTY
resize, scrollback, reviewed paste and foreground-job cleanup under
`/mnt/data/synara-native`. That path is a temporary execution workspace, not a
portable project dependency. Recovery and comparison with remote are required
before this work can be accepted. The latest recovery attempt encountered local
execution transport timeouts. The remote branch remains accessible.

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

## A. Recover and complete the native terminal

Status: **Partial / pending recovery**. Ownership: `synara-runtime`, `synara-app`.

- [ ] A1 Recover the previous local diff without resetting, cleaning or replacing
  uncommitted work. Compare every file with the published baseline. Preserve a
  recoverable checkpoint before resolving overlaps.
- [ ] A2 Integrate direct keyboard input, native cell rendering, ANSI colors,
  Unicode/wide characters, cursor, title and working-directory updates.
- [ ] A3 Exercise interactive shells, control keys, application cursor modes,
  alternate screen, terminal replies, resize, focus, selection/copy and IME.
- [ ] A4 Complete bounded scrollback and user-owned scroll position. Retain final
  output after process exit without retaining dead PTY resources.
- [ ] A5 Require deliberate review for dangerous pasted control sequences or
  multiline commands. Support bracketed paste without silent execution.
- [ ] A6 Prove interrupt, stop, close and restart clean up the shell and foreground
  job/process group. Cover exit-before-stop, concurrent stop and reader teardown.
- [ ] A7 Run full desktop regression and focused terminal tests, then publish the
  coherent terminal checkpoint. Extend CI smoke to the accepted behavior.

Acceptance: a real PTY shell accepts input in the native grid, resizes and renders
correctly, preserves history, never pastes an unreviewed command into execution,
and leaves no owned foreground process after shutdown. Linux proof does not close
Windows ConPTY or macOS acceptance.

## B. Complete generic agent and connection lifecycle

BCD checkpoint in progress on `astra/session-bcd`: request-scoped cancellation
now expires login forms with their parent RPC, and terminal transport failures
retain their first diagnostic. New deterministic request/interaction tests are
being verified. B2/B5 and D5/D6 remain partial until the candidate-specific
checks and the remaining acceptance matrix pass. See
[the BCD handoff](docs/bcd-session-handoff.md) for scope and open gates.

Status: **Partial**. Ownership: `synara-agent`, `synara-acp`, `synara-runtime`.

- [ ] B1 Audit ACP method/capability coverage against current primary protocol and
  pinned Rust SDK documentation. Keep stable/unstable behavior explicit.
- [ ] B2 Complete and test connection start, initialize, authenticate, connect,
  fail, exit, restart and disconnect transitions, including crash during requests.
- [ ] B3 Complete capability-driven new/load/resume/close/list/delete session
  behavior where supported, extra directories, titles and concurrent loading.
- [ ] B4 Prove a connection owns multiple sessions without duplicate ownership,
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

Acceptance: one adapter passes protocol, lifecycle and concurrency tests for
multiple agent configurations. ACP dependency upgrades stay inside the adapter
boundary rather than changing unrelated domain or UI types.

BCD isolated continuation: first interaction candidate
`a3693807deb7531959a14051c249a71f311e310e` passed full Linux checks and native smoke
in [run 35255541692](https://github.com/cmdr-chara/synara/actions/runs/35255541692).
The next B2/B3/B4 slice adds explicit authentication-required state, prevents late
login/setup resurrection, rejects duplicate task ownership, serializes shutdown
against connection acquisition and tests two-session cancellation/permission
isolation. Focused local Linux tests and Clippy passed. Candidate-wide verification
remains required before closing any complete task. See the
[BCD handoff](docs/bcd-session-handoff.md) for evidence and the terminal-lifetime
request reserved for A/J/M.

## C. Prove real-agent interoperability

Status: **Partial**, reviewed-release installation and no-credentials proof passed.
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
- [ ] C5 Repeat with a user-defined command/args/environment profile without source
  changes, including paths and arguments containing spaces and Unicode.
- [ ] C6 Record executable/version, OS, supported methods, unsupported features,
  results and redacted diagnostics. Do not fabricate provider/model availability.

Acceptance: a compatibility matrix distinguishes fixture, no-credential protocol
and authenticated end-to-end results. Missing credentials leave C3/C4 open but do
not prevent transport, registry, UI or other independent work.

## D. Conversation, permissions and structured questions

Status: **Partial**. Ownership: `synara-core`, `synara-agent`, `synara-app`.

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

Acceptance: native UI tests cover both positive and rejected interactions, stale
requests, concurrent requests, keyboard operation and restored conversations.

## E. Registry, installation and custom agents

Status: **Partial**. Ownership: `synara-registry`, `synara-workspace`, `synara-app`.

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

Acceptance: fixture and real-distribution tests demonstrate consent, integrity,
recoverable failure and no automatic launch merely from browsing/importing.

## F. Workspaces, projects, tasks and persistence

Status: **Partial**. Ownership: `synara-core`, `synara-workspace`, `synara-app`.

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

Acceptance: edit/save/reopen and conflict-recovery journeys preserve content and
formatting, and oversized/untrusted inputs do not freeze or escape the workspace.

## H. Git workflows

Status: **Partial**. Ownership: Rust system-Git service and Changes UI.

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

Acceptance: secret-canary tests cover storage, logs, export and screenshots.
Platform-native UX and credential behavior need real per-platform tests.

## J. SSH and remote development

Status: **Partial**, pinned transport and controlled-server proof integrated.
Ownership: execution host, workspace services and native remote UI.

The current checkpoint proves selected identity/trust files, no silent host-key
enrollment, argument/cwd quoting, separated streams and the same ACP adapter over
real SSH for two fixture configurations. J1/J2/J7 remain open for their remaining
UI, remote-ownership and full-workspace requirements. Remote filesystem callbacks
must stay unavailable until they can enforce remote containment.

- [ ] J1 Complete host profiles, identity/authentication, known-host verification,
  explicit host-key enrollment and actionable changed-key refusal.
- [ ] J2 Prove remote cwd/environment/executable quoting, agent stdio transport and
  remote process ownership through the same backend as local execution.
- [ ] J3 Implement remote filesystem read/write/explorer and guarded saves with
  remote containment. Never fall back to local paths on transport failure.
- [ ] J4 Add remote PTY input/output/resize, Git and local/remote task association.
- [ ] J5 Implement reconnect, network loss, timeouts, session ownership and cleanup.
  Killing a local SSH client is not proof of remote process cleanup.
- [ ] J6 Add explicit port forwarding, remote development-server discovery and
  consented browser connection with loopback/security defaults.
- [ ] J7 Test against a controlled SSH server with pinned host keys, then exercise
  the native remote-workspace journey. Include failure and hostile-input cases.

Acceptance: remote files, agent, terminal and Git operate on the selected host,
unknown/changed hosts fail closed, and disconnects preserve data and ownership.

## K. Browser host

Status: **Open**. Ownership: narrow native browser-host boundary and GPUI shell.

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

Acceptance: a real supported simulator/device can be displayed and controlled,
with reliable disconnect/cleanup and no undocumented privilege escalation.

## M. Application runtime and service boundaries

Status: **Partial**. Ownership: `synara-runtime` and platform adapters.

- [ ] M1 Complete POSIX process groups, Windows process trees/Job Objects and
  PTY/ConPTY lifetime ownership, including spawn and partial-start failures.
- [ ] M2 Audit executable/environment resolution, shell quoting and WSL behavior
  where offered. Keep arguments structured until the actual shell boundary.
- [ ] M3 Define bounded HTTP/WebSocket/RPC/IPC services only where a product lane
  needs them, with timeouts, cancellation, origin/authentication and error policy.
- [ ] M4 Ensure agents, terminals and browser/device helpers do not become orphaned
  after task close, restart, connection failure or application shutdown.

Acceptance: lifecycle and resource tests measure owned processes/readers/handles
before and after repeated cycles, with platform-specific evidence.

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

Status: **Open**. Ownership: native UI, services and QA.

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

Status: **Open**, Linux development proof already exists.

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

Acceptance: each advertised target has its own working artifact and interaction
results. CI cross-compilation alone does not qualify a target as supported.

## Q. Final integration, documentation and delivery

Status: **Open**, applied at every checkpoint.

- [ ] Q1 Run the applicable verification commands below against the exact final
  candidate, not only an earlier green commit.
- [ ] Q2 Map every implemented UI feature to tests and every remaining requested
  feature to an open task here. Remove dead scaffolding and stale claims.
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
| Editor/explorer, diffs and system Git workflows | G, H |
| Native terminal, PTY/ConPTY and process supervision | A, M |
| SSH, remote files/agents/PTY/Git and forwarding | J |
| Browser, OAuth, downloads and automation | K, I, N |
| Device/simulator presentation and native IPC | L, M |
| Credentials, updater, packaging and owner decisions | I, P |
| Security, performance, platform acceptance and documentation | N, O, P, Q |

## Immediate execution queue

1. Finish final-candidate native, static, SSH and vendor-probe verification. Keep
   the published roadmap and its README link current with exact evidence.
2. Recover and finish A1-A7. If local execution remains unavailable, preserve that
   gate and advance independent, non-overlapping work using the remote branch and
   CI rather than overwriting the pending terminal implementation.
3. Continue J1-J7 remote workspace boundaries and the local application workflow.
   C1/C2 now have real OpenCode release evidence. C3/C4 need separately authorized
   provider credentials, while fixture coverage and no-credentials work can proceed.
4. Update this roadmap with actual evidence at the next published checkpoint.
