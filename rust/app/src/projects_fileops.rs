//! `projects.readFile` / `projects.writeFile` / `projects.searchContent` /
//! `projects.createLocalFilePreviewGrant` — workspace file access.
//!
//! Semantics mirror the workspace filesystem contract: versioned text reads
//! (sha256 of the raw bytes), BOM-aware decoding, line-ending detection and
//! normalization, guarded atomic saves, and a short-lived preview grant that
//! authorizes reads of absolute paths.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::projects_search::index_entries;
use crate::todo::WsErr;

const READ_FILE_PATH_MAX: usize = 2048;
const WRITE_FILE_PATH_MAX: usize = 512;
const READ_FILE_MAX_BYTES: usize = 1_000_000;
const WRITE_CONTENT_MAX_BYTES: usize = 1_000_000;
const VERSION_MAX: usize = 128;
const SEARCH_MIN_QUERY: usize = 2;
const SEARCH_MAX_QUERY: usize = 256;
const SEARCH_DEFAULT_LIMIT: usize = 50;
const SEARCH_MAX_LIMIT: usize = 100;
const SEARCH_MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
const SEARCH_MAX_LINE: usize = 1024;
const GRANT_TTL: Duration = Duration::from_secs(120);

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("FILE_IO"))
}

fn cwd_of(payload: &Value) -> Result<PathBuf, WsErr> {
    payload
        .get("cwd")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| PathBuf::from(shellexpand(s)))
        .ok_or_else(|| err("cwd is required"))
}

fn shellexpand(s: &str) -> String {
    if s == "~" {
        return home_dir().to_string_lossy().into_owned();
    }
    if let Some(rest) = s.strip_prefix("~/") {
        return format!("{}/{rest}", home_dir().display());
    }
    s.to_string()
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn canon(path: &Path) -> PathBuf {
    if let Ok(p) = path.canonicalize() {
        return p;
    }
    // Missing target: canonicalize the nearest existing ancestor so the
    // result still agrees with a canonicalized root (e.g. /tmp on macOS).
    let mut dir = path.parent().map(Path::to_path_buf);
    while let Some(d) = dir {
        if let Ok(c) = d.canonicalize() {
            let rest = path.strip_prefix(&d).unwrap_or(path);
            return c.join(rest);
        }
        dir = d.parent().map(Path::to_path_buf);
    }
    path.to_path_buf()
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

// ─── preview grants ─────────────────────────────────────────────────────────

#[derive(Clone)]
struct Grant {
    real_path: PathBuf,
    expires_at: Instant,
}

static GRANTS: LazyLock<Mutex<HashMap<String, Grant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn grant_token() -> String {
    let mut buf = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut buf);
    } else {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        buf[..8].copy_from_slice(&nanos.to_le_bytes());
        buf[8..].copy_from_slice(&std::process::id().to_le_bytes());
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn grant_lookup(token: &str) -> Option<PathBuf> {
    let mut grants = GRANTS.lock().ok()?;
    grants.retain(|_, g| g.expires_at > Instant::now());
    grants.get(token).map(|g| g.real_path.clone())
}

/// Absolute paths need a live preview grant whose recorded real path matches
/// the target. Relative paths must stay inside `cwd` (canonicalized both
/// sides so `..` and symlink escapes fail closed).
fn resolve_read_path(
    cwd: &Path,
    input: &str,
    grant: Option<&str>,
) -> Result<(PathBuf, PathBuf), WsErr> {
    let raw = PathBuf::from(shellexpand(input));
    let cwd_canon = canon(cwd);
    if raw.is_absolute() {
        let token = grant.ok_or_else(|| err("absolute paths require a preview grant"))?;
        let granted = grant_lookup(token).ok_or_else(|| err("preview grant expired or unknown"))?;
        let target = canon(&raw);
        if target != granted {
            return Err(err("preview grant does not match the requested path"));
        }
        return Ok((target, raw));
    }
    let joined = canon(&cwd.join(&raw));
    if !joined.starts_with(&cwd_canon) {
        return Err(err("path escapes the workspace"));
    }
    let rel = joined.strip_prefix(&cwd_canon).unwrap().to_path_buf();
    Ok((joined, rel))
}

/// Fallback for a missing relative read: a unique suffix hit in the project
/// search index (same idea as resolveWorkspaceFileReferences).
fn suffix_fallback(cwd: &Path, rel: &str) -> Option<PathBuf> {
    let suffix = format!("/{rel}");
    let mut hit: Option<String> = None;
    for entry in index_entries(cwd) {
        if entry.rel == rel || entry.rel.ends_with(&suffix) {
            if hit.is_some() {
                return None;
            }
            hit = Some(entry.rel.clone());
        }
    }
    hit.map(|rel| cwd.join(rel))
}

// ─── text decode/encode ─────────────────────────────────────────────────────

fn is_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0)
}

