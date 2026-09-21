# Device + Settings implementation and validation receipt

Date: September 21, 2026.
Session branch: `astra/device-settings`.
Integration branch: `astra/gpui-clean-rewrite`.
Exact starting head: `980d86b59a1f06636a55aa0b75b71eef41fd3841`.
Final tested source: `744830ec608b806b6fad11e6dd10e84bebead172`.
Final tested tree: `61f75b4c00e9e87a5e99cd1ad303db69247297c6`.
Only the requested session branch was created. No main edit, PR or release.

## Implementation boundaries

- Runtime: owned and bounded ADB/simctl adapters, strict discovery parsing,
  distinct real/emulated/unknown/stale/unauthorized/unsupported states, supported
  lifecycle commands, PNG capture, target-bound transient input grants and numeric
  input allowlists. Apple command construction stays in the Apple adapter.
- Workspace: bounded PNG decoding and RGBA expansion, existing frame validation,
  aspect-fit pointer mapping, compatible persisted preferences and normalized
  effective shortcut conflicts. Device joins the four-tab saved Environment layout.
- Native app: Device tab, selection/refresh/capture, dimensions/orientation, explicit
  consent, focused controls, epoch-fenced replies and hidden-view cancellation.
  Missing helper setup and asynchronous failures both revoke authority and stale the
  old list. Hiding or closing the view does not shut down devices or destroy chats.
- Settings: restore-last-chat behavior, recent-attachment-list presentation, editable
  native navigation shortcuts, actual agent/config controls, higher contrast,
  notification preference/test, real secret-store status, shared trace clearing and
  typed-confirmation archived-thread deletion. Existing appearance profiles, usage,
  SSH settings and Glass/Zen presentation remain under their existing owners.
- Storage/controller: deletion reserves against prompts, serializes with session
  creation, uses setup-before-lifetime lock order, bounds session close and checks
  archive state inside the SQLite writer transaction. Shutdown can complete while
  deletion waits for setup. No workspace files or external agent history are deleted.

The [Settings inventory and unsupported matrix](../ui/device-settings.md) separates
new work, retained functionality and gaps. No agent device tools, physical Apple
support, Android cold AVD boot, AppSnap or invented account/model data are claimed.

## Published source commits

| Commit | Purpose |
| --- | --- |
| `523ef44d6aa19ffaed15d112aac58821fdc1250f` | Runtime device helpers and desktop notification transport |
| `3aca44968932ebdad07c2c68f5f33adadbb26e47` | Native Device viewer, functional Settings, persistence and deletion safeguards |
| `2bc493a2becc0069169a7c73076b8572b230bd43` | Source inventory, roadmap, gap audit and unsupported notes |
| `9e941c56bd3b494a2445771b34199d3aea495596` | Pinned rustfmt on new modules only |
| `9dcb06a30c233ad29b7338dd361cca8a1867832d` | Runtime doc-comment correction, bounded RGBA expansion and reserved-key validation |
| `c5583bd01aaa91b3d8f2bf3ff9c8070b086f8bc0` | Explicit native appearance identifier and color-mask integer types |
| `744830ec608b806b6fad11e6dd10e84bebead172` | Deletion/shutdown lock order, missing-helper cleanup and added regressions |

Session-only source transport and cleanup commits remain in history:
`be611337195eb826def7d3d57688cb5b8f491239`,
`c923ef5c165b49ca4bd7f60464181096835d82ee`,
`0353be9b197012c322cb1a07f762e1461b8f9a21`,
`37392ee10812bf7a40e53f53345a151bd1f6b324`, and
`ab231229e0f448f40c3849657f2350b2b24d7366`.
Their temporary workflows/payloads are absent from the final tree. The historical
source-checkpoint workflow was restored byte-for-byte. No dependency, lockfile or
Rust toolchain file changed. Subsequent receipt publication is documentation only.

## Local evidence and transport

The local sandbox has no Rust/cargo/rustfmt and cannot resolve GitHub/Rust hosts.
Authenticated GitHub source bundles supplied exact reviewed source. Bundle revision,
ZIP SHA-256, patch digests and clean tree IDs were checked before publication.
The source-only checkpoint run `35627237645` ran no tests. Each publisher was guarded
to this repository, the one session branch, an exact parent and an exact reviewed
patch/tree. Pushes were non-force and only targeted the session branch.

