# ACP cleanup under event backpressure

Scope: B2/B5, D4 and M4 backend lifecycle behavior. Product UI and Q are excluded.
Baseline: `aaf6183b42be6f3b07013b78e746030eba986626`.
Verification date: September 18, 2026.

## Reproduced defects

1. `CallbackServices::stop_terminals` killed and delivered output for one terminal
   before signalling the next. A blocked event sink kept other owned PTYs alive.
2. `Connection::failed` delivered session errors before cleaning callback PTYs.
   Blocked error consumers delayed terminal cleanup even after transport failure.
   Session cancellation tokens already inherited transport cancellation. The
   defect was delayed process cleanup, not loss of that token inheritance.
3. `AcpSession::cancel` delivered `CancellationRequested` before its wire
   notification. Delivery failure prevented the cancel notification from reaching
   the external agent.

All three regressions fail against the original production implementation and pass
with this checkpoint. Fixtures are isolated, credential-free processes.

## Implemented invariants

- Signal every drained terminal before awaiting process exit or diagnostic output.
- Use a shared three-second exit budget and shared fifteen-second output budget,
  rather than multiplying each budget by the terminal count.
- Return cleanup and diagnostic failures rather than silently reporting success.
- Clean callback terminals before delivering connection-failure errors.
- Deliver cancellation to the agent even when the presentation/event sink fails.
- Exclude new prompts while cancellation cleanup/delivery is pending, preventing a
  late cancellation event from clearing a newer turn's interactions.
- A failed connection exposes incomplete cleanup/diagnostic delivery in its
  observable state without reviving a disconnected connection.

`NativeTerminal::kill` is a nonblocking stop flag plus worker wakeup on both
implemented platform paths. This change does not claim Windows Job Object or
complete macOS descendant-ownership acceptance.

## Deciding regressions

- `callbacks::lifecycle_tests::terminal_cleanup_stops_every_process_before_delivering_output`
- `cleanup_integration::cancellation_reaches_agent_before_blocked_diagnostics_and_excludes_next_turn`
- `cleanup_integration::connection_failure_reaps_callback_terminal_before_blocked_error_delivery`

The last test observes the isolated terminal PID disappearing from Linux `/proc`
while error delivery remains blocked. The multi-terminal test runs on Unix.
Cancellation ordering is portable and remains in the native backend CI matrix.

## Local verification

Environment: `Linux x86_64`, Rust/Cargo 1.98.1.
Dependencies were taken from the checksum-verified pinned CI input artifacts and
all local Cargo commands ran offline.

```text
cargo fmt --check                                                   PASS
cargo check --locked --workspace --exclude synara-app               PASS
cargo clippy --locked --workspace --exclude synara-app \
  --all-targets --all-features -- -D warnings                         PASS
cargo test --locked --workspace --exclude synara-app \
  --all-features --no-fail-fast                                      PASS
  245 passed, 0 failed, 15 intentionally ignored
python scripts/test_audit_workspace.py                              PASS (12 tests)
python scripts/audit_workspace.py                                   PASS
python scripts/test_apply_source.py                                 PASS (6 tests)
python scripts/check_roadmap.py --self-test                          PASS (9 tests)
python scripts/check_roadmap.py                                     PASS
git diff --check                                                   PASS
```

Ignored tests require opt-in vendor or controlled SSH environments. No vendor
credentials were used and no authenticated agent journey is claimed.
Native application and macOS/Windows verification are CI gates, not local results.
This is an accepted Linux cleanup slice, not completion of the entire B-P mission.
