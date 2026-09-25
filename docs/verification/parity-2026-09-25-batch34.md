# Parity continuation verification - batch 34

Date: 2026-09-25

## Accepted

### A07 SSH worktree/search acceptance

GitHub Actions run `36200443462`, job `108285773063`, completed successfully
for exact candidate `7105eb0b0cab24cf89d481ebfa2c31e6c8731000`.

The isolated SSH acceptance lane passed:

- 11 `synara-runtime` SSH tests covering pinned host identity, guarded remote
  filesystem/search, PTY behavior, reconnect/disconnect handling and transport
  boundaries;
- 7 `synara-workspace` SSH tests covering pinned Git operations, managed remote
  worktree lifecycle and cleanup/recovery, forwarding, mutations and trust
  failure behavior;
- the native GPUI remote journey with persisted pinned enrollment;
- guarded remote file editing and save;
- remote content search and filename search;
- direct remote terminal input;
- explicit stop-and-restart with a replacement SSH PTY;
- application close while a foreground remote command was active, with terminal
  ownership shut down before the native process exited.

The native journey reported:

`PASS: remote native UI smoke: remote-panel-pinned-enrollment, remote-files-guarded-save, remote-content-and-name-search-over-pinned-ssh, remote-terminal-direct-input, remote-terminal-restart, terminal-shutdown-before-window-close`

## Acceptance fixes made while closing the gate

The close flow now resumes automatically after a required terminal-layout save
instead of cancelling the user's quit request permanently. Save errors and
interactive terminal confirmations still fail closed and keep Synara open.

The A07 harness now:

- explicitly confirms a running-terminal restart;
- exposes that confirmation through opt-in layout diagnostics;
- waits for a newly rendered replacement terminal surface rather than treating
  append-only layout logs as disappearing state;
- routes terminal lifecycle, confirmation UI and the A07 harness itself through
  the dedicated SSH acceptance lane.

## Other CI

The same candidate passed the repository roadmap/formatting workflow. Separate
broader regression jobs still contain unrelated pre-existing workspace test
compilation failures and are not used as A07 deciding evidence.

## Still open

A01, A04, A05, A06, A08, A09 and A10 remain open.