fn detect_line_ending(text: &str) -> Option<&'static str> {
    let bytes = text.as_bytes();
    let (mut crlf, mut lf, mut cr) = (0usize, 0usize, 0usize);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    crlf += 1;
                    i += 1;
                } else {
                    cr += 1;
                }
            }
            b'\n' => lf += 1,
            _ => {}
        }
        i += 1;
    }
    let kinds = [crlf > 0, lf > 0, cr > 0].iter().filter(|b| **b).count();
    match (kinds, crlf, lf, cr) {
        (0, ..) => None,
        (n, ..) if n > 1 => Some("mixed"),
        (_, n, ..) if n > 0 => Some("crlf"),
        (_, _, n, ..) if n > 0 => Some("lf"),
        _ => Some("cr"),
    }
}

fn decode_text(raw: &[u8]) -> Result<(String, &'static str), WsErr> {
    if is_binary(raw) {
        return Err(err("refusing to read a binary file"));
    }
    let (encoding, body) = if raw.starts_with(&[0xEF, 0xBB, 0xBF]) {
        ("utf8-bom", &raw[3..])
    } else {
        ("utf8", raw)
    };
    let text = String::from_utf8(body.to_vec()).map_err(|_| err("file is not valid UTF-8"))?;
    Ok((text, encoding))
}

fn encode_text(contents: &str, encoding: &str, line_ending: &str) -> Vec<u8> {
    let mut body = contents.to_string();
    let mut normalized = String::with_capacity(body.len());
    let bytes = body.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' => {
                normalized.push('\n');
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    i += 1;
                }
            }
            _ => normalized.push(bytes[i] as char),
        }
        i += 1;
    }
    body = match line_ending {
        "crlf" => normalized.replace('\n', "\r\n"),
        "cr" => normalized.replace('\n', "\r"),
        _ => normalized,
    };
    let mut out = Vec::with_capacity(body.len() + 3);
    if encoding == "utf8-bom" {
        out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    out.extend_from_slice(body.as_bytes());
    out
}

// ─── methods ────────────────────────────────────────────────────────────────

pub fn read_file(payload: &Value) -> Result<Value, WsErr> {
    let cwd = cwd_of(payload)?;
    let rel = payload
        .get("relativePath")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && s.len() <= READ_FILE_PATH_MAX)
        .ok_or_else(|| err("relativePath is required"))?;
    let max_bytes = payload
        .get("maxBytes")
        .and_then(|v| v.as_u64())
        .filter(|n| *n <= READ_FILE_MAX_BYTES as u64)
        .map(|n| n as usize)
        .unwrap_or(READ_FILE_MAX_BYTES);
    let grant = payload.get("previewGrant").and_then(|v| v.as_str());

    let (mut target, display_rel) = resolve_read_path(&cwd, rel, grant)?;
    let mut rel_out = display_rel.to_string_lossy().into_owned();
    if !target.exists() && !PathBuf::from(rel).is_absolute() {
        if let Some(alt) = suffix_fallback(&cwd, rel) {
            if alt.exists() {
                rel_out = alt
                    .strip_prefix(canon(&cwd))
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_else(|_| alt.to_string_lossy().into_owned());
                target = alt;
            }
        }
    }
    let is_symlink = std::fs::symlink_metadata(&target)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);
    let meta = std::fs::metadata(&target).map_err(|_| err(format!("file not found: {rel_out}")))?;
    if !meta.is_file() {
        return Err(err(format!("not a file: {rel_out}")));
    }
    let raw = std::fs::read(&target).map_err(|_| err(format!("cannot read {rel_out}")))?;

    let truncated = raw.len() > max_bytes;
    let body = if truncated {
        &raw[..max_bytes]
    } else {
        &raw[..]
    };
    let (contents, version, encoding, line_ending) = if truncated {
        (
            String::from_utf8_lossy(body).into_owned(),
            Value::Null,
            Value::Null,
            Value::Null,
        )
    } else {
        let (text, encoding) = decode_text(body)?;
        let le = detect_line_ending(&text);
        let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
        (
            normalized,
            json!(format!("sha256:{}", hex_digest(&raw))),
            json!(encoding),
            le.map(|s| json!(s)).unwrap_or(Value::Null),
        )
    };

    let mut out = json!({
        "relativePath": rel_out,
        "contents": contents,
        "truncated": truncated,
        "version": version,
        "encoding": encoding,
        "lineEnding": line_ending,
    });
    if is_symlink {
        out["symlink"] = json!(true);
    }
    Ok(out)
}

