# Native navigation foundation

Status: partial implementation checkpoint, not visual parity or release readiness.

## Product reference

The design reference is Emanuele's Synara, including the user-supplied home-screen
capture and the pinned references in `reference-register.md`. Its narrow sidebar,
compact project/chat rows, Settings footer and secondary menus are the target.
No web frontend source, screenshot assets or another application's UI code is
included in the native implementation. The small line icons are independently
authored geometry drawn by GPUI.

## Source ownership

- `src/ui.rs`: semantic dark material roles, geometry, font selection, common
  controls, native tooltip views and line icons. New action controls share a
  single callback for mouse, keyboard and AccessKit activation.
- `src/shell/navigation.rs`: compact project/chat navigation, disclosure, bounded
  pages and transient focus state. The catalog and task state remain owned by
  the existing workspace/controller. No second persistent task model exists.
- `src/shell/chrome.rs`: window content chrome, work-surface navigation and the
  More popover. Escape restores trigger focus. Arrow/Tab navigation stays within
  the menu while open. Normal Tab bubbles after editor/terminal input handling.
- Existing conversation, filesystem, Git, permission, registry and terminal
  implementations are retained. The common legacy button style delegates to
  the new visual primitive without changing those operation callbacks.

Project and task selections use the existing `select_task`, `create_task`,
`WorkspaceService` and async jobs. Creating a chat preserves the previous draft.
Dirty-file project switching retains the existing guard. Registry approval from
the new menu still passes through the real review/consent implementation and does
not start an agent. No backend production API changed for this UI checkpoint.

## Verification

The candidate builder runs formatting, locked workspace compilation, strict
Clippy, workspace tests, the workspace/provenance audit and roadmap checks. It
then builds the application and runs both `native_smoke.py` and
`native_navigation_smoke.py` on separately owned, isolated Xvfb displays.
Candidate-specific results and screenshot hashes are recorded in
`native-navigation-evidence.json`; an absent or failed receipt is not acceptance.
The temporary candidate builder does not advance a branch. Publication is a
separate non-forced update after review of its exact source tree and results.

The navigation smoke checks real persistent selection, new-chat ACP streaming,
draft restoration, keyboard menu navigation, Escape focus restoration, registry
consent without execution, collapse without domain events, resizing and shutdown.
Resize survival is not proof that every intermediate layout is visually correct.

## Parity and gaps

| Area | Current status |
| --- | --- |
| Native shell/navigation primitives | Implemented foundation, screenshot comparison still required |
| Project/chat rows and Settings footer | Compact native layout, real backend selection and creation |
| Menus/tooltips/keyboard | More popover and new navigation actions implemented; legacy controls still require migration |
| Sidebar list size | Bounded 64-row pages; mature infinite/virtual sidebar behavior remains different |
| Theme and typography | Centralized dark roles for migrated surfaces; full persisted appearance integration remains open |
| Main conversation/composer/Markdown | Existing published implementation retained; earlier local overhaul not yet recovered |
| Transparency and wallpaper | Not implemented here; no supplied screenshot is embedded as the application |
| Pinned/unread rows, context actions | Still missing or partial; no synthetic status is invented |
| Kanban, PRs, automations, handoffs | Not represented as working navigation destinations without backend integration |
| Browser/device host surfaces | Existing backend/platform gaps remain; no fake preview is presented |
| Linux UI | CI interaction/render evidence only for private X11/Xvfb |
| Wayland, macOS, Windows UI | No interactive or visual parity claim |
| Accessibility | Roles, labels and action callbacks added; no complete screen-reader audit claimed |

## Recovery remains separate

The earlier local UI at `/mnt/data/ui-work/native-integrated` is still unverified
because the authoring runtime was returning transport timeouts. These changes
were developed against the published branch, not represented as recovery of that
work. Preserve the old working tree and reconcile overlapping files when access
returns. See `continuation.md` for the recovery boundary.
