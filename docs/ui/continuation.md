# Native UI continuation

## Scope and recovery boundary

Continue the independently authored Rust/GPUI Synara UI on
`astra/gpui-clean-rewrite` only. Emanuele's Synara is the product/design reference,
including the user-supplied home-screen capture. No PR, merge, release, default
branch change or legacy source import is authorized by this mission.

The earlier UI working tree at `/mnt/data/ui-work/native-integrated` was reported
as locally tested but not published. On this continuation, both the container and
Python access paths returned `TransportTimeoutError`, including a trivial Python
probe. Recovery is therefore unverified, not complete. Do not reset, clean or
replace that working tree. If it becomes accessible, archive its tracked diff and
untracked source before integrating, then compare against the current branch.
Do not treat old screenshots or reported local tests as current-branch evidence.

## Baseline repair

Starting candidate: `10ed1cc3ebed66c9e8a67162fa06be8ae98d652d`.
Native workflow `35398466421`, job `105773940974`, identified:

- the outbound RPC tests still constructed `State::incoming_ids` as a set after
  the production state moved to an `incoming` cancellation-token map;
- an unused `StorageError` import prevented strict Clippy;
- the Windows process test had one unformatted method chain.

The repair aligns the test harness with the production state without changing
protocol behavior, removes the unused import and applies that formatting change.
It does not remove tests, relax lints or mark a UI milestone complete.

## Completion gates

| Gate | Required evidence | State |
| --- | --- | --- |
| Recover earlier local UI work | Readable worktree, preserved source archive/diff, comparison with published branch | BLOCKED: execution environment access |
| Restore the integrated verification baseline | Exact-candidate format, check, strict Clippy, tests, structural checks and native smoke | OPEN: CI required |
| Publish native UI foundation and backend integration | Source diff, real service calls and focused regressions | OPEN |
| Verify visual fidelity | Native renders compared with Emanuele's references at equivalent sizes | OPEN |
| Preserve branch and lineage | Non-forced update on authorized branch, independent root audit | OPEN: recheck at final checkpoint |

A blocked recovery gate does not prevent safe changes to the published source.
Any new overlapping UI implementation must be reconciled explicitly with the
recovered local work later. No visual-parity, release or cross-platform UI claim
is made by this continuation record.
