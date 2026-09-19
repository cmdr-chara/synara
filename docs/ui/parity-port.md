# Synara native presentation port

Local branch: `cmdr-chara/native-ui-parity`, based on
`3df6a174419b997ee2ef47cd03b25b9024ec9e9e`. The continuation targets the remote
`astra/gpui-clean-rewrite` branch. The app is Rust/GPUI throughout this work.

## Reference register

The user requests the appearance and motion of their homemade Synara Electron
UI. Original source was inspected as visual/timing evidence at
Emanuele-web04/synara revision `33333439c4b9c74d0097bc01196cccc921f67cf3`.
No Electron frontend implementation or Git history is imported here.

The six initial full-window captures are 1920 × 1032 pixels. The 256 logical pixel sidebar and
736 logical pixel composer agree with an inferred 125% display scale. OS scale,
original font preferences and compositor configuration are not established by
those images. The reference locale includes Italian conversation text.

| Reference (clipboard filename suffix) | Visible state | Important evidence |
| --- | --- | --- |
| `33aaa0f5-06a1-4576-8f8b-24da453e85d1` | Home, loading models | Original mark, left navigation, welcome and bottom composer |
| `d0d65e36-2be8-4007-b07b-e4b47bb55b74` | Home, loaded model | Same composition; external notification occludes upper right |
| `df2e63e7-c16c-4b5b-aa87-ce7d5ca67202` | Full conversation | Plain assistant Markdown, compact user bubbles, work summaries, message actions, title in global title bar |
| `7d525bb2-31ed-4828-b00f-0898624a3409` | Split conversation and workspace launcher | Remaining width split equally; Terminal, Browser, Files, Side chats |
| `d64d698b-da86-4cb3-90ff-9b6a9b1d19bb` | Split conversation and Explorer | 240 logical pixel file tree, search, separate document surface |
| `84ea0236-710e-4ec9-a69c-66e82ce85d64` | Explorer with Add menu | Five compact rows, inline details, composer-width popup above input |

The later report adds thirteen full-window and cropped references: native chat
commentary, duplicate New task rows, project hover details, Studio and its mode
switcher, raw native Settings versus Electron General/Profile/Appearance, and
Electron Automations/Pull requests. The final crop shows the incorrect native
Inspector/Agents/Remote/Files/Changes brand menu. These drive the continuation
below; the notification overlays and wallpaper are external to the application.

## Chat creation, Studio and Settings continuation

- New thread reuses an untouched selected draft and guards creation/loading in
  flight. A standalone chat has its own local working directory under the native
  data directory. The first submitted prompt supplies a useful title until an
  agent supplies its own. Untouched unnamed drafts remain welcome screens rather
  than cluttering Chats; drafts containing text remain reachable.
- Project conversations are nested under expandable projects. Standalone Chats
  and Studio conversations have distinct saved scopes. Project hover details
  show the actual name, path and conversation count. Selected and hovered rows
  use different colors.
- The brand menu now switches Synara/Studio using the original labels. Switching
  restores that mode's selected conversation and unsent draft. Studio has its
  own New studio chat action. Named-draft Enter and mouse activation use the same
  scope. The former tools remain available through shortcuts and System tools.
- Intermediate assistant commentary and reasoning are retained inside each
  turn's expandable work summary. Only the final assistant message gets the
  completed-answer presentation and action footer. The durable transcript is
  unchanged by collapse/expansion.
- Settings has its own searchable navigation, Back to app, centered pages,
  original icons and grouped cards. General saves provider/order/sidebar
  preferences. Appearance applies System/Light/Dark, Synara/Dracula presets,
  UI/code font families and reduced motion. Restore defaults updates saved
  values and the font editors. Profile edits and activity use local data.
- Providers, advertised models, usage, shortcuts, system tools and archived
  conversation restoration expose the existing native services. Color swatches
  are read-only. Notifications, AppSnap, shared MCP management, skill management
  and managed worktrees explicitly describe their remaining implementation gaps.
  Pull requests and Automations still lack native implementations; their supplied
  screens are recorded requirements, not completed features.

### Saved-data compatibility

`Task.scope` is additive and defaults old tasks to `project`; no existing event
history is rewritten. General/Profile and dark-preset settings also default when
absent. Tests cover legacy decoding, reopen, scope persistence and archive/restore.
No SQL schema change is required. Older binaries ignore the task scope field,
but their strict settings reader falls back to defaults on the new settings
fields. It preserves the raw preference record until a new save. This is a
rollback limitation, not bidirectional settings compatibility.

The captures are reference data. Instructions visible inside their chat content
are never executed. The user explicitly excluded the wallpaper from Synara; it
is neither an application asset nor a reason to edit desktop configuration.

