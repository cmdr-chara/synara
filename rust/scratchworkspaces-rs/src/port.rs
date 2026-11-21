//! The Rust port of `apps/server/src/scratchWorkspaces.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;
use std::sync::LazyLock;

use crate::js::{as_number, is_undefined, number_to_string, string_units};

static UNDEF: LazyLock<Value> = LazyLock::new(crate::js::undefined);

const DIRNAME: &str = "synara-codex-workspaces";
const PRIVATE_DIR_MODE: u32 = 0o700;

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

/// `String(v)` / template coercion.
fn js_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if is_nullish(e) {
                    String::new()
                } else {
                    js_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if is_undefined(v) {
                "undefined".to_string()
            } else if let Some(x) = as_number(v) {
                number_to_string(x)
            } else if let Some(u) = string_units(v) {
                String::from_utf16_lossy(&u)
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn js_falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_none_or(|x| x == 0.0 || x.is_nan()),
        Value::String(s) => s.is_empty(),
        Value::Object(_) => is_undefined(v) || as_number(v).is_some_and(|x| x.is_nan() || x == 0.0),
        Value::Array(_) => false,
    }
}

fn sha256_hex(s: &str) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(s.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn process_uid() -> Option<u32> {
    static UID: LazyLock<Option<u32>> = LazyLock::new(|| {
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse().ok())
    });
    *UID
}

fn homedir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/Users/devin".to_string())
}

fn tmpdir() -> String {
    std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string())
}

// --- node:path posix subset -------------------------------------------------

fn normalize(p: &str) -> String {
    if p.is_empty() {
        return ".".to_string();
    }
    let is_abs = p.starts_with('/');
    let trailing = p.ends_with('/');
    let mut stack: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if stack.last().is_some_and(|t| *t != "..") {
                    stack.pop();
                } else if !is_abs {
                    stack.push("..");
                }
            }
            s => stack.push(s),
        }
    }
    let joined = stack.join("/");
    let mut res = format!("{}{}", if is_abs { "/" } else { "" }, joined);
    if res.is_empty() {
        res = if is_abs { "/" } else { "." }.to_string();
    } else if trailing && res != "/" {
        res.push('/');
    }
    res
}

fn js_join(segments: &[String]) -> String {
    let mut joined: Option<String> = None;
    for seg in segments {
        if seg.is_empty() {
            continue;
        }
        joined = Some(match joined {
            Some(j) => format!("{j}/{seg}"),
            None => seg.clone(),
        });
    }
    match joined {
        Some(j) => normalize(&j),
        None => ".".to_string(),
    }
}

fn js_resolve(p: &str) -> String {
    if p.starts_with('/') {
        normalize(p)
    } else {
        let cwd = std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/".to_string());
        normalize(&format!("{cwd}/{p}"))
    }
}

fn js_dirname(p: &str) -> String {
    let end = p.trim_end_matches('/');
    if end.is_empty() {
        return "/".to_string();
    }
    match end.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(i) => end[..i].to_string(),
    }
}

// --- module internals -------------------------------------------------------

fn scratch_owner_segment(home_directory: &str) -> String {
    let owner = match process_uid() {
        Some(uid) => format!("uid:{uid}"),
        None => home_directory.to_string(),
    };
    sha256_hex(&owner)[..16].to_string()
}

fn scratch_cache_root(home_directory: &str, platform: &Value) -> String {
    if platform.as_str() == Some("darwin") {
        js_join(&[
            home_directory.to_string(),
            "Library".to_string(),
            "Caches".to_string(),
            "synara".to_string(),
        ])
    } else if platform.as_str() == Some("win32") {
        js_join(&[
            home_directory.to_string(),
            "AppData".to_string(),
            "Local".to_string(),
            "Synara".to_string(),
            "Cache".to_string(),
        ])
    } else {
        js_join(&[
            home_directory.to_string(),
            ".cache".to_string(),
            "synara".to_string(),
        ])
    }
}

fn scratch_workspace_segment(thread_id: &Value) -> String {
    let raw = js_string(thread_id);
    let mut safe = String::with_capacity(raw.len());
    let mut last_dash = false;
    for ch in raw.chars() {
        let ok = ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-';
        if ok {
            safe.push(ch);
            last_dash = false;
        } else if !last_dash {
            safe.push('-');
            last_dash = true;
        }
    }
    let safe = safe.trim_start_matches('.');
    let safe: String = safe.chars().take(64).collect();
    let digest = &sha256_hex(&raw)[..12];
    format!(
        "{}-{digest}",
        if safe.is_empty() { "thread" } else { &safe }
    )
}

