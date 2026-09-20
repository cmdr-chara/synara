# Native repository workspace delivery

Date: 2026-09-20. Starting revision: fe0fc11430892ca4c2ddcccd153b00e2d6046e6d.
Integrated parent: 48bf3f9d9f3a9684fd00ea655ec880292505af02.
Upstream reviewed: e7cd15281e6d16cf8fc55a91496dcff035475e54.

## Delivered source

The existing Changes tab now hosts a Repository panel rather than a second shell.
It uses Synara's semantic surfaces, icons, wrapping controls and contained list
regions inside the existing Environment split/maximize lifecycle. Navigation
between Branches, Remotes, Worktrees and Stashes does not execute a mutation.

Branches have search, current-branch identification, object metadata, creation
from HEAD/full refs/object IDs, switching, renaming and merged-only deletion.
Remote controls configure named URLs and expose explicit single-branch fetch,
fast-forward-only pull and non-force push. Worktrees show their branch, location
and lock state, with creation, path copy and non-force removal of eligible linked
worktrees. Stashes can save tracked changes, optionally include untracked files,
and apply a captured object ID without dropping the stash.

Each action opens a review form with its actual target and consequence. Deletion
requires an exact typed name/path. Repository filters/helpers require a separate
execution grant. Network actions default to HTTPS and disabled credential helpers.
Configured noninteractive credentials and SSH transport are separate choices.
Hooks and signing remain disabled under the existing backend policy. There is no
shell command field, automatic retry, force push/delete, reset, stash pop/clear,
agent approval bypass or credential storage. Backend failures retain the form and
explain that rollback is not automatic.

Panels retain their originating project/root and form state across navigation.
The app cannot close while an action is running or a form is unfinished. Reads
and mutations run on the existing runtime and host-aware GitOperations service,
with its timeout/output/input policies. Worktree path handling never interprets
an SSH path on the local host. Search and rows are bounded to 400 shown items.

## Integration and ownership

The branch advanced during work. The concurrent 48bf3f9 multi-editor, search,
Markdown preview, adaptive Explorer and command-palette changes are preserved.
An overlapping local editor-search implementation was not published. No existing
working checkout was overwritten and no force update or PR was used. Main,
archive/pre-rewrite-main-2026-09-17, releases and default branch settings are untouched.
The shared Markdown renderer receives the existing pinned formatter correction,
not a second rewrite. No Zeron/MonoCode source, assets, text or dimensions are used.

## Verification state

Feature implementation precedes broad/native tests as explicitly requested.
Source review, transfer identity, Python/roadmap structure and whitespace checks
are distinguished from compilation and native execution. The local container has
no Rust toolchain and DNS resolution for static.rust-lang.org failed. The existing
CI configuration is unchanged. No full suite was manually started or repeatedly
polled to delay feature development. New focused model regressions are present
but native interaction, platform and authenticated remote acceptance are pending.
No new native screenshots are claimed for this batch.

The earlier Git diff/draft implementation passed its recorded f527aff Linux
fixture run. That evidence does not prove the new Repository controls or the
concurrent editor changes. Browser, attachments, Side chats, Automations, remote
Pull Requests, hunk staging/conflict resolution and broader platform parity remain
open. Repository forms are retained in memory and block orderly close, not durable
crash-recovery records. Review/commit drafts retain their existing durable storage.

## Upstream disposition

Relative to b58f273, e7cd152 adds passive delegated gateway result delivery with
human-send reservation/replay fingerprint safeguards (4a886e9), and a macOS icon
persistence fix (e7cd152). Both are recorded as deferred native gaps. Existing
vendor-specific Claude context-budget and Artifacts work remains deferred rather
than represented by fake generic ACP capabilities.