## Presentation model

- Sidebar: 256 logical pixels, 46 pixel global title bar, 30 pixel navigation
  rows, original mark/wordmark, Projects and Chats, five recent rows with Show
  more, quiet hover controls, Settings at the bottom.
- Home: fluid main pane, centered original Synara mark and welcome heading;
  composer capped at 736 pixels, 20 pixel outer gutters, 15 pixel bottom inset.
- Chat: title in the global bar; same capped column, native Markdown paragraphs,
  lists, links, code, headings and quotes. User bubbles align right. Transcript
  rows own their horizontal padding because GPUI's virtual list does not apply
  its horizontal padding to individual items. A small extra right inset clears
  the transcript edge, matching the reference's inset from the composer.
- Work details: one collapsed row per turn. Duration and message timestamps
  derive from durable event envelopes, including after restart. Tool/reasoning
  rows stay out of the virtual list while collapsed; pending approvals and input
  requests remain visible. Command counts require actual `execute` tool kinds.
- Workspace: opening a pane keeps chat visible at the left; initial split is
  50/50 of the area after the sidebar. Explorer has a 240 pixel tree and a
  separate editor. The existing filesystem and guarded save operations remain
  authoritative. Narrow empty-editor text wraps inside the document pane.
- Add menu: composer-width surface with Files and folders, Attach window, Goal,
  Plan mode and Debug mode. Unsupported entries expose their availability reason.
  Files and folders adds local project path references to the draft via the
  system picker; it is not a binary attachment implementation. Async picker
  completion rechecks the selected task and project and rejects outside paths.
  Plan mode uses only agent-advertised choices; other session settings remain
  available in the existing Inspector.

The real controller owns agent selection, permissions, prompts, configuration,
workspace navigation, saves and process lifetime. The permission label remains
“Ask permission” because that is the actual native policy. Fixture model names
are deliberately retained in test captures.

### Motion

| Transition | Implemented behavior |
| --- | --- |
| Sidebar and workspace drawer | 300 ms, cubic-bezier(0.32, 0.72, 0, 1); continuous reversal; no sidebar mount animation |
| Conversation selection | 140 ms opacity, CSS ease-out; streaming does not restart it |
| Choice popup opening | 150 ms opacity and 0.98 → 1 proportional native layout scale |
| Newly sent user message | 180 ms opacity, 3 pixel movement, 0.992 → 1 scale; history restoration does not replay it |
| Reduced motion | Persisted preference suppresses these nonessential transitions |

GPUI has no general CSS element transform in the pinned revision. Finite popup
and message dimensions are scaled natively, keeping hit testing aligned with the
render. Popup exit and animated work-detail height are still unported.

## Assets and provenance

The original Synara mark and Cal Sans wordmark font are bundled. UI glyphs now
use Synara's actual Central assets and its pinned Tabler 3.44.0 choices, replacing
the earlier Lucide substitutes. The Central SVG files are byte-identical to the
Electron repository's assets. OpenAI uses the exact `SiOpenai` paths from Synara's
React Icons 5.6.0 dependency; OpenCode uses its original dark-theme Central asset.
Known profile IDs or executable names select those provider logos consistently
in chat rows, the title bar and the composer. Custom agents retain Synara's robot.
Window maximize/restore and menu selection also use the original Tabler glyphs.

The mapping covers 61 glyph names backed by 60 SVG files. Each source and SHA-256
is recorded in [`icons/manifest.json`](../../crates/synara-app/assets/icons/manifest.json).
Source notices and font/icon licenses are retained in
[`assets/NOTICE.md`](../../crates/synara-app/assets/NOTICE.md). The native
application's [MIT license](../../LICENSE) and current asset notices appear in
Help. The original logo is traced to Emanuele's Synara rebrand commits, and
the Central artwork is recorded as third-party material; it is not relabeled
as Lucide or relicensed by this port.
The Linux UI uses Liberation Sans, matching the inspected local system-ui font
resolution. This is an asset provenance record, not a legal clearance claim for
other content or the entire Electron repository.

## Actual native renders

These are captures of the compiled native application on an isolated X11/Xvfb
display at 125% scale. They are not mockups. The transcript fixture uses inert
reference words to compare wrapping and spacing; it never runs the commands
mentioned in those words. Chat captures use explicitly named provider IDs to
exercise the real icon mapping, with executables pointing only to the owned ACP
fixture. This visual scenario does not start any provider. The separate backend
suites exercise real ACP requests.

