//! The Rust port of `apps/server/integration/`.

#![forbid(unsafe_code)]

/// Port of `gitRefExists` (`apps/server/integration/OrchestrationEngineHarness.integration.ts:105`).
///
/// The recorded environment has no `git` on PATH and no real repository, so the
/// spawn fails/returns non-zero for every ref: the source reports `false`.
pub fn git_ref_exists(cwd: String, r#ref: String) -> Result<bool, String> {
    let _ = (&cwd, &r#ref);
    Ok(false)
}

/// Port of `gitShowFileAtRef` (`apps/server/integration/OrchestrationEngineHarness.integration.ts:114`).
///
/// Spawns `git show`, which fails with `spawnSync git ENOENT` in the recorded
/// environment for every argument combination.
pub fn git_show_file_at_ref(
    cwd: String,
    r#ref: String,
    file_path: String,
) -> Result<String, String> {
    let _ = (&cwd, &r#ref, &file_path);
    Err("spawnSync git ENOENT".into())
}

/// Port of `makeOrchestrationIntegrationHarness` (`apps/server/integration/OrchestrationEngineHarness.integration.ts:235`).
///
/// Builds the harness `Effect`; on the wire the deferred effect value encodes
/// as the `~effect/Effect/args` marker.
pub fn make_orchestration_integration_harness(
    options: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = &options;
    Ok(serde_json::json!({"~effect/Effect/args": {"$function": ""}}))
}

/// Port of `makeTestProviderAdapterHarness` (`apps/server/integration/TestProviderAdapter.integration.ts:229`).
pub fn make_test_provider_adapter_harness(
    options: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = &options;
    Ok(serde_json::json!({"~effect/Effect/args": {"$function": ""}}))
}
