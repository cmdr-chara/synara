# Native conversation and session controls

Status: implementation checkpoint under verification, not full visual parity.
Baseline: `1548e3ade5a0ea292b560727af579ae0a5a16ca0`.

## Reference and scope

Emanuele's Synara remains the authoritative product reference. The reference
branch was re-read at `33333439c4b9c74d0097bc01196cccc921f67cf3`. The supplied
1666 x 896 home capture and the pinned 1280 x 803 split-view capture establish a
compact bottom composer, secondary session controls, an open assistant transcript
and a distinct user bubble. The home capture's display scale, exact font and
wallpaper rights are unknown. No image was embedded as an interface or extracted
as an application asset. No frontend source or history was imported.

The inspected published navigation capture still had a full-width blue input,
a permanent provider button row, click-to-cycle options, a large ready-state card
and equally prominent message cards. This checkpoint replaces those central
controls, not the whole product shell. The native backend and safety boundaries
remain unchanged.

## Ownership

| Module | Responsibility |
| --- | --- |
| `ui/menu.rs` | Virtualized choice rows, active-descendant semantics, one focus owner, bounded rendered list, press/release keyboard activation |
| `shell/controls.rs` | Advertised choice projection, transient popup/context state, stale-choice validation and asynchronous controller commands |
| `shell/composer.rs` | Capped input surface, project context, welcome state and send/stop presentation |
| `shell/messages.rs` | Right-aligned user bubble, open assistant/reasoning text and native copy action |
| Existing `input.rs` | Original text buffer, native IME, clipboard, selection and undo ownership, with composer-only material treatment |
| Existing `transcript.rs` | Original virtual list, durable event projection, follow mode and user-owned scroll anchor |

The model/mode/configuration selection is never applied optimistically. Visible
values come from the normalized durable thread configuration. Advertised config
categories supersede legacy selectors, including an empty advertised category.
The coding-agent selection uses persisted profiles and `Controller::switch_agent`.
Choosing a profile does not connect or submit. Draft contents are not replaced.

Every selection is checked against the current task, agent, connection, session
and currently advertised choices. Busy, connecting and in-flight configuration
operations block another selector operation or prompt submission for that task.
Completion/error responses are task-scoped. No transport, database, credential,
filesystem, registry, Git, SSH or process-ownership implementation was replaced.

Menus are anchored to measured native trigger bounds and fitted to the window.
They use a virtual list rather than creating a focus entity for every advertised
model. Escape and click-away restore trigger focus. Arrow/Home/End/Tab navigation
changes the active row. Enter/Space commit on matching release, with armed state
cleared by navigation or blur. Pointer and keyboard commands share the same
validated controller dispatch path. Unsupported choices are not fabricated.

Geometry diagnostics are opt-in under `synara_ui_layout=debug`. New records use
static control identifiers and bounds, never prompts, key sequences, credentials,
file paths, interaction IDs or provider configuration values.

## Verification gates

| Gate | Required evidence | Current state |
| --- | --- | --- |
| Rust/static correctness | Locked workspace check, strict Clippy, workspace tests, formatting and structural/roadmap/publisher checks | PASS locally, exact source hashes in session-controls-local.json |
| Explicit backend controls | Native profile/model/mode/boolean changes checked against SQLite events and fixture responses | Native CI required |
| Focus, drafts and lifecycle | Native press/release, Escape, click-away, busy/stop and restart checks | Native CI required |
| Existing journeys | Existing desktop and navigation suites, including permissions and guarded saves | Native CI required |
| Visual iteration | Inspect real native captures at reference and intermediate dimensions | Pending candidate captures |
| Publication and lineage | Exact tree receipt, independent root and only the authorized branch advanced | Pending publication |
| Whole UI mission | Complete parity inventory and remaining product/platform acceptance | OPEN |

The previous baseline's native run `35441081714` was re-read and completed
successfully. It does not prove this checkpoint. The new
`scripts/native_controls_smoke.py` exercises actual native controls, not direct SQL
writes. Existing permission and provider smoke targets now use measured native
control bounds rather than obsolete prototype coordinates. Both previous suites
remain in CI alongside the new suite.

The local compiler is Rust 1.98.1 on Debian 13 with isolated, previously exported
native development inputs. A local native launch reached GPUI but failed surface
creation because this container has no Vulkan ICD. The cached Ubuntu software
driver requires LLVM 20.1, which is absent. No incompatible LLVM substitute or
system configuration change was made. Native GPU/window checks therefore need the
repository's existing isolated Linux CI environment. A build is not counted as a
rendering or interaction pass.

Local Rust/static verification passed all nine commands in
[`session-controls-local.json`](session-controls-local.json), including 348 passing
workspace tests, zero failures and 21 deliberately ignored environment-specific
checks. Ignored tests are not accepted as passing. The receipt includes the hashes
of the source, test and CI files checked. New menu unit tests prove same-key
press/release, exactly-once activation and navigation cancellation. The full native
and visual gates above remain open pending CI, independently of these results.

## Remaining parity gaps

Markdown formatting/selection, collapsible work groups, richer context actions,
attachment and command workflows, complete appearance settings, wallpaper and
transparency, exact reference fonts/brand assets, and mature window chrome remain
open. The sidebar foundation and surrounding files/Git/terminal/registry/remote
surfaces retain their previously documented limitations. Terminal services were
not replaced. Browser/device and other absent backend capabilities are not faked.
Native macOS, Windows, Wayland and full assistive-technology acceptance remain
unverified. No overall fidelity percentage or completed vertical slice is claimed.

The earlier `/mnt/data/ui-work/native-integrated` worktree is absent in this
execution environment. This work started from the verified published source bundle,
not a recovered local rewrite. Preserve and reconcile that old worktree if it
becomes available. See `continuation.md` for its existing recovery boundary.
