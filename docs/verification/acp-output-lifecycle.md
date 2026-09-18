# ACP output lifetime and resource bounds

Roadmap scope: B5 and the protocol-output portion of M3/N6. This checkpoint does
not close the complete B5 matrix or claim completion of B, M or N.

## Defect and contract

At base `6f830d9d6268c2f445acb64d38b24c4c8851f310`, request timeout/drop removed
pending IDs and expired interactions, but the output channel retained plain byte
vectors. A frame could therefore execute after its request owner had expired.
The count-only queue also allowed 64 maximum-size frames to accumulate.

Output frames now own their request lifetime and a shared byte reservation.
A cancelled frame is skipped before its first write. Cancellation after a partial
write closes the connection rather than appending a new message to incomplete
JSON. Once the complete newline-terminated frame is delivered, cancellation does
not disconnect sibling sessions. No claim is made that dropping a delivered RPC
undoes remote work. Prompt cancellation still uses the existing session/cancel
protocol path.

The shared output budget is `2 * (MAX_FRAME + 1)` bytes, including serialization
reservations, queued bytes and the active write. Per-frame serialized JSON remains
limited to 8 MiB. This is not a total-process-memory limit and does not account for
upstream serde_json Values. Reservations are acquired before serialization and
released on validation failure, cancellation, queue closure and writer teardown.

Primary transport reference, reviewed September 18, 2026:
https://agentclientprotocol.com/protocol/v1/transports

## Verification contract

```sh
cargo +1.98.1 test --locked -p synara-acp rpc::outbound
cargo +1.98.1 test --locked -p synara-acp
cargo +1.98.1 fmt --check
cargo +1.98.1 clippy --locked --workspace --all-targets --all-features -- -D warnings
```

`rpc_outbound_tests.rs` exercises timeout and task abort before writer startup,
partial-write cancellation using a one-byte duplex stream, delivered-frame
cancellation with a healthy sibling, oversized serialization, a full shared byte
budget, a closed queue, pre-cancelled input and shutdown during a budget wait.
The first two tests synchronize on a queued frame rather than assuming a sleep
means the writer is blocked. Existing request-lifetime tests remain unchanged.

State at initial publication: implementation and regressions published for CI,
verification pending. Acceptance requires the exact candidate's native backend
matrix, full Linux workspace and native desktop/SSH regression results. No vendor
credentials are used by these deterministic tests.
