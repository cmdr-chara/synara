//! The Rust port of `apps/server/src/remoteBillingPolicy.ts`.
//!
//! Ports `recordRemoteBillingPolicyHint`: validates `checkedAt`, resolves the hint file
//! path (`<stateDir>/remote-billing-policy-<sha256(issuer)>.json`), then persists
//! `{"apiIssuer","enforced","checkedAt"}` through the same atomic-write discipline as
//! `writeFileStringAtomically` (private parent dirs, O_EXCL temp file, fsync, rename,
//! directory sync), serialized per file path like `withPolicyWrite`.
//!
//! `requestedAt`/`adopt` only engage a live policy registered by `startRemoteBillingPolicy`,
//! which stays in TS (`js_only`): the port always takes the `persist()` path.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::js::{as_number, is_undefined, number_to_string};

const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const PRIVATE_FILE_MODE: u32 = 0o600;

// --- JS value helpers ---

/// `Date.now()` in milliseconds.
fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// The `checkedAt` validation at the top of the original: `undefined` defaults to
/// `Date.now()`; non-finite or negative values throw when the policy is disabled and
/// collapse to `0` when enforced.
fn resolve_checked_at(value: Option<&Value>, enforced: bool) -> Result<f64, String> {
    let raw = match value {
        Some(v) if is_undefined(v) => return Ok(now_ms()),
        Some(v) => as_number(v).unwrap_or(f64::NAN),
        None => f64::NAN,
    };
    if !raw.is_finite() || raw < 0.0 {
        if !enforced {
            return Err("A disabled remote billing policy requires a valid clock".into());
        }
        return Ok(0.0);
    }
    Ok(raw)
}

/// `accountApiIssuer`: trailing slashes stripped, `/api/v1` appended.
fn api_issuer(account_url: &str) -> String {
    format!("{}/api/v1", account_url.trim_end_matches('/'))
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `JSON.stringify({ apiIssuer, enforced, checkedAt })` with JS number formatting
/// (`-0` prints as `0`, integers without a decimal point).
fn hint_json(api_issuer: &str, enforced: bool, checked_at: f64) -> Result<String, String> {
    Ok(format!(
        "{{\"apiIssuer\":{},\"enforced\":{},\"checkedAt\":{}}}",
        serde_json::to_string(api_issuer).map_err(|e| e.to_string())?,
        if enforced { "true" } else { "false" },
        number_to_string(checked_at),
    ))
}

// --- node:path posix semantics (normalize/resolve/join/dirname only) ---

/// `normalizeString`: resolve `.`/`..` segments and collapse empty ones.
fn normalize_string(path: &str, allow_above_root: bool) -> String {
    let mut res: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." if res.last().is_some_and(|&s| s != "..") => {
                res.pop();
            }
            ".." if allow_above_root => res.push(".."),
            ".." => {}
            _ => res.push(seg),
        }
    }
    res.join("/")
}

/// POSIX `path.normalize` on one path.
fn normalize(p: &str) -> String {
    if p.is_empty() {
        return ".".to_string();
    }
    let absolute = p.starts_with('/');
    let mut out = normalize_string(p, !absolute);
    if out.is_empty() {
        return if absolute { "/" } else { "." }.to_string();
    }
    if p.ends_with('/') {
        out.push('/');
    }
    if absolute {
        out.insert(0, '/');
    }
    out
}

/// POSIX `path.join(a, b)`.
fn join(a: &str, b: &str) -> String {
    if b.is_empty() {
        normalize(a)
    } else if a.is_empty() {
        normalize(b)
    } else {
        normalize(&format!("{a}/{b}"))
    }
}

/// POSIX `path.resolve(p)` against the process cwd.
fn resolve(p: &str) -> String {
    if p.starts_with('/') {
        normalize(p)
    } else {
        let cwd = std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/".to_string());
        normalize(&format!("{cwd}/{p}"))
    }
}

