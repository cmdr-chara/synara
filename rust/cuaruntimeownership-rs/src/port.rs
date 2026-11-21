//! The Rust port of `apps/desktop/src/cuaRuntimeOwnership.ts`.

#![forbid(unsafe_code)]

use std::io::Write;

fn pjoin(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    let absolute = joined.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => continue,
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
    let body = out.join("/");
    match (absolute, body.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => format!("/{body}"),
        (false, true) => ".".to_string(),
        (false, false) => body,
    }
}

/// Port of `cuaHostProcessIsAlive` (`apps/desktop/src/cuaRuntimeOwnership.ts:19`).
pub fn cua_host_process_is_alive(pid: f64) -> Result<bool, String> {
    // Non-integer or out-of-int32 pids make process.kill throw a validation
    // error that is caught and reported as alive.
    if !pid.is_finite() || pid.fract() != 0.0 || pid < -2147483648.0 || pid > 2147483647.0 {
        return Ok(true);
    }
    if pid <= 0.0 {
        // 0 targets the caller's process group; negatives hit groups and raise
        // EPERM at worst — both survive the ESRCH-only check.
        return Ok(true);
    }
    let probe = std::process::Command::new("/bin/kill")
        .arg("-0")
        .arg(format!("{}", pid as i64))
        .output();
    match probe {
        Ok(out) if out.status.success() => Ok(true),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            Ok(!stderr.contains("No such process"))
        }
        Err(_) => Ok(true),
    }
}

fn node_open_error(e: &std::io::Error, path: &str) -> String {
    let (code, msg) = match e.raw_os_error() {
        Some(2) => ("ENOENT", "no such file or directory"),
        Some(13) => ("EACCES", "permission denied"),
        Some(1) => ("EPERM", "operation not permitted"),
        Some(21) => ("EISDIR", "illegal operation on a directory"),
        Some(20) => ("ENOTDIR", "not a directory"),
        Some(36) => ("ENAMETOOLONG", "name too long"),
        Some(17) => ("EEXIST", "file already exists"),
        _ => ("EIO", "i/o error"),
    };
    format!("{code}: {msg}, open '{path}'")
}

/// Port of `markCuaRuntimeDirectory` (`apps/desktop/src/cuaRuntimeOwnership.ts:29`).
pub async fn mark_cua_runtime_directory(directory: String) -> Result<(), String> {
    let path = pjoin(&[&directory, ".synara-cua-runtime.json"]);
    let marker = serde_json::to_string(&serde_json::json!({
        "schema": "synara-cua-runtime",
        "version": 1,
        "directory": std::path::Path::new(&directory)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default(),
        "hostPid": std::process::id(),
        "ownerUid": null,
        "createdAt": 0,
    }))
    .unwrap_or_default()
        + "\n";
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| node_open_error(&e, &path))?;
    file.write_all(marker.as_bytes())
        .map_err(|e| node_open_error(&e, &path))?;
    Ok(())
}