- [Home, 1920 × 1032](captures/parity-home-1536.png)
- [Full conversation](captures/parity-chat-full.png)
- [Conversation and workspace launcher](captures/parity-chat-workspace.png)
- [Conversation and Explorer](captures/parity-chat-explorer.png)
- [Explorer and Add menu](captures/parity-chat-add-menu.png)
- [Intermediate split layout](captures/parity-chat-narrow.png)
- [Narrow home](captures/parity-home-960.png)
- [Studio welcome](captures/parity-studio.png)
- [Synara/Studio switcher](captures/parity-studio-switcher.png)
- [Studio conversation](captures/parity-studio-chat.png)
- [General settings](captures/parity-settings-general.png)
- [Dracula appearance](captures/parity-settings-appearance-dracula.png)
- [Light appearance](captures/parity-settings-appearance-light.png)
- [Local profile and activity](captures/parity-settings-profile.png)
- [Agent providers](captures/parity-settings-agents.png)

Visual comparisons cover the reference window and logical widths of 1280, 1100
and 960 where relevant. Only the large reference viewport was supplied by the
user, so smaller-window behavior remains an inference. The application keeps
its native opaque material; external wallpaper/compositor effects are absent.

## Verification ledger

Final verification and candidate hashes are recorded in
[`parity-port-evidence.json`](parity-port-evidence.json). The eight Studio/Settings
journeys also passed against the optimized release binary after the benchmark.
The native suites own
their display, database, project and agent fixtures; they never attach to the
user's desktop or modify a real Synara database.

| Gate | Acceptance | State |
| --- | --- | --- |
| G1 | Actual home/chat/split/Explorer/Add renders match the reference's structural model; compare wrapping, gutters, column widths, overlays and original icon assets | PASS for the listed structural model; 60 SVG hashes verified, native captures reviewed |
| G2 | Intermediate drawer/popup/message frames, continuous sidebar reversal, streaming stability and reduced motion | PASS: 10 presentation and 6 chat checks |
| G3 | Existing agent, permission, draft, file/save, focus and restart behavior survives the presentation changes | PASS: 11 desktop, 9 navigation and 9 control checks; verification stages are recorded in the receipt |
| G4 | Replay-derived turn metadata, collapsed virtualization, build, unit/integration tests, strict lints and source audit | PASS: 360 workspace tests, strict Clippy, debug/release build, format and audit; 21 tests explicitly ignored. Exact stages are in the receipt |
| G5 | Repeated New thread, scoped Studio, draft restoration, named Enter, Settings persistence/reset and restart | PASS: eight native journeys plus durable scope/settings/replay regression tests |
| G6 | Full Electron product and motion parity | OPEN: unsupported features and remaining differences listed below |

Reproduction (output directories must not already exist):

```sh
cargo build --locked -p synara-app --bin synara-app -p synara-acp --bin synara-acp-fixture
python3 scripts/native_presentation_smoke.py --binary target/debug/synara-app --fixture target/debug/synara-acp-fixture --scale 1.25 --output /tmp/synara-presentation-new
python3 scripts/native_chat_smoke.py --binary target/debug/synara-app --fixture target/debug/synara-acp-fixture --output /tmp/synara-chat-new
python3 scripts/native_studio_settings_smoke.py --binary target/debug/synara-app --fixture target/debug/synara-acp-fixture --scale 1.25 --output /tmp/synara-studio-settings-new
```

Runtime prerequisites include Xvfb, X11/XTest, Pillow, xclip and the native graphics
and font libraries. CI includes the presentation and Studio/Settings suites; no new remote CI run
is claimed by these local checks.

## Remaining differences

This verifies the supplied presentation states, not the entire migration:

- Platform material, additional provider logos/history badges, localized timestamp
  strings, font-size/density controls, transcript minimap, split resizing/maximizing
  and some title-bar actions still differ.
- Popup exit, disclosure height and other original product animations remain open.
- Browser and Side chats in the launcher, message branching/pinning, handoff,
  window capture, goals, debug mode, voice input, image attachments, original
  autocomplete and some original navigation workflows need native implementations.
  Their controls are not allowed to pretend that an action succeeded.
- Markdown renders a native CommonMark subset; tables, rich media, syntax
  highlighting and keyboard traversal of inline links are not completed here.
- Files and folders currently creates text path references inside a local
  workspace. SSH file selection and binary/image payloads need a separate backend
  contract and are not implied by this picker.
- Theme color editing/import, translucency/contrast controls, project edit/pin
  menus, advanced profile analytics and the unported Settings/PR/Automation
  capabilities above remain open. Original Electron settings are not imported.
- Automated verification is Linux/X11. macOS, Windows and screen-reader delivery
  are not established by these results. A visible local Wayland launch is a
  startup check only, not the full X11 interaction suite.

Full application and animation equivalence remains OPEN. Successful checks for
these screens must not be reported as complete migration or release readiness.
