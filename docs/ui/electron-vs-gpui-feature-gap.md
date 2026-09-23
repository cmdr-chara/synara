# Electron Synara to native Rust/GPUI: current feature-gap audit

Previous checkpoint: 2026-09-22, after feature-closure sprint 2 at
`20c78d7164e5e6e9be2b5bc1825be9babfeaa769`. Earlier feature-closure, Sessions 1-4 and maximum-feature
checkpoints remain preserved. This inventory distinguishes user-visible implementation
from Linux fixture evidence and still-open provider, hardware and platform acceptance.

The complete preceding Electron comparison, source links, dated tables and earlier
continuation notes remain in the [preserved pre-Hubs inventory](electron-vs-gpui-feature-gap-before-hubs-2026-09-21.md).
The full acceptance backlog remains in [ROADMAP.md](../../ROADMAP.md). No historical
gap is closed merely because this current view is shorter or a heading was renamed.

## September 23 current-main re-audit

Current upstream was re-read at
`Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`
against native GPUI starting reference
`cmdr-chara/synara@fc0024b9c0dd1ecdbcf41caa844c8269befc9fdf`. The current
classification includes the September 23 continuation documented below.

The old **21 present / 27 partial / 0 missing** result is now historical only.
It used the older 48-capability boundary and treated very different situations as
the same "Partial" status. Current upstream also exposes product surfaces that the
old census did not count cleanly.

### Genuinely missing current-head surfaces (2)

| Surface | Upstream evidence | GPUI evidence |
| --- | --- | --- |
| Voice recording/transcription | Composer voice controller/recorder plus desktop/server transcription pipeline | No voice/transcription source surface; composer explicitly renders voice input unavailable |
| Headless/web workspace | Documented headless server release, readiness, remote binding/auth/TLS and update workflow | No equivalent native headless/server product surface |

### Largest material depth gaps

| Area | GPUI has | Still missing versus current upstream |
| --- | --- | --- |
| First-run onboarding/setup replay | Persistent six-step first-run guide, local agent command summary, appearance/project entry and Settings replay | Integrated provider enable/sign-in terminal, project creation and deeper upstream setup interactions |
| Browser sessions/WebMCP | Embedded native browser, manual uploads/inspection/viewport capture/link downloads, reviewed popup handoff, bounded network diagnostics and approved task browser-use bridge | Saved logins, cookie/session import, complete popup auth, agent upload/download, console diagnostics, restored sessions and page-declared WebMCP |
| iOS Simulator/device | Discovery, boot/shutdown and screenshots | Live input, swipe/type/buttons, recording, app install/launch, URL open, accessibility tree and element targeting |
| Editor/diff | Native tabs, find/replace, Markdown preview, changed-file/diff-row navigation, Explorer and Git review | Autosave/conflict parity, syntax-highlighted depth, compare scopes, blame and richer diff editing |
| Managed worktrees | Git worktree operations | First-class per-task managed isolation and environment-aware fork/orchestration ownership |
| Provider/model/context | Generic ACP controls plus direct-model runtime | Starred model+effort presets, provider ordering, quick cycling, richer compaction/context UX and provider/account usage telemetry |
| Computer Use | Reviewed X11 selected-window observation/input | Broader action/input/preview/target semantics plus macOS/other-platform parity |
| Automations | Durable fixed-offset daily/weekly schedules, run/failure stop limits, history and cancellation | IANA/DST schedules, broader recurrence, retry policy and deeper orchestration |
| Slash commands/keybindings | Qualified native workflow commands plus provider commands, palette and constrained remaps | Richer command arguments, upstream semantics and broad context-aware custom keybindings |
| Releases/updater | Local version history/read state | Verified release feed, signed install/update/rollback lifecycle |
| Profile/activity | Local activity, UTC active-hour distribution and real task token/context values | Provider/model mix, heatmap and richer account/usage statistics |
| Handoff/forks | Reviewed related continuation and context-derived branch drafts | Same-task continuation, provider-native forks and explicit local/new-worktree selection |
| Attachments/media | PNG/JPEG/text intake, bounded one-level folder snapshots and transcript images | Persistent folder references, broader formats and PDF/document viewer |
| Studio | Native Studio/Hubs and file/output preview | Current upstream long-running/output-oriented Studio depth |

