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
| Sidebar search | SidebarSearchPalette, WorkspaceSearchPalette | `Input` + client filter + `searchThreads` hits | 🔶 (palette overlay missing) |
| Sidebar projects | ProjectFolderList, SidebarProjectRowContent, ProjectMenuPicker, ProjectHoverCardContent | clickable rows w/ branch+dirty badge | 🔶 (no hover card/context menu) |
| Sidebar chats | SidebarThreadRowContent, ThreadRowQuickActions, ThreadPinToggleButton, ThreadRunningSpinner, SidebarListSection | collapsible `row()` list + pin markers | 🔶 (pin via header menu, local state; no quick-actions) |
| Empty home | ChatEmptyStateHero, ChatShell, SponsorLandingBanner | watermark columns + Υ + Support card | ✅ |
| Composer | ComposerCapsule, ComposerInputBanners, ComposerModelPicker*, ComposerEffortSliderCard, ComposerExtrasPanel, ComposerCommandMenu, ComposerQueuedHeader | composer card; attachments (+picker/upload/chips), runtime-mode dropdown | 🔶 (model picker blocked: needs provider CLI installed; no slash menu, no effort) |
| Transcript | ChatTranscriptPane, MessagesTimeline, TurnHeaderLine, MessageTrail, ChatMarkdown*, ThreadErrorBanner | right-aligned user pills, `TextView::markdown` (tables/code/bold/lists, selectable), Cmd+F find-bar w/ live count + accent highlight | 🔶 (no selection actions, code highlighting) |
| Activities | MessageTrail, ToolCallGroupSummaryRow, SubagentRunCard, BackgroundTaskRow, WorkflowRunCard, ProposedPlanCard, TaskProgressSteps | collapsible tool-call cards ("N actions ▸") + plan cards | 🔶 (no subagent/background-task cards) |
| Approvals | ComposerPendingApprovalPanel, AsyncUserInputCard, UserInputQuestionForm | approve/decline buttons | 🔶 (no form-type questions) |
| Thread chrome | OpenThreadTabStrip, ChatHeader, ThreadStatusPillChip, ThreadArchiveActionButton, RenameThreadDialog | multi-tab strip, ⋯ menu (Rename inline → thread.meta.update, Pin/Unpin local), Spinner + status chips, Stop/Archive | ✅ |
| Files pane | ProjectFolderList, workspaceExplorer, WorkspaceFilePreview* | dir list + viewer | 🔶 (flat dir listing, no tree) |
| File editor | EditorWorkspaceView, WorkspaceFileEditorPane, codeEditor/* (Monaco) | `Editor` + `EditorState` (gpui native editor) | 🔶 (view/edit/save; no diff editor, no syntax lang detection) |
| Dev servers | (dev-server UI in server surfaces) | DEV SERVERS pane list/run/stop | ✅ |
| Settings | settings/* (44 comps), SettingsSidebarNav, ThemePackEditor, DebugFeatureFlagsMenu | Settings pane: appearance + about | 🔶 (theme+about only) |
| Diff panel | DiffPanel* (9 comps), DiffLineBlamePopover, DiffWorkerPoolProvider | CHANGES dock: git.status file list (+ins/-del) + colored patch view | 🔶 (no blame, no staged/unstaged split) |
| Terminal | terminal/*, TerminalWorkspaceTabs, ThreadTerminalDrawer, TerminalSearch | bottom TERMINAL drawer: PTY via terminal.open, subscribeEvents stream (ANSI-stripped), $ input -> terminal.write, restart/close | 🔶 (no tabs/multi-session, no search, no true tty rendering) |
| Inbox | inbox/*, githubInbox/* (4) | INBOX pane: PR/issue rows (icon, #, repo, author, rel time), opens URL | 🔶 (no filters/actions) |
| Browser | BrowserPanel, BrowserTabStrip, BrowserVault* | — | ⬜ (wry webview only as fallback) |
| Right dock | RightDock, SubagentsDockPane, BackgroundTasksDockPane, SidechatDockPane, PlanSidebar | right-side dock container w/ CHANGES pane | 🔶 (only changes pane; no subagents/tasks/sidechat) |
| Kanban | kanban/* (11) | — | ⬜ |
| Pull requests | pullRequest/* (34), PullRequestThreadDialog, GitCreatePrDialog | — | ⬜ |
| Automation | automation/* (5), RailAutomationsPanel, ComposerAutomationSetupBanner | — | ⬜ |
| Hosts/device | hosts/* (11), device/* (3), account/* (6), profile/* (5) | — | ⬜ |
| Dialogs/sheets | ~20 dialogs (RenameDialog, ShortcutsDialog, WhatsNewDialog, FeatureTourDialog, …) | — | ⬜ (Dialog component exists in kit) |
| Misc chrome | SplashScreen, AppShellTopStrip, AppSnapCoordinator, RouteSurface, KeepAwakeControls, AnnouncementSheet, OneTimeCoachmark | — | ⬜ |

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
