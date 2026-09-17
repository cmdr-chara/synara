# B/C/D session handoff

Branch: `astra/session-bcd`. Required common base:
`1cd24dd6f5ac9571c1ea2b7329bcb5fc1a4ad121`.

This is an in-progress, isolated checkpoint, not a merge or a release.
No PR is requested. Protected branches and A/J/M ownership are unchanged.

## First interaction slice

B2/B5, D5/D6: **PARTIAL**. Changes bind permission results to the original
turn token and connection-scoped elicitation to the originating RPC lifetime.
Cancellation dominates a simultaneously ready UI reply. Completion, timeout,
dropped callers and shutdown expire the request token. The first transport
failure is retained instead of being overwritten by a concurrent teardown.

Focused tests cover completed-turn approval, parent-login completion, overlapping
permissions, expired reply channels, cancellation/reply races and request-token
isolation. Candidate `a3693807deb7531959a14051c249a71f311e310e` passed all required
Linux checks and isolated native desktop smoke in
[run 35255541692](https://github.com/cmdr-chara/synara/actions/runs/35255541692).
The run was triggered by carrier `e39459f3384504493aea502fff55b33bc3f32ac2`,
but its recorded checkout and source snapshot identify the candidate above.

## Connection and session ownership slice

B2/B3/B4 and D5: **PARTIAL**, final candidate verification pending. Authentication
required now has its own protocol-independent state, distinct from a login in
progress. Authentication, logout and setup completions cannot revive a disconnected
transport. Concurrent authentication returns Busy. Dropped or timed-out login
ownership fails closed instead of leaving an invisible operation. The fixture's
logout capability now uses the pinned protocol's structured `auth.logout` field.

One connection rejects duplicate task/session ownership before a second setup RPC.
Registration and disconnect share a state-map boundary. Concurrent session close
sends one remote close. Manager shutdown waits for an in-flight connect and then
closes it, rather than detaching a slot that can later acquire a live process.
An invalid restart does not disconnect the existing connection.

Local Linux x86_64, pinned Rust 1.98.1: `cargo test --locked -p synara-agent
-p synara-acp --all-targets` and focused all-feature Clippy passed. Thirteen new
external-process lifecycle tests cover login success/failure/timeout/owner-drop,
process exit, negotiated logout, concurrent sessions, cancellation and permission
isolation, close, duplicate ownership and initialization cleanup. Three manager
regressions cover shutdown/acquisition, invalid restart and auth-state reuse.
The initial three handshake test failures were a test-observability mistake:
extension method names are deliberately redacted in traces. Tests now wait for
the fixture's redacted inbound handshake without weakening production redaction.

## Open gates

B1/B6/B7 and D1/D2/D3/D4/D7: **NOT TOUCHED** in these checkpoints.
C1/C2: prior evidence retained, not re-executed. C3/C4 authenticated workflows:
**BLOCKED**, no user-authorized vendor credentials were supplied to this session.
C5/C6: **NOT TOUCHED**. No real agent has been executed in this session.
Credentials used: **no**.

Native Linux checks run through the isolated BCD workflow. The source publisher
accepts only integrity-checked diffs on this branch, checks its required ancestry,
rejects other-lane paths and refuses a stale remote tip. Verification uses the
actual emitted source commit rather than the package-carrier commit.

## Integration boundary

These slices change `synara-agent` and `synara-acp`, plus BCD verification and
documentation. Shared edits: `synara-core/src/model.rs` adds the explicit
`AuthenticationRequired` enum variant, and the conversation connection header
uses that state instead of matching error text. No shared runtime API changes. No runtime, PTY, SSH, terminal-rendering or
terminal-test changes. The existing source publisher remains unchanged.

A/J/M request: inspect terminal spawn's post-approval lifetime check separately.
`CallbackServices::create_terminal` currently obtains `session.interaction()` again
after asynchronous spawn. A cancelled turn can end before that second lookup,
which may observe the session lifetime instead of the original turn. This session
has not changed terminal spawn or process ownership. Capture the original turn
ownership before approval/spawn and recheck that same token after spawn, then
prove cleanup under the A/J/M terminal tests.

Integration order: take the BCD source commits as a coherent branch delta on the
common base, resolve only explicitly shared shell/core/controller changes, then
integrate A/J/M and rerun full workspace, native interaction and SSH checks.
Do not integrate the transient source package. No integration was performed here.