### Near-parity lanes (5)

Theme/density is now substantial enough that it should not be grouped with the
largest feature gaps. File/source search, thread export, reply/context reuse and the
generic multi-provider workspace also have bounded deltas rather than requiring a
new subsystem. Their remaining exact interaction/format/provider breadth should be
tracked separately from the large gaps above.

Empty-install onboarding now has a persistent six-step guide, local agent-command
discovery, appearance and project setup, and Settings replay. Existing installs are
not forced through it. Authentication is described as guidance; the guide does not
claim sign-in or provider health. Browser popup requests use reviewed manual-tab
handoff, while network diagnostics are bounded and redact URL credentials, queries
and fragments. Editor and search navigation, folder snapshots, fixed-offset weekly
automations with run/failure limits, and UTC profile active hours also advanced.
The classification is **2 missing, 14 material-depth and 5 near-parity lanes**.

Native subagents/workflows, Agent Gateway and incoming external MCP remain
substantially implemented. Computer Use is retained as a depth gap because current
upstream behavior is materially broader than the present X11 slice.

## September 23: manual download and ZIP continuation

The native manual browser now saves explicit HTTP(S) links through a reviewed
local destination, private staging and no-overwrite publication. Per-tab epochs,
shared-profile transfer admission, cancellation and byte/time limits retain
ownership. Agent and authentication partitions still deny downloads. Source landed
at `e4a740f11450e03f26b0435f0d3e442dc1325e7b`. Browser/WebMCP remains a material-depth gap.

Native conversation ZIP export now packages `thread.json` and `transcript.md`
from one completed SQLite snapshot. It preserves exact text/roles, known metadata
and recorded image references without bundling credentials, unsent drafts, tool
payloads or file bytes. The native action and `/synara/export-zip` reuse the existing
save-dialog and no-overwrite export owners. Packaging is implemented. Broader
structured metadata and native save-picker/platform acceptance remain bounded
near-parity work. The three missing surfaces and historical A-Q states do not change.

See [download evidence](../verification/manual-browser-downloads.md) and
[ZIP evidence](../verification/conversation-zip-export.md). These source comparisons
use upstream `eaa61eded31b6755d4f30ba8eabc5d905cf817cb` and are not a new full-product census.

## September 23: scoped native continuation

Upstream reference remains `eaa61eded31b6755d4f30ba8eabc5d905cf817cb`.
The browser now has an explicit manual viewport-image clipboard action and
one-shot-approved, bounded task scrolling. Scrolling retires element references
until a new document read. Manual upload selection and inspector/dialog support
were already integrated before these changes. This does not implement saved-login
vaults, protected cookie import, popup auth, downloads, full-page or agent capture,
session restoration, page-declared WebMCP or additional native platforms.

The new `/synara/` command layer reuses native Debug, Goal, Recap, workflow,
automation, Computer Use, usage, Markdown export and reconstructed-fork owners.
Plan mode uses negotiated ACP choices and the existing session-control dispatcher.
Only exact bare commands are accepted. Provider commands are not shadowed,
unsent commands do not execute on restart, and command execution grants no implicit
provider prompt, scheduler or computer-input authority. Upstream's unqualified
names, richer command arguments, ZIP export, same-task/native-provider forks and
broader keybindings remain open.

Both areas stay in **material depth gap**, not Present or near parity.
All three missing product surfaces and the other current parity lanes remain open.
See [workflow details](native-commands.md) and the
[verification record](../verification/parity-continuation-2026-09-23.md).


### September 23: manual PNG viewport export

