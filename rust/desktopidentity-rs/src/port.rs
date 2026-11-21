//! The Rust port of `packages/shared/src/desktopIdentity.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const DEV_FLAVORS: [&str; 4] = ["canary", "cua", "beta", "remote"];
const PACKAGED_FLAVORS: [&str; 5] = ["canary", "cua", "beta", "remote", "production"];
const SMOKE_FLAVORS: [&str; 3] = ["beta", "cua", "remote"];
const SOURCE_MARKER: &str = "synara-source-desktop-build-v2";

fn prop_err(prop: &str, undefined: bool) -> String {
    format!(
        "Cannot read properties of {} (reading '{}')",
        if undefined { "undefined" } else { "null" },
        prop
    )
}

fn truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| (c.is_whitespace() || c == '\u{feff}') && c != '\u{85}')
}

/// JS ToString for `${v}` concatenation.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if x.is_null() || crate::js::is_undefined(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if crate::js::is_undefined(v) {
                "undefined".into()
            } else {
                "[object Object]".into()
            }
        }
    }
}

/// Port of `desktopUpdateChannel` (`packages/shared/src/desktopIdentity.ts:57`).
pub fn desktop_update_channel(flavor: String) -> Result<String, String> {
    Ok(match flavor.as_str() {
        "beta" => "beta",
        "remote" => "remote",
        _ => "synara",
    }
    .to_string())
}

/// Requested dev-channel flavor, or the development/production fallback.
/// `packaged` = packaged build without a packagedFlavor: overrides need `allowDevelopmentOverride`.
fn resolve_dev_flavor(input: &serde_json::Value, packaged: bool) -> Result<String, String> {
    let req = match input.get("requestedFlavor") {
        None => None,
        Some(v) if crate::js::is_undefined(v) || v.is_null() => None,
        Some(Value::String(s)) => Some(js_trim(s).to_lowercase()),
        Some(_) => {
            return Err("input.requestedFlavor?.trim is not a function".into());
        }
    };
    let ado = truthy(&input["allowDevelopmentOverride"]);
    let dev = truthy(&input["isDevelopment"]);
    if packaged && !ado {
        return Ok("production".into());
    }
    if let Some(f) = &req {
        if !packaged || ado {
            if DEV_FLAVORS.contains(&f.as_str()) {
                return Ok(f.clone());
            }
            if f == "development" && ado {
                return Ok("development".into());
            }
        }
        if packaged && dev && !f.is_empty() {
            return Ok("development".into());
        }
    }
    if packaged && dev && req.is_none() && ado {
        return Ok("development".into());
    }
    Ok(if !packaged && dev {
        "development"
    } else {
        "production"
    }
    .to_string())
}

/// Port of `resolveSynaraDesktopFlavor` (`packages/shared/src/desktopIdentity.ts:75`).
pub fn resolve_synara_desktop_flavor(input: serde_json::Value) -> Result<String, String> {
    if input.is_null() || crate::js::is_undefined(&input) {
        return Err(prop_err("requestedFlavor", crate::js::is_undefined(&input)));
    }
    resolve_dev_flavor(&input, false)
}

/// Port of `resolveSynaraDesktopRuntimeFlavor` (`packages/shared/src/desktopIdentity.ts:100`).
pub fn resolve_synara_desktop_runtime_flavor(input: serde_json::Value) -> Result<String, String> {
    if input.is_null() || crate::js::is_undefined(&input) {
        return Err(prop_err("isPackaged", crate::js::is_undefined(&input)));
    }
    if truthy(&input["isPackaged"]) {
        return match input.get("packagedFlavor") {
            None => Ok(resolve_dev_flavor(&input, true)?),
            Some(v) if crate::js::is_undefined(v) => Ok(resolve_dev_flavor(&input, true)?),
            Some(Value::String(f)) if PACKAGED_FLAVORS.contains(&f.as_str()) => Ok(f.clone()),
            Some(_) => Err(
                "The packaged Synara desktop flavor is invalid. Rebuild the application.".into(),
            ),
        };
    }
    resolve_dev_flavor(&input, false)
}

