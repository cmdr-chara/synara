//! The Rust port of `packages/shared/src/filesystemPlatform.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io;

use crate::js::{as_number, is_undefined};

/// `platform === undefined ? process.platform : platform` — only `undefined` defaults to "darwin".
fn resolved_is(platform: &Value, name: &str) -> bool {
    match platform {
        v if is_undefined(v) => name == "darwin",
        Value::String(s) => s == name,
        _ => false,
    }
}

fn open_error(code: &str, msg: &str, path: &str) -> String {
    format!("{code}: {msg}, open '{path}'")
}

fn io_error(e: &io::Error, path: &str) -> String {
    match e.raw_os_error() {
        Some(2) => open_error("ENOENT", "no such file or directory", path),
        Some(20) => open_error("ENOTDIR", "not a directory", path),
        Some(13) => open_error("EACCES", "permission denied", path),
        Some(21) => open_error("EISDIR", "illegal operation on a directory", path),
        Some(62) => open_error("ELOOP", "too many symbolic links encountered", path),
        Some(1) => open_error("EPERM", "operation not permitted", path),
        Some(36) => open_error("ENAMETOOLONG", "name too long", path),
        Some(40) => open_error("ELOOP", "too many symbolic links encountered", path),
        Some(n) => format!("EIO: {e} (os error {n}), open '{path}'"),
        None => format!("{e}, open '{path}'"),
    }
}

/// Port of `supportsPosixPermissions` (`packages/shared/src/filesystemPlatform.ts:11`).
pub fn supports_posix_permissions(platform: serde_json::Value) -> Result<bool, String> {
    Ok(!resolved_is(&platform, "win32"))
}

/// Port of `syncDirectoryEntry` (`packages/shared/src/filesystemPlatform.ts:16`).
pub async fn sync_directory_entry(
    directory_path: String,
    platform: serde_json::Value,
) -> Result<(), String> {
    if resolved_is(&platform, "win32") {
        return Ok(());
    }
    let meta = fs::symlink_metadata(&directory_path).map_err(|e| io_error(&e, &directory_path))?;
    if !meta.is_dir() {
        // Anything that is not a real directory — file, symlink, other — cannot be opened.
        return Err(open_error("ENOTDIR", "not a directory", &directory_path));
    }
    File::open(&directory_path)
        .and_then(|f| f.sync_all())
        .map_err(|e| io_error(&e, &directory_path))
}

/// Port of `syncRegularFile` (`packages/shared/src/filesystemPlatform.ts:40`).
pub async fn sync_regular_file(
    file_path: String,
    platform: serde_json::Value,
) -> Result<(), String> {
    let _ = &platform;
    let meta = fs::symlink_metadata(&file_path).map_err(|e| io_error(&e, &file_path))?;
    if meta.file_type().is_symlink() {
        return Err(open_error(
            "ELOOP",
            "too many symbolic links encountered",
            &file_path,
        ));
    }
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(&file_path)
        .and_then(|f| f.sync_all())
        .map_err(|e| io_error(&e, &file_path))
}

/// JS `===` for the dynamic `dev`/`ino` fields: identical primitives match,
/// `NaN` matches nothing, distinct objects/arrays never match.
fn strict_eq(a: &Value, b: &Value) -> bool {
    if is_undefined(a) && is_undefined(b) {
        return true;
    }
    if is_undefined(a) || is_undefined(b) {
        return false;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Number(_) | Value::Object(_), Value::Number(_) | Value::Object(_)) => {
            match (as_number(a), as_number(b)) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            }
        }
        _ => false,
    }
}

/// Port of `sameFileIdentity` (`packages/shared/src/filesystemPlatform.ts:56`).
pub fn same_file_identity(
    left: serde_json::Value,
    right: serde_json::Value,
    platform: serde_json::Value,
) -> Result<bool, String> {
    if resolved_is(&platform, "win32") {
        return Ok(true);
    }
    let undefined = Value::Object(serde_json::Map::from_iter([(
        "$undefined".to_string(),
        Value::Bool(true),
    )]));
    let get = |v: &Value, key: &str| -> Result<Value, String> {
        match v {
            Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
            v if is_undefined(v) => Err(format!(
                "Cannot read properties of undefined (reading '{key}')"
            )),
            _ => Ok(v.get(key).cloned().unwrap_or_else(|| undefined.clone())),
        }
    };
    let l_dev = get(&left, "dev")?;
    let r_dev = get(&right, "dev")?;
    let l_ino = get(&left, "ino")?;
    let r_ino = get(&right, "ino")?;
    Ok(strict_eq(&l_dev, &r_dev) && strict_eq(&l_ino, &r_ino))
}

/// Port of `isMacAppBundlePath` (`packages/shared/src/filesystemPlatform.ts:69`).
pub fn is_mac_app_bundle_path(
    directory_path: String,
    platform: serde_json::Value,
) -> Result<bool, String> {
    if !resolved_is(&platform, "darwin") {
        return Ok(false);
    }
    let trimmed = directory_path.trim_end_matches('/');
    let base = trimmed.rsplit('/').next().unwrap_or(trimmed);
    Ok(base.len() > 4 && base.to_lowercase().ends_with(".app"))
}
