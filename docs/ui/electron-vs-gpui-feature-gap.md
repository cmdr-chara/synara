# Electron Synara to native Rust/GPUI: current feature-gap audit

Checkpoint: 2026-09-21, following Glass/branching recovery `ae64983` and the first
optional-Hubs integration, extended with compact workspace controls and Hub tasks.
This is a source inventory, not native runtime or cross-platform acceptance.

The complete preceding Electron comparison, source links, dated tables and earlier
continuation notes remain in the [preserved pre-Hubs inventory](electron-vs-gpui-feature-gap-before-hubs-2026-09-21.md).
The full acceptance backlog remains in [ROADMAP.md](../../ROADMAP.md). No historical
gap is closed merely because this current view is shorter or a heading was renamed.

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
records actual evidence separately. Earlier "no workflow" statements below describe
their earlier checkpoints, not this session's scoped validation.

PR/Automations/Browser and Plugins/Skills/MCP had not landed at the last checked
integration head `980d86b`. No integration placeholder is claimed complete or
replaced with guessed settings. All original acceptance gates remain open where
native, platform or hardware evidence is missing.

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

## Current native delta

| Surface | Source implemented | Remaining acceptance or functionality |
| --- | --- | --- |
| Transparent Glass | Continuous window tint, compositor transparency/blur request, corrected panel alpha, non-opaque editor/terminal/Git roots, bounded local wallpaper decode/blur | Actual OS blur, full/narrow native screenshots, contrast/focus/restart matrix; I7/I10/P |
| Zen | Shared presentation preference, native Environment reveal, narrow tool deck, existing draft/process ownership, exit and input guards | Native keyboard/IME/modal interactions, compositor behavior and feature-depth review; D8/G7/I10 |
| Optional Hubs | Managed/chosen local folder, Main/child tasks, flat navigation/home, context editor, revision-checked saves, archive/restore | Native journeys, Settings compatibility labels, multiple roots/sources, richer organization; F1/F2/F8 |
| Studio compatibility | Existing Studio tasks projected as Hubs without rewriting identities, drafts, sessions or files; malformed metadata preserved | Full migration/backup acceptance before removing serialized Studio compatibility; F2/F4/F9 |
| Shared Hub context | Visible new-thread draft seeding, explicit current-draft insertion and source-message promotion into a reviewed editor | Retrieval, automatic memory and context policies are not implemented; D9/D10/F2 |
| Hub Library | Existing file/preview service plus bounded same-directory peer-thread reporting and Open reporting thread | Binary intake, remote previews, deeper provenance and multi-source Library; F9/G1/D11 |
| Message branching | Bounded quoted user/assistant context saved as a new unsent same-workspace draft | Not a provider-session clone, file rollback or automatic send; D9/D12 |
| Composer | Bounded multiline input plus PNG/JPEG/text file picker, image/file clipboard, drop, durable attachment tray, previews, recent reuse and negotiated Image/Context delivery | Native/real-agent acceptance, more formats, historical media replay and richer mentions; D8/D11/D12 |
| Saved follow-ups | Task-local persisted text list with edit/reorder/remove, draft-preserving Queue and Append, stale-write guards | Manual only, not automatic queue/steer; native restart/keyboard acceptance remains; D4/D8/F2 |
| Hub Kanban | Scoped task view, captured Hub creation target, literal search, status/attention filters, pinning and explicit Run/Stop using existing services; global Kanban remains separate | Native execution/restart acceptance, richer task movement and task-context controls; F10 |
| Editor workspace | Guarded sequential Save all, stop-between-files, Close saved/other saved, eight retained closed buffers, tab reordering and compact chrome | Native conflict/IME/SSH journeys, disk refresh and restart/crash recovery; G2/G3/G8/I10 |
| Terminal workspace | Flatter pane/tab controls and saved active-tab reordering without moving PTYs | Native focus, restart and platform checks; A8/G7/I10 |

Zeron informs Zen interaction and restraint only. MonoCode's supplied workspace
captures inform pane-local controls, compact tabs and restrained navigation. Neither repository supplies code, assets, tokens or copied
screen composition. Normal Synara retains its own product concepts and services.

## Important still-open product surfaces

| Area | Remaining work / ownership |
| --- | --- |
| Attachments and voice | Native attachment acceptance, broader binary formats, historical media/export, capture permissions and voice/transcription; D11/D12/I8 |
| Rich conversation workflows | Edit/resend, supported queue/steer, safe rollback/handoff, structured result cards and Side chats; D4/D8-D12/G7 |
| Pull Requests | Actual authenticated discovery/list/detail/review/actions, scoped errors and concurrency; H6 |
| Automations | Durable definitions/scheduling/history, explicit execution consent and restart without duplicates; F11 |
| Skills/plugins/MCP | Discovery, installed-state management, pairing/test/revoke and capability ownership; E8/I9 |
| Browser | Real embedded host, navigation and lifecycle, isolated cookies/auth/downloads, bounded approved automation; K1-K6 |
| Device tooling | ADB/simctl discovery, capture, supported lifecycle and explicit Android input now have source adapters/viewer. Native helper protocol depth, physical Apple, Android cold boot and hardware acceptance remain; L1-L5 |
| Settings and platform | Remaining functional sections, label migration, secrets, notifications, accessibility, packaging/updater and macOS/Windows interaction; I/P/O |

## Evidence boundary and next work

The roadmap's local structural check passes with 17 lanes and 120 unchanged task
bodies/checkbox states. Three focused Hub Rust tests are prepared but not run.
No current native compile, running GPUI capture or compositor acceptance is claimed.
No GitHub test workflow was dispatched and no workflow configuration was changed.

Continue Hub integration and real composer/attachment workflows next, then Side chats,
PRs and Automations. Keep the existing service boundaries and compare actual native
interactions when a build is available. Browser material studies are not substitutes
for native screenshots. See [Hubs](hubs.md) and [Glass/Zen](zen-personalization.md).

The [workspace adaptation receipt](monocode-workspace-adaptation.md) records the
exact source scope, reference archive, guards and remaining native acceptance.
