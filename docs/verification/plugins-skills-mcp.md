# Plugins, Skills and MCP implementation receipt

Date: 2026-09-21. Session branch: `astra/plugins-skills-mcp`.
Exact integration starting point: `980d86b59a1f06636a55aa0b75b71eef41fd3841`.
Toolkit consulted: repository-intelligence, unlazy/completion gates and narrow
toolchain-preflight. No repository-local AGENTS.md exists in this source tree.
No additional feature branch, PR, release or main change is authorized.

## Executed checkpoint

The implemented management slice is integrated into `astra/gpui-clean-rewrite`.
The non-force update was confirmed by a GitHub ref read at
`12e93d86bceea7c7dc8a37ffe6da4ed95686badd`. It contains the tested reconciliation
`1ce1e419dc7d2081d04f8cacfbfcfb3a4d029693` plus removal of the temporary publishing
workflow. The final documentation-only successor does not change executable
sources, test scripts, dependencies or toolchain. This receipt records the observed
integration checkpoint, not a self-referential claim about its own commit hash.

The final focused run passed 38 Rust tests, the native application build, and six
native journey assertion groups on Linux/X11. Eight native screenshots were
inspected. This proves the bounded slice below, not full E8/I9 completion, vendor
interoperability, production credentials or cross-platform acceptance. The initial
pending ledger is retained as history and superseded by the executed evidence.

## Architecture and scope

The direct owners are `synara-workspace::integrations`, its storage/controller
modules, the generic `ContextServer` contract, and native `shell::integrations`.
The existing preference store, secret-store injection, task/Hub ownership and ACP
session negotiation remain in charge. No provider runtime or global task grant
was added. Shared Settings/shell/module declarations are the principal merge
collision surfaces. Historical roadmap task bodies remain unchanged.

[Feature guide and limitations](../integrations.md) describes exact support,
including document-only skills, unknown external plugin state, local-task HTTP
MCP, temporary discovery evidence, credential-store unavailability and logical
rather than OS-level scope isolation.

## Completion ledger at initial source publication

| Gate | Check | State / evidence |
| --- | --- | --- |
| Branch lineage | GitHub ref and create_branch exact SHA | PASS: session created from `980d86b` |
| Plugins ownership-aware UI | Source plus native Plugins screenshot | Source implemented, native execution pending |
| Reviewed Skills lifecycle | Storage tests and native import/enable/draft/remove/restart | Prepared, execution pending |
| Scoped MCP lifecycle and consent | Storage/controller tests, native config/test/remove | Prepared, execution pending |
| MCP protocol evidence | Real loopback HTTP fixtures, modern/legacy/error/SSE tests | Prepared, execution pending |
| Secret boundary | Synthetic locked/unavailable stores, redaction and persistence tests | Prepared, production OS store remains unavailable |
| Documentation integrity | Roadmap historical-body comparison and script | Local whitespace and roadmap self-test passed before publication |
| Integration and ref | Non-force integration plus final GitHub ref read | Pending |

The local environment has no Rust compiler/toolchain or direct repository network
route. A narrowly scoped source bundle was fetched through the GitHub connector.
The reviewed SHA-256 source package is applied using the repository's existing
publisher, restricted explicitly to this one session branch. Formatting is limited
to the new session-owned Rust modules, not a broad baseline rewrite. Focused CI
runs at the exact resulting commit and exports its source bundle and native
artifacts. The temporary source workflow is removed before final integration.

## Focused verification commands

```sh
rustfmt +1.98.1 --edition 2024 --check crates/synara-workspace/src/integrations.rs crates/synara-workspace/src/storage/integrations.rs crates/synara-workspace/src/controller/integrations.rs crates/synara-app/src/shell/integrations.rs
cargo +1.98.1 test --locked -p synara-workspace --lib integrations
cargo +1.98.1 test --locked -p synara-acp --lib wire::tests::capabilities_use_protocol_advertisements_not_agent_names -- --exact
cargo +1.98.1 test --locked -p synara-acp --lib wire::tests::negotiated_session_parameters_cover_directories_and_mcp_transports -- --exact
cargo +1.98.1 build --locked -p synara-app --bin synara-app -p synara-acp --bin synara-acp-fixture
python3 scripts/native_integrations_smoke.py --binary target/debug/synara-app --fixture target/debug/synara-acp-fixture --output /tmp/plugins-native
python3 scripts/check_roadmap.py
```

The native script uses an owned Xvfb desktop, real input controls, readonly SQLite
assertions, and a literal-loopback MCP fixture. It does not seed the integration
catalog using SQL, use vendor credentials or claim full provider interoperability.
Executed results, candidate identity and limitations are recorded below.

## Executed verification and cleanup

