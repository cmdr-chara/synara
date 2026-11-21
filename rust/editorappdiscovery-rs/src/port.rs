//! The Rust port of `apps/server/src/editorAppDiscovery.ts`.

use serde_json::{Value, json};

fn is_undefined(v: &Value) -> bool {
    v.as_object().is_some_and(|o| o.contains_key("$undefined"))
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

fn undefined() -> Value {
    json!({"$undefined": true})
}

fn js_in_error(key: &str, v: &Value) -> String {
    format!(
        "Cannot use 'in' operator to search for '{key}' in {}",
        jscore::js_to_string(v)
    )
}

/// `"key" in editor`: primitives throw, objects/arrays answer by key presence.
fn js_in<'a>(v: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    match v {
        Value::Object(o) => Ok(o.get(key)),
        Value::Array(_) => Ok(None),
        _ => Err(js_in_error(key, v)),
    }
}

/// Port of `getEditorMacApplications` (`apps/server/src/editorAppDiscovery.ts:47`).
pub fn get_editor_mac_applications(editor: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(js_in(&editor, "macApplications")?
        .cloned()
        .unwrap_or_else(undefined))
}

/// Port of `getEditorWindowsUriScheme` (`apps/server/src/editorAppDiscovery.ts:51`).
pub fn get_editor_windows_uri_scheme(
    editor: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(js_in(&editor, "windowsUriScheme")?
        .cloned()
        .unwrap_or_else(undefined))
}

/// Port of `getEditorWindowsStorePackages` (`apps/server/src/editorAppDiscovery.ts:55`).
pub fn get_editor_windows_store_packages(
    editor: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(js_in(&editor, "windowsStorePackages")?
        .cloned()
        .unwrap_or_else(undefined))
}

/// Port of `normalizeMacApplicationBundleName` (`apps/server/src/editorAppDiscovery.ts:61`).
pub fn normalize_mac_application_bundle_name(app_name: String) -> Result<String, String> {
    if app_name.ends_with(".app") {
        Ok(app_name)
    } else {
        Ok(format!("{app_name}.app"))
    }
}

/// POSIX `path.join`: collapses duplicate slashes, resolves `.`/`..`,
/// keeps a leading `/`.
fn posix_join(parts: &[&str]) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut absolute = false;
    for (i, part) in parts.iter().enumerate() {
        for seg in part.split('/') {
            if seg.is_empty() {
                if i == 0 && !absolute {
                    absolute = true;
                }
                continue;
            }
            if seg == "." {
                continue;
            }
            if seg == ".." {
                out.pop();
                continue;
            }
            out.push(seg.to_string());
        }
    }
    let joined = out.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

/// Port of `resolveMacApplicationSearchPaths` (`apps/server/src/editorAppDiscovery.ts:66`).
pub fn resolve_mac_application_search_paths(
    app_name: String,
    env: serde_json::Value,
) -> Result<Vec<String>, String> {
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'HOME')".to_string());
    }
    if is_undefined(&env) {
        return Err("Cannot read properties of undefined (reading 'HOME')".to_string());
    }
    let bundle_name = normalize_mac_application_bundle_name(app_name)?;
    let home = match prop(&env, "HOME") {
        None => None,
        Some(v) if v.is_null() || is_undefined(v) => None,
        Some(Value::String(s)) => Some(s.trim_matches(jscore::is_space).to_string()),
        Some(_) => {
            return Err("env.HOME?.trim is not a function".to_string());
        }
    };
    let home = home.filter(|h| !h.is_empty());

    let mut paths: Vec<String> = Vec::with_capacity(7);
    if let Some(h) = &home {
        paths.push(posix_join(&[h, "Applications", &bundle_name]));
        paths.push(posix_join(&[
            h,
            "Applications",
            "JetBrains Toolbox",
            &bundle_name,
        ]));
    }
    paths.push(posix_join(&["/Applications", &bundle_name]));
    paths.push(posix_join(&["/Applications", "Utilities", &bundle_name]));
    paths.push(posix_join(&[
        "/Applications",
        "JetBrains Toolbox",
        &bundle_name,
    ]));
    paths.push(posix_join(&["/System", "Applications", &bundle_name]));
    paths.push(posix_join(&[
        "/System",
        "Applications",
        "Utilities",
        &bundle_name,
    ]));
    Ok(paths)
}

fn candidate_is_dir(candidate: &str) -> bool {
    std::fs::metadata(candidate).is_ok_and(|m| m.is_dir())
}

/// Port of `resolveMacApplicationBundlePath` (`apps/server/src/editorAppDiscovery.ts:89`).
pub fn resolve_mac_application_bundle_path(
    app_names: serde_json::Value,
    platform: serde_json::Value,
    env: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !platform.is_string()
        || platform.as_str() != Some("darwin")
        || app_names.is_null()
        || is_undefined(&app_names)
    {
        return Ok(Value::Null);
    }
    let names = app_names.as_array().cloned().unwrap_or_default();
    for name in &names {
        let Some(name_str) = name.as_str().map(str::to_string) else {
            continue;
        };
        for candidate in resolve_mac_application_search_paths(name_str, env.clone())? {
            if candidate_is_dir(&candidate) {
                return Ok(Value::String(candidate));
            }
        }
    }
    Ok(Value::Null)
}

/// Port of `resolveAvailableMacApplication` (`apps/server/src/editorAppDiscovery.ts:109`).
pub fn resolve_available_mac_application(
    app_names: serde_json::Value,
    platform: serde_json::Value,
    env: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !platform.is_string()
        || platform.as_str() != Some("darwin")
        || app_names.is_null()
        || is_undefined(&app_names)
    {
        return Ok(Value::Null);
    }
    let names = app_names.as_array().cloned().unwrap_or_default();
    for name in &names {
        let Some(name_str) = name.as_str().map(str::to_string) else {
            continue;
        };
        if resolve_mac_application_search_paths(name_str, env.clone())?
            .iter()
            .any(|c| candidate_is_dir(c))
        {
            return Ok(name.clone());
        }
    }
    Ok(Value::Null)
}

/// Port of `clearWindowsStorePackageDiscoveryCache` (`apps/server/src/editorAppDiscovery.ts:178`).
pub fn clear_windows_store_package_discovery_cache() -> Result<serde_json::Value, String> {
    Ok(undefined())
}
