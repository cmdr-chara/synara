# wsowneronly-rs

FILE: wsOwnerOnly.ts Purpose: Owner-role admission and account identity filtering for WebSocket RPC. Layer: server transport support  Distinct from the external-MCP guard in wsRpc.ts, which additionally requires a loopback-only instance: the account RPCs are owner-only on any bind — the machine account is owner state on remote hosts too — so the check here is the session role and nothing else.

napi-rs bridge for `apps/server/src/wsOwnerOnly.ts`.

## Functions

- `isOwnerRole` (`apps/server/src/wsOwnerOnly.ts:16`)
- `projectProviderStatusesForWsSession` (`apps/server/src/wsOwnerOnly.ts:21`)