//! The Rust port of `packages/shared/src/cloudflared.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;
use std::path::{Path, PathBuf};

const CLOUDFLARED_VERSION: &str = "2026.10.3";

struct Asset {
    name: &'static str,
    sha256: &'static str,
}

fn asset_for(key: &str) -> Option<Asset> {
    match key {
        "darwin-arm64" => Some(Asset {
            name: "cloudflared-darwin-arm64.tgz",
            sha256: "1165d14b6f6360a3b8c283d7d1100024d6ec7cd9bc9e32519e96150645f3ec55",
        }),
        "darwin-x64" => Some(Asset {
            name: "cloudflared-darwin-amd64.tgz",
            sha256: "81b7b85f2376815c4f59b1aa323887b62e16ce8b67555c2e1577dc23f2a82000",
        }),
        "linux-arm64" => Some(Asset {
            name: "cloudflared-linux-arm64",
            sha256: "89297ebfb9e8c99c0dc013a1140601453cf3798d8306c5591651f4a3f5210be8",
        }),
        "linux-x64" => Some(Asset {
            name: "cloudflared-linux-amd64",
            sha256: "053b80b0a23031a038fdbf3c073f59f82b8f1f152aa67971d7383ffafc0dbab0",
        }),
        "win32-x64" => Some(Asset {
            name: "cloudflared-windows-amd64.exe",
            sha256: "76b2805253d0d6ffacb092a7acfbee99e7af6917835c0c5410e23f3ef5052da2",
        }),
        _ => None,
    }
}

/// Node `os.platform()` on the running target.
fn node_platform() -> &'static str {
    match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

/// Node `os.arch()` on the running target.
fn node_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "arm64",
        "x86_64" => "x64",
        other => other,
    }
}

/// JS ToString for template interpolation (`${value}`).
fn to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(a) => a
            .iter()
            .map(|v| {
                if v.is_null() || js::is_undefined(v) {
                    String::new()
                } else {
                    to_string(v)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) if o.len() == 1 => {
            if let Some(n) = o.get("$num") {
                match n.as_str() {
                    Some("NaN") => "NaN".to_string(),
                    Some("Infinity") => "Infinity".to_string(),
                    Some("-Infinity") => "-Infinity".to_string(),
                    Some("-0") => "0".to_string(),
                    _ => js::number_to_string(
                        o.get("$num").and_then(Value::as_f64).unwrap_or(f64::NAN),
                    ),
                }
            } else if js::is_undefined(value) {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// JS truthiness.
#[allow(clippy::float_cmp, clippy::double_comparisons)]
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None => false,
        Some(v) if js::is_undefined(v) => false,
        Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x > 0.0 || x < 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() || dir.ends_with('/') {
        format!("{dir}{name}")
    } else {
        format!("{dir}/{name}")
    }
}

fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut buf);
    } else {
        let mut x = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
            .unwrap_or(0x9e3779b97f4a7c15);
        for b in &mut buf {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            *b = (x >> 24) as u8;
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn download(url: &str, dest: &Path) -> Result<Vec<u8>, String> {
    let status = std::process::Command::new("curl")
        .args(["-fsSL", "--max-time", "120", "-o"])
        .arg(dest)
        .arg(url)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Could not download the pinned Cloudflare connector".to_string());
    }
    let bytes = std::fs::read(dest).map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > 128 * 1024 * 1024 {
        return Err("Cloudflare connector download was empty".to_string());
    }
    Ok(bytes)
}

/// Port of `installCloudflared` (`packages/shared/src/cloudflared.ts:32`).
pub async fn install_cloudflared(
    directory: String,
    platform: Value,
    arch: Value,
    signal: Value,
) -> Result<String, String> {
    let platform = if js::is_undefined(&platform) {
        node_platform().to_string()
    } else {
        to_string(&platform)
    };
    let arch = if js::is_undefined(&arch) {
        node_arch().to_string()
    } else {
        to_string(&arch)
    };
    let asset = asset_for(&format!("{platform}-{arch}"))
        .ok_or_else(|| "Cloudflare remote is unsupported on this platform".to_string())?;

    if directory.is_empty() {
        return Err("ENOENT: no such file or directory, mkdir ''".to_string());
    }
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let cache = join(&directory, asset.name);
    let mut bytes = match std::fs::read(&cache) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e.to_string()),
    };
    if bytes.is_none() {
        // `signal ? AbortSignal.any([signal, timeout]) : timeout` throws on any
        // truthy non-AbortSignal before the fetch is even attempted.
        if truthy(Some(&signal)) {
            return Err("signals[0] is not of type AbortSignal.".to_string());
        }
        let url = format!(
            "https://github.com/cloudflare/cloudflared/releases/download/{CLOUDFLARED_VERSION}/{}",
            asset.name
        );
        let staging = PathBuf::from(join(&directory, &format!(".dl-{}", random_hex(8))));
        let result = download(&url, &staging);
        let _ = std::fs::remove_file(&staging);
        bytes = Some(result?);
    }
    let bytes = bytes.unwrap_or_default();
    if sha256_hex(&bytes) != asset.sha256 {
        return Err("Cloudflare connector integrity check failed".to_string());
    }
    // `signal?.throwIfAborted()`: skips only on null/undefined; other falsy
    // values (0, "", false) hit a missing-method TypeError.
    match Some(&signal) {
        Some(v) if !v.is_null() && !js::is_undefined(v) => {
            return Err("signal?.throwIfAborted is not a function".to_string());
        }
        _ => {}
    }
    let binary_name = if platform == "win32" {
        "cloudflared.exe"
    } else {
        "cloudflared"
    };
    let binary = join(&directory, binary_name);
    let temporary = join(&directory, &format!(".install-{}", random_hex(8)));
    std::fs::create_dir_all(&temporary).map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        if asset.name.ends_with(".tgz") {
            let archive = join(&temporary, "release.tgz");
            std::fs::write(&archive, &bytes).map_err(|e| e.to_string())?;
            let status = std::process::Command::new("tar")
                .args(["-xzf", &archive, "-C", &temporary, "cloudflared"])
                .status()
                .map_err(|e| e.to_string())?;
            if !status.success() {
                return Err("Could not unpack the verified Cloudflare connector".to_string());
            }
        } else {
            std::fs::write(join(&temporary, binary_name), &bytes).map_err(|e| e.to_string())?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                join(&temporary, binary_name),
                std::fs::Permissions::from_mode(0o755),
            )
            .map_err(|e| e.to_string())?;
        }
        std::fs::rename(join(&temporary, binary_name), &binary).map_err(|e| e.to_string())?;
        let cache_temp = join(&directory, &format!(".asset-{}", random_hex(16)));
        std::fs::write(&cache_temp, &bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&cache_temp, &cache).map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&temporary);
    result?;
    Ok(binary)
}