// --- fs helpers (posix, O_NOFOLLOW-equivalent via symlink_metadata) ---------

fn ensure_private_dir(path: &Value) -> Result<(), String> {
    let p = match path {
        Value::String(s) => s.clone(),
        _ => {
            return Err(format!("Failed to create private path {}", js_string(path)));
        }
    };
    if p.is_empty() {
        return Err(format!("Failed to create private path {p}"));
    }
    if std::fs::create_dir_all(&p).is_err() {
        return Err(format!("Failed to create private path {p}"));
    }
    match std::fs::symlink_metadata(&p) {
        Ok(md) if md.file_type().is_symlink() || !md.is_dir() => {
            return Err(format!(
                "Failed to open without following symlinks private path {p}"
            ));
        }
        Err(_) => {
            return Err(format!(
                "Failed to open without following symlinks private path {p}"
            ));
        }
        _ => {}
    }
    use std::os::unix::fs::PermissionsExt;
    if std::fs::set_permissions(&p, std::fs::Permissions::from_mode(PRIVATE_DIR_MODE)).is_err() {
        return Err(format!("Failed to set mode on private path {p}"));
    }
    Ok(())
}

fn is_owned_real_directory(p: &str) -> bool {
    use std::os::unix::fs::MetadataExt;
    match std::fs::symlink_metadata(p) {
        Ok(md) => {
            md.is_dir()
                && !md.file_type().is_symlink()
                && process_uid().is_none_or(|u| md.uid() == u)
        }
        Err(_) => false,
    }
}

/// openOwnedDirectoryDescriptor on a posix host: is the path openable as an
/// owned, non-symlink directory.
fn open_owned_directory(p: &str) -> bool {
    is_owned_real_directory(p)
}

fn repair_owned_workspace(p: &str) -> Option<String> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if !open_owned_directory(p) {
        return None;
    }
    let opened = std::fs::metadata(p).ok()?;
    let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(PRIVATE_DIR_MODE));
    let current = std::fs::symlink_metadata(p).ok()?;
    if !current.is_dir()
        || current.file_type().is_symlink()
        || current.dev() != opened.dev()
        || current.ino() != opened.ino()
    {
        return None;
    }
    Some(p.to_string())
}

fn secure_owned_legacy_root(p: &str) -> bool {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    if !open_owned_directory(p) {
        return false;
    }
    let Some(opened) = std::fs::metadata(p).ok() else {
        return false;
    };
    let mode = opened.mode();
    let writable_by_others = (mode & 0o022) != 0;
    let has_sticky = (mode & 0o1000) != 0;
    if writable_by_others && !has_sticky {
        let _ =
            std::fs::set_permissions(p, std::fs::Permissions::from_mode((mode & 0o7777) | 0o1000));
    }
    match std::fs::symlink_metadata(p) {
        Ok(cur) => {
            cur.is_dir()
                && !cur.file_type().is_symlink()
                && cur.dev() == opened.dev()
                && cur.ino() == opened.ino()
        }
        Err(_) => false,
    }
}

fn repair_legacy_scratch_workspace(legacy_root: &str, segment: &str) -> Option<String> {
    if !secure_owned_legacy_root(legacy_root) {
        return None;
    }
    let dir = js_join(&[legacy_root.to_string(), segment.to_string()]);
    repair_owned_workspace(&dir)
}

// --- exports ----------------------------------------------------------------

fn resolve_root(options: &Value) -> Result<String, String> {
    if options.is_null() && !is_undefined(options) {
        return Err("Cannot read properties of null (reading 'homeDirectory')".to_string());
    }
    let home_v = member(options, "homeDirectory");
    let home = if is_undefined(home_v) {
        homedir()
    } else {
        js_string(home_v)
    };
    let platform_v = member(options, "platform");
    let default_platform = Value::String("darwin".to_string());
    let platform = if is_nullish(platform_v) {
        &default_platform
    } else {
        platform_v
    };
    let cache_root = scratch_cache_root(&home, platform);
    let owner_container = js_join(&[
        cache_root,
        format!(".synara-{}", scratch_owner_segment(&home)),
    ]);
    Ok(js_join(&[owner_container, DIRNAME.to_string()]))
}

