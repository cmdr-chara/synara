//! The Rust port of `apps/server/src/privatePathPermissions.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;
const PRIVATE_EXECUTABLE_FILE_MODE: u32 = 0o700;

fn err(op: &str, path: &str) -> String {
    format!("Failed to {op} private path {path}")
}

fn js_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `supportsPosixPermissions(platform = process.platform)` — only "win32" is
/// non-posix; undefined/junk all count as posix.
fn supports_posix(platform: Option<&Value>) -> bool {
    match platform {
        Some(v) => v.as_str() != Some("win32"),
        None => true,
    }
}

/// `options.platform` — null throws TypeError, missing/undefined/junk → default.
fn options_platform(options: &Value) -> Result<Option<Value>, String> {
    match options {
        Value::Null => Err("Cannot read properties of null (reading 'platform')".to_string()),
        Value::Object(o) => Ok(o.get("platform").cloned()),
        _ => Ok(None),
    }
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => !js_undefined(v),
    }
}

fn options_executable(options: &Value) -> bool {
    match options {
        Value::Object(o) => o.get("executable").is_some_and(js_truthy),
        _ => false,
    }
}

#[cfg(unix)]
fn set_mode(file: &std::fs::File, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(mode))
}

#[cfg(unix)]
fn chmod(path: &str, mode: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
}

fn repair_private_file_inner(file_path: &str, executable: bool, posix: bool) -> Result<(), String> {
    if !posix {
        return Ok(());
    }
    let mode = if executable {
        PRIVATE_EXECUTABLE_FILE_MODE
    } else {
        PRIVATE_FILE_MODE
    };
    let file = std::fs::File::open(file_path)
        .map_err(|_| err("open without following symlinks", file_path))?;
    if !file
        .metadata()
        .map_err(|_| err("set mode on", file_path))?
        .is_file()
    {
        return Err(err("set mode on", file_path));
    }
    set_mode(&file, mode).map_err(|_| err("set mode on", file_path))
}

fn path_join(a: &str, b: &str) -> String {
    if a.is_empty() {
        b.to_string()
    } else {
        format!("{a}/{b}")
    }
}

/// Port of `ensurePrivateDirectorySync` (`apps/server/src/privatePathPermissions.ts:37`).
pub fn ensure_private_directory_sync(
    directory_path: String,
    platform: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    std::fs::create_dir_all(&directory_path).map_err(|_| err("create", &directory_path))?;
    if !supports_posix(platform.as_ref()) {
        return Ok(crate::js::undefined());
    }
    let dir = std::fs::File::open(&directory_path)
        .map_err(|_| err("open without following symlinks", &directory_path))?;
    if !dir
        .metadata()
        .map_err(|_| err("set mode on", &directory_path))?
        .is_dir()
    {
        return Err(err("set mode on", &directory_path));
    }
    set_mode(&dir, PRIVATE_DIRECTORY_MODE).map_err(|_| err("set mode on", &directory_path))?;
    Ok(crate::js::undefined())
}

/// Port of `repairPrivateFileSync` (`apps/server/src/privatePathPermissions.ts:64`).
pub fn repair_private_file_sync(
    file_path: String,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let platform = options_platform(&options)?;
    repair_private_file_inner(
        &file_path,
        options_executable(&options),
        supports_posix(platform.as_ref()),
    )?;
    Ok(crate::js::undefined())
}

/// Port of `ensurePrivateFileSync` (`apps/server/src/privatePathPermissions.ts:88`).
pub fn ensure_private_file_sync(
    file_path: String,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let platform = options_platform(&options)?;
    let mode = if options_executable(&options) {
        PRIVATE_EXECUTABLE_FILE_MODE
    } else {
        PRIVATE_FILE_MODE
    };
    if !supports_posix(platform.as_ref()) {
        // O_WRONLY|O_CREAT without truncation
        #[allow(clippy::suspicious_open_options)]
        let _ = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&file_path)
            .map_err(|_| err("create", &file_path))?;
        return Ok(crate::js::undefined());
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .map_err(|_| err("open without following symlinks", &file_path))?;
    if !file
        .metadata()
        .map_err(|_| err("set mode on", &file_path))?
        .is_file()
    {
        return Err(err("set mode on", &file_path));
    }
    set_mode(&file, mode).map_err(|_| err("set mode on", &file_path))?;
    Ok(crate::js::undefined())
}

/// Port of `repairPrivateFile` (`apps/server/src/privatePathPermissions.ts:121`).
pub async fn repair_private_file(
    file_path: String,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let platform = options_platform(&options)?;
    repair_private_file_inner(
        &file_path,
        options_executable(&options),
        supports_posix(platform.as_ref()),
    )?;
    Ok(crate::js::undefined())
}

/// Port of `repairPrivateTreeSync` (`apps/server/src/privatePathPermissions.ts:154`).
pub fn repair_private_tree_sync(
    root_path: String,
    platform: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    if !supports_posix(platform.as_ref()) {
        return Ok(crate::js::undefined());
    }
    fn visit(entry_path: &str, is_root: bool) -> Result<(), String> {
        let stat = std::fs::symlink_metadata(entry_path).map_err(|_| err("inspect", entry_path))?;
        if stat.file_type().is_symlink() {
            if is_root {
                return Err(err("repair", entry_path));
            }
            return Ok(());
        }
        if stat.is_dir() {
            chmod(entry_path, PRIVATE_DIRECTORY_MODE)
                .map_err(|_| err("set mode on", entry_path))?;
            let entries = std::fs::read_dir(entry_path).map_err(|_| err("read", entry_path))?;
            for entry in entries.flatten() {
                visit(
                    &path_join(entry_path, &entry.file_name().to_string_lossy()),
                    false,
                )?;
            }
            return Ok(());
        }
        if stat.is_file() {
            #[cfg(unix)]
            let executable = {
                use std::os::unix::fs::MetadataExt;
                stat.mode() & 0o100 != 0
            };
            #[cfg(not(unix))]
            let executable = false;
            repair_private_file_inner(entry_path, executable, true)?;
        }
        Ok(())
    }
    visit(&root_path, true)?;
    Ok(crate::js::undefined())
}
