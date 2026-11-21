//! The Rust port of `packages/shared/src/staticSnapshot.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::fs;

fn node_path_err(v: &Value) -> String {
    let d = if v.is_null() {
        "null".to_string()
    } else if crate::js::is_undefined(v) {
        "undefined".to_string()
    } else if let Value::Bool(b) = v {
        format!("type boolean ({b})")
    } else if let Value::Number(n) = v {
        format!("type number ({n})")
    } else if v.is_array() {
        "an instance of Array".to_string()
    } else {
        "an instance of Object".to_string()
    };
    format!("The \"path\" argument must be of type string. Received {d}")
}

#[rustfmt::skip]
fn node_fs_err(e: &std::io::Error, syscall: &str, from: &str, to: Option<&str>) -> String {
    let (code, msg) = match e.raw_os_error() {
        Some(1) => ("EPERM", "operation not permitted"),
        Some(2) => ("ENOENT", "no such file or directory"),
        Some(13) => ("EACCES", "permission denied"),
        Some(17) => ("EEXIST", "file already exists"),
        Some(20) => ("ENOTDIR", "not a directory"),
        Some(21) => ("EISDIR", "illegal operation on a directory"),
        Some(28) => ("ENOSPC", "no space left on device"),
        Some(30) => ("EROFS", "read-only file system"),
        #[cfg(target_os = "macos")]
        Some(66) => ("ENOTEMPTY", "directory not empty"),
        #[cfg(not(target_os = "macos"))]
        Some(39) => ("ENOTEMPTY", "directory not empty"),
        #[cfg(target_os = "macos")]
        Some(63) => ("ENAMETOOLONG", "name too long"),
        #[cfg(not(target_os = "macos"))]
        Some(36) => ("ENAMETOOLONG", "name too long"),
        _ => ("UNKNOWN", ""),
    };
    let mut s = if code == "UNKNOWN" {
        format!("UNKNOWN: unknown error, {syscall} '{from}'")
    } else {
        format!("{code}: {msg}, {syscall} '{from}'")
    };
    if let Some(t) = to {
        s.push_str(&format!("' -> '{t}'"));
    }
    s
}

/// Node `path.posix.join`: join with `/`, collapse slashes, resolve `.`/`..`.
fn path_join(parts: &[&Value]) -> Result<String, String> {
    let mut segs = Vec::with_capacity(parts.len());
    for p in parts {
        match p.as_str() {
            Some(s) => segs.push(s.to_string()),
            None => return Err(node_path_err(p)),
        }
    }
    let joined = segs.join("/");
    let absolute = joined.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|t| *t != "..") {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let r = if absolute {
        format!("/{}", out.join("/"))
    } else if out.is_empty() {
        ".".to_string()
    } else {
        out.join("/")
    };
    Ok(r)
}

fn exists_sync(p: &str) -> bool {
    fs::metadata(p).is_ok()
}

fn copy_tree(src: &str, dst: &str) -> Result<(), String> {
    let rd = fs::read_dir(src).map_err(|e| node_fs_err(&e, "scandir", src, None))?;
    fs::create_dir_all(dst).map_err(|e| node_fs_err(&e, "mkdir", dst, None))?;
    for ent in rd {
        let ent = ent.map_err(|e| node_fs_err(&e, "scandir", src, None))?;
        let ft = ent
            .file_type()
            .map_err(|e| node_fs_err(&e, "stat", &ent.path().to_string_lossy(), None))?;
        let name = ent.file_name().to_string_lossy().to_string();
        let s = format!("{src}/{name}");
        let d = format!("{dst}/{name}");
        if ft.is_dir() {
            copy_tree(&s, &d)?;
        } else if ft.is_file() {
            fs::copy(&s, &d).map_err(|e| node_fs_err(&e, "copyfile", &s, Some(&d)))?;
        }
    }
    Ok(())
}