/// Port of `resolveScratchWorkspacesRoot` (`apps/server/src/scratchWorkspaces.ts:32`).
pub fn resolve_scratch_workspaces_root(options: serde_json::Value) -> Result<String, String> {
    resolve_root(&options)
}

fn ensure_private_scratch_root(workspace_root: &Value, default_root: &str) -> Result<(), String> {
    if workspace_root.as_str() == Some(default_root) {
        let owner_container = js_dirname(default_root);
        ensure_private_dir(&Value::String(js_dirname(&owner_container)))?;
        ensure_private_dir(&Value::String(owner_container))?;
    }
    ensure_private_dir(workspace_root)
}

fn ensure_private_scratch_workspace(
    workspace_root: &Value,
    segment: &str,
    default_root: &str,
) -> Result<String, String> {
    ensure_private_scratch_root(workspace_root, default_root)?;
    let dir = js_join(&[js_string(workspace_root), segment.to_string()]);
    ensure_private_dir(&Value::String(dir.clone()))?;
    Ok(dir)
}

/// Port of `ensureIsolatedScratchWorkspace` (`apps/server/src/scratchWorkspaces.ts:176`).
pub fn ensure_isolated_scratch_workspace(
    thread_id: serde_json::Value,
    workspace_root: serde_json::Value,
    legacy_workspace_root: serde_json::Value,
) -> Result<String, String> {
    let default_root = resolve_root(&serde_json::json!({}))?;
    let root: &Value = if is_undefined(&workspace_root) {
        &Value::String(default_root.clone())
    } else {
        &workspace_root
    };
    let segment = scratch_workspace_segment(&thread_id);
    let legacy: Option<String> = if is_nullish(&legacy_workspace_root) {
        if root.as_str() == Some(default_root.as_str()) {
            Some(js_join(&[tmpdir(), DIRNAME.to_string()]))
        } else {
            None
        }
    } else {
        Some(js_string(&legacy_workspace_root))
    };
    if let Some(legacy) = legacy
        && let Some(repaired) = repair_legacy_scratch_workspace(&legacy, &segment)
    {
        return Ok(repaired);
    }
    ensure_private_scratch_workspace(root, &segment, &default_root)
}

/// Port of `resolveScratchWorkspaceCwd` (`apps/server/src/scratchWorkspaces.ts:195`).
pub fn resolve_scratch_workspace_cwd(
    thread_id: serde_json::Value,
    persisted_cwd: serde_json::Value,
    roots: serde_json::Value,
) -> Result<String, String> {
    if roots.is_null() {
        return Err("Cannot read properties of null (reading 'workspaceRoot')".to_string());
    }
    let default_root = resolve_root(&serde_json::json!({}))?;
    let wr_member = member(&roots, "workspaceRoot");
    let workspace_root: &Value = if is_undefined(wr_member) {
        &Value::String(default_root.clone())
    } else {
        wr_member
    };
    let legacy_member = member(&roots, "legacyWorkspaceRoot");
    let legacy_root: Value = if is_undefined(legacy_member) {
        Value::String(js_join(&[tmpdir(), DIRNAME.to_string()]))
    } else {
        legacy_member.clone()
    };
    let segment = scratch_workspace_segment(&thread_id);
    if js_falsy(&persisted_cwd) {
        return ensure_isolated_scratch_workspace(thread_id, workspace_root.clone(), legacy_root);
    }
    let persisted = js_string(&persisted_cwd);
    let private_workspace = js_join(&[js_string(workspace_root), segment.clone()]);
    if js_resolve(&persisted) == js_resolve(&private_workspace) {
        return ensure_private_scratch_workspace(workspace_root, &segment, &default_root);
    }
    let legacy_workspace = js_join(&[js_string(&legacy_root), segment.clone()]);
    if js_resolve(&persisted) == js_resolve(&legacy_workspace) {
        let repaired = match &legacy_root {
            Value::String(lr) => repair_legacy_scratch_workspace(lr, &segment),
            _ => None,
        };
        return match repaired {
            Some(p) => Ok(p),
            None => ensure_private_scratch_workspace(workspace_root, &segment, &default_root),
        };
    }
    Ok(persisted)
}
