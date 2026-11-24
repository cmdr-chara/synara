# wsconnectionsessions-rs

Purpose: correlate authenticated WebSocket upgrades with RPC handler execution. Layer: server transport support  RpcServer.toHttpEffectWebsocket forks the RPC server on the layer-build scope, so services provided around the per-connection HTTP upgrade effect never reach handler fibers.

napi-rs bridge for `apps/server/src/wsConnectionSessions.ts`.

## Functions

- `withoutWsConnectionSessionHeaders` (`apps/server/src/wsConnectionSessions.ts:44`)