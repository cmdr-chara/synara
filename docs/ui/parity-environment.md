# Native Environment workspace checkpoint

Date: September 20, 2026. Roadmap: G7, I6, I10, F2 (bounded progress).
Base: `0f176c3f7067021d6983b4e756ad05e2b473c37b`.
Reference: Electron `948875954f432978eab7dd5fa44c3028b8d99a81` and supplied
screenshots 08, 73, 74, 78, 79, 88-90. Dracula is an optional appearance choice,
not the product specification or a new default. No palette or theme defaults change.

## Implemented slice

Environment now has a shared native tab strip for Terminal, Explorer and Changes,
a trigger-anchored Add menu, pointer and keyboard split resizing, equal-width
reset, and maximize/restore. Tabs reuse the existing host-aware services and
retained editor/terminal entities. This is not multiple terminal-session support.
Browser and Side chats remain explicitly unavailable rather than fake working tabs.
Changes is the existing native Git pane, an additional native tool in this menu.

A versioned, bounded Environment preference retains opened tabs, selected tool,
desired split ratio and Open by default. Explicit open/hide updates that preference.
General settings exposes the preference and an explicit Reset layout action.
Normal chat navigation restores the pane when requested. Studio does not
implicitly open it. Restoring a Terminal tab never starts a shell or an agent.

Hiding Environment does not stop a running terminal or discard editor/chat text.
Closing the application still uses the existing owned-process cleanup. Maximizing
only changes chrome and preserves the chat entities. Resizing maintains minimum
pane widths at smaller viewports without overwriting the desired saved ratio.

Layout writes are coalesced, serialized and separate from AppSettings. An edit
cannot overtake an in-flight write. Failure keeps the last good preference and
leaves the window open on a failed close-time save. Invalid/newer saved layouts
are preserved until an explicit reset. Navigation never silently overwrites them.

## Verification record

Local: Debian 13 x64, pinned Rust 1.98.1 and the locked dependency cache with an
isolated native development sysroot. No user workspace or desktop was used.

- PASS: four Environment storage/validation/recovery regressions.
- PASS: four Environment split/state/save-order regressions.
- PASS: strict changed-package Clippy and native application/ACP fixture build.
- PASS: fourteen verification-scope selector regressions and Python/YAML parsing.
- Native interaction acceptance is recorded here after the published candidate's
  focused run. Local desktop tooling lacks the required complete Vulkan runtime.

The focused Environment lane checks the new native journey plus directly affected
chat/split, Studio restoration, guarded-close/permissions and Kanban-to-chat paths.
It does not run full workspace tests, vendor probes or download dependency caches.
Unknown/backend/dependency changes retain the full verification path. Mixed UI
changes retain broader presentation coverage, including the Environment journey.

## Remaining limits

The layout is application-wide, not independently saved for every thread. There
is one tab per existing native tool, not multiple PTYs, editor tabs or side chats.
Tool tabs remain open until Reset layout; Hide Environment is deliberately
nondestructive. Embedded browser/device tooling, remote forwarding, Studio outputs,
full per-platform accessibility/IME acceptance and all 73-state visual parity
remain open. Screenshots alone do not establish browser, approval or animation
runtime behavior. Existing project/file/agent authority remains unchanged.
