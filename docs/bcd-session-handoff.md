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
isolation. Candidate `a3693807deb7531959a14051c249a71f311e310e` passed all required
Linux checks and isolated native desktop smoke in
[run 35255541692](https://github.com/cmdr-chara/synara/actions/runs/35255541692).
The run was triggered by carrier `e39459f3384504493aea502fff55b33bc3f32ac2`,
but its recorded checkout and source snapshot identify the candidate above.

## Connection and session ownership slice

B2/B3 and D5: **PARTIAL**. B4: **PASS** for the Linux candidate and evidence below. Authentication
required now has its own protocol-independent state, distinct from a login in
progress. Authentication, logout and setup completions cannot revive a disconnected
transport. Concurrent authentication returns Busy. Dropped or timed-out login
ownership fails closed instead of leaving an invisible operation. The fixture's
logout capability now uses the pinned protocol's structured `auth.logout` field.

One connection rejects duplicate task/session ownership before a second setup RPC.
Registration and disconnect share a state-map boundary. Concurrent session close
sends one remote close. Manager shutdown waits for an in-flight connect and then
closes it, rather than detaching a slot that can later acquire a live process.
An invalid restart does not disconnect the existing connection.

Local Linux x86_64, pinned Rust 1.98.1: `cargo test --locked -p synara-agent
-p synara-acp --all-targets` and focused all-feature Clippy passed. Thirteen new
external-process lifecycle tests cover login success/failure/timeout/owner-drop,
process exit, negotiated logout, concurrent sessions, cancellation and permission
isolation, close, duplicate ownership and initialization cleanup. Three manager
regressions cover shutdown/acquisition, invalid restart and auth-state reuse.
The initial three handshake test failures were a test-observability mistake:
extension method names are deliberately redacted in traces. Tests now wait for
the fixture's redacted inbound handshake without weakening production redaction.

Candidate `0e9e66d46b68c27e1e50ebca1b0d63328f0c6756` passed every required
Linux workspace check, focused tests and isolated desktop smoke in the rerun of
[run 35258450705](https://github.com/cmdr-chara/synara/actions/runs/35258450705).
B4 evidence is `lifecycle_integration` (13 tests), the manager ownership tests and
`process_integration`'s two-agent/configuration coverage. One connection owns two
sessions, rejects duplicate thread IDs before sending setup, isolates permissions
and cancellation, and routes each event to its original thread.

The initial focused repeat failed the existing
`terminal_callbacks_release_live_resources_but_keep_history` assertion at
`process_integration.rs:330` (missing `terminal-proof`). The full suite in that same
run passed, and the rerun passed both. This is retained as a flaky terminal-output
handoff, not hidden by retries or claimed as a resolved runtime issue. A/J/M should
verify that terminal exit/wait drains captured output before returning a snapshot.

## Schema/configuration and custom profile slice

B1/B6 and C5/C6: **PARTIAL**, next candidate verification pending. Stable lifecycle
request/response schema validation now also covers resume6/close/list/delete,
authentication/logout, modes, config options and elicitation callbacks. The SDK
schema still does not replace adapter resource bounds and semantic validation.
The audit caught a missing `type: "boolean"` discriminator in configuration writes.
The adapter now emits it and a fixture rejects the old malformed request. Select
values retain their compatible string representation.

`custom_profile` exercises a persisted, user-defined command profile with spaces,
Unicode, literal shell metacharacters and a controlled environment. The same
backend launches both configurations. Only variable names are stored. No provider
credentials or real vendor executables are used. Focused tests and Clippy pass
locally with the pinned toolchain. The compatibility document states exactly what
this fixture-based result can and cannot prove.

## Open gates at the first three checkpoints

B7 and D1/D2/D3/D4/D7: **NOT TOUCHED** in these checkpoints.
C1/C2: prior evidence retained, not re-executed. C3/C4 authenticated workflows:
**BLOCKED**, no user-authorized vendor credentials were supplied to this session.
C5/C6: **PARTIAL**, as described above. No real agent has been executed in this session.
Credentials used: **no**.

Native Linux checks run through the isolated BCD workflow. The source publisher
accepts only integrity-checked diffs on this branch, checks its required ancestry,
rejects other-lane paths and refuses a stale remote tip. Verification uses the
actual emitted source commit rather than the package-carrier commit.

## Integration boundary

These slices change `synara-agent` and `synara-acp`, plus BCD verification and
documentation. Shared edits: `synara-core/src/model.rs` adds the explicit
`AuthenticationRequired` enum variant, and the conversation connection header
uses that state instead of matching error text. No shared runtime API changes. No runtime, PTY, SSH, terminal-rendering or
terminal-test changes. The existing source publisher remains unchanged.

A/J/M equest: inspect terminal spawn's post-approval lifetime check separately.
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


## Recovery and fourth-checkpoint verification, September 17, 2026

The recovered remote tip was `217e2ada21fa17292e452c48d465b6c457f6f113`.
GitHub comparison confirmed the required common base was its merge base, with
14 commits ahead and no commits behind. The exact source archive was recovered
from CI artifact `10515361924`, with its recorded SHA-256 verified. Its Git tree
and reconstructed commit object matched the remote object IDs exactly. The local
checkout is shallow at that recovered tip, not a new orphan rewrite or a reset of
the remote branch.

The fourth checkpoint added bounded input validation, scoped elicitation and
virtual transcript state. Its Linux CI run `35263170576` recorded a successful
application build and isolated native smoke, but failed Clippy and workspace test
compilation: two transcript assertions compared `gpui::ListOffset`, which does not
implement `PartialEq`. That run must not be described as a green candidate.

Source fix: `3e91c35a440d6af7f6ec98029738b4dc53f7d6c7`,
`test(conversation): compare virtual scroll offset fields explicitly`.
The two assertions now compare both `item_ix` and `offset_in_item`, preserving
the user-owned scroll-position checks. No production behavior, dependency, test
threshold or ignored-test setting changed. Follow-up edits only record evidence
in this handoff and the compatibility document.

### Local results

Platform: Debian 13 x86_64, Rust/Cargo 1.98.1. The previously exported pinned
public toolchain and vendored sources were recovered with verified artifact
SHA-256 digests. The exported Cargo.lock digest matches this candidate. Builds
used offline Cargo with an isolated cache and target directory.

| Check | Result |
| --- | --- |
| `cargo fmt --check` | PASS |
| `cargo test --locked -p synara-agent -p synara-acp -p synara-core` | PASS, 88 tests, 3 ignored opt-in/helper entries |
| `cargo test --locked --workspace --exclude synara-app` | PASS, 151 tests, 9 ignored opt-in/helper entries. This includes the 88 above, not 151 additional tests |
| Focused agent/ACP/core all-targets, all-features Clippy with `-D warnings` | PASS |
| `python3 scripts/test_apply_source.py` | PASS, 6 tests |
| `python3 scripts/test_bcd_source.py` | PASS, 4 tests |
| `python3 scripts/check_roadmap.py --self-test` | PASS, 9 tests |
| `python3 scripts/check_roadmap.py` | PASS, existing 3 checked tasks unchanged |
| `python3 scripts/audit_workspace.py` | Exited 0 for structure. Its root check sees the shallow boundary, so this is NOT full historical-root proof |
| Workspace `cargo check , all-targets/all-features Clippy, `cargo test` | BLOCKED, missing native `fontconfig.pc` |
| Native application/fixture build | BLOCKED by the same native dependency |
| Native desktop smoke on the recovery candidate | BLOCKED, no native candidate binary. The prior smoke is not reused as proof of this candidate |

Ordinary tests passed for persisted custom profiles, Unicode and literal argument
handling, allowlisted environment-variable names, lifecycle ownership, callback
cancellation, input validation and durable replay. Live SSH and vendor probes
were not opted into. The custom-profile helper's ignored marker is intentional:
its ordinary parent test invokes the guarded helper in a clean child process.
No vendor executable or provider credential was used in this recovery.

### Task classification and remaining work

D3: **PARTIAL**. The test compilation defect is corrected, but the corrected
GPUI tests, interactive scroll ownership and long-transcript desktop behavior
still need a runnable native verification environment. No D task is closed.
D5/D6/D7: **PARTIAL** in the fourth checkpoint, with fresh non-GUI unteraction and
validation evidence. Positive/rejected forms and permissions, stale/overlapping
requests, keyboard interaction and restoration still require native acceptance.
D1/D2/D4: **NOT TOUCHED** by this recovery. Their remaining rendering, ordering,
composer and recovery acceptance items remain open.

B1/B2/B3/B5/B6: **PARTIAL**, prior implementation retained and applicable
non-GUI tests rerun. B4 retains its previously recorded **PASS** evidence at
`0e9e66d46b68c27e1e50ebca1b0d63328f0c6756`. B7 is **NOT TOUCHED** by this
recovery, although the structural ACP dependency boundary check passed.
No additional B task is closed.

C1/C2: **NOT TOUCHED** by this recovery, previous PASS evidence retained.
C3/C4: **BLOCKED**, authenticated real-agent journeys were not run and no vendor
credentials were supplied. C5/C6: **PARTIAL**, custom-profile fixture evidence
now records a locally tested source candidate. No additional C task is closed.
No acceptance criteria or roadmap checkboxes were weakened or changed.

### Publication and integration

The recovery source commit is local only. A normal, non-force push of
`HEAD:refs/heads/astra/session-bcd` failed with exit 128 because `github.com`
could not be resolved. Explicit discovery of `create_file`, `update_file`,
`delete_file`, `create_blob`, `create_tree`, `create_commit`, `update_ref`,
`create_branch`, `create_pull_request` and `update_pull_request` did not expose a
callable file/git/PR writer in this connection. This is a tool-surface limitation,
not an inferred repository permission denial. The existing workflow only accepts
pushes, so rerunning it would recheck the old source rather than publish this fix.
The native package bootstrap also failed resolving `deb.debian.org`.

Recovery edits are limited to the two assertions in
`crates/synara-app/src/shell/transcript.rs`, this handoff and the compatibility
record. No new shared core/workspace/shell, runtime, terminal, process or SSH edit
was made. The existing branch delta includes `synara-core/src/model.rs` and
`crates/synara-app/src/shell.rs`, which remain integration-sensitive.

A/J/M requests remain the two already recorded above: drain terminal output before
returning a final snapshot, and preserve the original turn token across permission
approval and asynchronous terminal spawn. This recovery did not implement them.

Recommended order: first apply/publish the recovery delta on `astra/session-bcd`
without resetting existing work, then run full native candidate verification.
After both lanes are verified, integrate the coherent BCD source delta before
A/J/M as previously recorded, resolve shared shell/core/controller edits, and run
full workspace, native interaction and SSH checks on the integration candidate.
Do not integrate transient source-publisher packages. No merge or PR was created.
