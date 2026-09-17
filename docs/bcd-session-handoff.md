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
isolation. Evidence is pending the candidate-specific verification run.

## Open gates

B1/B3/B4/B6/B7 and D1/D2/D3/D4/D7: **NOT TOUCHED** in this checkpoint.
C1/C2: prior evidence retained, not re-executed. C3/C4 authenticated workflows:
**BLOCKED**, no user-authorized vendor credentials were supplied to this session.
C5/C6: **NOT TOUCHED**. No real agent has been executed in this session.
Credentials used: **no**.

Native Linux checks run through the isolated BCD workflow. The source publisher
accepts only integrity-checked diffs on this branch, checks its required ancestry,
rejects other-lane paths and refuses a stale remote tip. Verification uses the
actual emitted source commit rather than the package-carrier commit.

## Integration boundary

This slice changes `synara-agent` and `synara-acp` interaction code only, plus BCD
verification and documentation. No runtime, PTY, SSH, terminal-rendering or
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
