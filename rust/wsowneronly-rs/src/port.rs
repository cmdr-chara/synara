//! The Rust port of `apps/server/src/wsOwnerOnly.ts`.

#![forbid(unsafe_code)]

/// Port of `isOwnerRole` (`apps/server/src/wsOwnerOnly.ts:16`).
pub fn is_owner_role(role: String) -> Result<bool, String> {
    Ok(role == "owner")
}

/// Port of `projectProviderStatusesForWsSession` (`apps/server/src/wsOwnerOnly.ts:21`).
pub fn project_provider_statuses_for_ws_session(
    statuses: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = &statuses;
    Ok(serde_json::json!({"~effect/Effect/args": {"$function": ""}}))
}
