# UI port map — React (apps/web) → gpui-kit (rust/shell)

Checklist for the 1:1 native port. React components cannot be machine-ported
(JSX/hooks/DOM have no Rust equivalent); each is reimplemented in gpui-kit.
UI *logic* lives in the already-ported crates (`rust/*-rs`); this file tracks
the render layer only. Status: ✅ done · 🔶 partial/stub · ⬜ missing.

Source of truth for web components: `apps/web/src/components/**.tsx`
(~150 top-level, ~380 in feature dirs = ~530 components).

## Surfaces

| Surface | Web components (source dir) | gpui-kit target | Status |
| --- | --- | --- | --- |
| App rail | AppRail, AppRailMoreMenu, AppRailUsage, DesktopUpdateRailButton | 44px v_flex icon column, outline-when-active ghost Buttons | ✅ |
| Sidebar header | SidebarHeaderNavigationControls, "Synara ▾" app menu | DropdownMenu w/ PopupMenuItem → open_url | ✅ |
| Sidebar search | SidebarSearchPalette, WorkspaceSearchPalette | palette overlay: client filter + `searchThreads`, ⌘P files, ⇧⌘F content | ✅ |
| Sidebar projects | ProjectFolderList, SidebarProjectRowContent, ProjectMenuPicker, ProjectHoverCardContent | clickable rows w/ branch+dirty badge + expand/collapse toggle + context menu (Finder/kanban/copy/edit/pin/archive/delete/remove) + hover card | ✅ |
| Sidebar chats | SidebarThreadRowContent, ThreadRowQuickActions, ThreadPinToggleButton, ThreadRunningSpinner, SidebarListSection | collapsible `row()` list + pin markers + context menu (rename/pin/fork/archive/copy/close) + drag between projects + hover quick actions + hover card | ✅ |
| Empty home | ChatEmptyStateHero, ChatShell, SponsorLandingBanner, AsciiWaveBackground | animated ASCII field (3-layer value-noise patterns w/ 20s cycle + 4s crossfade, glyph scramble, click ripples, center fade) + Υ + Support card | ✅ |
| Composer | ComposerCapsule, ComposerInputBanners, ComposerModelPicker*, ComposerEffortSliderCard, ComposerExtrasPanel, ComposerCommandMenu, ComposerQueuedHeader | composer card; attachments (+picker/upload/chips), runtime-mode dropdown, slash menu, @-mention fs browser | 🔶 (model picker blocked: needs provider CLI installed; no effort) |
| Transcript | ChatTranscriptPane, MessagesTimeline, TurnHeaderLine, MessageTrail, ChatMarkdown*, ThreadErrorBanner | right-aligned user pills, `TextView::markdown` (tables/code/bold/lists, selectable), Cmd+F find-bar w/ live count + accent highlight, copy/retry actions | ✅ |
| Activities | MessageTrail, ToolCallGroupSummaryRow, SubagentRunCard, BackgroundTaskRow, WorkflowRunCard, ProposedPlanCard, TaskProgressSteps | collapsible tool-call cards ("N actions ▸") + plan cards | 🔶 (no subagent/background-task cards) |
| Approvals | ComposerPendingApprovalPanel, AsyncUserInputCard, UserInputQuestionForm | approve/decline buttons | 🔶 (no form-type questions) |
| Thread chrome | OpenThreadTabStrip, ChatHeader, ThreadStatusPillChip, ThreadArchiveActionButton, RenameThreadDialog | multi-tab strip, ⋯ menu (Rename inline → thread.meta.update, Pin/Unpin local), Spinner + status chips, Stop/Archive | ✅ |
| Files pane | ProjectFolderList, workspaceExplorer, WorkspaceFilePreview* | lazy expand/collapse dir tree (indent 8+12·depth, chevrons, hidden-dir filter) + viewer | ✅ (search box, right-click ctx menu, drag-to-chat drop) |
| File editor | EditorWorkspaceView, WorkspaceFileEditorPane, codeEditor/* (Monaco) | `Editor` + `EditorState` (gpui native editor) | 🔶 (view/edit/save + syntax lang detection; no diff editor) |
| Dev servers | (dev-server UI in server surfaces) | DEV SERVERS pane list/run/stop | ✅ |
| Settings | settings/* (44 comps), SettingsSidebarNav, ThemePackEditor, DebugFeatureFlagsMenu | all 18+ sections w/ verbatim copy, interactive keybind rebinding | 🔶 (no ThemePackEditor, no debug flags) |
| Diff panel | DiffPanel* (9 comps), DiffLineBlamePopover, DiffWorkerPoolProvider | CHANGES dock: git.status file list (+ins/-del), staged/unstaged split + colored patch view | ✅ (blame popover on diff-line click + fuzzy file search) |
| Terminal | terminal/*, TerminalWorkspaceTabs, ThreadTerminalDrawer, TerminalSearch | bottom TERMINAL drawer: PTY via terminal.open, subscribeEvents stream (ANSI-stripped), $ input -> terminal.write, restart/close | 🔶 (multi-session tabs + in-buffer search ✅; no true tty rendering) |
| Inbox | inbox/* (InboxTasks, InboxTaskList, Recap/Usage tiles), githubInbox/* (4) | Inbox pane: greeting + "N need you", Today's tasks tile w/ todo rows (check toggle, due chips), thread groups Needs you / In progress / Ready for review / Failed (inbox.logic port), recap + usage tiles, sign-in, provider-update toast | 🔶 (no GitHub inbox rows, no dismissal pills) |
| Browser | BrowserPanel, BrowserTabStrip, BrowserVault* | — | ⬜ (wry webview only as fallback) |
| Right dock | RightDock, SubagentsDockPane, BackgroundTasksDockPane, SidechatDockPane, PlanSidebar | right-side dock: 4 panes (Git/Subagents/Background tasks/Side chats) w/ scrollable icon+label tab strip + overflow chevrons | ✅ |
| Kanban | kanban/* (11) | rail pane: per-project columns, cards ordered updatedAt desc | ✅ |
| Pull requests | pullRequest/* (34), PullRequestThreadDialog, GitCreatePrDialog | Code review rail pane + header git menu (Pull/Commit/Push/Create PR) + commit/PR dialogs via git.runStackedAction | 🔶 (pane is list-only; no PR detail thread) |
| Automation | automation/* (5), RailAutomationsPanel, ComposerAutomationSetupBanner | rail pane + definition list + detail pane (runs) + create/edit dialog + runNow/delete | ✅ |
| Hosts/device | hosts/* (11), device/* (3), account/* (6), profile/* (5) | — | ⬜ |
| Dialogs/sheets | ~20 dialogs (RenameDialog, ShortcutsDialog, WhatsNewDialog, FeatureTourDialog, …) | ShortcutsDialog ⌘/, FeedbackDialog (live POST), RenameThread, GitCommit/GitCreatePr, tour | 🔶 (core set done) |
| Misc chrome | SplashScreen, AppShellTopStrip, AppSnapCoordinator, RouteSurface, KeepAwakeControls, AnnouncementSheet, OneTimeCoachmark | feature tour overlay | 🔶 (tour only) |

## Port order (by user-visible value)

1. Transcript completeness: real markdown (tables, code highlight via `Editor`
   highlighter), find-bar, tool-call group cards, subagent cards.
2. Composer completeness: attachments (+), model picker from provider catalog,
   effort control, slash commands.
3. Thread chrome: multi-tab strip, pin/rename/archive quick actions, running
   spinner.
4. Right dock panes (background tasks, subagents, sidechat).
5. Terminal upgrade: tabs/multi-session, search, truer rendering (colors/cursor).
6. Kanban, automation, hosts/device/account surfaces (blocked on relayed/T3-derived domains).
7. Dialogs/sheets on demand.
8. Composer completeness: model picker (needs provider CLI) + slash menu.
