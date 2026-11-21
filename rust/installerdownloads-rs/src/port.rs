//! The Rust port of `packages/shared/src/installerDownloads.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const MAX_SAFE: f64 = 9007199254740991.0;

fn is_safe_integer(v: &Value) -> bool {
    match v {
        Value::Number(n) => n
            .as_f64()
            .map(|f| f.fract() == 0.0 && f.abs() <= MAX_SAFE)
            .unwrap_or(false),
        _ => false,
    }
}

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => {
            crate::js::is_undefined(v)
                || matches!(
                    v.get("$num").and_then(|m| m.as_str()),
                    Some("NaN") | Some("-0") | Some("0")
                )
        }
    }
}

fn platform_of(name: &Value) -> Option<&'static str> {
    let s = name.as_str()?;
    if s.encode_utf16().count() > 512 {
        return None;
    }
    let lower = s.to_lowercase();
    if lower.ends_with(".dmg") {
        Some("darwin")
    } else if lower.ends_with(".exe") {
        Some("win32")
    } else if lower.ends_with(".appimage") {
        Some("linux")
    } else {
        None
    }
}

fn download_count_of(value: &Value) -> Option<f64> {
    if is_safe_integer(value) && value.as_f64()? >= 0.0 {
        value.as_f64()
    } else {
        None
    }
}

/// Port of `installerPlatform` (`packages/shared/src/installerDownloads.ts:8`).
pub fn installer_platform(name: serde_json::Value) -> Result<Option<String>, String> {
    Ok(platform_of(&name).map(str::to_string))
}

/// Port of `installerDownloadCount` (`packages/shared/src/installerDownloads.ts:16`).
pub fn installer_download_count(value: serde_json::Value) -> Result<Option<f64>, String> {
    Ok(download_count_of(&value))
}

/// Port of `countInstallerDownloads` (`packages/shared/src/installerDownloads.ts:21`).
pub fn count_installer_downloads(releases: serde_json::Value) -> Result<f64, String> {
    let releases = match releases {
        Value::Array(a) => a,
        _ => return Err("Invalid GitHub releases".into()),
    };
    let mut total = 0.0f64;
    for release in &releases {
        if falsy(release) {
            continue;
        }
        let assets = match release.get("assets") {
            Some(Value::Array(a)) => a,
            _ => continue,
        };
        for asset in assets {
            if falsy(asset) || platform_of(asset.get("name").unwrap_or(&Value::Null)).is_none() {
                continue;
            }
            let count = match download_count_of(asset.get("download_count").unwrap_or(&Value::Null))
            {
                Some(c) => c,
                None => return Err("Invalid GitHub installer counter".into()),
            };
            total += count;
            if !(total.fract() == 0.0 && total.abs() <= MAX_SAFE) {
                return Err("GitHub installer total exceeds bounds".into());
            }
        }
    }
    Ok(total)
}
