# Typed Git operation backend

Status: IMPLEMENTED, CI acceptance pending. This is not a claim that H or the
backend completion mission is complete. No product-UI files are changed.

## Contract for the frontend

`synara_workspace::GitOperations` owns an execution host and workspace root.
Construct one service per host/workspace and clone it for callers so operations
share a bounded queue and serialization lock. `execute` accepts a typed
`GitOperation`, bounded `GitOperationOptions`, a `CancellationToken`, and an
optional Tokio watch sender for `GitOperationProgress`.

Operations cover branch listing/create/rename/safe delete/switch, remote
listing/add/update/remove, explicit branch fetch, fast-forward-only pull,
non-force explicit-ref push, worktree listing/add/non-force remove, stash
listing/save/apply-by-object-ID, and commit. Applying a stash retains its recovery
copy even on conflict. No force-push, hard reset, forced deletion, stash pop, or
stash clear is exposed. Existing `GitService` status/diff/stage/unstage APIs remain
unchanged and available.

All mutations require user approval. Checkout, stash, worktree and network paths
also require explicit repository-execution trust because repository filters and
configured helpers may execute programs. These policy objects are intentionally
not deserializable. Do not create a grant from agent text or tool output.

Hooks and signing are disabled by default, with explicit per-operation options to
honor the host's configured behavior. Credential helpers are disabled by default.
Approved configured helpers receive noninteractive settings and bounded execution.
No override rewrites the user's Git configuration. Network access is disabled by
default and can be granted for HTTPS, HTTPS plus SSH, or filesystem transport on
the selected host. HTTP TLS verification is enforced and automatic HTTP redirects
are disabled. Unsupported protocols and ext helpers remain disabled. Configured
remote URL rewriting and configured executable helpers belong to the separately
approved repository/host trust boundary. A network policy is not endpoint pinning
and this service is not an OS sandbox.

New remote URLs accept HTTPS, explicit ssh:// URLs and absolute filesystem paths.
Embedded HTTPS credentials, URL query/fragment parameters, helper syntax and
option-shaped inputs are rejected. Existing remotes remain readable for migration.
Raw output is bounded, untrusted and potentially private. Neither Debug, errors nor
progress copies raw stderr, command arguments or URLs. Display/export raw output
only through an explicit user-directed, escaped/redacted presentation boundary.

Default timeout is 30 seconds including queue wait, capped at five minutes. At most
16 calls per shared service are admitted. Stdout is capped at 8 MiB and stderr at
256 KiB, with lower defaults. Slow progress consumers cannot block process cleanup.
Cancellation or dropping the execution future requests process stop. An explicit
failure waits at most eight additional seconds for process-exit confirmation.
`may_have_mutated` warns that Git cancellation is not transactional rollback.
`cleanup_confirmed` describes owned process exit, not remotely detached descendants.
Remote errors never retry against the local filesystem or local Git.

## Acceptance evidence to collect

The added unit/integration suite covers input and policy boundaries, unavailable
remote Git, queue saturation, queued cancellation/deadline, branch lifecycle,
unmerged deletion refusal, dirty checkout, index lock retention, worktree recovery,
stash recovery, local bare-remote fetch/pull/push and divergence, cancellation,
future-drop process ownership, output/deadline limits and opt-in hook execution.

Run the existing Linux/macOS/Windows Backend acceptance workflow and the Linux
Native verification and SSH workflows against the final formatted candidate.
Record actual results here before closing any roadmap acceptance gate.

Authenticated network Git and real signing identities are not fixture evidence.
No credentials, production repositories, PRs or releases are used as test fixtures.

## Primary references

- https://git-scm.com/docs/git-config
- https://git-scm.com/docs/git-fetch
- https://git-scm.com/docs/git-push
- https://git-scm.com/docs/git-stash
- https://git-scm.com/docs/git-worktree
