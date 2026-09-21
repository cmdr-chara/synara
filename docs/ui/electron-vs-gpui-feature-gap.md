# Electron Synara to native Rust/GPUI: current feature-gap audit

Checkpoint: 2026-09-22, after consolidation of the four completed feature
implementation sessions for conversation depth, Pull Requests/Automations/Browser,
Plugins/Skills/MCP and Device/Settings. This is a source inventory with explicitly
scoped Linux evidence below. It is not general native runtime, provider, hardware
or cross-platform acceptance.

The complete preceding Electron comparison, source links, dated tables and earlier
continuation notes remain in the [preserved pre-Hubs inventory](electron-vs-gpui-feature-gap-before-hubs-2026-09-21.md).
The full acceptance backlog remains in [ROADMAP.md](../../ROADMAP.md). No historical
gap is closed merely because this current view is shorter or a heading was renamed.

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

`Emanuele-web04/synara` main was reviewed at
`e7cd15281e6d16cf8fc55a91496dcff035475e54`. It is unchanged from the previous review.
The known passive delegated-result delivery/human-send reservation and macOS icon
persistence fixes remain deferred under D12/F11 and P respectively. No new upstream
feature, workflow, setting or icon delta was found in this review.

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

Production bootstrap still supplies `UnavailableSecretStore`. Authenticated MCP
therefore remains blocked without plaintext fallback. External plugin catalogs,
provider-native skill lifecycle, OAuth, other transports and macOS/Windows
acceptance remain open. Native tests used explicit-path skill review, not a desktop
file-picker portal. Earlier failed builds and input-driver failures are retained
in the receipt rather than represented as successful acceptance.

## Current native delta

| Surface | Source implemented | Remaining acceptance or functionality |
| --- | --- | --- |
| Plugins/integrations | Native searchable built-in and managed inventory, ownership and reported-capability separation | External catalog/installed-state/lifecycle requires an actual provider contract, not inferred support; E8/I9 |
| Skills | Reviewed local Markdown documents, hashes/origin/version, explicit disabled install/update, enable, unsent draft insertion and removal | Remote catalogs, provider-native bundles, native picker/update and broader input/platform acceptance; E8/I9 |
| MCP | Native scoped add/edit/enable/test/remove, secret references, modern/legacy HTTP discovery, generic negotiated session context and safe retirement | OS secret-store adapter, OAuth, SSH, process/legacy SSE, vendor and platform acceptance; E8/I9 |
| Transparent Glass | Continuous window tint, compositor transparency/blur request, corrected panel alpha, non-opaque editor/terminal/Git roots, bounded local wallpaper decode/blur | Actual OS blur, full/narrow native screenshots, contrast/focus/restart matrix; I7/I10/P |
| Zen | Shared presentation preference, native Environment reveal, narrow tool deck, existing draft/process ownership, exit and input guards | Native keyboard/IME/modal interactions, compositor behavior and feature-depth review; D8/G7/I10 |
| Optional Hubs | Managed/chosen local folder, Main/child tasks, flat navigation/home, context editor, revision-checked saves, archive/restore | Native journeys, Settings compatibility labels, multiple roots/sources, richer organization; F1/F2/F8 |
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
| Attachments and voice | Native attachment acceptance, broader binary formats, historical media/export, capture permissions and voice/transcription; D11/D12/I8 |
| Rich conversation workflows | Side chats and additive edit/resend/revision branching are implemented. Provider-supported queue/steer, file-affecting rollback/handoff, richer structured result cards, Side-chat attachment parity and broader native acceptance remain; D4/D8-D12/G7 |
| Pull Requests | Feature implementation is present. Live authenticated GitHub interoperability, broader native interaction coverage, enterprise/non-GitHub providers, deeper inline-review parity and Hub/thread association remain; H6 |
| Automations | Durable definitions, fixed-offset scheduling, owned run history, cancellation and explicit restart arming are implemented. IANA/DST, cron/calendar schedules, automatic retry/pruning, direct Hub context and production scheduler/provider restart acceptance remain; F11 |
| Skills/plugins/MCP | Synara-owned management is implemented. Provider-owned lifecycle/catalog contracts, full skill bundles, production OS credentials/OAuth, remaining transports and real-provider/broader native acceptance remain; E8/I9 |
| Browser | Real Linux/X11 embedded WebKit hosting, navigation/lifecycle and task-isolated approved automation are implemented. Native Wayland/Windows/macOS, downloads/capture, production auth, accessibility/IME/HiDPI and live-model acceptance remain; K1-K6 |
| Device tooling | ADB/simctl discovery, bounded capture, supported lifecycle and explicit Android input have source/UI implementation. Physical Apple, Android cold boot, hardware/native interaction and broader platform acceptance remain; L1-L5 |
| Settings and platform | Session 4 implemented the scoped remaining native Settings functionality. OS credential-store integration, broader notification/accessibility/platform behavior, packaging/updater and macOS/Windows acceptance remain; I/P/O |

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
