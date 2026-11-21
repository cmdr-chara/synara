use serde_json::Value;

/// Port of `providerSupportsNativeTurnSteering` (`packages/shared/src/providerMetadata.ts:151`).
pub fn provider_supports_native_turn_steering(kind: Value) -> Result<bool, String> {
    Ok(matches!(
        kind,
        Value::String(ref s) if matches!(s.as_str(), "codex" | "claudeAgent" | "pi")
    ))
}
