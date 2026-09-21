# Device + Settings implementation receipt

Date: September 21, 2026.
Session branch: `astra/device-settings`.
Exact integration base: `980d86b59a1f06636a55aa0b75b71eef41fd3841`.
No additional feature branch, PR, release or main-branch change is authorized.

## Implementation boundaries

- Runtime: owned and bounded ADB/simctl adapters, strict discovery parsing,
  distinct real/emulated/unknown/stale/unauthorized/unsupported states, supported
  lifecycle commands, PNG capture, target-bound transient input grants and numeric
  input allowlists. Apple code remains in the Apple adapter.
- Workspace: bounded capture decoding, shared frame validation, aspect-fit pointer
  mapping, persisted native preferences and normalized effective shortcut conflicts.
- Native app: Device Environment tab, selection/refresh/capture, metadata, consent,
  focused controls, stale-reply fencing, error recovery, hidden-view cancellation,
  session-preserving shutdown and explicit unsupported presentation.
- Settings: startup restore, attachment-list presentation, editable shortcuts,
  actual agent/config controls, contrast, notification preference/test, actual
  secret-store status, trace clearing and guarded archived-thread deletion.
- Storage/controller: deletion reserves against prompts, serializes with session
  creation, bounds session close and checks archive state inside the SQLite writer
  transaction. No filesystem deletion or implicit provider-history erasure.

The Settings inventory and unsupported matrix are in
[Device and Settings](../ui/device-settings.md). Existing profile/usage information,
SSH setup, appearance profiles and Glass/Zen behavior were preserved.

## Local evidence before publication

PASS: exact source retrieved from the session branch through a read-only source
checkpoint. The snapshot workflow ran no tests. Git bundle HEAD matched
`be611337195eb826def7d3d57688cb5b8f491239` after the checkpoint-only commit.
PASS: `git diff --check`.
PASS: `python scripts/check_roadmap.py` reports 17 lanes, 120 tasks and 16 checked.
Original task bodies/checkboxes are unchanged. Historical receipts remain intact.
PASS: changed Rust delimiter scan, excluding string/comment tokens. This is only a
structural check, not a Rust parser, build or native acceptance test.

Local Rust/cargo/rustfmt are unavailable. The sandbox cannot resolve GitHub or
Rust hosts. Publication uses the authenticated GitHub connection. A narrowly scoped
session-only publication/validation workflow is prepared, not a broad test matrix.
Its transport files and the temporary snapshot trigger are removed/restored before
integration. Test results must be recorded separately below, not assumed from source.

## Focused validation plan

- `cargo test --locked -p synara-runtime --lib device_tools`
- `cargo test --locked -p synara-runtime --lib device::tests`
- `cargo test --locked -p synara-workspace --lib device_capture`
- `cargo test --locked -p synara-workspace --lib settings::`
- `cargo test --locked -p synara-workspace --lib environment::tests`
- `cargo test --locked -p synara-workspace --lib device_settings_tests`
- `cargo check --locked -p synara-app --bin synara-app`
- Format only new Rust modules, preserving unrelated historical formatting.

Prepared regressions cover discovery/identity classification and rejection,
unsupported Apple runtimes, input bounds and hostile keys, owned command output
and cancellation, capture corruption/geometry, effective shortcut collisions,
settings reopen and default authority, startup restoration and archived/busy deletion.
Unit fixtures are test-only and are not product device-success evidence.

## Acceptance ledger

| Gate | Status |
| --- | --- |
| Exact base and one session branch | PASS |
| Source implementation and local structural checks | PASS |
| Focused Rust build/tests | Pending the scoped validation run |
| Hardware / device input / native capture | OPEN, no actual target available |
| macOS / Windows acceptance | OPEN, not exercised |
| Native GPUI device journey, keyboard/focus and accessibility | OPEN |
| Notification visible delivery and OS capture permissions | OPEN |
| Integration publication and final ref verification | Pending |

No L1-L5, full Settings, packaging or cross-platform acceptance checkbox is closed
by this source implementation. There are no fabricated device frames or native
screenshots. Agent device capabilities remain absent until a real negotiated bridge
and explicit authority path exist.
