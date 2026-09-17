# EKOP verification and remaining work

Work branch: `astra/session-ekop`. This is an independently integratable development
checkpoint, not a release and not acceptance of all E/K/O/P tasks.
Common base: `1cd24dd6f5ac9571c1ea2b7329bcb5fc1a4ad121`.

## Verified registry/browser checkpoint

Actual tested source: `643fc364b09daa9ee081e375a9750ca4f8236bee`.
Workflow trigger: `314acdbff87c7c3465378e0994d79d2ca05d3e6e`.
[CI run 35266023184](https://github.com/cmdr-chara/synara/actions/runs/35266023184)
completed successfully on September 17, 2026. The source job formatted owned Rust
files and published the actual source revision above. Every downstream checkout
used that revision, not a moving branch name.

| Evidence | Result | Limits |
| --- | --- | --- |
| Registry native macOS arm64 | Formatting, registry/consumer compilation, registry Clippy and tests passed | Not the complete macOS GUI or signing/distribution path |
| Registry native Windows x64 | Same registry checks passed | Not full Windows GUI/ConPTY acceptance |
| Registry native Linux x64 | Same registry checks passed | No Wayland or arbitrary-distribution claim |
| Browser policy on all three targets | Standalone Rust policy tests passed | No embedded browser/DOM/capture adapter is wired into Synara |
| Linux full workspace | Formatting, compilation, Clippy, tests, publisher and roadmap checks passed | Other sessions were not integrated |
| Linux native interaction | App/fixture built, existing isolated native smoke passed | Existing smoke does not prove every new registry edge case or visual quality |
| Exact-source backup | Bundle, revision, lockfile and checksums exported | CI artifact retention is short, source commits remain on the branch |

The earlier `4e24d88...` candidate passed registry compilation/lints/tests on the
three hosts but failed formatting. That failure was corrected, not ignored.
An intervening source-export workflow commit `61aa436...` was preserved when a
non-fast-forward push correctly refused to overwrite it.

## Implemented outcomes and open block boundaries

E advances: valid offline catalog fallback after failed refresh, fixed warning
classifications, immutable plan fingerprints for update review, numeric upgrade/
downgrade classification, same-version revised metadata, native registry UI wiring,
Windows device-alias and directional-control path rejection, bounded path depth.
Existing installer and workspace APIs remain compatible. No provider credentials
or real model calls were used by this session.

E remains partial: full install cancellation/progress, cross-process active-use
removal ownership, richer custom-profile UX, interrupted install/rollback matrix,
full real-distribution platform matrix and all E acceptance conditions remain open.
Do not mark the whole E block complete from these changes.

K advances: bounded isolated Rust consent/lifecycle policy, one-shot grants,
revocation on document navigation/close/crash, expiry and negative tests. Remaining
native embedding and initial navigation/payload-binding obligations are explicit
in [browser-host.md](../architecture/browser-host.md). K1-K5 remain open.

R/S/T advances: [shared GUI specification](../architecture/zen-synaric.md) and
[roadmap supplement](../roadmap/zen-synaric.md). These document Synaric default,
global Zen switching, one state/backend and the canonical model-picker behavior.
They do not implement either new shell or the picker.

## Measurement harness

`scripts/ekop/measure.py` measures bounded process-exit duration for an explicitly
chosen finite command. It records a local candidate SHA, finite duration samples,
platform and exit/timeout status. It discards stdout/stderr, does not retain the
command/environment, refuses existing output files and never sends network data.

This is NOT a first-frame GUI startup, idle RAM/CPU, input-latency or accessibility
benchmark. Use only reviewed finite commands in an isolated test environment.
Do not use a command that deliberately daemonizes or leaves detached children.
The timeout cleanup is harness supervision, not Synara process-ownership proof.

```sh
python3 scripts/ekop/measure.py --self-test
python3 scripts/ekop/measure.py --candidate "$(git rev-parse HEAD)" \
  --runs 5 --warmups 1 --timeout 30 -- python3 -c 'pass'
```

The sample command checks harness mechanics only. It is not an application
performance result. For comparable measurements, record compiler/build flags,
hardware, operating system, workload and warm/cold conditions in a separate local
engineering record. Do not compare different workloads as an optimization claim.

`--diagnostic-preview` is explicit opt-in to a fresh allowlisted aggregate, not a
sender or automatic telemetry. It omits commit IDs, raw timing samples, paths,
commands, timestamps, output, identifiers and arbitrary error strings. Unknown
fields fail closed. A production uploader and its transport/privacy review remain
unimplemented. Raw support exports are a separate explicit consent boundary.

## Accessibility groundwork

The GUI specification requires keyboard equivalents to hover menus, semantic
focus restoration, IME-safe layout switching, logical scroll anchoring, contrast,
reduced motion and meaningful accessibility labels. O4 remains open until actual
VoiceOver, supported Windows screen-reader and Linux platform journeys are run.
No screen-reader or manual visual acceptance is claimed here.

## Integration preparation

`scripts/ekop/integration_audit.py` is read-only and reports ancestry, exact input
SHAs, path overlap, cross-owner edits, shared files and dirty worktree state.
See [integration.md](../parallel/integration.md). Its tests create disposable Git
repositories only. It does not fetch or merge the other sessions.

## Next candidate checks

The measurement/integration scripts and GUI specification are a subsequent
checkpoint. Their self-tests and applicable exact-candidate CI must pass before
claiming those outcomes verified. Native macOS/Windows application compile lanes
are groundwork only and do not close P1/P2 interaction acceptance.

After integration: shared state contracts and GUI R/T/S, remaining E lifecycle
work, native browser adapter spike, real GUI measurements/accessibility, then
secure updater/distribution with owner-provided signing/endpoint decisions.
No package, updater endpoint, signing identity, account service or telemetry
collection has been created by this checkpoint.