/// POSIX `path.dirname`.
fn dirname(p: &str) -> String {
    if p.is_empty() {
        return ".".to_string();
    }
    let trimmed = p.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    match trimmed.rfind('/') {
        None => ".".to_string(),
        Some(i) => {
            let head = trimmed[..i].trim_end_matches('/');
            if head.is_empty() {
                "/".to_string()
            } else {
                head.to_string()
            }
        }
    }
}

// --- writeFileStringAtomically ---

/// `syncDirectoryEntry`: fsync the directory entry; EINVAL/ENOTSUP/EBADF from the
/// sync call are tolerated, everything else propagates.
#[cfg(unix)]
fn sync_directory_entry(directory_path: &str) -> Result<(), String> {
    let handle = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(directory_path)
        .map_err(|e| e.to_string())?;
    match handle.sync_all() {
        Ok(()) => Ok(()),
        Err(e) => match e.raw_os_error() {
            Some(libc::EINVAL) | Some(libc::ENOTSUP) | Some(libc::EBADF) => Ok(()),
            _ => Err(e.to_string()),
        },
    }
}

#[cfg(not(unix))]
fn sync_directory_entry(_directory_path: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn create_private_dir(path: &str) -> std::io::Result<()> {
    fs::DirBuilder::new()
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(path)
}

#[cfg(not(unix))]
fn create_private_dir(path: &str) -> std::io::Result<()> {
    fs::create_dir(path)
}

/// `fs.open(dir, O_RDONLY | O_DIRECTORY | O_NOFOLLOW)` + fstat on posix; a plain
/// metadata read elsewhere.
#[cfg(unix)]
fn final_dir_stat(directory_path: &str) -> Result<fs::Metadata, String> {
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(directory_path)
        .and_then(|f| f.metadata())
        .map_err(|e| e.to_string())
}

#[cfg(not(unix))]
fn final_dir_stat(directory_path: &str) -> Result<fs::Metadata, String> {
    fs::metadata(directory_path).map_err(|e| e.to_string())
}

/// `ensurePrivateDirectory`: walk up to the nearest existing ancestor (must be a real,
/// non-symlinked, non group/other-writable directory), create the missing chain with
/// private modes, then re-verify the final directory through a no-follow descriptor.
fn ensure_private_directory(directory_path: &str) -> Result<(), String> {
    let mut missing: Vec<String> = Vec::new();
    let mut existing = resolve(directory_path);
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(stat) => {
                if !stat.is_dir() || stat.file_type().is_symlink() {
                    return Err(format!(
                        "Atomic write parent is not a real directory: {existing}"
                    ));
                }
                #[cfg(unix)]
                if stat.mode() & 0o022 != 0 {
                    return Err(format!(
                        "Atomic write parent directory is group/other writable: {existing}"
                    ));
                }
                break;
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {
                missing.insert(0, existing.clone());
                let parent = dirname(&existing);
                if parent == existing {
                    return Err(e.to_string());
                }
                existing = parent;
            }
            Err(e) => return Err(e.to_string()),
        }
    }
    for missing_dir in &missing {
        let mut created = false;
        match create_private_dir(missing_dir) {
            Ok(()) => created = true,
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
        let stat = fs::symlink_metadata(missing_dir).map_err(|e| e.to_string())?;
        if !stat.is_dir() || stat.file_type().is_symlink() {
            return Err(format!(
                "Atomic write parent is not a real directory: {missing_dir}"
            ));
        }
        #[cfg(unix)]
        {
            if created {
                fs::set_permissions(
                    missing_dir,
                    fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
                )
                .map_err(|e| e.to_string())?;
            } else if stat.mode() & 0o022 != 0 {
                return Err(format!(
                    "Atomic write parent directory is group/other writable: {missing_dir}"
                ));
            }
        }
        sync_directory_entry(&dirname(missing_dir))?;
    }
    let stat = final_dir_stat(directory_path)?;
    if !stat.is_dir() {
        return Err(format!(
            "Atomic write parent path is not a directory: {directory_path}"
        ));
    }
    #[cfg(unix)]
    if stat.mode() & 0o022 != 0 {
        return Err(format!(
            "Atomic write parent directory is group/other writable: {directory_path}"
        ));
    }
    Ok(())
}

/// `fs.open(tempPath, O_WRONLY | O_CREAT | O_EXCL | O_NOFOLLOW, mode)`.
#[cfg(unix)]
fn open_temp_excl(temp_path: &str, mode: u32) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .custom_flags(libc::O_NOFOLLOW)
        .open(temp_path)
}

