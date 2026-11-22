# Synara server HTTP route table

Extracted 2026-11-22 from `apps/server/src` (`HttpRouter.add` declarations + `*_ROUTE_PATH` constants). 30 routes.

The bulk of Synara's API is WebSocket RPC (`apps/server/src/wsRpc.ts`), not HTTP — this table is the complete HTTP surface. Real `GET /api/*` traffic that is not listed here falls through to the static handler (currently recorded as 503s; the `GET *` wildcard).

| Method | Path | Source |
|---|---|---|
| GET | `*` | http.ts (static/dev fallback) |
| * | `/api/attachments/cancel` | http.ts |
| * | `/api/attachments/upload` | http.ts |
| * | `/api/auth/*` | http.ts (auth session/bootstrap subtree) |
| POST | `/api/desktop/computer/emergency-stop` | http.ts |
| POST | `/api/desktop/shutdown` | http.ts |
| GET | `/api/editor-icon` | http.ts |
| * | `/api/library/upload` | http.ts |
| GET | `/api/local-image` | http.ts |
| GET | `/api/mcp/external/integrations` | externalMcp/httpRoute.ts |
| POST | `/api/mcp/external/integrations` | externalMcp/httpRoute.ts |
| POST | `/api/mcp/external/integrations/pairing` | externalMcp/httpRoute.ts |
| POST | `/api/mcp/external/integrations/revoke` | externalMcp/httpRoute.ts |
| POST | `/api/mcp/external/pair` | externalMcp/httpRoute.ts |
| POST | `/api/mcp/external/runtime-challenge` | externalMcp/httpRoute.ts |
| GET | `/api/project-favicon` | http.ts |
| GET | `/api/site-favicon` | http.ts |
| GET | `/api/thread-export` | http.ts |
| * | `/api/voice/transcribe` | http.ts |
| GET | `/health` | http.ts |
| DELETE | `/mcp` | agentGateway/httpRoute.ts (405) |
| GET | `/mcp` | agentGateway/httpRoute.ts (405) |
| POST | `/mcp` | agentGateway/httpRoute.ts |
| POST | `/mcp/external` | externalMcp/httpRoute.ts |
| GET | `/oauth/mcp/callback` | mcpConnectors/httpRoute.ts |
| POST | `<AGENT_GATEWAY_STDIO_BOOTSTRAP_PATH>` | agentGateway/httpRoute.ts |
| DELETE | `<CONNECTOR_ROUTE>` | mcpConnectors/httpRoute.ts |
| GET | `<CONNECTOR_ROUTE>` | mcpConnectors/httpRoute.ts |
| POST | `<CONNECTOR_ROUTE>` | mcpConnectors/httpRoute.ts |
| GET | `<HOST_REMOTE_WS_PATH>` | remoteSessions/httpRoute.ts (WS upgrade) |

## Notes for the Rust front door

- `GET *` (static/dev fallback) is wired to `staticassets` and answers the recorded 503 body in this deployment; each recorded static path is routed in `src/units/mod.rs`.
- Auth-protected routes (401 recorded) stay proxied: a 401 is correct only for unauthenticated requests, and the glue cannot distinguish a valid session cookie from an invalid one.
- 404/empty recorded responses stay proxied: they depend on server state (missing project, absent resource).
- The WS RPC surface (`wsRpc.ts`, `remoteSessions/httpRoute.ts` upgrade path) is the real remaining backend work — the front door proxies upgrades today.
