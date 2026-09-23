# Electron main to GPUI replication

## Contract and immutable inputs

The requested outcome is a 1:1 native GPUI replication of the synced Electron
product, including observable interactions, not just a similar landing page.
The native Rust/GPUI renderer, generic ACP boundary, separate direct-model
runtime, task ownership, persisted user data and permission boundaries remain
in place. This work does not replace the Rust application with a webview shell.

- Electron source: `Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.
- Synced fork main: `cmdr-chara/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.
- Native starting point: `9b460dd09205d33d8ba20facc155ee4cd7affabf`.
- Implementation branch: `astra/electron-main-parity`.
- Integration target: `astra/gpui-clean-rewrite`, not Electron `main`.
- Existing Electron v0.9.1 and native screenshot inventories are reference
  evidence. A screenshot's presence does not establish that its feature works.

**Status: in progress. Full 1:1 equivalence has not been demonstrated.**

## Scope ledger

| Gate | Required observable outcome | State | Deciding verification |
| --- | --- | --- | --- |
| G1 | Fork main and the Electron source agree at the frozen revision | PASS | Both branch reads return the same commit and tree |
| G2 | Every reference feature and state has an implementation and evidence entry | OPEN | Reconcile the feature-gap audit, source routes and screenshot inventory |
| G3 | Shared geometry, typography, density and semantic materials match | OPEN | Source contracts, native tests and matched rendered comparisons |
| G4 | Shell, projects, spaces, search, history and environment navigation match | OPEN | State-by-state native interaction and screenshot comparison |
| G5 | Composer, model controls, permissions, transcripts and handoff match | OPEN | Real fixture sessions, input/ownership tests and rendered comparisons |
| G6 | Files, editor, diffs, Git, terminals, browser and split views match | OPEN | Native tool journeys, persistence and comparable populated states |
| G7 | Kanban, PRs, automations, goals, debug, checkpoints and orchestration match | OPEN | Per-workflow integration tests and error/recovery states |
| G8 | All primary settings sections and the welcome flow match | OPEN | Every section and control is rendered, exercised and persisted |
| G9 | Existing native data, explicit permissions and runtime ownership survive | OPEN | Accumulated Rust and native fixture regression |
| G10 | Target and intermediate viewport/theme comparisons meet fidelity criteria | OPEN | Same state, content, viewport, scale, theme and scroll position |
| G11 | Final integrated tests, formatting and cleanup are complete | OPEN | Checks against the final published source, not a prior candidate |
| G12 | Reviewed implementation is published for integration | OPEN | Non-force branch update and pull request against the native branch |

The existing 48-capability audit and its remaining acceptance gates are not
silently promoted by presentation changes. Computer Use, inbound external MCP,
Agent Gateway and native subagents require real native workflows. Labels, status
pages, screenshots and stored-but-unused settings do not close those gates.

## Source-derived settings and shared controls

The first implementation slice targets the known shared presentation drift and
stale Settings routing:

- Reuse the reference's 13 px UI, 12 px code and 12 px terminal defaults for new
  profiles. Explicit saved native font choices are not migrated or overwritten.
- Use the reference density factors 0.85, 1.0 and 1.15, including 28 px base rows
  and 10 px base vertical settings-row padding.
- Provide the reference typography roles, with rounding tests across every
  supported Electron base font size. Larger saved native accessibility sizes
  remain usable.
- Remove the extra 1 px type-size offset from shared action controls.
- Restore the 16 primary Settings sections in the reference order, including
  the Computer use Beta entry and Archived threads wording.
- Keep native-only Project import, Direct models, Device/capture, Plugins and
  Privacy reachable from System tools and Settings search. Do not delete their
  implementation or persisted data to make the sidebar look more similar.
- Replace the stale AppSnap-unavailable page with the existing reviewed,
  task-owned native capture workflow. Opening Settings does not capture pixels.
- Replace the stale worktrees-unavailable page with the existing repository
  owner, its real worktree listing and its guarded operation forms. This is a
  selected-repository view, not yet Electron's cross-project managed-worktree
  inventory.
- Keep Computer use explicitly unavailable until an actual permissioned
  desktop-control backend exists. AppSnap consent must never become input
  control authority.

The taxonomy is derived from `apps/web/src/settingsNavigation.ts`. Metrics come
from `apps/web/src/lib/appDensity.ts`, `appTypography.ts`, `appSettings.ts` and
`settingsPanelStyles.ts` at the frozen reference revision. The upstream license
is retained in [electron-reference-LICENSE.txt](electron-reference-LICENSE.txt).

## Verification

The native capture journey is:

```sh
python3 scripts/native_electron_parity_smoke.py \
  --binary target/debug/synara-app \
  --fixture target/debug/synara-acp-fixture \
  --output /tmp/synara-electron-parity
```

It uses a private Xvfb display and fresh fixture profile. It opens every primary
Settings section at 1420x930 and 960x700 in Light and Dark, captures window-only
images, checks fresh per-page rendering probes, preserves an unsent draft and
asserts that browsing Settings produces no agent events. It also checks native
extension access and real worktree/AppSnap routing.

That journey proves its stated navigation, persistence, rendering and ownership
checks only. It does not by itself prove pixel equivalence, authenticated agent
compatibility, Computer Use, macOS, Windows, Wayland or production readiness.

## Execution environment and temporary bootstrap

The conversation's local container and both Python execution paths returned
`caas.internal.errors.ClientError`. Repository reads and writes remain available.
A branch-scoped CI bootstrap therefore applies a reviewed, exact-blob-checked
source delta in a fresh checkout, compiles and tests it, and may publish only the
allowlisted Rust changes to `astra/electron-main-parity` by non-force push.
It cannot write Electron main or the native integration branch.

The bootstrap is temporary development support. Its write-enabled workflow and
one-shot application script must be removed after the generated source has been
published and inspected. Keep the ordinary native tests and capture journey.
Do not call bootstrap preparation or a successful source export a completed port.

## Known differences to resolve

The following remain open until separately implemented and verified:

- Matched same-state screenshots beyond the Settings navigation journey.
- Exact theme materials, font rasterization, menu geometry, responsive reflow,
  control density and panel/header alignment throughout the application.
- The full Electron Chat behavior control set and its runtime effects.
- Cross-project managed-worktree ownership and cleanup presentation.
- Full welcome-tour and first-run parity.
- Native Computer Use, external MCP clients connecting to Synara, Agent Gateway
  and native subagent/workflow ownership.
- The material partial-feature gaps in the existing feature-gap audit, including
  document media, release/update lifecycle, browser/dev-server depth, model
  controls, editor/search and device workflows.

No reference capture is embedded as an interactive screen. No credentials,
private production profile or live external agent account is needed for the
fixture checks. Production and cross-platform claims require their own evidence.