/// Port of `canOverrideDesktopSmokeUserData` (`packages/shared/src/desktopIdentity.ts:129`).
pub fn can_override_desktop_smoke_user_data(input: serde_json::Value) -> Result<bool, String> {
    if input.is_null() || crate::js::is_undefined(&input) {
        return Err(prop_err("packagedFlavor", crate::js::is_undefined(&input)));
    }
    if let Some(v) = input.get("packagedFlavor")
        && !crate::js::is_undefined(v)
    {
        return Ok(v.as_str().is_some_and(|f| SMOKE_FLAVORS.contains(&f)));
    }
    Ok(input
        .get("sourceBuildMarker")
        .is_some_and(|m| m == SOURCE_MARKER))
}

/// Port of `synaraDesktopIdentity` (`packages/shared/src/desktopIdentity.ts:142`).
pub fn synara_desktop_identity(flavor: String) -> Result<serde_json::Value, String> {
    Ok(match flavor.as_str() {
        "development" => {
            json!({"flavor":"development","displayName":"Synara (Dev)","bundleId":"com.emanueledipietro.synara.dev","scheme":"synara","origin":"synara://app","entryUrl":"synara://app/index.html","userDataDirectoryName":"synara-dev","defaultHomeDirectoryName":".synara-dev","usesScriptedUpdates":false})
        }
        "beta" => {
            json!({"flavor":"beta","displayName":"Synara Beta","bundleId":"com.emanueledipietro.synara.beta","scheme":"synara-beta","origin":"synara-beta://app","entryUrl":"synara-beta://app/index.html","userDataDirectoryName":"synara-beta","defaultHomeDirectoryName":".synara-beta","usesScriptedUpdates":false})
        }
        "canary" => {
            json!({"flavor":"canary","displayName":"Synara Canary","bundleId":"com.emanueledipietro.synara.canary","scheme":"synara-canary","origin":"synara-canary://app","entryUrl":"synara-canary://app/index.html","userDataDirectoryName":"synara-canary","defaultHomeDirectoryName":".synara-canary","usesScriptedUpdates":true})
        }
        "cua" => {
            json!({"flavor":"cua","displayName":"Synara Cua","bundleId":"com.emanueledipietro.synara.cua","scheme":"synara-cua","origin":"synara-cua://app","entryUrl":"synara-cua://app/index.html","userDataDirectoryName":"synara-cua","defaultHomeDirectoryName":".synara-cua","usesScriptedUpdates":true})
        }
        "remote" => {
            json!({"flavor":"remote","displayName":"Synara Remote","bundleId":"com.emanueledipietro.synara.remote","scheme":"synara-remote","origin":"synara-remote://app","entryUrl":"synara-remote://app/index.html","userDataDirectoryName":"synara-remote","defaultHomeDirectoryName":".synara-remote","defaultAccountUrl":"https://synara-account-api-trial.synara-orgs.workers.dev","usesScriptedUpdates":false})
        }
        _ => {
            json!({"flavor":flavor,"displayName":"Synara","bundleId":"com.emanueledipietro.synara","scheme":"synara","origin":"synara://app","entryUrl":"synara://app/index.html","userDataDirectoryName":"synara","defaultHomeDirectoryName":".synara","usesScriptedUpdates":false})
        }
    })
}

/// Port of `synaraSourceDesktopIdentity` (`packages/shared/src/desktopIdentity.ts:228`).
pub fn synara_source_desktop_identity(
    identity: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if identity.is_null() || crate::js::is_undefined(&identity) {
        return Err(prop_err("flavor", crate::js::is_undefined(&identity)));
    }
    let Value::Object(mut map) = identity else {
        return Ok(identity);
    };
    if matches!(
        map.get("flavor").and_then(Value::as_str),
        Some("beta") | Some("remote")
    ) {
        let mut patch = |key: &str, suffix: &str| {
            let s = map
                .get(key)
                .map_or_else(|| "undefined".to_string(), js_to_string);
            map.insert(key.to_string(), Value::String(format!("{s}{suffix}")));
        };
        patch("displayName", " (Source)");
        patch("bundleId", ".source");
    }
    Ok(Value::Object(map))
}
