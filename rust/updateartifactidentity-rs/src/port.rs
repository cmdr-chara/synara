//! The Rust port of `apps/desktop/src/updateArtifactIdentity.ts`.
//!
//! `path.resolve` is reproduced with POSIX semantics (the desktop build targets
//! POSIX paths here); `createHash("sha512")` maps to `sha2`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use sha2::Digest;
use std::io::Read;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `typeof v === "object"` (JS view): real objects only — arrays and the
/// `$num`/`$utf16`/`$undefined` wire markers count as number/string/undefined.
fn is_js_object(v: &Value) -> bool {
    match v {
        Value::Object(o) => {
            !(o.len() == 1
                && (o.contains_key("$num")
                    || o.contains_key("$utf16")
                    || o.contains_key("$undefined")))
        }
        _ => false,
    }
}

/// A JS string's view of `v`: `String` plus the `{"$utf16": ...}` marker.
fn as_js_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.get("$utf16").and_then(Value::as_array).map(|units| {
            let u: Vec<u16> = units
                .iter()
                .filter_map(|x| x.as_u64().map(|n| n as u16))
                .collect();
            String::from_utf16_lossy(&u)
        }),
        _ => None,
    }
}

/// A JS number's view of `v`: `Number` plus the `{"$num": ...}` marker.
fn as_js_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Object(o) if o.len() == 1 => match o.get("$num")?.as_str()? {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}

/// `v.key`: TypeError on null/undefined, `undefined` on primitives and arrays,
/// the member on objects.
fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => {
            if is_undefined(v) {
                Err(format!(
                    "Cannot read properties of undefined (reading '{key}')"
                ))
            } else {
                Ok(o.get(key))
            }
        }
        _ => Ok(None),
    }
}

/// Node `path.posix.normalize`: collapse `//`, `.`, `..`; keep a trailing `/`.
fn posix_normalize(p: &str) -> String {
    let absolute = p.starts_with('/');
    let trailing = p.ends_with('/') && p.len() > 1;
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|s| *s != "..") {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let mut joined = if absolute {
        format!("/{}", out.join("/"))
    } else {
        out.join("/")
    };
    if joined.is_empty() {
        joined = if absolute {
            "/".to_string()
        } else {
            ".".to_string()
        };
    }
    if trailing && joined != "/" {
        joined.push('/');
    }
    joined
}

/// Node `path.resolve` (POSIX): absolute inputs normalize in place; relative
/// inputs resolve against the process cwd.
fn resolve_path(p: &str) -> String {
    if p.starts_with('/') {
        posix_normalize(p)
    } else {
        let cwd = std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/".to_string());
        posix_normalize(&format!("{cwd}/{p}"))
    }
}

/// `x === y` for wire values: markers decode to their JS values first; real
/// objects only equal themselves (never true across two wire objects).
fn strict_eq(a: &Value, b: &Value) -> bool {
    if is_undefined(a) && is_undefined(b) {
        return true;
    }
    if is_undefined(a) || is_undefined(b) {
        return false;
    }
    match (as_js_num(a), as_js_num(b)) {
        (Some(x), Some(y)) => return x == y, // -0 === 0, NaN !== NaN (f64 ==)
        (Some(_), None) | (None, Some(_)) => return false,
        _ => {}
    }
    match (as_js_str(a), as_js_str(b)) {
        (Some(x), Some(y)) => return x == y,
        (Some(_), None) | (None, Some(_)) => return false,
        _ => {}
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => false, // arrays and plain objects compare by identity: false here
    }
}

