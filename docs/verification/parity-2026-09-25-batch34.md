# Parity continuation verification - batch 34

Date: 2026-09-25

## Accepted

### A06 Real macOS Simulator/device acceptance

GitHub Actions run `36175979603`, job `108207088049`, completed successfully
on hosted macOS arm64 for exact candidate
`f62150e05b276523e74d861cd2db4a5cb79d86f6`.

The live acceptance test used Apple's installed CoreSimulator runtime through
Synara's Apple Simulator backend. It:

- discovered an available iOS Simulator;
- booted it when necessary and waited for Synara to observe it as ready;
- captured a real Simulator frame through Synara and verified PNG output;
- opened an HTTPS URL through Synara;
- restored the Simulator to its prior stopped state when the test had booted it;
- preserved the exact candidate identity and uploaded the run evidence.

The only later branch change before this receipt touched the native terminal
confirmation view and does not modify the Simulator runtime/backend or this live
acceptance test. This accepts A06 as the real-platform proof for the currently
implemented Simulator/device workflow. The broader D3 gate and M13-M16 remain
open for the still-missing live streaming, richer input, recording and
accessibility-targeting feature scope.

### A07 SSH worktree/search acceptance

GitHub Actions run `36176621086`, job `108209006017`, completed successfully
on the current branch head
`e1cd2ebb99c33668fcdae78df7869736a725462e`.

The isolated loopback SSH acceptance exercised the actual pinned OpenSSH
transport and native GPUI workflow. Evidence includes:

- 11 passing `synara-runtime` SSH transport/filesystem/search/PTY tests;
- 7 passing `synara-workspace` managed remote worktree/Git lifecycle tests;
- host-key and approved-identity rejection paths with no local fallback;
- remote name/content search over the pinned SSH helper;
- managed remote worktree lifecycle and dirty-recovery preservation;
- native remote-panel enrollment, guarded remote file save, direct terminal
  input and terminal restart;
- native remote content/name search and clean terminal shutdown before window
  close.

The run uploaded its exact-candidate evidence bundle and completed with no
ignored acceptance tests in those two live SSH suites. A07 is therefore
accepted. Broader D5, D12 and N2 remain open only where their own full parity
criteria extend beyond this acceptance item.

## Still open

A01, A04, A05, A08, A09 and A10 remain open. They require live microphone and
provider accounts, authenticated browser state, signed release infrastructure,
representative multi-provider interoperability, or broader cross-platform
visual/accessibility/save-picker evidence that these runs do not provide.