Local PASS: `git diff --check`, roadmap structure, exact historical task body and
checkbox comparison, no workflow/script/dependency delta at the clean source head,
and independent replay of the reviewed source. The roadmap reports 17 lanes, 120
tasks and 16 checked. Those counts are unchanged, not a completion percentage.
A delimiter scan was also used but is not represented as Rust compilation evidence.

## Executed focused validation

Host: GitHub Actions `ubuntu-24.04`, pinned Rust `1.98.1`, locked dependencies.
One initial targeted run and two corrective targeted runs were used, not repeated
broad repository matrices. Full logs were downloaded and their hashes are recorded
in [the machine-readable evidence ledger](device-settings-checks.json). The ledger
also preserves every command, revision, exit code and passing test name.

| Run | Tested source | Actual result |
| --- | --- | --- |
| `35631803200` | `9e941c5` | FAILED. An incorrectly placed runtime crate doc comment introduced in this session blocked all seven Rust commands. No Rust test ran. Roadmap and clean-tree checks passed |
| `35632663192` | `9dcb06a` | 38 targeted tests PASSED. Native check FAILED on five pre-existing integer-type errors in appearance code. Both affected files were unchanged from the integration base before the fix. Roadmap and clean-tree checks passed |
| `35633699789` | `744830e` | All six check commands PASSED, including 24 affected tests and native Linux `cargo check`. The two new regressions pass. Source-only publication and cleanup also passed |

Latest passing evidence by suite:

| Command (all cargo commands use `+1.98.1` and `--locked`) | Passing tests | Source |
| --- | --- | --- |
| `cargo test -p synara-runtime --lib device_tools` | 8 | `9dcb06a` |
| `cargo test -p synara-runtime --lib device::tests` | 5 | `9dcb06a` |
| `cargo test -p synara-workspace --lib device_capture` | 3 | `9dcb06a` |
| `cargo test -p synara-workspace --lib settings::` | 14 | `744830e` |
| `cargo test -p synara-workspace --lib environment::tests` | 8 | `744830e` |
| `cargo test -p synara-workspace --lib device_settings_tests` | 2 | `744830e` |
| `cargo check -p synara-app --bin synara-app` | Compilation check PASS | `744830e` |
| `python3 scripts/check_roadmap.py` | PASS, 17/120/16 unchanged | `744830e` |
| `git diff --exit-code` | PASS, clean checked source | `744830e` |

This is 40 distinct passing targeted tests across the latest applicable runs, not
40 newly added tests or a full-suite run. Runtime/device/capture sources are identical
between `9dcb06a` and `744830e`, verified with Git, so those 16 tests were not repeated.
The native check has one unused `ROW_HEIGHT` constant warning. It is not a linked
release build, a running window, or proof of native interaction.

Regression coverage includes strict discovery/identity rejection, unsupported Apple
runtimes, hostile input keys and bounds, real owned subprocess output/cancellation,
PNG corruption/geometry/expansion, effective shortcut conflicts, preference reopen,
restore opt-out and archive handling, four-tab layout round trip, archived/busy
permanent deletion, and queued deletion versus shutdown. Fixtures are test-only,
not Android/Apple hardware or vendor-agent success evidence.

## Acceptance and integration gates

| Gate | Status |
| --- | --- |
| Exact base, one session branch and coherent source publication | PASS |
| Focused tests and Linux native compilation check | PASS at the revisions above |
| Cleanup, unchanged historical evidence and dependency/toolchain files | PASS |
| Hardware, actual Android input and native device/capture presentation | OPEN |
| macOS and Windows runtime/build/interaction acceptance | OPEN |
| Native GPUI journey, keyboard/focus, screen reader and contrast | OPEN |
| Visible notification delivery and OS window/screen capture permissions | OPEN |
| Final integration ref | Must be re-read after the non-force integration update, reported in the completion response |

The source checkpoint's earlier "prepared" validation statement is superseded by
this executed ledger. Historical receipts are preserved, not retrospectively turned
into passing evidence. Integration should fast-forward only after re-reading the
latest integration head, preserving any concurrent work. This document deliberately
does not self-assert a future integration ref. The post-update GitHub ref read is the
publication evidence.

No full L1-L5, Settings, packaging, release or cross-platform gate is closed. There
are no fabricated frames or screenshots. Device agent capabilities remain absent
until a real negotiated bridge and an explicit authority path exist.
