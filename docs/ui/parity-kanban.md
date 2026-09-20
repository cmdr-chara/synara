# Original Synara UI analysis and native Kanban checkpoint

Date: September 20, 2026. Roadmap: F1/F2/F10, D4 and I10 (bounded progress).
Delivery branch: `cmdr-chara/synara`, `astra/gpui-clean-rewrite`.
Electron reference: `948875954f432978eab7dd5fa44c3028b8d99a81`.

## Accepted source checkpoint

The previously pending integrated-source run is now **PASS for the bounded Linux
Kanban slice**, not full product or cross-platform acceptance.

- Published implementation: `67f276bbc1164d192b2d35998303c669a34bc296`.
- Dialog rendering and keyboard correction: `cb21a722d99ddd914091a98306a137feeac1b19f`.
- Formatted and verified source: `770eae1c4e134b8dd285ecd46b9563fb79736199`.
- Source tree: `0f0e4439ab9e69a0e7cf3d47185318dd67fcb60d`.
- [Focused native run 35505674865](https://github.com/cmdr-chara/synara/actions/runs/35505674865),
  presentation job `106064975562`: PASS, including the no-source-mutation check.
- [Static run 35505674726](https://github.com/cmdr-chara/synara/actions/runs/35505674726): PASS.

The rendering correction was already present when this verification receipt was
refreshed. This documentation update does not claim a second implementation of
that fix or change the tested Rust/Python source.

## Reference inspection is now available

The previously inaccessible archive was recovered. All 73 PNGs were indexed,
hashed and reviewed in seven contact sheets. The task dialog and populated Kanban
reference were also inspected at full resolution. See the
[reference register](reference-9488759-index.json), notably
`48-new-task-draft-ready.png` and `49-kanban-draft-task.png`.

The archive's README identifies an isolated Electron source build, owned demo
project/draft data and a private Xvfb display. It does not establish authenticated
vendor conversations or a genuine pending approval request. Most captures are
1920 by 1032 pixels, with two smaller initial captures. Pixel dimensions are not
assumed to be logical layout units on every platform. Earlier receipts reporting
inaccessible screenshots describe their respective runs, not the current state.

## How the original interface is composed

### Shared surfaces, typography and motion

The original is not a collection of unrelated page designs.
[`index.css`](https://github.com/Emanuele-web04/synara/blob/948875954f432978eab7dd5fa44c3028b8d99a81/apps/web/src/index.css)
maps Tailwind utilities to semantic surface, text, border, status and radius
variables. UI/chat/code typography has separate settings-driven roles. Sidebar
material is shared with badges rather than painted as disconnected opaque tiles.
The outer sidebar seam and lighter internal pane dividers deliberately have
separate strengths. The content edge is straight, not a rounded floating card.

[`RouteInsetSurface`](https://github.com/Emanuele-web04/synara/blob/948875954f432978eab7dd5fa44c3028b8d99a81/apps/web/src/components/RouteInsetSurface.tsx)
reuses the sidebar/inset relationship across routes. Height constraints and scroll
ownership belong to nested regions, so a long board or transcript does not turn
the entire application chrome into a scrolling page. Motion is similarly shared:
sidebar/seam transitions use a common easing curve, while stepped status animation
cadence is coordinated. A still screenshot cannot verify timing or reduced motion.

Native implication: reuse `ui::Palette`, font roles, bundled glyphs, the shared
window toolbar and existing motion helpers. Do not add a second Kanban toolbar or
hard-code a page-sized screenshot as the interface. This checkpoint retains the
existing application themes rather than claiming every theme token now matches.

### Route composition and behavior

[`KanbanView`](https://github.com/Emanuele-web04/synara/blob/948875954f432978eab7dd5fa44c3028b8d99a81/apps/web/src/components/kanban/KanbanView.tsx)
separates route navigation, live board projection, the new-task dialog and the
shared header. Its home is a project overview, not the single-project status
board. The repeat-safe new-task shortcut belongs to Kanban.

[`KanbanOverview`](https://github.com/Emanuele-web04/synara/blob/948875954f432978eab7dd5fa44c3028b8d99a81/apps/web/src/components/kanban/KanbanOverview.tsx)
uses compact 288-pixel project columns, hides empty projects, prioritizes active
work, caps the overview and drills into the project board. Each project then has
Draft, In Progress and Done columns. Cards show conversation identity and state,
not an independent runtime state invented by their location on screen.

[`KanbanNewTaskDialog`](https://github.com/Emanuele-web04/synara/blob/948875954f432978eab7dd5fa44c3028b8d99a81/apps/web/src/components/kanban/KanbanNewTaskDialog.tsx)
composes the existing prompt editor, project/provider controls and draft/run
intent inside a capped, rounded modal. Its text is isolated from the currently
selected chat. The model and approval menus are product controls with underlying
capability/state requirements, not decorations to be enabled without support.

Across the other supplied states, the same dense controls, provider-icon tabs,
neutral surfaces, narrow settings content column and nested Environment tabs
recur. Those references guide later settings, Studio outputs and browser work.
They do not justify claiming those features are implemented by adding their labels.

## Implemented native slice

- A project-column overview with nonempty-project filtering, active-first cards,
  counts, capped lists and drilldown into the three-column project board.
- Shared window-toolbar heading, count, back navigation and New task action,
  including the Kanban-scoped Ctrl/Command+Alt+T shortcut.
- An isolated native task composer with existing-project selection, provider
  selection, draft/run intent, blank-input rejection and duplicate-create guards.
- Task and unsent prompt persisted in one immediate SQLite transaction. Creation
  alone never connects an agent, starts a turn or rewrites conversation history.
- Explicit Run draft and Stop controls through the existing controller. Columns
  follow actual task state, including waiting-for-input and failure labels.
- Request-generation leases for asynchronous draft loading. A canceled load cannot
  start later, consume a newer request or survive accepted application shutdown.
- Unsaved-form discard confirmation and window-close protection, with the prior
  conversation's text and selection preserved during creation.
- Reference-aligned compact cards, modal dimensions, primary action treatment,
  bundled Synara glyphs and a reduced-motion-aware dialog entrance.
- A primary Create button with one hover definition, avoiding the GPUI assertion
  that previously prevented the dialog from rendering.
- Explicit modal and confirmation focus cycles, nondestructive initial
  confirmation focus and focus restoration to the project/provider picker trigger.
- Trigger-relative picker placement constrained to the native window.

Domain operations remain in Rust services. No dependency, database schema,
approval policy, vendor authentication or browser execution boundary was changed.
Legacy chat and Studio creation still use the same validated creation path.

## Verification and failures

The first prepared run (`35501863905`) caught a test-only SQLite error conversion
in the injected rollback case. It was corrected without weakening the rollback
assertion. The next prepared run (`35502375929`, job `106056335958`) passed 4
creation tests, 8 draft-preference tests, 19 UI/state tests, strict Clippy for the
two changed packages, application/fixture build and the existing Studio/settings
and native close suites. It failed the new Kanban harness after creating,
restoring and running a draft: the harness incorrectly expected an entire streamed
answer inside a single delta. The harness now joins assistant chunks by message
identity, with regressions rejecting accidental joins across messages or roles.
That run is not described as green.

The published-source run `35504073637` exposed a separate application defect.
The task dialog added a second `.hover()` handler to a button whose shared helper
already owned one. GPUI panicked with `hover style already set` before the dialog
could paint. The harness's `task-create` geometry timeout was a secondary symptom,
not a reason to increase its timeout or remove the opening check. `cb21a722` gives
the primary action one style owner and retains its contrasting treatment.
`770eae1` applies the pinned formatting to that corrected source.

Local compilation and targeted checks used the repository's pinned Rust 1.98.1,
locked dependency sources and isolated build/sysroot directories. Local native
window creation was unavailable because the software Vulkan runtime was
incomplete. The accepted native interaction evidence below comes from the
Ubuntu/private-Xvfb runner, not the user's desktop.

### Final integrated-source results

Candidate: `770eae1c4e134b8dd285ecd46b9563fb79736199`.
Platform: Ubuntu 24.04 x64, Rust 1.98.1, locked GPUI, private X11/Xvfb,
owned SQLite/project data and ACP fixture agents.

| Check in run 35505674865 | Result |
| --- | --- |
| Candidate identity, verification-scope regressions and roadmap validation | PASS |
| Changed-package formatting | PASS |
| Atomic task creation and chat-preference regressions | PASS |
| Native task, draft and close unit regressions | PASS |
| Selected presentation/input regression step | PASS |
| Strict Clippy for `synara-app` and `synara-workspace` | PASS |
| Native application and ACP fixture build | PASS |
| Native Kanban creation and lifecycle journey | PASS |
| Native creation and Studio restoration journey | PASS |
| Native input and guarded-close journey | PASS |
| Verify no candidate source mutation | PASS |
| Export source identity and reproduction bundle | PASS |

The full native `linux` lane and unrelated broader presentation journeys were
skipped by the focused selector. They are not counted as passing or rerun merely
to refresh this documentation. The successful Kanban journey replaces the earlier
failed opening check as evidence for the corrected source only.

The retained
[focused-native-presentation artifact](https://github.com/cmdr-chara/synara/actions/runs/35505674865/artifacts/10603229842)
contains captures, logs, results and the source reproduction bundle. Artifact ID:
`10603229842`. ZIP SHA-256:
`e6588f6ef3d423217b897a40e3ab1ee9e0aa12341119a084b06c22bb68202ba3`.
GitHub reports expiry on September 27, 2026. These captures are interaction
evidence, not a completed visual comparison against all 73 Electron states.

A temporary preparation script and transient CI source rewriting were removed
from the delivered source. The permanent presentation workflow retains the
previous broader UI journeys. A tested, closed Kanban-only diff selects
creation/storage, board/menu/draft/close checks and the three directly affected
native suites. Mixed menu, settings or input changes retain broader presentation
coverage. Unknown paths still fall back to full native verification.
Documentation-only changes do not rebuild the application.

## Remaining differences and limits

F10 remains open for drag/drop transitions, card context actions, richer draft
editing, follow-up drafts on existing conversations, a unified standalone Chats
overview group and creating tasks directly into a standalone Chat destination.
The modal currently chooses an existing project and an agent's default model.
Preconnection model/effort presets, attachments and broader approval controls are
not faked. Capabilities can be configured later through the existing task flow.

The board uses current application catalog state and refreshes known tasks.
Cross-process catalog discovery and board-view restoration are not complete.
Failed tasks remain explicitly marked Failed even in the terminal-status column.
The full 73-state visual comparison, platform font differences, screen readers,
real IME composition and macOS/Windows/Wayland interaction acceptance stay open.

Browser embedding, Studio output/image views, Spaces, automations, PR workflows,
voice/attachments, additional settings and full approval parity remain separate
roadmap work. None is counted as delivered by this Kanban checkpoint.
