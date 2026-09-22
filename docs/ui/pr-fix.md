# PR Fix: reviewed unresolved-comment draft

The native workflow uses the existing Pull Requests, task and draft owners.
Live authenticated GitHub accounts and broader platform acceptance remain separate
from the focused Linux/native fixture checks described below.

## Workflow

Select the target chat and its project. In Pull Requests, load that project,
select its GitHub repository and open the PR. Choose "Review unresolved fixes...".
The existing PullRequests owner reads review threads through the selected host's
GitHub CLI. It does not create a task, switch checkouts or invoke an agent.

The native review shows repository, PR number, exact reviewed head, target chat,
task/project IDs, collection time, unresolved-thread count and comment count.
Its separate instruction editor does not overwrite the PR comment/review editor
or the target chat's existing draft. Edit the proposed instructions as needed.

"Recheck and add to target draft" explicitly reads the same bounded review again.
If the head, resolution state, comment identities, bodies, replies, paths, lines
or review context changed, insertion is rejected and edits remain available.
A stale review must be explicitly discarded and collected again. Copy useful
edits before discarding. No silent refresh overwrites edited instructions.

After a successful check, the instructions append to the original chat's normal
draft. The existing draft owner handles debounced persistence and save failures.
The composer remains editable and requires the normal explicit Send action.
Adding a fix review does not send, execute tools, resolve comments, submit a PR
review, merge, push or change the local checkout.

Review edits are transient until added to the chat draft. Closing the app is
blocked while a fix review is open until it is added or explicitly discarded.
After restart, saved chat drafts remain unsent. No review or agent work restarts.

## Ownership, stale callbacks and trust

Collection captures the selected task and selection revision. Draft insertion
checks project/root identity, exact task/thread ownership, current non-archived
state, draft-loading state, IME state and selection revision again. Task/workspace
ownership is re-read from the workspace service before and after verification.
SSH reads reuse the loaded pinned workspace and existing host/profile boundary.

Every consumed PR response advances its request generation. Duplicate, cancelled
and stale responses cannot append the same instructions twice. Navigation during
a read rejects the callback rather than redirecting it to a different chat.
An existing chat draft is appended to, never silently replaced.

External comments are JSON-quoted review data, explicitly not authority. The
prepared text preserves Node IDs, canonical comment URLs, file identity, current
and original line ranges, diff side, outdated status, diff hunks and original/
current commit IDs where supplied. Missing locations stay null. There is no
provider-name capability guessing or approval/session/secret inheritance.

The prompt asks the agent to compare the current checkout HEAD with the reviewed
head before proposing changes and stop on a mismatch. This is a reviewed prompt,
not a new filesystem permission boundary or proof that the checkout matches.
Comments can change after insertion. The prompt is explicitly a timestamped,
reviewed snapshot, not a live claim about GitHub state at eventual Send time.

## Bounds and failure behavior

At most 500 review threads are scanned in 20 pages. A snapshot accepts at most
32 unresolved threads and 128 comments overall, with at most 20 comments per
unresolved thread. An incompletely paginated thread fails collection rather than
silently omitting replies. Review context is limited to 256 KiB, edited fix
instructions to 512 KiB and the combined normal chat draft to 1 MiB.

Bodies are limited to 16 KiB and diff hunks to 32 KiB per comment. Existing API
limits retain 2 MiB stdout, bounded stderr, a per-request deadline and owned
process cancellation. The complete collection has a 60-second deadline. GraphQL
partial errors, malformed identities, unsafe paths, forged links, duplicate IDs,
repeated cursors, head/state changes and missing pagination state fail closed.
No automatic retry or remote write occurs.

## Verification entry points

Focused tests cover exact unresolved identity/context, GraphQL truncation,
head/open-state fencing, fingerprint changes, path/URL/input bounds, explicit
unknown/outdated lines, query-only request encoding and pre-spawn cancellation.
App tests cover task/thread/project/directory/archive ownership fencing and bounded, non-destructive draft composition.

Run in an environment with the pinned Rust toolchain and native dependencies:

```sh
cargo +1.98.1 test --locked -p synara-workspace pull_requests
cargo +1.98.1 test --locked -p synara-app --bin synara-app shell::pull_requests
cargo +1.98.1 check --locked -p synara-app
```

`scripts/native_sprint2_pr_fix_smoke.py` drives real GPUI controls and the real
Git/process owner against an owned offline `gh` fixture. It covers two-page
collection, identity/context retention, edited instructions, changed comments,
changed head, cancellation, explicit insertion, empty reviews, confirmed discard,
normal draft persistence/restart and no agent, local Git or remote write.

```sh
python3 scripts/native_sprint2_pr_fix_smoke.py --binary target/debug/synara-app \
  --fixture target/debug/synara-acp-fixture --output /tmp/pr-fix-native-evidence
```

The fixture is not evidence of live-account interoperability. Broader cross-task,
SSH, IME, accessibility and non-Linux journeys remain acceptance work. Verification
receipts must identify the exact candidate and actual test/native results.

Schema reference inspected: GitHub GraphQL Pulls reference,
https://docs.github.com/en/graphql/reference/pulls (September 22, 2026).