Manual browser tabs now offer **Save visible page image as PNG...** beside
clipboard capture. The native chooser selects a new local filename. The existing
capture owner bounds dimensions and device scale, encodes asynchronously into
private staging and publishes without overwriting only while the document epoch
and visible page remain current. Cancellation, timeout and teardown publish no
file. Agent capture and download capabilities remain disabled.

This advances browser capture/export within the material-depth lane. Full-page
capture, automatic composer attachment, restored sessions, authenticated-session
import and WebMCP are not implied. See
[PNG export evidence](../verification/browser-png-export.md).

## September 22: feature-closure sprint 2

At this checkpoint the inventory had **18 substantially present, 26 partial and 4 missing**
capabilities, not a release-readiness percentage. Stacked pull requests, AppSnap,
two-task split views and bounded checkpoints/revert move Missing -> Present. Rich media in transcript moves Missing
-> Partial because native image workflows are real, but PDF/document viewing remains
materially absent. No earlier Partial capability is promoted to Present.

| Capability | Native workflow | Remaining scope |
| --- | --- | --- |
| Stacked PRs | Deterministic stack detection/order, parent/base representation, current position/readiness, navigation and explicitly confirmed selected-prefix merge pinned to reviewed heads | Live authenticated GitHub breadth and wider platform interaction acceptance |
| Transcript media (partial) | Exact uploaded/agent-returned image bytes, distinct provenance, bounded decode, inline/expanded rendering, original export, missing/corrupt refusal and restart persistence | PDF/document viewing and broader media types |
| AppSnap | Explicit Linux/X11 visible-window discovery and one-window capture with target identity into durable pending attachments; no silent desktop capture | macOS/Windows/Wayland implementations and OS permission acceptance |
| Two-task split | Existing task/session owners render same- or cross-project tasks together with independent drafts/streams, focused Send/Stop, replacement/close and narrow fallback | Broader platform/input/accessibility acceptance |
| Checkpoints/revert | Explicit reviewed rollback of app-owned unsent draft plus notes/checklist with bounded retained history, recovery checkpoint, restart persistence, stale/owner/active-task fencing and atomic failure recovery | Deliberately excludes workspace files, Git/index, transcript, provider sessions, approvals, attachments and broader state |

Focused/native runs 35762961299, 35766712749, 35767806370, 35770961136 and
35772987553 passed for these slices. At that checkpoint the remaining Missing capabilities were native
subagents/workflows, Agent Gateway, external-MCP-to-Synara and Computer Use.

## September 22: breadth-first feature closure

At this checkpoint the inventory had **14 substantially present, 25 partial and 9 missing**
capabilities, not a release-readiness percentage. Five capabilities move from
Missing to Present: Debug, persistent goals, thread recap, PR Fix and inline file
comments. In-app releases moves Missing to Partial. No Partial -> Present claim is
made. The prior 9/24/15 checkpoint below remains historical evidence.

| Capability | Native workflow | Remaining scope |
| --- | --- | --- |
| Debug | Persisted app-owned evidence phases, explicit unsent step preparation, required verification and native command/composer integration | Live-agent debugging efficacy and broader interaction/platform acceptance |
| Goals | Durable objective/editor, explicit transient arm and first Send, two-follow-up/ten-minute budget, user priority, blockers, elapsed time and human-reviewed achievement | Live-provider/status-contract breadth and platform acceptance, no unbounded autonomy or restart autostart |
| Recap | Review request, independent unsent generation task using existing runtime, explicit bounded source-owned cache and refresh | Live-model summary quality, binary/hidden context intentionally not copied |
| PR Fix | Bounded unresolved-thread collection, exact identity/order, head and comment recheck, editable instruction and correct unsent destination | Authenticated live GitHub acceptance and non-GitHub providers |
| Inline comments | Saved editor range/context, bounded task-owned persistence, multi-comment prompt and stale/deleted/renamed refusal | Broader editor/SSH/platform acceptance, not remote PR-comment publishing |
| Releases (partial) | Compiled version, bundled development notes, local version history and read/dismiss/transition state | Verified remote release history/feed/signatures and production update/install lifecycle are unconfigured, not invented |

