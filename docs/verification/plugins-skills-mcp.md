# Plugins, Skills and MCP implementation receipt

Date: 2026-09-21. Session branch: `astra/plugins-skills-mcp`.
Exact integration starting point: `980d86b59a1f06636a55aa0b75b71eef41fd3841`.
Toolkit consulted: repository-intelligence, unlazy/completion gates and narrow
toolchain-preflight. No repository-local AGENTS.md exists in this source tree.
No additional feature branch, PR, release or main change is authorized.

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
Actual results, candidate identity and limitations must be appended after running.

## Remaining acceptance

Production OS keychain/Secret Service integration, authenticated remote services,
provider-owned plugin/skill lifecycle APIs, authoritative remote catalogs, full
skill bundles, OAuth, process/legacy SSE configuration, SSH reachability, broader
keyboard/IME/accessibility coverage, macOS and Windows remain open. A passing
fixture journey is not full E8/I9 or product/release acceptance.