[Final focused run 35635894653](https://github.com/cmdr-chara/synara/actions/runs/35635894653)
ran on Ubuntu 24.04.5 with Rust 1.98.1, locked dependencies and two build jobs.
The workflow trigger was `88ebbcc45b73874d4b3511e7bc8e43884eae40b5`, but its source
job produced and the focused job actually checked out
`1ce1e419dc7d2081d04f8cacfbfcfb3a4d029693`. Job `106453261181` completed successfully.
This candidate includes the current Device/Settings integration `0b2d1ec` through
a reviewed merge. It preserves both sessions' Settings and controller ownership.

| Check on the tested candidate | Observed result |
| --- | --- |
| Integration storage/controller and loopback HTTP tests | 20 passed, 0 failed, 0 ignored |
| Two exact ACP negotiation regressions | 1 passed each, 0 failed, 0 ignored |
| Settings regression filter | 14 passed, 0 failed, 0 ignored |
| Device/Settings deletion and shutdown filter | 2 passed, 0 failed, 0 ignored |
| Native application and ACP fixture build | Passed |
| Native integration journey | 6 assertion groups passed |
| Session-module rustfmt, roadmap structure, Python compilation | Passed |
| Candidate remained unchanged during checks | Passed, `git diff --exit-code` |

The 38 Rust tests are distinct tests in selected filters, not a full workspace run.
The build retains one existing `ROW_HEIGHT` dead-code warning in `synara-app/src/ui.rs`.
No clippy, full workspace, macOS or Windows pass is claimed. Reconciliation also ran:

```sh
cargo +1.98.1 test --locked -p synara-workspace --lib settings::
cargo +1.98.1 test --locked -p synara-workspace --lib device_settings_tests
```

The native journey executed a real GPUI process on a private Xvfb/X11 display,
used native input, and checked persisted state read-only. Its six groups proved:

1. Plugins ownership is visible without automatically launching an agent or
   requesting a network connection.
2. Explicit MCP testing negotiates with an owned loopback HTTP server and lists
   advertised tools without executing tools or enabling the saved connection.
3. Editing disables a connection. An unavailable secret store blocks testing before
   network access, without a plaintext fallback.
4. Removal requires native confirmation and removes the local record without a
   claim to revoke the external provider's credentials.
5. A local skill review retains the source and hash. Installation is disabled,
   enabling is explicit, search filters actual state, and insertion remains an
   unsent draft rather than an agent operation.
6. Restart restores the library and draft. Removal leaves the original file intact.

The eight captures are `plugins-ownership.png`, `mcp-probe-evidence.png`,
`mcp-secret-store-unavailable.png`, `skill-review-origin.png`,
`skills-installed.png`, `skills-filter-empty.png`, `skill-unsent-draft.png` and
`skills-restored-narrow.png`. Full captures are 1420 x 930, and the restored Skills
capture is 1100 x 800. Visual review found no clipped or overlapping in-scope
controls in those captured states. This does not establish a complete keyboard,
IME, screen-reader, native picker or platform layout matrix.

[Native evidence artifact](https://github.com/cmdr-chara/synara/actions/runs/35635894653/artifacts/10656641125)
contains the result, logs and captures. Its ZIP SHA-256 is
`bdadc8e8559e576417e345490e80d82c5049a85e0b53b02599ac8f354343839c`.
The source artifact is `10656032217`, ZIP SHA-256
`8c56b34e59250f295d90a7c87eea97ddafd340dc6bb1258aedd729e809c908a9`.
The [machine-readable ledger](plugins-skills-mcp-checks.json) preserves identities,
counts, native assertions, screenshot hashes and evidence boundaries after artifact
retention expires. Native artifacts expire September 28, source artifacts September 23.

### Earlier failures and corrective evidence

| Run | Tested source | Outcome and correction |
| --- | --- | --- |
| 35631827192 | `4f45529` | 18 integration tests and two ACP tests passed. Build failed on two new UI type/visibility errors and five pre-existing appearance type errors. Native journey skipped. Corrected before subsequent builds. |
| 35632926645 | `88a33fc` | 19 integration tests, two ACP tests and build passed. Native endpoint-input check failed because the test input driver did not enter the requested punctuation. |
| 35633596858 | `9612b4d` | 20 integration tests, two ACP tests and build passed. Native endpoint-input check still failed. The remaining slash keysym was corrected explicitly instead of weakening the exact-input assertion. |
| 35634340215 | `8acf1e9` | 20 integration tests, two ACP tests, build and six native journey groups passed after the input-driver correction. |
| 35635894653 | `1ce1e41` | Reconciliation candidate passed all 38 selected Rust tests, build and six native groups. |

These were corrective source revisions, not unexplained retries to obtain a green
badge. No external service credentials or provider-installed plugin were involved.

### Completion gates for the supported slice

| Gate | State and deciding evidence |
| --- | --- |
| Branch lineage and concurrent integration | PASS: exact base `980d86b`, reviewed merge `1ce1e41`, non-force integration confirmed at `12e93d86` |
| Ownership-aware Plugins surface | PASS: source inspection plus native ownership/no-launch/no-network assertion and screenshot |
| Reviewed local Skills lifecycle | PASS: storage tests, native review/install/enable/filter/insert/restart/remove. Native update picker remains outside this evidence. |
| Scoped HTTP MCP lifecycle | PASS: storage/controller tests plus native configuration, test, disable-on-edit and confirmed removal |
| Protocol discovery and defensive parsing | PASS: real loopback HTTP fixtures, advertised tools, bounded JSON/SSE, malformed/auth/redirect/legacy cases |
| Secret references and unavailable-store refusal | PASS: synthetic boundary tests and native refusal. A production OS store remains unavailable, not passed. |
| Documentation and historical roadmap preservation | PASS: source checks, nine roadmap self-tests, all 120 original full task bodies and checkbox states compared unchanged |
| Cleanup | PASS: session publisher and transfer payloads absent. `12e93d86` changes only the temporary workflow relative to the tested candidate. |

Executable-equivalence anchors are the `crates` tree
`e29742a2ad69eac3bed449d92d22c737274ee2ae`, `scripts` tree
`6658c19a8e6751e7db3b07e60a38a33f17c64483`, and the lockfile/toolchain objects recorded
in the JSON ledger. Cleanup and documentation do not require repeated native
builds when these exact tested inputs remain unchanged. Final remote refs must
still be read after publishing the documentation successor.

## Remaining acceptance

Production OS keychain/Secret Service integration, authenticated remote services,
provider-owned plugin/skill lifecycle APIs, authoritative remote catalogs, full
skill bundles, OAuth, process/legacy SSE configuration, SSH reachability, broader
keyboard/IME/accessibility coverage, macOS and Windows remain open. A passing
fixture journey is not full E8/I9 or product/release acceptance.