See [workflow and ownership details](feature-closure.md) and the
[exact verification receipt](../verification/feature-closure-sprint.md). Subsequent
dated sections describe their own earlier checkpoints and do not supersede these counts.

## September 22: direct models, Project Import and provider continuation

At this earlier checkpoint the 48-capability inventory had **9 substantially present, 24 partial and
15 missing** capabilities. This is not a release-readiness percentage. Compared
with the consolidated Sessions 1-4 checkpoint, Project Import moves from missing
to present, and direct multi-provider runtime and provider handoff move from missing
to partial. The 75+ provider target remains open.

| Area | Current implementation | Remaining scope |
| --- | --- | --- |
| Direct models | Native Settings, catalog/profile/model review, endpoint-bound OS keys, explicit send/Stop, durable text conversations, OpenAI-compatible/Anthropic/Google transport families, usage and bounded local structured-output validation | Verified 75+ provider breadth, OAuth/cloud auth, native multimodal conversations, approved tool execution, same-session state transfer and authenticated/platform acceptance |
| Project Import | Native read-only Codex/Claude discovery, preview/branch selection, local destination review, explicit atomic text import, duplicate receipt, retry/recovery and source preservation | Broader real-history/platform acceptance, binary history and incremental sync. Provider sessions, approvals and hidden state are intentionally not transferable. |
| Provider continuation | Native target picker, reviewed editable context, fresh unsent related task in the same working folder, original link and inert restart | Product handoff is partial: no in-place provider-session transfer, file rollback or autonomous delegation. |

See [direct models](direct-models.md), [Project Import](project-import.md),
[provider continuation](provider-handoff.md) and the
[exact verification receipt](../verification/max-feature-sprint.md). Native evidence
uses owned HTTP/ACP fixtures. Registry metadata is not authentication or a count
of proven provider integrations. The subsequent dated checkpoints remain historical.

## September 22: consolidated Sessions 1-4 status

All four requested **feature implementation sessions are complete** in the current
native tree. That statement does not close broader production/platform/provider
acceptance gates.

| Area | Consolidated implementation | Remaining acceptance / deliberately unsupported scope |
| --- | --- | --- |
| Conversation depth | Side chats are independent related tasks with independent drafts/sessions; edit/resend is additive; revision branches are explicit new unsent conversations from bounded visible context | Provider-specific rollback/steer/handoff, broader attachment parity in Side chats and cross-platform interaction acceptance remain open |
| Pull Requests | Existing Git/process ownership discovers GitHub repositories; bounded list/detail/files/commits/checks/activity use the shared diff/editor surfaces; create/comment/review/draft-ready/close-reopen/merge are explicit confirmed provider actions pinned to loaded scope/head where required | Live authenticated account interoperability, enterprise/GitLab scope, broader inline-review parity and native interaction acceptance remain open |
| Automations | SQLite definitions/run ledger, explicit agent/project ownership, fixed-offset scheduling, pause/resume/edit/delete/run-now/history/cancellation, atomic scheduled-slot claims and owned conversations; scheduler starts disarmed after restart | IANA/DST, cron/calendar schedules, automatic retry/pruning, production restart/shutdown behavior, live-provider completion/cancellation and direct Hub context selection remain open |
| Browser | Existing BrowserHost/Session plus native Linux/X11 WebKitGTK child surface, tabs/navigation/history/stop/title/loading lifecycle and task-isolated one-shot-approved navigate/read/click/fill bridge | Native Wayland/Windows/macOS hosts, downloads/capture export, IME/accessibility/HiDPI, production authenticated websites and live-model browser-use acceptance remain open |
| Plugins / Skills / MCP | Ownership-aware integrations inventory, reviewed local Markdown skills, scoped HTTP MCP management/discovery and reference-only credential persistence | Provider-owned catalogs/lifecycle, OAuth/other transports, production OS credential store and real-provider/cross-platform acceptance remain open |
| Device / Settings | Bounded ADB/simctl discovery/capture/lifecycle, explicit probed Android input, Device viewer, archive/delete safeguards, notification/privacy/navigation/appearance and newer integration settings | Hardware/physical Apple acceptance, Android cold boot, screen-reader/notification-delivery acceptance, OS credential store and macOS/Windows acceptance remain open |

