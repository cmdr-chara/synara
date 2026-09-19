# Native UI continuation

## September 19 local presentation work

The working branch `cmdr-chara/native-ui-parity` starts at `3df6a174` and adds the
source-referenced home and transcript geometry, native Markdown, turn summaries,
split workspace/Explorer, composer Add menu, Synara's original icon assets,
project picker, search, navigation history and native motion.
See [`parity-port.md`](parity-port.md) for actual native captures and remaining
differences, and [`parity-port-evidence.json`](parity-port-evidence.json) for
measured checks and candidate hashes. This continuation targets
`astra/gpui-clean-rewrite`; the verification was recorded before publication.
The older published checkpoint below is historical evidence.

The latest continuation fixes repeated empty drafts, introduces persisted
Project/Chat/Studio scopes, restores the Synara/Studio switcher and separates
intermediate commentary from final answers. Settings now has the original
navigation and usable General, Appearance, Profile and native-service pages.
Eight new native journeys cover creation, Studio, saved preferences, reset and
restart. Full parity remains open; the exact unavailable pages and compatibility
limits are listed in the presentation document. The older published checkpoint
below does not describe this continuation.

## Published checkpoint

Native navigation source is now published on `astra/gpui-clean-rewrite`:

- Commit: `495bbfd13dec8d6f75419cf53092139446a77273`.
- Published tree: `d5c68d5dc91f47ae8471078cc9d6401e65850cfe`.
- Verified source tree before adding the evidence receipt and captures:
  `0f313d0a5907bd7fdd786f85bcd6e865a66b73dc`.
