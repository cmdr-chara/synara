# Parity batch 23: provider-native ACP session fork

Implementation commits:
- `f128ef218424c55248909f1109cd4e8c25d5d71a` — capability/schema/runtime support;
- `46587b2833060d22a0628c555230bb31eddc6453` — whole-session product action and fallback;
- `91f5c4b31c5eec7bde98558ff19b80c02009a944` — product-level native/fallback regressions;
- `8ec39a28f6445f96e161e56e399279706cd7a210` — legacy capability decode default.

## S08 complete

Synara enables only the pinned `agent-client-protocol = 2.1.0`
`unstable_session_fork` feature. Provider names never imply support. ACP
initialization records native fork only when the agent advertises
`sessionCapabilities.fork`, and Synara additionally requires resume or load
recovery before issuing a fork.

The ACP boundary validates `session/fork` request/response with the pinned
schema. A fork can start only from an owned, open, idle source session. The
returned session ID must differ from the source and from every already-owned
session. The forked session is registered under the requested child ThreadId
before it is returned.

## Product fallback and ownership

Native fork is exposed as a whole-session action in the Handoff UI only while
the currently connected source advertises fork plus recovery. It is intentionally
separate from message-level branching because the ACP extension has no
message-anchor parameter.

The controller commits the ordinary reviewed retained-context child first. It
will attempt native fork only when the source already has a saved provider
session reference matching the current task route/root. No empty source session
is silently created just to claim native fork.

On success, the child stores the forked provider session reference and a short
visible unsent continuation draft; no prompt is sent. The source session remains
unchanged.

On unsupported capability, provider rejection, timeout/ambiguous outcome,
session persistence failure or child ownership failure, native fork is never
automatically retried. The already-created retained-context child remains intact
as the fallback. A successfully created fork that cannot be persisted is closed
best-effort rather than left as hidden client ownership.

## Focused verification

ACP fixture coverage verifies advertised fork, new-session identity and the rule
that fork without load/resume recovery sends zero `session/fork` requests.
Controller fake-provider coverage verifies both native child-session attachment
and provider rejection fallback with exactly one fork attempt and exactly one
child task.

Source audit confirms the UI capability gate, fallback-first order, source-ID
checks, recovery gate, no conflict markers and legacy deserialization defaulting
`fork_session` to false.

No workflow status is attached to these commits, and this environment does not
expose a Rust toolchain/checkout. This receipt therefore does not claim a fresh
cargo, rustfmt or Clippy pass. S08 is complete as a product feature; D12 remains
OPEN for live provider/worktree acceptance.
