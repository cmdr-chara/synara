# Pull Requests

The native Pull Requests pane discovers GitHub.com repositories through the
existing Git service and runs authenticated provider operations using the
selected workspace host. It does not add a second Git implementation or silently
checkout, fetch, stage, commit or modify local index/working-tree state.

Load an explicit project and select one of its discovered repositories. List,
search, state filters and pages are bounded. Detail includes title/body, author,
base/head, metadata, commits, changed files, activity and checks where GitHub
provides them. Provider failures remain visible. File changes use Synara's
shared native diff presentation. Opening a corresponding local file uses the
existing editor surface, not a PR branch checkout.

Create, comment, review, draft/ready transitions, close/reopen and merge require
explicit native confirmation. Merge is guarded with the reviewed head SHA.
Loaded project/workspace identity and request generations are pinned so changing
selection cannot redirect a pending action. Timeouts, cancellation and bounded
process output use the workspace's existing host/process ownership.

The source does not establish live account acceptance, enterprise/GitLab support,
full inline review parity, unlimited remote pagination or durable Hub/thread
association. Those remain open. Tests use controlled provider fixtures and real
local Git repositories, including preservation of pre-existing staged and
unstaged content. See the session verification receipt for exact results.

## Reviewed PR Fix context

Use PR Fix on the selected PR to collect unresolved review threads. The reviewed
context preserves repository/PR/head/comment/file/line identity and deterministic
ordering. Edit the instruction and append explicitly to the destination's unsent
draft. Sending is a separate ordinary conversation action. Cancel does not launch
anything. A stale head or changed review set requires fresh review before append.

The existing provider/host performs only bounded reads for this workflow. Incomplete
pagination fails closed. No checkout, staging, review submission or merge occurs.
See [feature-closure workflows](feature-closure.md#pr-fix) for limits and ownership.
