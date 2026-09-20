# Native Spaces and saved context

Date: September 20, 2026. Branch: `astra/gpui-clean-rewrite` only.
Reference: Electron Synara `948875954f432978eab7dd5fa44c3028b8d99a81`.
The reference is its UI/UX. Dracula remains an optional theme, not the design target.

## Organization implementation

Space tabs and an explicit manager provide creation, name/icon editing, ordering,
project assignment and confirmed deletion. Void contains unassigned projects.
Deleting a Space does not delete projects, conversations or files. Project and
thread pin actions are durable metadata and sort before unpinned entries.
Space switching filters the sidebar without changing a running task's context.
Keyboard switching uses Ctrl/Command+Alt+Left/Right or 1-9, while focused Space
tabs use Left/Right for selection and Alt+Left/Right for reordering.

Writes apply explicit operations to the latest SQLite state in an immediate
transaction. Reads preserve malformed/future metadata and refuse to silently
replace it. Names, identifiers, assignments and pin counts are bounded. Unknown
project/task/Space references are refused. No agent or shell is launched by
organization changes. The native manager guards pending saves and unfinished
name/icon edits. Real filesystem/project ownership remains in existing services.

## Acceptance ledger

- Organization storage and concurrent/restart safety: tests written, pending run.
- Native Space manager, sidebar assignment/pinning and keyboard interaction: pending.
- Application compilation and changed-package formatting: pending.
- Original/native screenshot comparison and other platforms: pending.

This is partial F7/F8/F2/I10 progress. Pointer drag/drop, custom Void presentation,
per-Space standalone chats, import flows, rich project metadata/scripts and the
remaining open roadmap work are not claimed complete. This batch is not a 100%
feature-parity or release-readiness claim.
