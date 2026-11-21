//! The Rust port of `packages/shared/src/cli.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

/// Port of `optionalBooleanEnvironmentConfig` (`packages/shared/src/cli.ts:17`).
pub fn optional_boolean_environment_config(name: String) -> Result<serde_json::Value, String> {
    let _ = &name;
    unimplemented!("port `optionalBooleanEnvironmentConfig` from packages/shared/src/cli.ts")
}

/// Port of `optionalBooleanFlag` (`packages/shared/src/cli.ts:51`).
pub fn optional_boolean_flag(
    name: String,
    options: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = (&name, &options);
    unimplemented!("port `optionalBooleanFlag` from packages/shared/src/cli.ts")
}

/// Port of `getBooleanFlagValue` (`packages/shared/src/cli.ts:72`).
pub fn get_boolean_flag_value(input: serde_json::Value) -> Result<Option<bool>, String> {
    let _ = &input;
    unimplemented!("port `getBooleanFlagValue` from packages/shared/src/cli.ts")
}

/// Port of `resolveBooleanConfig` (`packages/shared/src/cli.ts:79`).
pub fn resolve_boolean_config(
    cli_input: serde_json::Value,
    environment_value: Option<bool>,
    fallback: bool,
) -> Result<bool, String> {
    let _ = (&cli_input, &environment_value, &fallback);
    unimplemented!("port `resolveBooleanConfig` from packages/shared/src/cli.ts")
}