pub fn write_file(payload: &Value) -> Result<Value, WsErr> {
    let cwd = cwd_of(payload)?;
    let rel = payload
        .get("relativePath")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && s.len() <= WRITE_FILE_PATH_MAX)
        .ok_or_else(|| err("relativePath is required"))?;
    let contents = payload
        .get("contents")
        .and_then(|v| v.as_str())
        .filter(|s| s.len() <= WRITE_CONTENT_MAX_BYTES)
        .ok_or_else(|| err("contents is required"))?;
    let expected = payload.get("expectedVersion").and_then(|v| v.as_str());
    if let Some(v) = expected {
        if v.len() > VERSION_MAX {
            return Err(err("expectedVersion too long"));
        }
    }
    let encoding = payload.get("encoding").and_then(|v| v.as_str());
    let line_ending = payload.get("lineEnding").and_then(|v| v.as_str());
    if expected.is_some() {
        match (encoding, line_ending) {
            (Some("utf8") | Some("utf8-bom"), Some("lf") | Some("crlf") | Some("cr")) => {}
            _ => {
                return Err(err(
                    "Guarded text saves require a supported encoding and consistent line endings.",
                ));
            }
        }
    }

    let raw_path = PathBuf::from(shellexpand(rel));
    let cwd_canon = canon(&cwd);
    let target = if raw_path.is_absolute() {
        canon(&raw_path)
    } else {
        canon(&cwd.join(&raw_path))
    };
    if !target.starts_with(&cwd_canon) {
        return Err(err("path escapes the workspace"));
    }

    if let Some(expected) = expected {
        let current = std::fs::read(&target).map_err(|_| err(format!("file not found: {rel}")))?;
        if is_binary(&current) {
            return Err(err("refusing to write a binary file"));
        }
        let version = format!("sha256:{}", hex_digest(&current));
        if version != expected {
            return Err(err("file has been modified since it was read"));
        }
    }

    let bytes = match (encoding, line_ending) {
        (Some(enc), Some(le)) => encode_text(contents, enc, le),
        _ => contents.as_bytes().to_vec(),
    };

    let parent = target
        .parent()
        .ok_or_else(|| err("path has no parent directory"))?;
    let name = target
        .file_name()
        .ok_or_else(|| err("path has no file name"))?
        .to_string_lossy();
    let temp = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        grant_token()
    ));
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| err(format!("cannot create temp file: {e}")))?;
        if let Err(e) = f.write_all(&bytes).and_then(|_| f.sync_all()) {
            let _ = std::fs::remove_file(&temp);
            return Err(err(format!("cannot write temp file: {e}")));
        }
        if let Ok(meta) = std::fs::metadata(&target) {
            let _ = std::fs::set_permissions(&temp, meta.permissions());
        }
    }
    if std::fs::rename(&temp, &target).is_err() {
        let _ = std::fs::remove_file(&temp);
        return Err(err("atomic rename failed"));
    }

    let version = format!("sha256:{}", hex_digest(&bytes));
    Ok(json!({ "relativePath": rel, "version": version }))
}

