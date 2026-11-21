//! The Rust port of `packages/shared/src/betaFeatures.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.get("$undefined") == Some(&Value::Bool(true)))
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !is_undefined(value),
    }
}

/// The shared `BETA_ONLY_FEATURES` list; currently empty upstream.
const BETA_ONLY_FEATURES: [&str; 0] = [];

const PRODUCTION_BUNDLE_ID: &str = "com.emanueledipietro.synara";

fn flavor_value(flavor: &str) -> Value {
    Value::String(flavor.to_string())
}

/// Port of `isBetaFeatureEnabled` (`packages/shared/src/betaFeatures.ts:68`).
pub fn is_beta_feature_enabled(feature: Value, flavor: Value) -> Result<bool, String> {
    let listed = feature
        .as_str()
        .is_some_and(|f| BETA_ONLY_FEATURES.contains(&f));
    Ok(!listed || flavor.as_str() != Some("production"))
}

/// Port of `desktopFlavorFromBundleId` (`packages/shared/src/betaFeatures.ts:76`).
pub fn desktop_flavor_from_bundle_id(bundle_id: Value) -> Result<serde_json::Value, String> {
    let id = match &bundle_id {
        Value::Null => return Ok(flavor_value("unknown")),
        v if is_undefined(v) => return Ok(flavor_value("unknown")),
        Value::String(s) => s.trim(),
        _ => return Err("bundleId?.trim is not a function".into()),
    };
    let flavor = match id {
        PRODUCTION_BUNDLE_ID => "production",
        "com.emanueledipietro.synara.beta" => "beta",
        "com.emanueledipietro.synara.canary" => "canary",
        "com.emanueledipietro.synara.cua" => "cua",
        "com.emanueledipietro.synara.remote" => "remote",
        "com.emanueledipietro.synara.dev" => "development",
        _ => "unknown",
    };
    Ok(flavor_value(flavor))
}

/// Port of `desktopFlavorFromProtocol` (`packages/shared/src/betaFeatures.ts:103`).
pub fn desktop_flavor_from_protocol(
    protocol: Value,
    is_dev_build: Value,
) -> Result<serde_json::Value, String> {
    let flavor = match protocol.as_str() {
        Some("synara:") => {
            if truthy(&is_dev_build) {
                "development"
            } else {
                "production"
            }
        }
        Some("synara-beta:") => "beta",
        Some("synara-canary:") => "canary",
        Some("synara-cua:") => "cua",
        Some("synara-remote:") => "remote",
        _ => "unknown",
    };
    Ok(flavor_value(flavor))
}
