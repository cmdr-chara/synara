# Workspace depth and compact native controls

Date: 2026-09-21. Starting source: `e418fff2116fc9ae2ce6a6e4015109e3af0bca9f`.
Original Synara reviewed: `e7cd15281e6d16cf8fc55a91496dcff035475e54`, unchanged.

## Design synthesis

The supplied MonoCode archive is pinned to
`768adb4b03b5cc7228b4a96a8b75094e99da54ac`. Its 30 curated images are copies from
51 raw captures, not 81 distinct screens. SHA-256 deduplication yields 50 unique
image contents. The contact sheet, full-size file workspace, terminal workspace
and the supplied translucent macOS workspace inform this implementation. The
README confirms session-oriented tabs and agent-owned authentication. No competitor
source implementation, assets, text, tokens or measurements are reused.

Adopt the interaction principles, not the four-pane composition: controls sit beside
the pane they affect, a tab is the open buffer rather than another floating card,
status lives in a quiet secondary row, and tools expose detail only when requested.
Keep Synara's logo, glyphs, navigation, semantic palette and Environment split.
Zeron remains a reference for Zen only. Zen is not replaced or given another domain.

Hub task views use a continuous canvas with plain rows, thin separators and text tabs
for filters. Task activity remains controller-owned. Run/Stop are explicit actions,
not consequences of browsing or moving a decorative card. Waiting and failed work is
prioritized without color-only signals. At narrow widths, filters and toolbar actions
wrap instead of squeezing multiple Kanban columns into the conversation.

Editor tabs use one highlight and an underline, not a highlighted button inside a
highlighted card. Dirty state, close and filename remain visible. Inactive close
controls can soften on pointer use but remain visible on keyboard focus. The tab
strip scrolls separately from fixed pane actions. Bulk actions are progressively
revealed in an inline command row, avoiding another modal or popup focus owner.
Text, files, undo history and process state are never transformed to get this look.

## Feature batch A: Hub tasks

Hub home and sidebar open a dedicated task view. It filters only that Hub's legacy-
compatible Studio-scope tasks. Normal Kanban still excludes them. The same atomic
Hub task-creation service and existing Kanban Run/Stop lifecycle are reused.
A task composer captures its Hub when opened and cannot change scope on later
navigation. Create-and-run sends only the text reviewed in that composer. Shared
context is not silently appended. Existing New Hub thread remains the visible
context-seeding path.

The task view adds literal title/agent search, All/Drafts/Active/Needs attention/
Finished filters, stage counts, bounded Show more, pinning, task opening and explicit
Run/Stop. Archived Hubs remain readable but cannot create or launch new drafts from
the board. Filter choices are presentation state for this app session, not persisted
agent or task state. Tasks and their drafts remain durable in the existing store.

## Feature batch B: editor workspace

Save all snapshots dirty buffers once and writes them sequentially using the existing
local or pinned-SSH version-checked service. It does not change the selected tab or
replace live input text. A successful write advances only the disk baseline, so
newer text typed during saving remains unsaved. The first failed file stops later
writes. Stop takes effect between atomic file writes and never claims to roll back
already saved files. Orderly app close waits for the batch and returns to review on
failure, cancellation or still-dirty input. A failed SSH setup releases the batch
guard through the same explicit result path.

Close saved and Close other saved preserve every dirty buffer. Up to eight clean
closed buffers can be reopened with their input entity, selection, scroll and undo
history. Discarded dirty text is not secretly retained. Reopening restores an
in-process snapshot, not a fresh disk read. Guarded saves still detect external
changes, and the notice identifies that retained-snapshot behavior. Closed history
is cleared when changing workspaces and is not restart/crash recovery.

Editor tabs can be reordered without recreating inputs. Ctrl/Cmd+Shift+S saves all,
Ctrl/Cmd+Shift+T reopens a buffer, and Alt+PageUp/PageDown moves the active tab. The
commands are scoped to the file editor and defer to IME/modal/terminal ownership.

## Terminal continuation

The existing terminal workspace has flatter tabs, a quieter pane heading and smaller
pane-local action buttons. Tab-left/tab-right actions change the saved layout order
while retaining IDs, active/split selection and every PTY/input owner. New-shell,
Stop/Restart/Close confirmations, search and reviewed paste behavior are unchanged.
Terminal ordering survives restart as metadata, never as resumed commands.

## Evidence and limits

Native compiler/executable preflight: neither cargo nor rustc is installed. The
pinned toolchain host still fails DNS. No current native build or runtime screenshot
is claimed. Existing archive captures are research evidence only. Source delimiter,
reference and whitespace checks are not Rust compilation. No GitHub workflow is
manually dispatched and commits use the requested `[skip ci]` marker.

F10, G2/G8 and I10 remain open for complete native acceptance. In particular, task
execution with real agents, simultaneous dirty buffers, save conflicts, narrow
layouts, keyboard/IME and transparent compositor behavior need native verification.
