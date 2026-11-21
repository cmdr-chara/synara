//! Port of `apps/server/src/privatePathPermissions.ts` — port units only.

use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};

use serde_json::Value;

use crate::js::{is_undefined, undefined};

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;
const PRIVATE_EXECUTABLE_FILE_MODE: u32 = 0o700;

fn private_path_error(operation: &str, target_path: &str) -> String {
    format!("Failed to {operation} private path {target_path}")
}

/// `options.<key>` — reading a member of `null` throws; `undefined` options
/// fall back to the `= {}` default; non-object options read as `undefined`.
fn option_member(options: &Value, key: &str) -> Result<Value, String> {
    match options {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(options) => Ok(undefined()),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(undefined)),
        _ => Ok(undefined()),
    }
}

/// `platform !== "win32"` — every non-string value (including `undefined`'s
/// `process.platform` default) supports posix permissions.
fn is_win32(platform: &Value) -> bool {
    matches!(platform, Value::String(s) if s == "win32")
}

/// JS truthiness, decoding `$undefined`/`$num` wire markers.
fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(m) => {
            if m.contains_key("$undefined") {
                return false;
            }
            if let Some(n) = m.get("$num").and_then(Value::as_str) {
                return !matches!(n, "NaN" | "0" | "-0");
            }
            true
        }
    }
}

/// `openSync(path, O_RDONLY | O_NOFOLLOW)` (+ `O_DIRECTORY` when required):
/// a symlinked target fails at open (ELOOP), and `O_DIRECTORY` rejects
/// non-directories at open (ENOTDIR) — both report the same operation.
fn open_readonly_nofollow(path: &str, require_dir: bool) -> Result<File, String> {
    if fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(private_path_error("open without following symlinks", path));
    }
    let file = File::open(path)
        .map_err(|_| private_path_error("open without following symlinks", path))?;
    if require_dir && !file.metadata().map(|m| m.is_dir()).unwrap_or(false) {
        return Err(private_path_error("open without following symlinks", path));
    }
    Ok(file)
}

/// `fstat.isFile()` + `fchmod` inside the "set mode on" context.
fn fchmod_regular(file: &File, path: &str, mode: u32) -> Result<(), String> {
    if !file.metadata().map(|m| m.is_file()).unwrap_or(false) {
        return Err(private_path_error("set mode on", path));
    }
    file.set_permissions(fs::Permissions::from_mode(mode))
        .map_err(|_| private_path_error("set mode on", path))
}

fn file_mode(executable: bool) -> u32 {
    if executable {
        PRIVATE_EXECUTABLE_FILE_MODE
    } else {
        PRIVATE_FILE_MODE
    }
}

fn repair_file(file_path: &str, executable: bool) -> Result<(), String> {
    let file = open_readonly_nofollow(file_path, false)?;
    fchmod_regular(&file, file_path, file_mode(executable))
}

/// Port of `ensurePrivateDirectorySync`: `mkdirSync` always runs, then
/// posix platforms open the directory without following symlinks and chmod.
pub fn ensure_private_directory_sync(
    directory_path: String,
    platform: Option<Value>,
) -> Result<Value, String> {
    // Node `mkdirSync("", {recursive: true})` fails ENOENT; Rust's
    // `create_dir_all("")` succeeds — mirror the failure.
    if directory_path.is_empty() || fs::create_dir_all(&directory_path).is_err() {
        return Err(private_path_error("create", &directory_path));
    }
    if platform.as_ref().is_some_and(is_win32) {
        return Ok(undefined());
    }
    let file = open_readonly_nofollow(&directory_path, true)?;
    file.set_permissions(fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE))
        .map_err(|_| private_path_error("set mode on", &directory_path))?;
    Ok(undefined())
}

/// Port of `repairPrivateFileSync` (sync, posix-only, no creation).
pub fn repair_private_file_sync(file_path: String, options: Value) -> Result<Value, String> {
    let platform = option_member(&options, "platform")?;
    if is_win32(&platform) {
        return Ok(undefined());
    }
    let executable = js_truthy(&option_member(&options, "executable")?);
    repair_file(&file_path, executable)?;
    Ok(undefined())
}

/// Port of `repairPrivateFile` — same repair; recorded as the resolved value.
pub async fn repair_private_file(file_path: String, options: Value) -> Result<Value, String> {
    repair_private_file_sync(file_path, options)
}

/// Port of `ensurePrivateFileSync`: non-posix creates with mode `0o600` and
/// returns; posix opens `O_WRONLY|O_CREAT|O_APPEND|O_NOFOLLOW` and chmods.
pub fn ensure_private_file_sync(file_path: String, options: Value) -> Result<Value, String> {
    let platform = option_member(&options, "platform")?;
    if is_win32(&platform) {
        OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&file_path)
            .map_err(|_| private_path_error("create", &file_path))?;
        return Ok(undefined());
    }
    let executable = js_truthy(&option_member(&options, "executable")?);
    let mode = file_mode(executable);
    if fs::symlink_metadata(&file_path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err(private_path_error(
            "open without following symlinks",
            &file_path,
        ));
    }
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(mode)
        .open(&file_path)
        .map_err(|_| private_path_error("open without following symlinks", &file_path))?;
    fchmod_regular(&file, &file_path, mode)?;
    Ok(undefined())
}

/// Port of `repairPrivateTreeSync`: lstat-guided walk — a symlinked root is
/// refused, inner symlinks skipped, directories chmodded and recursed,
/// regular files repaired keeping the owner-executable bit.
pub fn repair_private_tree_sync(
    root_path: String,
    platform: Option<Value>,
) -> Result<Value, String> {
    if platform.as_ref().is_some_and(is_win32) {
        return Ok(undefined());
    }
    visit(&root_path, true)?;
    Ok(undefined())
}

fn visit(entry_path: &str, is_root: bool) -> Result<(), String> {
    let stat =
        fs::symlink_metadata(entry_path).map_err(|_| private_path_error("inspect", entry_path))?;
    if stat.file_type().is_symlink() {
        if is_root {
            return Err(private_path_error("repair", entry_path));
        }
        return Ok(());
    }
    if stat.is_dir() {
        fs::set_permissions(
            entry_path,
            fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .map_err(|_| private_path_error("set mode on", entry_path))?;
        let entries =
            fs::read_dir(entry_path).map_err(|_| private_path_error("read", entry_path))?;
        for entry in entries {
            let entry = entry.map_err(|_| private_path_error("read", entry_path))?;
            let child = std::path::Path::new(entry_path)
                .join(entry.file_name())
                .to_string_lossy()
                .into_owned();
            visit(&child, false)?;
        }
        return Ok(());
    }
    if stat.is_file() {
        repair_file(entry_path, stat.mode() & 0o100 != 0)?;
    }
    Ok(())
}