The current Environment persistence accepts the five Environment-owned tools:
Terminal, Explorer, Changes, Device and Side chats. Browser is implemented as its
own native panel because its embedded child surface has separate lifecycle/overlay
requirements; it is not represented as an unavailable Environment placeholder.
Zen continues to expose implemented tools through the existing owners rather than
creating alternate task/session state.

## September 21: Device and Settings continuation

On `astra/device-settings`, based on `980d86b`, real command-backed device discovery,
bounded screenshots, lifecycle actions, probed explicit Android input and native
viewer ownership replace the earlier protocol-only device slice. This does not
complete physical Apple support, Android cold boot, AppSnap or hardware acceptance.

Native Settings now wires startup restore, editable effective navigation shortcuts,
recent-attachment visibility, stronger text/separators, actual agent/config controls,
notification preference/test and privacy/deletion actions. The complete area-by-area
inventory, retained functionality and unavailable states are in
[Device/Settings](device-settings.md). The [receipt](../verification/device-settings-session.md)
records 40 distinct passing targeted tests across the latest applicable focused
runs and a passing Linux native `cargo check` at `744830e`. The first compile-blocked
attempt and the corrective run are retained, not hidden. Runtime/device/capture
sources were unchanged after their passing run and were not needlessly re-tested.
Earlier "no workflow" and "no native compile" statements below describe their earlier
checkpoints, not this session's scoped validation. A compilation check is not a
running GPUI window or hardware, input, macOS or Windows acceptance.

At the Device/Settings checkpoint, PR/Automations/Browser and Plugins/Skills/MCP
had not landed at integration head `980d86b`. The later Plugins/Skills/MCP
implementation and its bounded native evidence are recorded below. Original
acceptance gates remain open where provider, platform or hardware evidence is missing.

## Upstream review

Current upstream comparison head is
`Emanuele-web04/synara@eaa61eded31b6755d4f30ba8eabc5d905cf817cb`,
10 commits beyond the previous `f04341a67bc4941d1b2e91e0b23bbe782dfbc727`
snapshot used by the September 22/23 audit. The latest delta includes additional
Computer, model/provider, simulator, automation, release and runtime work.

The previous 48-capability count remains useful only for historical trend lines.
It is **not** the current completion denominator because current upstream includes
or documents user surfaces that the old boundary did not represent cleanly,
including voice recording/transcription, onboarding/setup replay and the
headless/web workspace. Current planning uses parity lanes instead of forcing those
surfaces into a stale denominator.

The Synara product requirement remains intentionally broader than upstream's named
provider list for direct models. Generic ACP stays the coding-agent architecture,
while `synara-model` remains a separate provider-neutral direct runtime.

## September 21: composer intake and saved follow-ups

Continuing `0bb20db`, native source now wires PNG/JPEG and UTF-8 file intake from
picker, clipboard and drop into persisted task-owned snapshots, preview/removal,
recent-snapshot reuse and explicit capability-checked prompt delivery. A compact
manual follow-up queue adds save/edit/reorder/remove/append without automatic send
or steering. The current backup validator accepts the new keys and Hub metadata.
[Implementation, bounds and verification](composer-intake-followups.md) records
prepared focused regressions separately from still-unverified native behavior.
No whole D4/D8/D11/D12/F2/F4/I10 gate is closed by this batch.

## September 21: Plugins, Skills and MCP

The `astra/plugins-skills-mcp` session continues `980d86b` with the existing generic
agent/Settings architecture. [Behavior and ownership](../integrations.md) and the
[verification receipt](../verification/plugins-skills-mcp.md) distinguish safe
local management from unsupported provider lifecycle and remaining acceptance.