pub fn search_content(payload: &Value) -> Result<Value, WsErr> {
    let cwd = cwd_of(payload)?;
    let query = payload
        .get("query")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|q| (SEARCH_MIN_QUERY..=SEARCH_MAX_QUERY).contains(&q.chars().count()))
        .ok_or_else(|| err("query must be 2-256 characters"))?;
    let limit = payload
        .get("limit")
        .and_then(|v| v.as_u64())
        .filter(|n| *n >= 1 && *n <= SEARCH_MAX_LIMIT as u64)
        .map(|n| n as usize)
        .unwrap_or(SEARCH_DEFAULT_LIMIT);

    let needle = query.to_lowercase();
    let mut matches = Vec::new();
    let mut truncated = false;
    'outer: for entry in index_entries(&cwd) {
        if entry.kind != "file" {
            continue;
        }
        let path = cwd.join(&entry.rel);
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() || meta.len() > SEARCH_MAX_FILE_BYTES {
            continue;
        }
        let Ok(raw) = std::fs::read(&path) else {
            continue;
        };
        if is_binary(&raw[..raw.len().min(8192)]) {
            continue;
        }
        let text = String::from_utf8_lossy(&raw);
        let mut found_in_file = false;
        for (idx, line) in text.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                let line_text = line.chars().take(SEARCH_MAX_LINE).collect::<String>();
                matches.push(json!({
                    "path": entry.rel,
                    "lineNumber": idx + 1,
                    "lineText": line_text,
                }));
                if matches.len() >= limit {
                    truncated = true;
                    break 'outer;
                }
                found_in_file = true;
            }
        }
        let _ = found_in_file;
    }
    Ok(json!({ "matches": matches, "truncated": truncated }))
}

pub fn create_local_file_preview_grant(payload: &Value) -> Result<Value, WsErr> {
    let path = payload
        .get("path")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty() && s.len() <= READ_FILE_PATH_MAX)
        .ok_or_else(|| err("path is required"))?;
    let raw = PathBuf::from(shellexpand(path));
    let candidate = if raw.is_absolute() {
        raw
    } else {
        let cwd = cwd_of(payload)?;
        let joined = canon(&cwd.join(&raw));
        if !joined.starts_with(canon(&cwd)) {
            return Err(err("path escapes the workspace"));
        }
        joined
    };
    let real = candidate
        .canonicalize()
        .map_err(|_| err(format!("file not found: {path}")))?;
    let meta = std::fs::metadata(&real).map_err(|_| err(format!("file not found: {path}")))?;
    if !meta.is_file() {
        return Err(err("path is not a file"));
    }

    let token = grant_token();
    let expires_at = Instant::now() + GRANT_TTL;
    GRANTS
        .lock()
        .map_err(|_| err("grant store unavailable"))?
        .insert(
            token.clone(),
            Grant {
                real_path: real.clone(),
                expires_at,
            },
        );
    let expires_ms = SystemTime::now() + GRANT_TTL;
    let secs = expires_ms
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(json!({
        "grant": token,
        "expiresAt": iso8601(secs),
        "canonicalPath": real.to_string_lossy(),
    }))
}

pub(crate) fn iso8601(secs: u64) -> String {
    let days = secs / 86_400;
    let tod = secs % 86_400;
    let (mut y, mut d) = (1970i64, days as i64);
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let days_in_year = if leap { 366 } else { 365 };
        if d < days_in_year {
            break;
        }
        d -= days_in_year;
        y += 1;
    }
    const MONTH_DAYS: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let mut m = 0usize;
    while m < 12 {
        let dim = MONTH_DAYS[m] + i64::from(m == 1 && leap);
        if d < dim {
            break;
        }
        d -= dim;
        m += 1;
    }
    format!(
        "{y:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        m + 1,
        d + 1,
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60
    )
}

pub fn handle(tag: &str, payload: &Value) -> Result<Option<Value>, WsErr> {
    match tag {
        "projects.readFile" => Ok(Some(read_file(payload)?)),
        "projects.writeFile" => Ok(Some(write_file(payload)?)),
        "projects.searchContent" => Ok(Some(search_content(payload)?)),
        "projects.createLocalFilePreviewGrant" => {
            Ok(Some(create_local_file_preview_grant(payload)?))
        }
        _ => Err(WsErr(format!("unknown tag {tag}"), None)),
    }
}
