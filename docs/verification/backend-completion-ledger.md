# Backend completion sweep

ROADMAP.md remains the authoritative completion ledger. This document records the
initial implementation/verification map and the evidence for coherent backend
slices. It does not replace or weaken any roadmap checkbox. Product UI work is
owned by Emanuele. Q and release finalization are outside this mission.

## Baseline and evidence rules

Baseline: `d468efbc2b59e358015967b7f7a8aa51c708f131`, September 18, 2026.
There are 75 unchecked B-P items at this baseline. Existing source and tests are
not assumed to prove the complete wording of a gate. The baseline Linux native
run is [35349583038](https://github.com/cmdr-chara/synara/actions/runs/35349583038)
and the controlled SSH run is
[35349582971](https://github.com/cmdr-chara/synara/actions/runs/35349582971).
Both passed. Those runs are baseline evidence, not evidence for later changes.

In the map below, test locations include inline Rust tests and the relevant
crate's integration tests. The acceptance gap column identifies what those tests
and the current implementation do not yet establish. OPEN is implementation or
verification work, not an external blocker. Only the explicitly named external
boundary may be BLOCKED, never the unfinished work before it.

## Initial B-P map

Paths beginning with a crate name are under `crates/`.

| Gate | Current implementation and existing test location | Missing implementation or acceptance | External boundary |
| --- | --- | --- | --- |
| B1 | synara-acp schema.rs, wire.rs, protocol fixtures | Complete current/pinned method and capability audit, stable/draft distinctions | None |
| B2 | synara-acp backend.rs, session.rs, lifecycle_integration.rs | Full start/auth/restart/exit/crash transition matrix | None |
| B3 | synara-acp backend.rs, wire.rs, lifecycle fixtures | Full negotiated session operations, directories/titles and concurrent-load evidence | None |
| B5 | synara-acp rpc.rs, rpc_lifecycle_tests.rs, process fixtures | Complete hostile-frame, correlation, request cancellation, aggregate backpressure and teardown matrix | None |
| B6 | synara-acp session.rs, wire.rs and configuration fixtures | Complete config/mode/model/MCP matrix and explicit treatment of draft compaction | None |
| B7 | synara-agent api.rs, synara-acp adapter, audit_workspace.py | Close dependency-alias/target-table/all-Rust-target audit bypasses and test them | None |
| C3 | synara-acp vendor_probe and reviewed vendor distributions | Authenticated prompt/tool/permission/cancel/restore journey | Authorized vendor credentials not supplied |
| C4 | Second-agent initialization/auth-required evidence in compatibility matrix | Second independent authenticated journey, retaining generic backend | Authorized vendor credentials not supplied |
| C6 | docs/agent-compatibility.md and vendor probe diagnostics | Complete method/version/platform/result and unsupported-feature accounting | Authenticated result rows depend on C3/C4 |
| D1 | synara-core model.rs/thread.rs and replay tests, workspace event store | Complete normalized event/replay matrix, with UI rendering tracked separately | Product UI acceptance belongs to Emanuele |
| D2 | Core replay tests, ACP lifecycle fixtures, storage tests | Ordering, duplicate, interruption, overlap and failed-restoration acceptance together | None |
| D3 | synara-app shell/transcript.rs and scroll-state tests | Long-history and responsive-streaming workload evidence, UI journey | Product UI acceptance belongs to Emanuele |
| D4 | Workspace drafts/preferences and agent prompt/cancel state | Durable queue/recovery contracts, explicit supported steering, send-state matrix | Product composer polish excluded |
| D5 | synara-agent interaction/input validation, ACP callback lifecycle tests | Complete explicit consent lifetime/persistence/denial/restart matrix | None |
| D6 | synara-acp elicitation.rs/elicitation_registry.rs and typed-input tests | Full text/choice/multi-choice/URL validation and cancellation matrix | Product forms polish excluded |
| D7 | Typed tool outputs, bounded input validation, browser consent foundation | End-to-end link/action/output trust boundaries and hostile-input tests | None |
| E1 | synara-registry catalog.rs/model.rs and catalog tests | Current schema/origin/platform/offline/failed-refresh acceptance matrix | None |
| E2 | synara-registry install.rs/download.rs/paths.rs and archive tests | Complete extraction/download/install negative and atomicity matrix | None |
| E3 | Registry launcher model and runtime resolution tests | Pinned npm/uv runtime discovery and error evidence | None |
| E4 | Registry review/install APIs, review and recovery tests | Cancellation/remove/update/retained rollback/in-use contracts and UX handoff | Product registry polish excluded |
| E5 | Registry receipt/path/recovery tests | Complete tamper/substitution/interruption/concurrency matrix | None |
| E6 | synara-workspace profiles.rs and custom-profile integration tests | Import/export/edit/delete/defaults and secret-reference completion | Product profile polish excluded |
| E7 | Prior registry/consumer cross-platform evidence | Revalidate installed-profile behavior and unsupported distributions on final backend | Native platform acceptance requires actual target runs |
| F1 | synara-workspace storage.rs/service.rs and catalog tests | Full create/select/rename/archive/delete/recent and retention semantics | Product workspace polish excluded |
| F2 | Storage sessions/preferences/tasks and service restoration tests | Complete working-directory/selection/draft/settings/UI-state persistence matrix | None |
| F3 | Storage event heads/activity projections and migration/concurrent-open tests | Complete durable ordering/replay/rollback/concurrent-writer and memory evidence | None |
| F4 | Atomic SQLite migrations and version rejection tests | Backup/restore, corruption/inaccessibility/disk-full and compatibility acceptance | None |
| F5 | Workspace startup restoration and agent/session references | Complete missing-directory/agent/session recovery, no-autoexecution proof | None |
| F6 | Workspace remote.rs, runtime execution-host and SSH tests | Final identity-isolation and no-local-fallback matrix | None |
| G1 | Runtime filesystem.rs/remote_fs.rs and workspace file services/tests | Explorer create/rename/delete/search/refresh and destructive consent contracts | Product explorer polish excluded |
| G2 | Workspace document services, synara-app input/editor state tests | Multiple-document/dirty/conflict state support, richer editor UI tracked separately | Product editor features/polish belong to Emanuele |
| G3 | Bounded file reads and guarded saves | Complete external-change/deletion/BOM/line-ending/encoding/binary/large-file modes | None |
| G4 | Capability-root filesystem and guarded-save negative tests | Full traversal/symlink/TOCTOU/permission matrix on offered platforms | Native target-specific evidence required |
| G5 | Workspace Git/diff service and diff fixtures | Complete rename/delete/binary/large-diff model and editor-location contract | Product diff presentation excluded |
| G6 | Workspace worker/controller and execution-host boundaries | Prove every scan/read/search/diff path avoids the render thread | None |
| H1 | Workspace tools.rs and literal unstage fixtures | Complete status/diff/stage/unstage/commit unusual-path matrix | None |
| H2 | System-Git execution service | Branch/remotes/fetch/pull/push/worktree/stash, cancellation/progress/consent | None |
| H3 | Existing Git invocation boundary | Explicit hooks/signing/credential policy and no execution on workspace-open tests | None |
| H4 | Git command error propagation and isolated fixtures | Conflict/auth/network/missing-Git/concurrent-index acceptance | None |
| H5 | Host-aware Git execution and controlled SSH tests | Revalidate async execution/output bounds and remote policy reuse | None |
| I1 | Preferences store and settings consumers | Validated/versioned settings, recovery and keybinding/theme/font models | Native settings UI belongs to Emanuele |
| I2 | ACP auth-required/authenticate/logout lifecycle and tests | Complete agent-owned protocol/terminal/browser auth hosting and retry/logout | Authorized real-agent credentials for authenticated acceptance |
| I3 | Platform/runtime boundaries, no complete credential-store implementation | OS credential abstraction, no plaintext fallback, locked/unavailable/redaction tests | Real store acceptance requires target environment |
| I4 | GPUI platform/input foundations and native smoke | Backend dialog/notification/capability boundaries and platform interaction evidence | Product accessibility/menu/focus presentation belongs to Emanuele |
| I5 | Protocol-independent AgentBackend contract | Explicit direct-model-provider separation and regression guard | None |
| J5 | runtime ssh.rs/process ownership, reconnect and SSH integration tests | Network-loss/timeouts/remote-descendant/session cleanup matrix | Controlled SSH fixture, not a user host |
| J6 | Pinned SSH execution-host boundary | Explicit forwarding/discovery/consent/browser-handoff and failure/cleanup tests | None |
| K1 | foundations/browser policy and browser-host architecture document | Actual engine/process/frame/IPC boundary per implemented target | Installed approved engine or native platform libraries |
| K2 | Browser consent/lifecycle policy tests only | Real host tabs/navigation/history/focus/redirect/popup/crash/recovery | Product tab presentation excluded |
| K3 | Browser context policy tests only | Real cookie/session/credential isolation and OAuth/scheme/origin validation | None before engine boundary |
| K4 | One-shot browser grant foundation | Real bounded screenshot/download/upload/input/automation IPC with payload-bound consent | None before engine boundary |
| K5 | Browser policy negative tests, no engine acceptance | Actual helper shutdown, auth isolation, downloads and malicious-content tests | Actual implemented platform/engine runs required |
| L1 | Roadmap/architecture scope, no device runtime | Discovery/lifecycle/failure-state domain model and helper boundary | Device/simulator visibility requires Apple environment |
| L2 | No complete device capture/input implementation | Portable bounded protocol and supported Apple-native capture/input helper | Apple APIs/SDK and supported simulator/device |
| L3 | Generic process foundations | Device helper ownership, stale-device/input/permission/resize/cleanup validation | Apple interaction evidence separate from deterministic fixtures |
| L4 | No real device/simulator acceptance | Actual supported Apple OS/simulator interaction | Apple environment not yet established, not a mock substitute |
| M1 | POSIX process groups and Linux PTY lifecycle tests | Windows Job Object/ConPTY and macOS ownership/partial-start evidence | Actual native target runs required |
| M2 | Structured LaunchSpec/host quoting and SSH hostile-input tests | Full executable/environment/shell-boundary/WSL audit | WSL acceptance requires WSL when offered |
| M3 | Bounded ACP/SSH transport foundations | Complete needed browser/device/service IPC timeouts/auth/origin/cancellation policy | None |
| M4 | Agent/terminal shutdown and Linux lifecycle tests | Integrate browser/device/remote-helper ownership into app/service shutdown | Native helper evidence on implemented targets |
| N1 | Existing architecture/security notes and boundary tests | Consolidated threat model across all implemented input and execution boundaries | None |
| N2 | Filesystem/registry/SSH/consent negative fixtures | Full integrated audit, including environment, redirects and persistent consent | None |
| N3 | synara-acp trace.rs/rpc.rs and trace tests | Complete bounded method/ID/timing/capability/version/stderr/state inspector model | Polished inspector UI excluded |
| N4 | Trace clear and redacted metadata foundation | Restart action and safe copy/export pipeline with explicit sensitive-content warnings | Polished inspector UI excluded |
| N5 | Locked dependencies and local measurement/privacy foundations | Dependency/license/advisory inventory, diagnostic bundles, rotation/retention/privacy | Owner project licensing decision remains P5 |
| N6 | Existing hostile-input tests across ACP/registry/files/SSH | Close newly implemented boundary denial/exhaustion/recovery gaps | None |
| O1 | scripts/ekop measurement/aggregation tools and tests | Reproducible actual backend workloads with hardware/build identity | Native UI timings must be measured by UI integration, not inferred |
| O2 | Bounded transcript/terminal/storage foundations | Actual streaming/history/flood/concurrency resource measurements | None for synthetic backend workloads |
| O3 | Measurement tools, no accepted comparative optimization evidence | Fix measured backend bottlenecks and repeat identical workload | Depends on measured findings, not an invented speed claim |
| O4 | IME/Unicode/scroll-state foundations and native smoke | Backend reduced-motion/focus/input state support and real accessibility journeys | Platform screen-reader/UI acceptance belongs to product integration |
| P1 | Linux native/SSH CI and historical macOS/Windows compile groundwork | Current native backend test/build lanes plus distinct X11/Wayland evidence | Native target/display environments |
| P2 | Historical native compile and Linux interaction smoke | Real dialogs/fonts/GPU/terminal/credentials/install-path acceptance | Actual target interactions, not compile-only evidence |
| P3 | Locked build inputs and architecture documents | Development packaging, prerequisites, architecture names and dependency notices | Final packaging/OS-minimum policy remains owner decision |
| P4 | No complete update implementation | Consented authenticated/signed download, integrity, interruption, rollback/data compatibility | Production trust keys and authorized endpoint not supplied |
| P5 | No release authorization or production signing policy | Record owner licensing/signing/endpoint/OS-minimum decisions without guessing | Owner decisions and signing identities required |

## Checkpoint evidence

### Protocol dependency boundary and native backend test lanes

Implementation adds alias-aware/workspace-aware/target-aware dependency checks,
checks every Rust target rather than only src, and adds disposable negative
fixtures. The adapter is identified by Cargo package identity, not its directory
name. The checker still reports that it does not perform Rust compilation.

A read-only backend matrix runs on Linux x64, macOS arm64 and Windows x64, verifies
the actual Rust host triple, runs backend check/Clippy/tests and structural checks,
and compiles the application on macOS/Windows. Linux native UI/SSH jobs remain
separate. This does not claim native UI, keychain, browser or device acceptance.

State: OPEN pending results for the published checkpoint. No roadmap checkbox is
closed from the existence of a workflow or an unexecuted test.


### Recoverable SQLite snapshots and platform path identity

Implemented bounded, cancellation-aware online backup and restore-to-new-path APIs,
private staging, no-clobber publication, schema/integrity/foreign-key/identity checks,
read-only older-schema import, rollback tests and asynchronous service entry points.
The same checkpoint fixes the concrete macOS system-directory alias failure without
following database leaves, and retains workspace root aliases through capability
handles. It also corrects the observed Windows fixture path type and lint/format
failures. No product UI changes are included.

Local Linux checks use the recovered exact Rust 1.98.1 toolchain and locked offline
inputs. Focused recovery tests and backend Clippy pass. Publication-specific native
CI and full backend results will be appended after observation. F4 remains OPEN
pending that verification. Other open items retain their inventory above, including
work that is still implementable. They are not relabeled as external blockers.