#[cfg(not(unix))]
fn open_temp_excl(temp_path: &str, _mode: u32) -> std::io::Result<File> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp_path)
}

/// `assertSameRegularFile`: descriptor stat and path stat must describe the same
/// regular file (dev/ino identity on posix).
fn assert_same_regular_file(
    descriptor_stat: &fs::Metadata,
    path_stat: &fs::Metadata,
    temp_path: &str,
) -> Result<(), String> {
    if !descriptor_stat.is_file() || !path_stat.is_file() || path_stat.file_type().is_symlink() {
        return Err(format!(
            "Atomic write temporary path is not a regular file: {temp_path}"
        ));
    }
    #[cfg(unix)]
    if descriptor_stat.dev() != path_stat.dev() || descriptor_stat.ino() != path_stat.ino() {
        return Err(format!(
            "Atomic write temporary path identity changed: {temp_path}"
        ));
    }
    Ok(())
}

/// `writeDurableTempFile` + `fs.rename` + `syncDirectoryEntry`, under `withTempFile`'s
/// guaranteed temp cleanup.
fn persist(file_path: &str, contents: &str) -> Result<(), String> {
    let directory_path = dirname(file_path);
    let temp_path = format!("{file_path}.{}.{}.tmp", std::process::id(), uuid_v4());
    let result = (|| {
        ensure_private_directory(&directory_path)?;
        let mut handle =
            open_temp_excl(&temp_path, PRIVATE_FILE_MODE).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        handle
            .set_permissions(fs::Permissions::from_mode(PRIVATE_FILE_MODE))
            .map_err(|e| e.to_string())?;
        handle
            .write_all(contents.as_bytes())
            .map_err(|e| e.to_string())?;
        handle.sync_all().map_err(|e| e.to_string())?;
        let descriptor_stat = handle.metadata().map_err(|e| e.to_string())?;
        drop(handle);
        let path_stat = fs::symlink_metadata(&temp_path).map_err(|e| e.to_string())?;
        assert_same_regular_file(&descriptor_stat, &path_stat, &temp_path)?;
        fs::rename(&temp_path, file_path).map_err(|e| e.to_string())?;
        sync_directory_entry(&directory_path)
    })();
    let _ = fs::remove_file(&temp_path);
    result
}

// --- withPolicyWrite: per-path write serialization ---

fn write_locks() -> &'static Mutex<HashMap<String, Arc<Mutex<()>>>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_for(file_path: &str) -> Arc<Mutex<()>> {
    write_locks()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(file_path.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// `pendingWrites.delete` when the map still points at this write's promise.
fn release_lock_if_current(file_path: &str, lock: &Arc<Mutex<()>>) {
    let mut map = write_locks().lock().unwrap_or_else(|e| e.into_inner());
    if Arc::strong_count(lock) == 2 {
        map.remove(file_path);
    }
}

// --- uuid for the temp file name ---

#[cfg(unix)]
fn getrandom_fill(buf: &mut [u8]) {
    use std::io::Read;
    if let Ok(mut f) = File::open("/dev/urandom") {
        let _ = f.read_exact(buf);
    }
}

#[cfg(not(unix))]
fn getrandom_fill(buf: &mut [u8]) {
    let mut x = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        .wrapping_add(std::process::id() as u64);
    for byte in buf.iter_mut() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *byte = x as u8;
    }
}

