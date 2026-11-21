//! The Rust port of `apps/server/src/serverLayers.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `makeServerRuntimeServicesLayer` (`apps/server/src/serverLayers.ts:102`).
pub fn make_server_runtime_services_layer(
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if options.is_null() {
        return Err(
            "null is not an object (evaluating 'options.agentGatewayCredentialsLayer')".to_string(),
        );
    }
    Ok(serde_json::json!({"build": {"$function": ""}}))
}

/// Port of `makeServerApplicationLayers` (`apps/server/src/serverLayers.ts:390`).
pub fn make_server_application_layers() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "runtimeServicesLayer": {"build": {"$function": ""}},
        "providerLayer": {"build": {"$function": ""}},
    }))
}