/// Port of `isUpdateArtifactIdentity` (`apps/desktop/src/updateArtifactIdentity.ts:17`).
pub fn is_update_artifact_identity(value: serde_json::Value) -> Result<bool, String> {
    if !is_js_object(&value) {
        return Ok(false);
    }
    let path = prop(&value, "path")?;
    let ok = match path.and_then(as_js_str) {
        Some(p) => {
            let units: Vec<u16> = p.encode_utf16().collect();
            !units.is_empty() && resolve_path(&p) == p
        }
        None => false,
    };
    if !ok {
        return Ok(false);
    }
    let size_ok = match prop(&value, "size")?.and_then(as_js_num) {
        Some(n) => n.fract() == 0.0 && n.abs() <= 9_007_199_254_740_991.0 && n > 0.0,
        None => false,
    };
    if !size_ok {
        return Ok(false);
    }
    let sha_ok = match prop(&value, "sha512")?.and_then(as_js_str) {
        Some(s) => {
            s.len() == 128
                && s.bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        }
        None => false,
    };
    Ok(sha_ok)
}

/// Port of `updateArtifactIdentitiesEqual` (`apps/desktop/src/updateArtifactIdentity.ts:32`).
pub fn update_artifact_identities_equal(
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<bool, String> {
    for key in ["path", "size", "sha512"] {
        let l = prop(&left, key)?;
        let r = prop(&right, key)?;
        let same = match (l, r) {
            (None, None) => true, // undefined === undefined
            (Some(a), Some(b)) => strict_eq(a, b),
            _ => false,
        };
        if !same {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Port of `fingerprintUpdateArtifact` (`apps/desktop/src/updateArtifactIdentity.ts:39`).
pub async fn fingerprint_update_artifact(file_path: String) -> Result<serde_json::Value, String> {
    let resolved = resolve_path(&file_path);
    let meta = std::fs::symlink_metadata(&resolved)
        .map_err(|e| format!("ENOENT: no such file or directory, lstat '{resolved}' ({e})"))?;
    if !meta.file_type().is_file() || meta.file_type().is_symlink() {
        return Err("Downloaded update payload must be a regular, non-symlink file.".to_string());
    }
    let mut file =
        std::fs::File::open(&resolved).map_err(|e| format!("open '{resolved}' failed ({e})"))?;
    let before = file
        .metadata()
        .map_err(|e| format!("stat '{resolved}' failed ({e})"))?;
    if !before.file_type().is_file() || before.len() == 0 {
        return Err("Downloaded update payload is not a regular file.".to_string());
    }
    let mut hasher = sha2::Sha512::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("read '{resolved}' failed ({e})"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let after_meta = std::fs::symlink_metadata(&resolved)
        .map_err(|e| format!("lstat '{resolved}' failed ({e})"))?;
    let after = file
        .metadata()
        .map_err(|e| format!("stat '{resolved}' failed ({e})"))?;
    if after.len() != before.len() || !after_meta.file_type().is_file() {
        return Err("Downloaded update payload changed while it was fingerprinted.".to_string());
    }
    let mut out = Map::new();
    out.insert("path".to_string(), Value::String(resolved));
    out.insert("size".to_string(), json!(after.len()));
    out.insert(
        "sha512".to_string(),
        Value::String(format!("{:x}", hasher.finalize())),
    );
    Ok(Value::Object(out))
}

/// Port of `verifyUpdateArtifactIdentity` (`apps/desktop/src/updateArtifactIdentity.ts:92`).
pub async fn verify_update_artifact_identity(expected: serde_json::Value) -> Result<bool, String> {
    if !is_update_artifact_identity(expected.clone())? {
        return Ok(false);
    }
    let path = prop(&expected, "path")?
        .and_then(as_js_str)
        .unwrap_or_default();
    let Ok(actual) = fingerprint_update_artifact(path).await else {
        return Ok(false);
    };
    let size = prop(&expected, "size")?.cloned().unwrap_or(Value::Null);
    let sha = prop(&expected, "sha512")?.cloned().unwrap_or(Value::Null);
    let actual_size = prop(&actual, "size")?.cloned().unwrap_or(Value::Null);
    let actual_sha = prop(&actual, "sha512")?.cloned().unwrap_or(Value::Null);
    Ok(strict_eq(&actual_size, &size) && strict_eq(&actual_sha, &sha))
}
