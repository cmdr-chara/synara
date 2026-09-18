# ACP SDK 2.1 stable/unstable audit

Evidence date: 2026-09-18

Synara pins `agent-client-protocol = =2.1.0`. That SDK release pins `agent-client-protocol-schema` 1.7.0 and keeps ACP protocol v1 as the stable wire version. Synara does not enable the SDK `unstable` umbrella or `unstable_protocol_v2`.

## Stable v1 surface

The pinned SDK registers these Client-to-Agent requests: `initialize`, `authenticate`, `logout`, `session/new`, `session/load`, `session/resume`, `session/list`, `session/delete`, `session/close`, `session/prompt`, `session/set_mode`, and `session/set_config_option`. Synara validates each supported request and response at `synara-acp/src/schema.rs` before translating it into protocol-neutral domain values.

Stable control notifications include the session-specific `session/cancel` notification and the protocol-level `$/cancel_request` notification. Synara uses `session/cancel` for prompt cancellation and owns `$/cancel_request` IDs for Agent-to-Client callbacks, cancelling only the matching callback and returning JSON-RPC error `-32800` without disconnecting sibling work.

Stable Agent-to-Client callbacks covered by the adapter include permission requests, file reads/writes, terminal create/output/wait/kill/release, and elicitation. Session updates are translated in `wire.rs`; unknown extension notifications remain ignorable rather than escaping protocol types into domain crates.

Session config options are the stable model-selection mechanism. The upstream protocol removed the never-stabilized `session/set_model` method on 2026-06-01. Synara therefore maps `AgentSession::set_model` only to a `category: model` config option and treats a session without such an option as unsupported. Legacy `models` response fields are ignored at the ACP boundary.

Terminal authentication is stable in schema 1.7.0 but capability-gated. Synara currently does not advertise the separate terminal-auth capability, so conforming agents must not send terminal auth methods to Synara. Agent-handled authentication, retry, logout, disconnect, and restart remain supported. Adding terminal-auth UI/PTY orchestration is tracked separately and must not be claimed from the normal terminal callback capability.

## Unstable surface in SDK 2.1.0

The SDK 2.1.0 `unstable` feature umbrella contains `unstable_end_turn_token_usage`, `unstable_llm_providers`, `unstable_mcp_over_acp`, `unstable_plan_operations`, `unstable_session_compaction`, `unstable_session_fork`, and `unstable_tool_call_name`. Protocol v2 is a separate `unstable_protocol_v2` opt-in. Synara enables none of these features in its ACP dependency.

Consequently, stable MCP server declarations supplied during session setup are supported when negotiated, while MCP-over-ACP is not advertised. Session compaction remains outside Synara's stable pinned surface and is intentionally not advertised. Dedicated v1 modes remain supported, while config options are preferred for model/configuration state.

## Isolation evidence

`scripts/audit_workspace.py` resolves normal, dev, build, target-specific, and aliased Cargo dependencies and fails if any crate other than `synara-acp` depends on an ACP protocol package or imports an ACP alias. Protocol types are decoded and encoded inside `synara-acp`; `synara-agent` and `synara-core` expose only Synara-owned connection/session/capability/configuration contracts.
