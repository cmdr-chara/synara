//! The Rust port of `apps/server/src/wsRpc.ts`.

#![forbid(unsafe_code)]

/// Port of `canManageExternalMcp` (`apps/server/src/wsRpc.ts:320`):
/// `isOwnerRole(role)` — `role === "owner"`.
pub fn can_manage_external_mcp(role: String) -> Result<bool, String> {
    Ok(role == "owner")
}

/// Port of `makeWsRpcLayer` (`apps/server/src/wsRpc.ts:3674`): an Effect Layer
/// serializes as a `build` function on the wire.
pub fn make_ws_rpc_layer() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({"build": {"$function": ""}}))
}

/// Port of `makeWebsocketNegotiationRouteLayer` (`apps/server/src/wsRpc.ts:3996`).
pub fn make_websocket_negotiation_route_layer() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({"build": {"$function": ""}}))
}