Reconciliation `1ce1e41` includes the Device/Settings integration `0b2d1ec`.
[Focused run 35635894653](https://github.com/cmdr-chara/synara/actions/runs/35635894653)
passed 38 selected Rust tests, the native application build and six native journey
assertion groups. Eight real GPUI/X11 screenshots were inspected, including Skills
after restart at a narrower window size. This proves local document management,
explicit loopback HTTP discovery, local removal and unavailable-store refusal,
not provider-owned installation or authenticated service interoperability.
The cleaned code was integrated and its remote ref verified at `12e93d86`.
The documentation successor preserves the exact tested executable inputs.

At that September 21 checkpoint, bootstrap supplied `UnavailableSecretStore` and
authenticated MCP was blocked. The later maximum-feature sprint installs a shared
OS credential-store adapter without fallback, but real-store/authenticated
acceptance remains open. External plugin catalogs,
provider-native skill lifecycle, OAuth, other transports and macOS/Windows
acceptance remain open. Native tests used explicit-path skill review, not a desktop
file-picker portal. Earlier failed builds and input-driver failures are retained
in the receipt rather than represented as successful acceptance.

## Current native delta

| Surface | Source implemented | Remaining acceptance or functionality |
| --- | --- | --- |
| Native subagents/workflows | Reviewed atomic child DAGs and dependency reports, real task/session owners, bounded Run/Pause/Stop, live usage, retry/recovery and archival guards | Real-provider/platform acceptance, richer authoring and optional worktree isolation |
| Agent Gateway | Negotiated ACP HTTP-MCP enrollment and scoped native-approved operations returning bounded child reports | Representative production agents and transport/platform acceptance |
| Incoming external MCP | Local clients connect to Synara with bearer leases, scoped tools, nonce receipts, native approvals and revocation | Representative client applications, remote/OAuth and transport breadth |
| Computer Use | Actual X11 window selection, observation/preview, one-shot input and takeover with fresh-frame/target checks | Partial: full desktop, richer input, Wayland/macOS/Windows and production application acceptance |
| Stacked PRs | Reviewed deterministic stack model and selected-prefix merge using existing PR provider/confirmation ownership | Authenticated live GitHub and cross-platform acceptance |
| Transcript images | Durable task-owned image bytes, provenance, native preview/expand/export and bounded corrupt/missing behavior | Partial: PDF/document viewing and broader media types |
| AppSnap | Explicit Linux/X11 single-window discovery/selection/capture into durable pending attachments | macOS/Windows/Wayland implementations and permission acceptance |
| Two-task split | Existing independent task/session owners rendered together with focus-routed composer/session actions and narrow fallback | Broader input/accessibility/platform acceptance |
| Checkpoints/revert | Bounded reviewed rollback for durable draft plus notes/checklist with recovery and stale/active fencing | Files, Git/index, transcript, provider/session, approvals and attachments are intentionally outside the first checkpoint boundary |
| Debug | App-owned persistent five-phase evidence workflow and explicit unsent composer preparation | Live-provider and platform acceptance, see feature-closure docs |
| Persistent goals | Explicitly armed bounded pursuit, pause/resume/clear, user priority, blocker/achievement history and inert restart | Live-provider and cross-platform acceptance, no unbounded retry |
| Thread recap | Reviewed generation using an independent unsent related task and explicit source-owned bounded cache | Summary quality/provider/platform acceptance |
| PR Fix | Head-pinned unresolved review context into an explicitly reviewed unsent destination | Live authenticated account acceptance |
| Inline file comments | Durable task-owned version-pinned range comments and reviewed unsent composer append | Broader editor/remote/platform interaction acceptance |
| Releases | Current compiled version, native notes, local observation history and read/dismiss state | Partial: verified release feed and production installer remain unavailable |
| Direct model providers | Separate `synara-model` runtime, three transport families, reviewed registry/custom endpoints, Settings/model selection, endpoint-bound OS references, streaming/Stop, usage and local structured-output checks | Partial toward 75+ interoperability, additional auth and approved tools. Reviewed multimodal context is implemented; see direct-model docs |
| Project Import | Reviewed local Codex/Claude text-history discovery/import, atomic receipt and recovery | Broader real-history/platform acceptance; no session/approval/secret transfer |
| Provider continuation | Reviewed unsent related ACP/direct conversations with original link and unchanged source session/root | No in-place same-task session migration or filesystem rollback |
| Plugins/integrations | Native searchable built-in and managed inventory, ownership and reported-capability separation | External catalog/installed-state/lifecycle requires an actual provider contract, not inferred support; E8/I9 |
| Skills | Reviewed local Markdown documents, hashes/origin/version, explicit disabled install/update, enable, unsent draft insertion and removal | Remote catalogs, provider-native bundles, native picker/update and broader input/platform acceptance; E8/I9 |
| MCP | Native scoped add/edit/enable/test/remove, secret references, modern/legacy HTTP discovery, generic negotiated session context and safe retirement | OS secret-store acceptance, OAuth, SSH, process/legacy SSE, vendor and platform acceptance; E8/I9 |
| Transparent Glass | Continuous window tint, compositor transparency/blur request, corrected panel alpha, non-opaque editor/terminal/Git roots, bounded local wallpaper decode/blur | Actual OS blur, full/narrow native screenshots, contrast/focus/restart matrix; I7/I10/P |
| Zen | Shared presentation preference, native Environment reveal, narrow tool deck, existing draft/process ownership, exit and input guards | Native keyboard/IME/modal interactions, compositor behavior and feature-depth review; D8/G7/I10 |
| Optional Hubs | Managed/chosen local folder, Main/child tasks, flat navigation/home, context editor, revision-checked saves, archive/restore | Explicit Hub creation and independent draft/restart regression now tested; broader context/platform journeys, multiple roots/sources and organization remain; F1/F2/F8 |
| Studio compatibility | Existing Studio tasks projected as Hubs without rewriting identities, drafts, sessions or files; malformed metadata preserved | Full migration/backup acceptance before removing serialized Studio compatibility; F2/F4/F9 |
| Shared Hub context | Visible new-thread draft seeding, explicit current-draft insertion and source-message promotion into a reviewed editor | Retrieval, automatic memory and context policies are not implemented; D9/D10/F2 |
| Hub Library | Existing file/preview service plus bounded same-directory peer-thread reporting and Open reporting thread | Binary intake, remote previews, deeper provenance and multi-source Library; F9/G1/D11 |
| Message branching | Bounded quoted user/assistant context saved as a new unsent same-workspace draft | Not a provider-session clone, file rollback or automatic send; D9/D12 |
| Composer | Bounded multiline input plus PNG/JPEG/text file picker, image/file clipboard, drop, durable attachment tray, previews, recent reuse and negotiated Image/Context delivery | Native/real-agent acceptance, more formats, historical media replay and richer mentions; D8/D11/D12 |
| Saved follow-ups | Task-local persisted text list with edit/reorder/remove/append, draft-preserving Queue and Append, stale-write guards | Manual only, not automatic queue/steer; native restart/keyboard acceptance remains; D4/D8/F2 |
| Hub Kanban | Scoped task view, captured Hub creation target, literal search, status/attention filters, pinning and explicit Run/Stop using existing services; global Kanban remains separate | Native execution/restart acceptance, richer task movement and task-context controls; F10 |
| Editor workspace | Guarded sequential Save all, stop-between-files, Close saved/other saved, eight retained closed buffers, tab reordering and compact chrome | Native conflict/IME/SSH journeys, disk refresh and restart/crash recovery; G2/G3/G8/I10 |
| Terminal workspace | Flatter pane/tab controls and saved active-tab reordering without moving PTYs | Native focus, restart and platform checks; A8/G7/I10 |

Zeron informs Zen interaction and restraint only. MonoCode's supplied workspace
captures inform pane-local controls, compact tabs and restrained navigation. Neither repository supplies code, assets, tokens or copied
screen composition. Normal Synara retains its own product concepts and services.

## Important still-open product surfaces

The major Sessions 1-4 feature areas have landed. The table below now tracks
acceptance and intentionally unimplemented extensions rather than describing those
features as absent.

| Area | Remaining acceptance or extension work |
| --- | --- |
| Direct multi-provider runtime | Real partial implementation now exists. Remaining P0 development: 75+ provider breadth/interoperability, additional auth families, approved tool execution and real-provider acceptance. Reviewed image/text context and multimodal replay are implemented. ACP stays separate. |
| Computer Use | A reviewed X11 app-window workflow is implemented. Full desktop, richer Unicode/IME and modifier/drag input, other platforms and production acceptance remain. |
| Project Import | Reviewed native text import is implemented. Broader real-history/platform acceptance and optional incremental/binary depth remain. |
| Attachments and voice | Native attachment acceptance, broader binary formats, historical media/export, capture permissions and voice/transcription; D11/D12/I8 |
| Rich conversation workflows | Side chats and additive edit/resend/revision branching are implemented. Reviewed related provider continuation is now implemented. Provider-supported queue/steer, in-place handoff, file-affecting rollback, richer structured result cards, Side-chat attachment parity and broader native acceptance remain; D4/D8-D12/G7 |
| Pull Requests | Feature implementation is present. Live authenticated GitHub interoperability, broader native interaction coverage, enterprise/non-GitHub providers, deeper inline-review parity and Hub/thread association remain; H6 |
| Automations | Durable definitions, fixed-offset scheduling, owned run history, cancellation and explicit restart arming are implemented. IANA/DST, cron/calendar schedules, automatic retry/pruning, direct Hub context and production scheduler/provider restart acceptance remain; F11 |
| Skills/plugins/MCP | Synara-owned management is implemented. Provider-owned lifecycle/catalog contracts, full skill bundles, production OS credentials/OAuth, remaining transports and real-provider/broader native acceptance remain; E8/I9 |
| Browser | Real Linux/X11 embedded WebKit hosting, navigation/lifecycle and task-isolated approved automation are implemented. Native Wayland/Windows/macOS, downloads/capture, production auth, accessibility/IME/HiDPI and live-model acceptance remain; K1-K6 |
| Device tooling | ADB/simctl discovery, bounded capture, supported lifecycle and explicit Android input have source/UI implementation. Physical Apple, Android cold boot, hardware/native interaction and broader platform acceptance remain; L1-L5 |
| Settings and platform | Session 4 implemented the scoped remaining native Settings functionality. OS credential-store acceptance, broader notification/accessibility/platform behavior, packaging/updater and macOS/Windows acceptance remain; I/P/O |

## Earlier workspace-checkpoint evidence and next work

This section preserves the earlier workspace checkpoint. Device/Settings and
Plugins/Skills/MCP validation above supersede its compile/workflow status only
for their explicitly tested candidates and journeys.

The preceding Hub checkpoint recorded the following evidence (not the later integration session):

The roadmap's local structural check passes with 17 lanes and 120 unchanged task
bodies/checkbox states. Three focused Hub Rust tests are prepared but not run.
No current native compile, running GPUI capture or compositor acceptance is claimed.
No GitHub test workflow was dispatched and no workflow configuration was changed.

That earlier checkpoint's next-work ordering is historical. Side chats,
Pull Requests, Automations and the Linux/X11 Browser have since landed, together
with Plugins/Skills/MCP and Device/Settings. Continue with the still-open acceptance
and extension lanes above while preserving the existing service boundaries. Native
platform evidence remains distinct from source presence. See [Hubs](hubs.md) and
[Glass/Zen](zen-personalization.md).

The [workspace adaptation receipt](monocode-workspace-adaptation.md) records the
exact source scope, reference archive, guards and remaining native acceptance.
