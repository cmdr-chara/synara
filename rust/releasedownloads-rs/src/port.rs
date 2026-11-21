//! The Rust port of `packages/shared/src/releaseDownloads.ts`.
#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn is_nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m
            .get(key)
            .cloned()
            .unwrap_or_else(|| json!({"$undefined": true})),
        _ => json!({"$undefined": true}),
    }
}

fn is_https_url(v: &Value) -> bool {
    matches!(v, Value::String(s) if s.starts_with("https://"))
}

fn nonempty_str(v: &Value) -> bool {
    matches!(v, Value::String(s) if !s.is_empty())
}

fn lower(v: &Value) -> String {
    match v {
        Value::String(s) => s.chars().flat_map(char::to_lowercase).collect(),
        _ => String::new(),
    }
}

fn asset_match(name: &Value, needle: &str, suffix: &str) -> bool {
    let n = lower(name);
    n.contains(needle) && n.ends_with(suffix)
}

fn find_url(assets: &[Value], pred: impl Fn(&Value) -> bool, fallback: &Value) -> Value {
    for asset in assets {
        if pred(asset) && is_https_url(&prop(asset, "browser_download_url")) {
            return prop(asset, "browser_download_url");
        }
    }
    fallback.clone()
}

/// Port of `mapReleaseAssets` (`packages/shared/src/releaseDownloads.ts:50`).
pub fn map_release_assets(
    release: serde_json::Value,
    fallback_url: String,
) -> Result<serde_json::Value, String> {
    if release.is_null() {
        return Err("Cannot read properties of null (reading 'assets')".to_string());
    }
    let assets_field = prop(&release, "assets");
    let assets: Vec<Value> = match &assets_field {
        Value::Array(items) => items.clone(),
        v if is_nullish(v) => Vec::new(),
        _ => return Err("release.assets.find is not a function".to_string()),
    };
    let version = prop(&release, "tag_name");
    let version = if is_nullish(&version) {
        Value::Null
    } else {
        version
    };
    let html_url = prop(&release, "html_url");
    let releases_url = if is_https_url(&html_url) {
        html_url
    } else {
        Value::String(fallback_url)
    };
    Ok(json!({
        "version": version,
        "releasesUrl": releases_url,
        "mac": {
            "arm64": find_url(&assets, |a| asset_match(&prop(a, "name"), "arm64", ".dmg"), &releases_url),
            "x64": find_url(&assets, |a| asset_match(&prop(a, "name"), "x64", ".dmg"), &releases_url),
        },
        "windows": find_url(&assets, |a| {
            let n = lower(&prop(a, "name"));
            n.ends_with(".exe") || n.ends_with(".msi")
        }, &releases_url),
        "linux": {
            "x64": find_url(&assets, |a| asset_match(&prop(a, "name"), "x86_64", ".appimage"), &releases_url),
            "arm64": find_url(&assets, |a| asset_match(&prop(a, "name"), "arm64", ".appimage"), &releases_url),
        },
    }))
}

/// Port of `parseReleaseDownloadsSnapshot` (`packages/shared/src/releaseDownloads.ts:85`).
pub fn parse_release_downloads_snapshot(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let valid = (|| -> Option<()> {
        let version = prop(&value, "version");
        if !nonempty_str(&version) {
            return None;
        }
        let releases_url = prop(&value, "releasesUrl");
        if !is_https_url(&releases_url) {
            return None;
        }
        let version_str = version.as_str().unwrap_or_default();
        if !releases_url
            .as_str()
            .unwrap_or_default()
            .contains(version_str)
        {
            return None;
        }
        let mac = prop(&value, "mac");
        if !is_https_url(&prop(&mac, "arm64")) || !is_https_url(&prop(&mac, "x64")) {
            return None;
        }
        if !is_https_url(&prop(&value, "windows")) {
            return None;
        }
        let linux = prop(&value, "linux");
        if !is_https_url(&prop(&linux, "x64")) || !is_https_url(&prop(&linux, "arm64")) {
            return None;
        }
        Some(())
    })();
    if valid.is_none() {
        return Ok(None);
    }
    Ok(Some(json!({
        "version": prop(&value, "version"),
        "releasesUrl": prop(&value, "releasesUrl"),
        "mac": {
            "arm64": prop(&prop(&value, "mac"), "arm64"),
            "x64": prop(&prop(&value, "mac"), "x64"),
        },
        "windows": prop(&value, "windows"),
        "linux": {
            "x64": prop(&prop(&value, "linux"), "x64"),
            "arm64": prop(&prop(&value, "linux"), "arm64"),
        },
    })))
}
