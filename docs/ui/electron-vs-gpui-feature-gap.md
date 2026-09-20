# Electron Synara → native Rust/GPUI feature gap

Audit date: 2026-09-19. Electron source: [`Emanuele-web04/synara` at `73cd181`](https://github.com/Emanuele-web04/synara/tree/73cd1811a81e4a62b6199a722fd6fac379abf9ba). Native source: `astra/gpui-clean-rewrite` at `c8e06e7` (the current local checkout). These are snapshots, not a claim about future branches. This is a source and screenshot audit, not an end-to-end run of every feature. “Missing” means no corresponding native user flow was found; “Partial” means a smaller or different native flow exists; “Backend only” means code/foundation exists but the Electron interaction is absent. Provider capabilities can also depend on installed agents and operating system.

The comparison covers the Electron app's top-level routes, chat/composer/workspace surfaces, sidebar, Kanban, Studio, pull requests, automations, all 15 settings sections, desktop integrations, and distribution. It excludes test harnesses and internal-only APIs. The wallpaper in the supplied screenshots is external to Synara and is not a missing app asset.

## September 20: repository continuation

Current upstream review: `e7cd15281e6d16cf8fc55a91496dcff035475e54`,
two commits after the previous `b58f27381e7ddd59678c9961500e8e43d3cc19ab`.
This continuation preserves the concurrent editor/command-palette batch `48bf3f9`.
The remaining dated tables retain their historical audit context.

| Area | Current source implementation | Remaining work |
| --- | --- | --- |
| Branches | List/filter/current indicator, create, switch, rename, merged-only delete, copy name | Native and cross-platform verification, advanced branch workflows |
| Remotes | Add/edit/remove names and URLs, explicit single-branch fetch, fast-forward pull, non-force push | Authenticated network/SSH acceptance, no credentials stored or implied |
| Worktrees | Host-scoped listing, path copy, creation and guarded removal | Per-task worktree routing, platform and dirty/locked runtime scenarios |
| Stashes | Save tracked changes with explicit untracked opt-in, apply by object ID while retaining stash | Native conflict/recovery and wider stash workflow acceptance |
| Repository UI | Existing Changes tool hosts the new panel, searchable bounded lists, retained operation forms, scoped loading/errors, app-close guard | Native visual screenshots and integrated execution still pending |
| Passive delegated results (4a886e9) | Newly identified F11/D12 gap, deferred | Creator inbox delivery, human-originated send reservation and replay fingerprints |
| macOS app icon after quit (e7cd152) | Newly recorded packaging gap, deferred | Native bundle/Dock behavior on macOS |

The latest user instruction places broad tests and full visual acceptance after
feature development. This is source implementation, not a declaration of full
Git/UI parity. Existing Browser, attachment intake, Side chats, Pull Requests,
Automations, vendor context-budget provenance and Artifacts gaps remain open.
[Delivery record](workspace-productivity.md).

## What the native app already has

Rust is a real native application, not just a visual mockup. It has project and standalone chat persistence, distinct Studio chat scope, a welcome/composer screen, ACP agent connection and discovered model/mode/options, streamed transcript with expandable work details, permission/input requests, basic message copy, file tree/editor with guarded save, staged/unstaged Git status and commit, local PTY terminal, SSH workspace operations, an agent registry, a basic Kanban overview, settings navigation, a profile activity grid, archive restoration, and some Synara icons and motion. These are foundations for parity, not proof of equal feature depth. [Native shell](../../crates/synara-app/src/shell.rs), [dock](../../crates/synara-app/src/shell/dock.rs), [settings](../../crates/synara-app/src/shell/settings.rs), [native roadmap](../../ROADMAP.md).

## Navigation, projects, and sidebar

Electron evidence: [Sidebar](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/Sidebar.tsx), [SpaceSwitcher](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/SpaceSwitcher.tsx), [search palette](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/SidebarSearchPalette.tsx). Native evidence: [navigation](../../crates/synara-app/src/shell/navigation.rs), [general settings](../../crates/synara-app/src/shell/settings.rs).

| Electron feature | Native status | Gap |
| --- | --- | --- |
| Spaces with tab switching, create/edit/delete, rename, drag reorder, activity dots and a Void group | Missing | Native has only the Synara/Studio mode switch. |
| Assign or drag projects into Spaces | Missing | No Space model or UI. |
| Rich command/search palette for threads, projects, commands and themes | Partial | Native title search filters sidebar threads; no combined command palette or content/project search. |
| Add an existing project, create a folder, import projects from local providers | Partial | Native can open a workspace path; import and create flows are absent. |
| Import an existing provider thread by external ID | Missing | Native creates its own chats/sessions. |
| Project editing, pinning, ordering and hover actions | Partial | Native shows project name/path/count and can open it; richer management is absent. |
| Project default provider, metadata and project scripts | Missing | A project can be selected, without Electron's management controls. |
| Nested project chats and standalone Chats with show-more pagination | Present/partial | Basic grouping exists; Electron row menus and richer status/context are absent. |
| Thread pin, rename, archive/delete, drag/move and richer row context menus | Partial | Native can rename and archive through limited flows and restore archives; pin, move and most context actions are absent. |
| Temporary chats and thread export/share | Missing | No equivalent native flow found. |
| Route/history navigation | Partial | Basic back/forward exists; Electron's richer route and palette navigation is absent. |

## Chat transcript and composer

Electron evidence: [ChatView](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/ChatView.tsx), [composer extras](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/chat/ComposerExtrasPanel.tsx). Native evidence: [messages](../../crates/synara-app/src/shell/messages.rs), [composer](../../crates/synara-app/src/shell/composer.rs), [controls](../../crates/synara-app/src/shell/controls.rs), [activity](../../crates/synara-app/src/shell/activity.rs).

| Electron feature | Native status | Gap |
| --- | --- | --- |
| Full transcript rendering: advanced Markdown, tables, code blocks, links, attachments and tool cards | Partial | Native has a smaller CommonMark renderer and text-first events. Rich structured blocks are not equivalent. |
| In-turn queued messages versus steering an active turn | Missing | Native has send/stop, without Electron's queue/steer behavior and queued header. |
| Edit/resend prior user messages and undo/revert turn file changes | Missing | No matching transcript action. |
| Fork/branch from an assistant message | Missing | The native action explicitly says unavailable. |
| Pin messages and show pinned context | Missing | The native action explicitly says unavailable. |
| Transcript find/search and minimap/jump navigation | Missing | Native sidebar title search and scroll-to-latest are narrower. |
| Work summaries, durations, tool activity and intermediate commentary | Partial | Native groups work and can expand it; Electron's detailed tool/result cards, workflow strip and recap have greater depth. |
| Plan sidebar, task checklist, subagent/workflow presentation | Missing | No matching native side surface. |
| Per-turn usage/context meter, model details and provider health/auth/rate-limit feedback | Partial | Native reports selected-chat token/context data and discovered options; lacks Electron's full session indicators. |
| File/folder mentions, thread/agent/skill mentions and slash command autocomplete | Missing | Native Add attaches path references; no equivalent inline picker/command system found. |
| Paste/drop images, image previews and generated-image handling | Missing | Native file references are paths, not a binary/image intake flow. |
| Attach another window and AppSnap context | Missing | Native Add entry is explicitly unavailable. |
| Voice recording and transcription | Missing | Native microphone action is explicitly unavailable. |
| Goal, debug and fast composer modes | Missing/partial | Native Add shows unavailable Goal/Debug; agent-advertised plan mode/options can be selected. |
| Saved model/effort presets, favorites and provider-specific controls | Partial | Native selects agent-discovered model/mode/options, without Electron's preset/favorite UX. |
| Structured questions, approval review, authentication recovery | Partial | Native handles protocol permission/input requests; Electron has more specialized presentation and recovery flows. |
| Hand off a conversation to another provider/worktree | Missing | No native handoff flow found. |
| Selection-to-chat, terminal/browser/PR context and cross-surface references | Missing | Native has basic project file path references. |
| Thread sharing/export and file-change review from the transcript | Missing | No matching native user flow found. |

## Per-chat Environment panel and plugin library

Electron evidence: [EnvironmentPanel](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/chat/environment/EnvironmentPanel.tsx), [plugin route](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/routes/_chat.plugins.tsx), [PluginLibrary](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/PluginLibrary.tsx). Native evidence: [dock and available panels](../../crates/synara-app/src/shell/dock.rs), [settings placeholders](../../crates/synara-app/src/shell/settings.rs).

| Electron feature | Native status | Gap |
| --- | --- | --- |
| Collapsible per-chat Environment panel, floating/docked layout and section visibility preferences | Missing | Native has separate utility panels, without this per-chat context surface. |
| Environment usage, branch/Git actions, PR and automations rows | Partial | Native has separate usage and basic Git views; no combined context or PR/automation rows. |
| Per-chat notes, project instructions and auto-generated recap | Missing | No matching native saved notes/instructions/recap UI found. |
| Pinned-message checklist with done, rename and jump actions | Missing | Native pin-message action is unavailable. |
| Local servers, editor targets and side-chat rows | Missing | No corresponding native Environment sections. |
| Plugin/skill library route with provider discovery, search and installed-state display | Missing | Native has an agent registry; the plugin route and skill browser are absent. |

## Workspace dock, browser, device, files, terminal and Git

Electron evidence: [BrowserPanel](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/BrowserPanel.tsx), [DevicePanel](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/DevicePanel.tsx), [EditorWorkspaceView](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/EditorWorkspaceView.tsx), [DiffPanel](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/DiffPanel.tsx), [TerminalWorkspaceTabs](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/TerminalWorkspaceTabs.tsx), [GitActionsControl](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/GitActionsControl.tsx). Native evidence: [dock](../../crates/synara-app/src/shell/dock.rs), [panels](../../crates/synara-app/src/shell/panels.rs), [roadmap K–L](../../ROADMAP.md).

| Electron feature | Native status | Gap |
| --- | --- | --- |
| Multi-pane dock tabs, adjustable split/maximize and Side chats | Partial | Native has an animated chat/workspace split with Files/Changes/Terminal; Side chats is unavailable, with no equivalent tab layout. |
| Embedded browser tabs and address/navigation controls | Missing | Native launcher explicitly marks Browser unavailable; Rust has policy groundwork only. |
| Browser local-server discovery, screenshots, annotation, cookie/vault and agent automation | Missing | Depends on the absent browser host and UI. |
| Device/iOS simulator viewer, input, screenshots and agent tools | Missing | Rust has device protocol types, but no device helper/viewer flow. |
| Multi-file editor tabs, code search, richer editor actions and workspace layout | Partial | Native opens one file in a simple editor with save/discard and filename filtering. |
| Rich diff review with changed-file navigator, scopes, syntax, comments and comparisons | Partial | Native shows bounded raw staged/unstaged diff text and stage/unstage. |
| Git branch management, push and create PR from Changes | Missing | Native Git UI is status/stage/commit; no push or PR action. |
| Merge/conflict resolution and repository network/auth workflows | Missing | No equivalent native GUI found. |
| Multiple terminal tabs, split terminals, search and session management | Partial | Native has one local or SSH PTY view with start/restart/stop. |
| Project scripts and local development server controls | Missing | No matching native surface. |
| SSH remote workspace files, terminal, Git and agent transport | Partial | Native has real SSH foundations and UI; Electron's whole workspace experience and some lifecycle cases are not yet matched. |

## Kanban and Studio

Electron evidence: [Kanban overview](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/kanban/KanbanOverview.tsx), [project board](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/kanban/KanbanProjectBoardView.tsx), [new task dialog](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/components/kanban/KanbanNewTaskDialog.tsx), [Studio route](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/routes/_chat.studio.index.tsx), [Studio outputs](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/server/src/studioOutputs.ts). Native evidence: [overview](../../crates/synara-app/src/shell/overview.rs), [navigation](../../crates/synara-app/src/shell/navigation.rs).

| Electron feature | Native status | Gap |
| --- | --- | --- |
| Kanban overview and per-project boards | Partial | Native has one simple cross-project four-column task overview. |
| Create task from a dedicated board composer, choose project/provider/options, preserve draft | Missing | Native Kanban cards only open existing tasks. |
| Drag task between columns and start a draft by moving it to In Progress | Missing | Native board is not draggable. |
| Card menus, project/status filtering and detailed task activity | Missing | Native cards show title/project only. |
| Separate Studio space, new Studio chat and persistent Studio history | Present/partial | Scope and switcher exist; advanced Studio workflow is absent. |
| Studio generated output tree, images/gallery and workspace scaffold | Missing | Native Studio is a scoped chat without these output surfaces. |

## Pull requests

Electron evidence: [PR route](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/routes/_chat.pull-requests.index.tsx), [operations](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/server/src/pullRequests/pullRequestOperations.ts). Native evidence: [disabled sidebar entry](../../crates/synara-app/src/shell/navigation.rs).

**The whole native PR route is missing.** The Electron route includes a list/search/filter, involvement and state filters, PR detail and code review, checks/timeline/comments, and actions such as draft/ready, close/reopen, merge and follow-up work. Native “Pull requests” is explicitly an unavailable action. Its basic local Git panel is not a substitute for remote PR integration.

## Automations

Electron evidence: [automation list](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/routes/_chat.automations.index.tsx), [detail route](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/routes/_chat.automations.$automationId.tsx), [scheduler](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/server/src/automation/Services/AutomationScheduler.ts). Native evidence: [disabled sidebar entry](../../crates/synara-app/src/shell/navigation.rs).

**The whole native automation route is missing.** Electron has create/edit/delete, active/paused filtering, pause/resume, manual run/stop, interval/calendar/cron scheduling, environment and model configuration, run history and results, stop/failure policy, and automation-linked conversations. Native “Automations” is explicitly unavailable. No native scheduler/service parity was found.

## Settings: all 15 Electron sections

Electron section registry: [settingsNavigation](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/settingsNavigation.ts); [settings route](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/web/src/routes/_chat.settings.tsx). Native section registry and implementations: [settings.rs](../../crates/synara-app/src/shell/settings.rs). A native heading alone is not parity.

| Section | Native status | Electron controls absent or narrower in Rust |
| --- | --- | --- |
| General | Partial | New-thread workspace mode, welcome tour, automation-run sidebar toggle, Environment-panel defaults/sections. Provider and project/thread order plus Chats/Studio toggles exist. |
| Profile | Partial | Avatar/photo, current/longest streak, provider/reasoning/hour/project insights, plugin/model usage and share card. Native name/username, tokens/prompts and local heatmap exist. |
| Appearance | Partial | Custom theme editing/import/copy, app icon, custom title bar, UI density, chat width, font sizes and terminal font, time format, contrast and translucent sidebar. Native offers system/light/dark, two dark presets, UI/code font fields and reduce motion; color swatches are read-only. |
| Notifications | Missing | Desktop notification preferences and test notification; native page explicitly says unported. |
| Chat behavior | Missing/partial | Queue/steer default, streaming preference, effort slider, device auto-open, review diff colors/wrapping and destructive confirmations; native page is explanatory, not a control panel. |
| Keybindings | Partial | Search and edit/capture shortcuts; native lists fixed shortcuts. |
| Usage & limits | Partial | Provider quota/credit/rate-limit panels; native shows selected-chat tokens/context and says account limits unavailable. |
| AppSnap | Missing | Capture shortcut, destination, sound and screen-capture permission setup; native page explicitly says unavailable. |
| MCP connections | Missing | Create/revoke/test scoped external connections and pairing; native page explicitly says unavailable. |
| Agent providers | Partial | Electron's individual Codex, Claude, Cursor, Antigravity, Grok, Droid, OpenCode, Pi and Devin installers/configuration, paths, availability, update checks and ordering. Native has configurable generic ACP launch profiles and registry, without those vendor-specific settings. |
| Models & writing | Partial | Custom model slugs/favorites and Git-writing model; native lists models discovered from the selected agent. |
| Agent skills | Missing | Discover, enable/disable and manage skills; native page explicitly says unavailable. |
| Managed worktrees | Missing | List/inspect/delete managed worktrees; native page explicitly says unavailable. |
| System tools | Partial | Electron session/recovery/version/release and desktop diagnostics are deeper. Native links to inspector, remote, registry and Help. |
| Archived threads | Partial | Native can restore archived chats; Electron also groups archived threads by project and supports permanent deletion. |

The native's generic ACP approach is a deliberate architectural difference. It can connect agents that advertise capabilities, but that does not recreate Electron's provider-specific onboarding, account status, and options. Electron's provider union is defined in [orchestration contracts](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/packages/contracts/src/orchestration.ts).

## Desktop and release integration

Electron evidence: [desktop source](https://github.com/Emanuele-web04/synara/tree/73cd1811a81e4a62b6199a722fd6fac379abf9ba/apps/desktop/src), [release documentation](https://github.com/Emanuele-web04/synara/blob/73cd1811a81e4a62b6199a722fd6fac379abf9ba/docs/release.md). Native evidence: [roadmap P](../../ROADMAP.md), [README](../../README.md).

| Electron feature | Native status | Gap |
| --- | --- | --- |
| Desktop capture/AppSnap and voice transcription | Missing | Explicitly unavailable in native UI. |
| Desktop notifications and tray/window behavior | Partial | Native window exists; notification preferences and Electron's integration behavior have not been ported. |
| In-app update and packaged release flow | Missing/unverified | Native update verification types exist, but updater UI and production installer path remain roadmap work. |
| macOS/Windows feature parity | Unverified | Native Linux path has been exercised; cross-platform terminal, dialogs, fonts/GPU, credentials and updater remain open in the roadmap. |
| Accessibility, keyboard navigation and reduced-motion parity | Partial/unverified | Native has some focus labels and reduced motion; full screen-reader, keyboard and interaction parity is not established. |

## Recommended completion order

1. **Restore complete chat use:** queued/steered turns, rich event cards and attachments, mentions/commands, edit/fork/pin, handoff, voice and context interactions.
2. **Build missing whole routes:** PRs and Automations, with their backend services, then full Kanban task creation/movement and Studio outputs.
3. **Complete workspace surfaces:** embedded Browser and Device; multi-tab editor/terminal, richer diff/Git and Side chats.
4. **Finish settings and desktop integration:** the placeholder sections, provider onboarding, appearance controls, notifications/AppSnap, worktrees, updater and platform validation.

The native app currently resembles Synara visually in its primary shell, but it is **not feature equivalent** to the pinned Electron version. The strongest proof is that several top-level navigation items and settings sections explicitly announce their own unavailability in the Rust source. This report is a migration backlog; each row still needs a runtime acceptance check as it is implemented.
