# Native backend threat model

Last reviewed: 2026-09-18.

This model covers the native Rust rewrite on `astra/gpui-clean-rewrite`. It is
a trust-boundary inventory, not a claim of OS sandboxing. New browser/device/
updater adapters must update this document when their authority changes.

## Protected assets

- user repositories, workspaces, Git state and unsaved file content;
- conversation history, prompts, tool output and diagnostic content;
- agent/provider credentials and Synara-owned secret values;
- SSH host trust, identity files and remote workspace identity;
- installed agent binaries, registry receipts and reviewed distribution origin;
- native process ownership and the user's local/remote execution boundary.

## Repository and filesystem boundary

Workspace filesystem access is rooted through capability-owned filesystem
services. Paths are validated as relative workspace paths, guarded writes detect
external modification, and destructive explorer operations require explicit
backend calls rather than interpretation of agent text. Symlink/traversal and
remote/local identity tests are part of runtime/workspace coverage.

Threats: traversal, symlink substitution, TOCTOU replacement, binary/oversized
input, permission failure, accidental local fallback for a remote path, and an
agent trying to turn displayed text into a filesystem action.

Invariant: a failed remote operation never becomes a local filesystem operation.

## Agent / ACP boundary

Only `synara-acp` depends directly on ACP schema packages. The adapter validates
pinned stable requests/responses, bounds protocol frames/queues/pending requests,
owns process lifetime and translates protocol values to Synara domain types.
Unknown extension methods do not become arbitrary host commands. Protocol
payloads are not retained by the inspector.

Threats: malformed/oversized stdout, contaminated protocol streams, duplicate or
forged request IDs, callback floods, stalled consumers, cancellation races,
stderr floods, hostile tool output and agent crashes.

Invariant: agent text/tool output is data. It does not itself grant filesystem,
terminal, browser, registry, Git or persistent-consent authority.

## Permission and elicitation boundary

Permission and typed-input requests are scoped to the connection/session/turn
that created them. Cancellation, session close and connection failure expire
stale interaction ownership. Responses are validated against offered choices or
schemas before returning to the agent.

Threats: replaying stale approvals, cross-task response confusion, unoffered
permission IDs, unsafe URL interactions and implicit promotion from one-shot to
persistent consent.

Invariant: consent is explicit and scoped. One-time consent is never silently
converted to persistent consent.

## Process, environment and terminal boundary

Launch arguments remain structured until an actual shell/SSH boundary. Generic
local launches inherit a narrow environment allowlist rather than arbitrary
credential variables. Owned processes and PTYs have bounded shutdown paths.
Terminal paste/control input is a separate user interaction boundary.

Threats: shell injection, environment substitution, orphaned descendants,
partial-start leaks, unbounded output and hidden execution through repository
configuration.

Invariant: opening a workspace or restoring state does not execute a saved
prompt or automatically launch an agent.

## Registry / installation boundary

Registry parsing, review fingerprints, digest verification, extraction limits,
path/link/device rejection, private staging and atomic publication separate
catalog browsing from installation authority. Managed installations retain
receipts and cannot be removed while durable tasks still depend on them.

Threats: origin substitution, missing/incorrect digest, archive traversal,
symlink/device entries, decompression/resource exhaustion, interrupted or
simultaneous install/update and executable/receipt tampering.

Invariant: browsing/importing metadata never launches an agent.

## Git boundary

Git operations use typed plans, bounded process output and explicit mutation,
repository-execution and network policy. Destructive force operations are not
part of the typed service. Repository hooks, signing and credential helpers are
disabled by default unless explicitly allowed for the operation.

Threats: unusual filenames/pathspec injection, hooks/configured helpers,
credential leakage in diagnostics, concurrent index changes, remote protocol
confusion and cancellation after partial mutation.

Invariant: Git cancellation is not represented as transactional rollback, and
remote Git failures do not fall back to local Git.

## SSH / remote boundary

Pinned SSH uses explicit known-host and identity files, rejects symlinks and
unsafe OpenSSH expansion characters, disables ambient SSH configuration and
forwarding, and keeps local/remote workspace identity distinct. Explicit port
forwarding is loopback-only with a separate consent object and
`ExitOnForwardFailure`.

Threats: changed host keys, ambient agent/X11 forwarding, multiplexed trust,
option injection, remote command quoting, detached remote descendants, port
exposure and silent local fallback.

Invariant: host trust is fail-closed and forwarding is explicit, loopback-bound
and separately owned.

## Browser boundary

The Rust browser domain owns tab/profile/history/consent state and a bounded
operation vocabulary. Manual, agent-task and authentication profiles use
separate storage partition identities. The policy foundation does not expose a
generic page-to-host command bridge.

Threats: malicious page content, redirects/scheme confusion, cookie or
credential crossing between contexts, popup authority inheritance, oversized
IPC, downloads/uploads and automation without consent.

Invariant: web content does not acquire workspace/process authority from being
rendered. Native engine adapters remain separately qualified per platform.

## Settings, secrets and platform services

Settings are versioned, bounded, non-secret SQLite preferences. Invalid/newer
settings recover without overwriting the original. Secret values are not
serializable, Debug is redacted, owned bytes are cleared on drop and unavailable
or locked OS credential stores fail closed. Native dialogs/notifications use
typed platform-service requests.

Threats: plaintext fallback, secret logging/export, malformed settings,
unbounded preference payloads and shell-based substitutes for native UI
services.

Invariant: a secret-store failure never causes a plaintext settings/database
fallback.

## Diagnostics and privacy

ACP trace entries retain timing, direction, standard method identity, hashed
request IDs and structural payload shape only. Stderr contents are replaced by
byte counts. Inspector exports omit raw connection errors, authentication
descriptions, host labels and protocol payloads, and enforce entry/export bounds.

No automatic telemetry is part of this backend. Conversation/file-content
exports are separate operations and may contain sensitive user data.

## Device and updater boundaries

Portable device and updater implementations are not complete. Mocks, compile
checks or architecture documents must not be described as real Apple hardware,
native credential, installer, signing or production update acceptance.

Future device helpers must have bounded IPC, explicit user-directed input and
owned shutdown. Future updater work must authenticate artifacts, preserve
incompatible user data, handle interruption/rollback and use owner-approved
signing identities/endpoints.

## Residual platform evidence

Linux interaction and controlled SSH evidence do not establish macOS/Windows
interaction behavior. Cross-compilation does not prove native terminal,
credential, browser, installer or accessibility behavior. Apple device support
requires an actual approved Apple environment. Production signing/update policy
requires owner decisions.
