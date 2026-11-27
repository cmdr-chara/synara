# Agent gateway contract

The agent gateway is Synara's public automation surface: the MCP endpoint that
agents, scripts and external orchestrators use to drive threads, kanban,
automations and hub work. This document is the stability contract for that
surface. It exists so a client written against it keeps working across releases.

## Transport

The gateway speaks the stateless subset of the MCP streamable-HTTP transport:

- `POST /mcp/agent-gateway` with a JSON-RPC 2.0 body; every POST gets a single
  `application/json` response (no SSE streams, no server-side session state).
- Methods: `initialize`, `ping`, `tools/list`, `tools/call`, plus notification
  acknowledgement. Unknown methods return `-32601`.
- Protocol versions accepted: `2025-06-18`, `2025-03-26`, `2024-11-05`. The
  server answers `initialize` with its preferred version; a client that pinned
  an older supported version keeps it.

## Authentication and caller identity

- Bearer token in the `Authorization` header. Two token kinds exist:
  - the **master token** (`SYNARA_AGENT_GATEWAY_TOKEN`) — server operator
    credential, unscoped: tools that require a calling thread refuse it;
  - **thread-scoped tokens** — issued to agent turns; the caller thread is part
    of the token's identity and is never accepted from the request body.
- Clients must never send a thread id expecting it to be trusted as identity.
  Thread ids in arguments address *targets*, never the *caller*.

## Stability guarantees

1. **Tool names are frozen.** A `synara_*` tool, once shipped, keeps its name.
   Renames ship as a new tool plus a deprecation window on the old one.
2. **Arguments are additive.** New optional arguments may appear; existing
   argument names, types and meanings never change. Required arguments never
   become optional or vice versa in-place.
3. **Result keys are additive.** Clients must ignore keys they do not know.
   Removing or retyping a key is a breaking change and ships only with a new
   tool version.
4. **Error strings are not contract.** A failed call returns `isError: true`
   with human-readable text. Clients branch on success/failure, never on
   message text; the text may change in any release.
5. **`tools/list` is the discovery surface.** Clients should enumerate rather
   than hardcode; descriptions there are the canonical per-tool documentation.

## Idempotency and replay

Mutating tools accept a caller-supplied `requestId`. A retried call with the
same `requestId` replays the original outcome instead of applying twice:

- `synara_create_thread` / `synara_create_threads` derive the thread id
  deterministically from the request id, so a retry returns the same thread.
- Hub work submissions freeze task keys at first decision: a retry addressing
  the same key replays the same worker rather than forking a new one, and a
  retry naming a *different* task under a frozen key is a conflict.
- Project reports (`synara_project_report_result`) and automation results are
  receipted: the stored receipt wins over any duplicate delivery.

Clients must generate a fresh `requestId` per logical operation and reuse it
across retries of that operation only.

## Security model

- **Approval boundaries are never bypassed by the gateway.** MCP connector
  changes are proposals: `synara_add_mcp_connector` / `synara_remove_mcp_connector`
  write a pending proposal the user resolves; nothing connects directly.
- Provider credentials never pass through the gateway. Authentication to
  providers is owned by the official provider binaries.
- Repository files, provider output and tool results are untrusted data: they
  must never authorize tools or cross approval boundaries on their own.
- Library copies are contained inside the project library root; symlink and
  traversal escapes are refused.

## Behavioral notes clients can rely on

- **Threads are the unit of work.** Kanban cards *are* threads; automations
  run threads; hub workers are threads. Anything that can wait, report or be
  diagnosed does so through the thread tools.
- **`synara_wait_for_threads` pins the turn set it was called with**: later
  turns do not extend the wait. Terminal states are `completed`, `failed`,
  `interrupted`, `cancelled`, `error`.
- **`synara_read_thread` cost telemetry** is informational and additive:
  absence (`null`) means no usage was recorded, not zero cost.
- **`accepted` does not mean `started`.** `synara_create_threads` from a hub
  coordinator enqueues durable work items; admission depends on worker slots
  and hub state.
- **Memory notes are dated files.** `synara_project_remember` deduplicates by
  content containment, extends in place when a new note strictly contains an
  existing one, and indexes into `memory/MEMORY.md`. `synara_project_forget`
  only removes dated notes — hand-written memory is never garbage-collected.

## Non-goals

The gateway does not expose provider internals, raw runtime-event ingestion,
or server administration. Those stay internal surfaces with no stability
guarantee.
