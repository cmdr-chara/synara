# Parity batch 13: worktree lifecycle and fork environments

M22 partial: an unassigned Synara-generated scratch worktree can be cleaned up
from the existing fork-environment menu after a separate review. The backend
rechecks the source task, local workspace, exact project worktree, generated
`synara/<uuid>` branch, matching `worktree-<uuid>` checkout, scratch parent and
live Git worktree metadata under the lifecycle lock. Removal uses the existing
typed `git worktree remove` operation without force. Dirty or locked checkouts
are retained, and the generated branch is deliberately retained. Automatic
cleanup after task archival/deletion and wider crash reconciliation remain open.

M24 partial: the source-message worktree menu is now a unified fork-environment
chooser. It offers the current workspace as a no-checkout unsent fork, existing
linked worktrees, recoverable managed worktrees and reviewed new local worktrees.
SSH sources keep current/existing choices but mark new managed checkout creation
unavailable instead of failing after selection. Existing task ownership and
selection-generation guards remain unchanged.

M29 partial: ODT is accepted as a binary document snapshot when the selected file
is a bounded ZIP archive with a readable `content.xml`. Extraction reads only
the document content stream, rejects embedded XML entity declarations, ignores
package relationships, embedded objects and external resources, and projects
bounded plain text. Composer preview/prompt context and Studio Library use the
same extractor. Original ODT bytes stay local unless explicitly exported through
the existing file flow.

M30 partial: the bounded text-preview snapshots that were previously session-only
are now stored per Hub in the workspace database. Identical refreshes are
deduplicated, each text snapshot is limited to 128 KiB, and the ledger keeps at
most 12 entries and 1 MiB total. Reopening Studio and previewing the file restores
the retained versions. Source files are never rewritten, and task deletion removes
the version ledger.

M27 partial: the typed Computer Use action contract now includes window-relative
pointer movement and double-click. Move emits only a reviewed window-addressed
pointer move; double-click reuses the bounded click path with exactly two clicks.
Both retain coordinate validation, fresh-frame identity checks, target
revalidation and one-shot frame consumption. No global pointer fallback or
arbitrary key sequence was added.

M26/S15 partial: direct-model execution now records a bounded durable route event
immediately after the turn starts, using the reviewed provider and model IDs.
Usage events received from the provider stream are also snapshotted onto the
currently active turn. The native activity summary shows the exact direct route
and reported input/output tokens. ACP turns may show provider-reported token
counts, but provider/model identity is deliberately left unknown rather than
inferred from thread-level configuration. Account quota, billing and ACP
per-turn route attribution remain open.

Validation pending with final focused checks.