fn uuid_v4() -> String {
    let mut b = [0u8; 16];
    getrandom_fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// Port of `recordRemoteBillingPolicyHint` (`apps/server/src/remoteBillingPolicy.ts:57`).
pub async fn record_remote_billing_policy_hint(
    state_dir: String,
    account_url: String,
    enforced: bool,
    checked_at: Option<serde_json::Value>,
    requested_at: Option<f64>,
) -> Result<(), String> {
    let checked_at = resolve_checked_at(checked_at.as_ref(), enforced)?;
    let issuer = api_issuer(&account_url);
    let file_path = join(
        &state_dir,
        &format!("remote-billing-policy-{}.json", sha256_hex(&issuer)),
    );
    let contents = hint_json(&issuer, enforced, checked_at)?;
    // `adopt` only exists while a TS-side `startRemoteBillingPolicy` registered one;
    // the Rust port always takes the `persist()` branch, so `requestedAt` is inert.
    let _ = requested_at;
    let lock = lock_for(&file_path);
    {
        let _guard = lock.lock().unwrap_or_else(|e| e.into_inner());
        persist(&file_path, &contents)?;
    }
    release_lock_if_current(&file_path, &lock);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issuer_strips_trailing_slashes() {
        assert_eq!(api_issuer("https://a.com/"), "https://a.com/api/v1");
        assert_eq!(api_issuer("https://a.com///"), "https://a.com/api/v1");
        assert_eq!(api_issuer("https://a.com"), "https://a.com/api/v1");
        assert_eq!(api_issuer(""), "/api/v1");
        assert_eq!(api_issuer("/"), "/api/v1");
    }

    #[test]
    fn checked_at_rules() {
        // undefined → Date.now()
        let t = resolve_checked_at(Some(&serde_json::json!({"$undefined": true})), false)
            .unwrap_or(0.0);
        assert!(t > 0.0);
        // null / non-number / non-finite / negative → enforced ? 0 : Err
        for v in [
            Value::Null,
            Value::Bool(true),
            Value::from("x"),
            serde_json::json!([]),
            serde_json::json!({"a": "b"}),
            serde_json::json!({"$num": "NaN"}),
            serde_json::json!({"$num": "Infinity"}),
            Value::from(-1),
        ] {
            assert_eq!(
                resolve_checked_at(Some(&v), true).unwrap_or(f64::NAN),
                0.0,
                "{v:?}"
            );
            assert_eq!(
                resolve_checked_at(Some(&v), false).unwrap_err().as_str(),
                "A disabled remote billing policy requires a valid clock",
                "{v:?}"
            );
        }
        // valid numbers pass through; -0 is valid
        assert_eq!(
            resolve_checked_at(Some(&Value::from(42)), false).unwrap_or(-1.0),
            42.0
        );
        assert_eq!(
            resolve_checked_at(Some(&serde_json::json!({"$num": "-0"})), false)
                .map(|n| n.to_bits()),
            Ok((-0.0f64).to_bits())
        );
    }

    #[test]
    fn hint_serializes_like_json_stringify() {
        assert_eq!(
            hint_json("https://a.com/api/v1", true, 0.0).unwrap_or_default(),
            r#"{"apiIssuer":"https://a.com/api/v1","enforced":true,"checkedAt":0}"#
        );
        assert_eq!(
            hint_json("a", false, -0.0).unwrap_or_default(),
            r#"{"apiIssuer":"a","enforced":false,"checkedAt":0}"#
        );
        assert_eq!(
            hint_json("a", true, 0.5).unwrap_or_default(),
            r#"{"apiIssuer":"a","enforced":true,"checkedAt":0.5}"#
        );
    }

    #[test]
    fn posix_path_helpers() {
        assert_eq!(join("a", "b.json"), "a/b.json");
        assert_eq!(join("", "b.json"), "b.json");
        assert_eq!(join("./x/**/*.json", "f.json"), "x/**/*.json/f.json");
        assert_eq!(join("a/", "b"), "a/b");
        assert_eq!(dirname("x/y/z.json"), "x/y");
        assert_eq!(dirname("f.json"), ".");
        assert_eq!(dirname("/a/b"), "/a");
        assert_eq!(dirname(""), ".");
        assert_eq!(dirname("/"), "/");
        assert_eq!(dirname("//a"), "/");
        assert_eq!(dirname("a/b/"), "a");
    }
}
