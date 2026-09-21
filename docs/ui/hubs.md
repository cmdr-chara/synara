# Optional Hubs: first native integration

Date: 2026-09-21. This source checkpoint continues Glass/branching commit
`ae64983de333b3026931336ad37a3fc8d422d3e6`. Product upstream was reviewed at
`e7cd15281e6d16cf8fc55a91496dcff035475e54`, unchanged.

## Product and presentation

Hubs replace the Studio-facing workspace with an optional place for related work.
Synara's existing normal chats, project threads, Spaces, editors and terminals
remain usable without creating a Hub. A Hub can use a chosen local folder or a
Synara-managed folder, so a Git repository is not a prerequisite.

The native home is a title, a short action row, thread rows and shared-context
information. It is not a dashboard of nested cards. The sidebar lists Hubs with
related threads. Long lists have display bounds and a thread-search escape route.
Zen remains a reversible presentation preference, not a third task scope. Zeron
is a Zen-only UX research reference. No competitor code, assets, theme values or
screen composition is imported into this implementation.

## Implemented source behavior

Creation registers a local folder through the existing workspace service, then
commits the Hub profile, Main task and empty unsent draft in one transaction.
Folder registration is a prior idempotent step, not part of that transaction.
A duplicate Hub for the same registered project is rejected. No agent is started.

The context editor supports name, description, instructions and curated shared
knowledge. Saves compare the expected revision with the stored profile. Conflicting
or failed saves retain the editor contents for copying/reconciliation. Navigation
and orderly app close guard unfinished Hub editing and pending creation/saves.

New threads can start with shared instructions and knowledge in their visible,
editable unsent draft. This is not a hidden system prompt or automatic send.
Existing conversations receive updated context only through an explicit Add to
draft action. Promoting a completed assistant message opens the context editor with
quoted text and source IDs, requiring Save before it becomes shared knowledge.
There is no automatic transcript harvesting, RAG or model-generated memory here.

Archive/restore changes the Hub profile flag. Archiving neither deletes files and
conversations nor stops running processes. New Hub thread creation is refused while
the Hub is archived. Existing task and approval ownership remains unchanged.

The Library reuses the existing contained local file browser, text/Markdown and
bounded PNG/JPEG previews. Completed tool diffs from up to 64 peer Hub threads in
the exact same working directory supply bounded reporting metadata. The scan limits
attribution to 4096 paths and marks truncated work. A reporting-thread action opens
the corresponding conversation. A reported path is not proof of authorship, and
another worktree is not treated as the same file authority. Remote Library previews,
binary attachment intake, moving/copying files between Hubs and comprehensive output
provenance remain open.

The composer also gains measured, bounded vertical growth for multiline text,
using its existing native input and scroll ownership rather than replacing it.

## Compatibility contract

Current state: Studio tasks use the existing Project, task, draft, session and
working-directory records. Target: optional Hubs add shared context and Library
navigation while retaining those identities and service owners.

The first stage uses a versioned `hub:<ProjectId>` preference. Existing Studio tasks
are projected into an imported Hub when no profile exists. Merely reading that
projection does not rewrite tasks or store default metadata. Invalid or future Hub
metadata produces an error rather than being replaced by a legacy/default profile.

The serialized `TaskScope::Studio` discriminant and `show_studio` settings key are
retained during this compatibility stage. They are implementation details, not a
claim that every Studio-facing label is already removed. Settings labels and some
older search descriptions still need migration. Existing files, IDs, transcripts,
drafts, provider sessions and permissions are not copied or rewritten.

Rollback to the preceding source can still read the task/draft records as Studio
work. It does not interpret the extra Hub profile preference. No destructive schema
migration is introduced. Removal of the legacy discriminant requires a separately
reviewed migration with old/new round trips, backups, restore and native navigation
acceptance. Until that evidence exists, retain compatibility rather than deleting
old data or declaring a full Studio removal.

## Remaining integration and acceptance

This is a Hub foundation, not the whole proposed product. Hub-scoped Kanban is not
wired in this checkpoint, and an unwired shortcut was removed instead of exposing
a visual-only control. Global Kanban remains unchanged. Multiple repositories,
connected sources, semantic retrieval, autonomous orchestration, scheduled tasks,
PR context and side-by-side Side chats remain separate roadmap work. Hubs do not
make local agents run in the cloud or while the host computer is off.

Three focused Rust regressions are included for legacy projection, context/revision
and archive/reopen/backup behavior, and preservation of malformed metadata. They
have not run in this sandbox. Native compilation, live focus/IME, concurrent saves,
restart journeys and screenshots remain unverified because the pinned Rust toolchain
and native executable are unavailable. Source inspection and the local roadmap check
are not substitutes for those acceptance gates. No GitHub test workflow was
dispatched, and workflow configuration is unchanged.

ROADMAP.md retains all 120 original task bodies and their checkbox states. The
preceding complete roadmap and feature inventory are archived without deleting
historical evidence. F1/F2/F8/F9 and D9/D10/D12 own the remaining Hub behavior.
I7/I10 continue to own Glass/Zen native acceptance. No broad gate is closed here.
