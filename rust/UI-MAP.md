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
| Sidebar chats | SidebarThreadRowContent, ThreadRowQuickActions, ThreadPinToggleButton, ThreadRunningSpinner, SidebarListSection | collapsible `row()` list | 🔶 (no pin/quick-actions/spinner) |
| Empty home | ChatEmptyStateHero, ChatShell, SponsorLandingBanner | watermark columns + Υ + Support card | ✅ |
| Composer | ComposerCapsule, ComposerInputBanners, ComposerModelPicker*, ComposerEffortSliderCard, ComposerExtrasPanel, ComposerCommandMenu, ComposerQueuedHeader | bordered composer card; runtime-mode + model dropdowns | 🔶 (no +/attachments, no effort/model pickers, no slash menu) |
| Transcript | ChatTranscriptPane, MessagesTimeline, TurnHeaderLine, MessageTrail, ChatMarkdown*, ThreadErrorBanner | right-aligned user pills, md_blocks, error chips | 🔶 (markdown-lite only; no find-bar, selection actions, code highlighting) |
| Activities | MessageTrail, ToolCallGroupSummaryRow, SubagentRunCard, BackgroundTaskRow, WorkflowRunCard, ProposedPlanCard, TaskProgressSteps | ◦ timeline rows + plan cards | 🔶 (no grouped tool cards/subagent cards) |
| Approvals | ComposerPendingApprovalPanel, AsyncUserInputCard, UserInputQuestionForm | approve/decline buttons | 🔶 (no form-type questions) |
| Thread chrome | OpenThreadTabStrip, ChatHeader, ThreadStatusPillChip, ThreadArchiveActionButton, RenameThreadDialog | tab strip header + status chips + Stop/Archive | 🔶 (single tab only, no rename) |
| Files pane | ProjectFolderList, workspaceExplorer, WorkspaceFilePreview* | dir list + viewer | 🔶 (flat dir listing, no tree) |
| File editor | EditorWorkspaceView, WorkspaceFileEditorPane, codeEditor/* (Monaco) | `Editor` + `EditorState` (gpui native editor) | 🔶 (view/edit/save; no diff editor, no syntax lang detection) |
| Dev servers | (dev-server UI in server surfaces) | DEV SERVERS pane list/run/stop | ✅ |
| Settings | settings/* (44 comps), SettingsSidebarNav, ThemePackEditor, DebugFeatureFlagsMenu | Settings pane: appearance + about | 🔶 (theme+about only) |
| Diff panel | DiffPanel* (9 comps), DiffLineBlamePopover, DiffWorkerPoolProvider | — | ⬜ |
| Terminal | terminal/*, TerminalWorkspaceTabs, ThreadTerminalDrawer, TerminalSearch | — | ⬜ (terminal RPCs native; no pane) |
| Inbox | inbox/*, githubInbox/* (4) | — | ⬜ (githubInbox.* RPCs native; no UI) |
| Browser | BrowserPanel, BrowserTabStrip, BrowserVault* | — | ⬜ (wry webview only as fallback) |
| Right dock | RightDock, SubagentsDockPane, BackgroundTasksDockPane, SidechatDockPane, PlanSidebar | — | ⬜ |
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
5. Diff panel + terminal pane (both have native RPCs already).
6. Inbox/githubInbox, kanban, automation, hosts/device/account surfaces.
7. Dialogs/sheets on demand.