- Candidate verification: [run 35440664654](https://github.com/cmdr-chara/synara/actions/runs/35440664654),
  job `105890763602`, September 19, 2026, Ubuntu 24.04 / Rust 1.98.1 / private X11/Xvfb.
- Result: all ten recorded Rust/static/build commands passed. Both native suites
  passed: eleven existing desktop checks and eight navigation checks.

The authenticated connector constructed the source tree from checked Git blobs,
retained the executable mode of `scripts/native_smoke.py`, confirmed exact tree
identity and performed a non-forced branch update. This is actual UI publication,
not merely a preparation workflow, unreferenced commit or local patch.

The three temporary candidate/recovery files were removed in the published tree:
`.github/workflows/ui-checkpoint.yml`, `.github/workflows/ui-recovery-audit.yml`
and `scripts/prepare_native_navigation.py`. Regular `native.yml` now retains the
navigation regression and its screenshot/result artifact. The full receipt is
[`native-navigation-evidence.json`](native-navigation-evidence.json).

The publication-triggered native run `35440897399` was pending behind the existing
workflow concurrency group at the time of this record. It is not counted as a
pass. The source evidence above comes from the completed candidate run and exact
published-tree comparison. Static run `35440897421` passed on `495bbfd`.

## Product and backend boundary

Emanuele's Synara, including the supplied home-screen capture, is the product and
visual specification. This checkpoint is a partial navigation/design foundation,
not a completed vertical slice or visual-parity claim. See the explicit inventory
in [`native-navigation.md`](native-navigation.md).

The sidebar creates/selects real backend tasks, preserves in-session drafts and
uses existing workspace services. Menus reach the existing inspector, registry
and remote panels. Registry approval remains consented and does not start an
agent. Tests exercise real native input, SQLite persistence and ACP fixture
processes. No production backend API or protocol implementation was replaced.

## Failures diagnosed rather than bypassed

1. Focusable transient controls exposed an actual native focus bug. After registry
   approval removed the focused button, its retired focus ID had no shell dispatch
   path. Panel shortcuts stopped working. Tests consequently typed into the wrong
   surface and appeared to show draft loss or an unsafe close. A retained,
   event-driven root focus-out observer now restores only retired/empty focus,
   respecting deliberate moves and close dialogs. Both affected journeys pass.
2. Keyboard activation must use GPUI's native press/release click path. An extra
   keydown handler could invoke an operation before release or more than once.
   One callback now serves mouse, keyboard and assistive click activation. A
   held-key test proves zero tasks before release and exactly one afterward.
3. The new multi-thread test initially sliced globally sequence-sorted events by
   old row count. SQLite sequences are per thread. That oracle could see a
   sibling's old completion while missing a new low-sequence user event. The
   test now correlates task identity, thread ID and sequence, verifies the exact
   restored prompt, and proves the sibling sequence is unchanged.

Native smoke input preconditions now inspect the actual isolated clipboard:
loaded editor content, typed edit and restored composer draft must match before
subsequent actions. These are behavioral checks, not synthetic database writes.
Optional `synara_ui_layout=debug` metadata reports geometry and focus state only,
not typed text, key sequences, paths, prompts or credentials. It is not enabled
by the normal application log default.

## Earlier local UI remains unrecovered

The previous worktree `/mnt/data/ui-work/native-integrated` was reported as locally
tested but unpublished. Container and both Python execution paths returned
`TransportTimeoutError`. Do not reset, clean or replace that worktree. If access
returns, archive tracked changes and untracked source first, then reconcile it
with the published navigation implementation. Old screenshots and reported
local tests do not prove the current branch.

A separate early source envelope at native commit
`6bdd1614e60fb598f9e75484a34e86f69ae58335`, blob
`1a9f70e44a8b9cb7aadb150ee2ab5da936c0f840`, was inspected without applying or
executing it. Strict Base64 failed. Canonicalizing trailing padding exposed a
GZIP CRC mismatch. No source from that envelope was accepted or integrated.
Recovery audit run `35440165626` records the failure. That envelope is not
established as a backup of the later local UI work.

## Remaining gates and next work

| Gate | Evidence/state |
| --- | --- |
| Publish backend-connected navigation foundation | PASS for the bounded implementation, `495bbfd` and candidate receipt |
| Existing native desktop regressions | PASS, eleven checks on the exact source |
| Navigation/focus/draft/resize regressions | PASS, eight checks on the exact source |
| Native render comparison against Emanuele's UI | GAP: captures retained, image-inspection paths failed |
| Earlier local UI recovery | BLOCKED: execution environment access; archive candidate also failed integrity |
| Full conversation/composer/model-menu overhaul | OPEN, not part of this navigation checkpoint |
| Complete appearance/transparency/sidebar/context behavior | OPEN; see parity inventory |
| macOS, Windows and Wayland UI interaction | OPEN, not established by Linux tests or compilation |
| Full release/provenance acceptance | OPEN for the complete product, not implied by this checkpoint |

Next: inspect the retained native captures against `reference-register.md` and the
user's home screenshot, recover the previous local work when possible, then
complete the central conversation/composer/provider-control slice using the
published primitives. Keep actual backend capabilities authoritative and add
focused native regressions for each new interaction. Do not extend the temporary
source-transfer workflow pattern now that source is published.

## Branch discipline

Only `astra/gpui-clean-rewrite` was advanced. No PR, merge, release, default-branch
change or unrelated branch write was performed. The expected parentless root
`43b1fb89bf19dadc388d18008f9ceb21b8215716` passed the workspace audit. Protected
refs were re-read after publication and remained:

- `main`: `657389cc86f345bcb7b11c843670fcdee326e91d`.
- `archive/pre-rewrite-main-2026-09-17`:
  `29b826b8d8e73cc270e311a4c0031b629316b2ec`.

Unreferenced diagnostic objects are not delivery branches or accepted candidates.
In particular, `d72151b09d4530a75f41f874ec7f8f65e4ecc8b6` only retained earlier
failed-candidate screenshots for inspection and must not be promoted as source.

## Recorded performance baseline

The local release build was compared with installed Electron Synara 0.8.4 using
isolated empty profiles, matched Xvfb windows and verified software renderers.
The [2026-09-19 benchmark](../benchmarks/2026-09-19-rust-electron/README.md) records
five warm launches per app, a separate fresh-profile run, whole-process memory
and idle CPU, raw samples, candidate hashes and a reproduction harness. This
measures the two current builds; it does not establish complete product parity.