fn snapshot_name(signature: &str) -> String {
    let mut out = String::with_capacity(signature.len());
    for u in signature.encode_utf16() {
        match u {
            0x30..=0x39 | 0x41..=0x5A | 0x61..=0x7A | 0x2D | 0x2E | 0x5F => {
                out.push(char::from_u32(u as u32).unwrap_or('_'))
            }
            _ => out.push('_'),
        }
    }
    out
}

/// Port of `findAsarArchivePath` (`packages/shared/src/staticSnapshot.ts:27`).
#[rustfmt::skip]
pub fn find_asar_archive_path(candidate_path: Value) -> Result<Option<String>, String> {
    if candidate_path.is_null() {
        return Err("Cannot read properties of null (reading 'split')".into());
    }
    if crate::js::is_undefined(&candidate_path) {
        return Err("Cannot read properties of undefined (reading 'split')".into());
    }
    let Some(p) = candidate_path.as_str() else {
        return Err("candidatePath.split is not a function".into());
    };
    let norm: String = p.chars().map(|c| if c == '\\' { '/' } else { c }).collect();
    let bytes = norm.as_bytes();
    let mut i = 0;
    while i + 5 <= bytes.len() {
        if &bytes[i..i + 5] == b".asar" && (i + 5 == bytes.len() || bytes[i + 5] == b'/') {
            return Ok(Some(norm[..i + 5].to_string()));
        }
        i += 1;
    }
    Ok(None)
}

/// Port of `snapshotDirectoryName` (`packages/shared/src/staticSnapshot.ts:36`).
pub fn snapshot_directory_name(signature: Value) -> Result<String, String> {
    if signature.is_null() {
        return Err("Cannot read properties of null (reading 'replace')".into());
    }
    if crate::js::is_undefined(&signature) {
        return Err("Cannot read properties of undefined (reading 'replace')".into());
    }
    let Some(s) = signature.as_str() else {
        return Err("signature.replace is not a function".into());
    };
    Ok(snapshot_name(s))
}

/// Port of `ensureStaticSnapshot` (`packages/shared/src/staticSnapshot.ts:41`).
#[rustfmt::skip]
pub fn ensure_static_snapshot(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'sentinelFile')".into());
    }
    if crate::js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'sentinelFile')".into());
    }
    let sentinel = match input.get("sentinelFile") {
        None => Value::String("index.html".into()),
        Some(Value::Null) => Value::String("index.html".into()),
        Some(v) if crate::js::is_undefined(v) => Value::String("index.html".into()),
        Some(v) => v.clone(),
    };
    let signature = input.get("signature").cloned().unwrap_or_else(crate::js::undefined);
    let dir_name = snapshot_directory_name(signature)?;
    let cache_root = input.get("cacheRoot").cloned().unwrap_or_else(crate::js::undefined);
    let dest_dir = path_join(&[&cache_root, &Value::String(dir_name.clone())])?;
    if exists_sync(&path_join(&[&Value::String(dest_dir.clone()), &sentinel])?) {
        return Ok(json!({ "dir": dest_dir, "reused": true }));
    }
    let source_dir = input.get("sourceDir").cloned().unwrap_or_else(crate::js::undefined);
    let sentinel_path = path_join(&[&source_dir, &sentinel])?;
    if !exists_sync(&sentinel_path) {
        let s = sentinel.as_str().unwrap_or("");
        let d = source_dir.as_str().unwrap_or("");
        return Err(format!("Static snapshot source is missing {s}: {d}"));
    }
    let staging = path_join(&[
        &cache_root,
        &Value::String(format!(".staging-{dir_name}-{}", rand_u32())),
    ])?;
    let src = match source_dir.as_str() {
        Some(s) => s.to_string(),
        None => return Err(node_path_err(&source_dir)),
    };
    copy_tree(&src, &staging)?;
    fs::rename(&staging, &dest_dir)
        .map_err(|e| node_fs_err(&e, "rename", &staging, Some(&dest_dir)))?;
    Ok(json!({ "dir": dest_dir, "reused": false }))
}

fn rand_u32() -> u32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    ((t.subsec_nanos() as u64 ^ t.as_secs() ^ (std::process::id() as u64)) % 100_000) as u32
}
