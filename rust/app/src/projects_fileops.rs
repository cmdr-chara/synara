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
const FORMAT_TEXT_MAX_BYTES: usize = 1_000_000;
const FORMAT_OUTPUT_MAX_BYTES: usize = 2_000_000;

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

// ─── tidy formatting ────────────────────────────────────────────────────────

fn format_err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("FORMAT_ERROR"))
}

fn tidy_failure(error: tidy::Error) -> WsErr {
    match error {
        tidy::Error::Syntax { message, at } => WsErr(
            format!("Syntax error at byte {at}: {message}"),
            Some("FORMAT_SYNTAX"),
        ),
        tidy::Error::TooDeep => WsErr(
            "The document nests too deeply to format.".to_string(),
            Some("FORMAT_TOO_DEEP"),
        ),
        _ => format_err("The document could not be formatted."),
    }
}

fn language_name(language: tidy::Language) -> &'static str {
    match language {
        tidy::Language::Html => "html",
        tidy::Language::Vue => "vue",
        tidy::Language::Mjml => "mjml",
        tidy::Language::Handlebars => "handlebars",
        tidy::Language::Css(tidy::css::Dialect::Css) => "css",
        tidy::Language::Css(tidy::css::Dialect::Scss) => "scss",
        tidy::Language::Css(tidy::css::Dialect::Less) => "less",
        tidy::Language::Json(tidy::json::Dialect::Json) => "json",
        tidy::Language::Json(tidy::json::Dialect::Jsonc) => "jsonc",
        tidy::Language::Json(tidy::json::Dialect::Json5) => "json5",
        tidy::Language::Json(tidy::json::Dialect::Stringify) => "json-stringify",
        tidy::Language::Toml => "toml",
        tidy::Language::Xml => "xml",
        tidy::Language::Yaml => "yaml",
        tidy::Language::Markdown => "markdown",
        tidy::Language::Dockerfile => "dockerfile",
        tidy::Language::Ini(tidy::ini::Dialect::Ini) => "ini",
        tidy::Language::Ini(tidy::ini::Dialect::Git) => "gitconfig",
        tidy::Language::Graphql => "graphql",
        tidy::Language::Js(tidy::js::Dialect::Js) => "javascript",
        tidy::Language::Js(tidy::js::Dialect::Ts) => "typescript",
        _ => "unknown",
    }
}

fn language_from_input(
    path: Option<&str>,
    language: Option<&str>,
) -> Result<tidy::Language, WsErr> {
    if let Some(name) = language.filter(|name| !name.trim().is_empty()) {
        return tidy::Language::hint(name.trim())
            .ok_or_else(|| format_err(format!("tidy does not format language '{name}'")));
    }
    let path = path.ok_or_else(|| format_err("path or language is required"))?;
    tidy::Language::file(path)
        .ok_or_else(|| format_err(format!("tidy does not format this file type: {path}")))
}

fn tidy_options(path: Option<&Path>, _payload: &Value) -> Result<tidy::Options, WsErr> {
    let resolver = tidy::config::Resolver::default();
    match path {
        Some(path) => resolver
            .options(path)
            .map_err(|error| format_err(format!("tidy config error: {error}"))),
        None => Ok(resolver.without_path()),
    }
}

fn preserve_line_endings(options: &mut tidy::Options, line_ending: Option<&str>) {
    let eol = match line_ending {
        Some("crlf") => Some(tidy::Eol::Crlf),
        Some("cr") => Some(tidy::Eol::Cr),
        Some("lf") => Some(tidy::Eol::Lf),
        _ => None,
    };
    if let Some(eol) = eol {
        options.share(&tidy::Shared {
            eol: Some(eol),
            ..tidy::Shared::default()
        });
    }
}

fn format_document(
    text: &str,
    language: tidy::Language,
    options: &tidy::Options,
    path: Option<&str>,
) -> Result<String, WsErr> {
    if text.len() > FORMAT_TEXT_MAX_BYTES {
        return Err(format_err("text exceeds the 1 MB formatting limit"));
    }
    let formatted = tidy::format(text, language, options, path, &mut tidy::embed::Verbatim)
        .map_err(tidy_failure)?;
    if formatted.len() > FORMAT_OUTPUT_MAX_BYTES {
        return Err(format_err("formatted output exceeds the 2 MB limit"));
    }
    Ok(formatted)
}

fn apply_request_config(options: &mut tidy::Options, payload: &Value) -> Result<(), WsErr> {
    if let Some(config) = payload.get("config") {
        let config = tidy::config::Config::from_value(config)
            .map_err(|error| format_err(format!("invalid tidy config: {error}")))?;
        config.apply(options);
    }
    Ok(())
}

pub fn format_text(payload: &Value) -> Result<Value, WsErr> {
    let text = payload
        .get("text")
        .or_else(|| payload.get("contents"))
        .and_then(Value::as_str)
        .ok_or_else(|| format_err("text is required"))?;
    let path = payload.get("path").and_then(Value::as_str);
    let language = language_from_input(path, payload.get("language").and_then(Value::as_str))?;
    let line_ending = payload.get("lineEnding").and_then(Value::as_str);
    let config_path = path.map(|path| {
        let raw = PathBuf::from(path);
        if raw.is_absolute() {
            raw
        } else if let Some(cwd) = payload.get("cwd").and_then(Value::as_str) {
            PathBuf::from(shellexpand(cwd)).join(raw)
        } else {
            raw
        }
    });
    let mut options = tidy_options(config_path.as_deref(), payload)?;
    if payload.get("preserveLineEnding").and_then(Value::as_bool) != Some(false) {
        preserve_line_endings(
            &mut options,
            line_ending.or_else(|| detect_line_ending(text)),
        );
    }
    // A caller-supplied config is authoritative even over line-ending preservation.
    apply_request_config(&mut options, payload)?;
    let formatted = format_document(text, language, &options, path)?;
    Ok(json!({
        "formatted": formatted,
        "changed": formatted != text,
        "language": language_name(language),
    }))
}

fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), WsErr> {
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
        if let Err(e) = f.write_all(bytes).and_then(|_| f.sync_all()) {
            let _ = std::fs::remove_file(&temp);
            return Err(err(format!("cannot write temp file: {e}")));
        }
        if let Ok(meta) = std::fs::metadata(target) {
            let _ = std::fs::set_permissions(&temp, meta.permissions());
        }
    }
    if std::fs::rename(&temp, target).is_err() {
        let _ = std::fs::remove_file(&temp);
        return Err(err("atomic rename failed"));
    }
    Ok(())
}

pub fn format_file(payload: &Value) -> Result<Value, WsErr> {
    let cwd = cwd_of(payload)?;
    let rel = payload
        .get("relativePath")
        .or_else(|| payload.get("path"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= WRITE_FILE_PATH_MAX)
        .ok_or_else(|| format_err("relativePath is required"))?;
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
    let raw = std::fs::read(&target).map_err(|_| err(format!("file not found: {rel}")))?;
    if raw.len() > FORMAT_TEXT_MAX_BYTES {
        return Err(format_err("file exceeds the 1 MB formatting limit"));
    }
    if let Some(expected) = payload.get("expectedVersion").and_then(Value::as_str) {
        if expected.len() > VERSION_MAX {
            return Err(err("expectedVersion too long"));
        }
        let version = format!("sha256:{}", hex_digest(&raw));
        if version != expected {
            return Err(err("file has been modified since it was read"));
        }
    }
    let (text, encoding) = decode_text(&raw)?;
    let line_ending = detect_line_ending(&text);
    let path_string = target.to_string_lossy().into_owned();
    let language = language_from_input(
        Some(&path_string),
        payload.get("language").and_then(Value::as_str),
    )?;
    let mut options = tidy_options(Some(&target), payload)?;
    if payload.get("preserveLineEnding").and_then(Value::as_bool) != Some(false) {
        preserve_line_endings(&mut options, line_ending);
    }
    apply_request_config(&mut options, payload)?;
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let formatted = format_document(&normalized, language, &options, Some(&path_string))?;
    let write = payload.get("write").and_then(Value::as_bool) == Some(true);
    let preserve_eol = payload.get("preserveLineEnding").and_then(Value::as_bool) != Some(false);
    let output_line_ending = if preserve_eol {
        match line_ending {
            Some("crlf") => "crlf",
            Some("cr") => "cr",
            _ => "lf",
        }
    } else {
        detect_line_ending(&formatted).unwrap_or("lf")
    };
    let output_bytes = encode_text(&formatted, encoding, output_line_ending);
    let changed = output_bytes != raw;
    let mut version = format!("sha256:{}", hex_digest(&raw));
    if write && changed {
        atomic_write(&target, &output_bytes)?;
        version = format!("sha256:{}", hex_digest(&output_bytes));
    }
    Ok(json!({
        "relativePath": rel,
        "formatted": formatted,
        "changed": changed,
        "written": write && changed,
        "language": language_name(language),
        "version": version,
        "encoding": encoding,
        "lineEnding": output_line_ending,
    }))
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

    let should_format = payload
        .get("format")
        .and_then(Value::as_bool)
        .unwrap_or_else(crate::server_settings::format_agent_files_enabled);
    let mut output_contents = contents.to_string();
    let mut formatting = json!({"applied": false});
    if should_format {
        formatting = match language_from_input(
            Some(&target.to_string_lossy()),
            payload.get("formatLanguage").and_then(Value::as_str),
        ) {
            Ok(language) => {
                let mut options = tidy_options(Some(&target), payload)?;
                let effective_line_ending = line_ending.or_else(|| detect_line_ending(contents));
                preserve_line_endings(&mut options, effective_line_ending);
                match format_document(
                    contents,
                    language,
                    &options,
                    Some(&target.to_string_lossy()),
                ) {
                    Ok(formatted) => {
                        output_contents = formatted;
                        json!({"applied": true, "language": language_name(language)})
                    }
                    Err(error) => json!({"applied": false, "error": error.0, "code": error.1}),
                }
            }
            Err(error) => json!({"applied": false, "error": error.0, "code": error.1}),
        };
    }

    let bytes = match (encoding, line_ending) {
        (Some(enc), Some(le)) => encode_text(&output_contents, enc, le),
        _ => output_contents.as_bytes().to_vec(),
    };

    atomic_write(&target, &bytes)?;

    let version = format!("sha256:{}", hex_digest(&bytes));
    Ok(json!({ "relativePath": rel, "version": version, "formatting": formatting }))
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

/// Resolves a live preview grant token to its real path, pruning expired
/// entries — mirrors upstream resolveLocalPreviewGrantRealPath (2-minute TTL).
pub(crate) fn resolve_grant_real_path(token: &str) -> Option<PathBuf> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    let mut grants = GRANTS.lock().ok()?;
    let now = Instant::now();
    grants.retain(|_, g| g.expires_at > now);
    grants.get(token).map(|g| g.real_path.clone())
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
        "projects.formatText" => Ok(Some(format_text(payload)?)),
        "projects.formatFile" => Ok(Some(format_file(payload)?)),
        "projects.searchContent" => Ok(Some(search_content(payload)?)),
        "projects.createLocalFilePreviewGrant" => {
            Ok(Some(create_local_file_preview_grant(payload)?))
        }
        _ => Err(WsErr(format!("unknown tag {tag}"), None)),
    }
}

#[cfg(test)]
mod format_tests {
    use super::*;

    fn temp_workspace(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "synara-format-test-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn formats_javascript_text() {
        let result = format_text(&json!({
            "text": "const x={a:1,b:2}\n",
            "language": "js"
        }))
        .unwrap();
        assert_eq!(result["formatted"], "const x = { a: 1, b: 2 };\n");
        assert_eq!(result["changed"], true);
        assert_eq!(result["language"], "javascript");
    }

    #[test]
    fn formats_json_by_path_and_applies_config() {
        let result = format_text(&json!({
            "text": "{\"a\":1,\"b\":[1,2]}",
            "path": "data.json",
            "config": {"width": 8}
        }))
        .unwrap();
        assert!(result["formatted"].as_str().unwrap().contains("\n"));
        assert_eq!(result["language"], "json");
    }

    #[test]
    fn rejects_unsupported_language_and_syntax_errors() {
        assert!(format_text(&json!({"text":"fn main() {}","language":"rust"})).is_err());
        let error = format_text(&json!({"text":"const = {","language":"js"})).unwrap_err();
        assert_eq!(error.1.as_deref(), Some("FORMAT_SYNTAX"));
    }

    #[test]
    fn format_file_writes_atomically_and_preserves_crlf() {
        let cwd = temp_workspace("crlf");
        std::fs::write(cwd.join("x.js"), "const x={a:1}\r\n").unwrap();
        let result = format_file(&json!({
            "cwd": cwd,
            "relativePath": "x.js",
            "write": true
        }))
        .unwrap();
        assert_eq!(result["written"], true);
        let written = std::fs::read_to_string(cwd.join("x.js")).unwrap();
        assert!(written.ends_with("\r\n"));
        assert!(written.contains("const x = { a: 1 };"));
        std::fs::remove_dir_all(&cwd).ok();
    }

    #[test]
    fn format_file_enforces_workspace_and_expected_version() {
        let cwd = temp_workspace("guard");
        assert!(
            format_file(&json!({
                "cwd": cwd,
                "relativePath": "../x.js"
            }))
            .is_err()
        );
        std::fs::write(cwd.join("x.js"), "const x={a:1}\n").unwrap();
        assert!(
            format_file(&json!({
                "cwd": cwd,
                "relativePath": "x.js",
                "write": true,
                "expectedVersion": "sha256:bad"
            }))
            .is_err()
        );
        std::fs::remove_dir_all(&cwd).ok();
    }

    #[test]
    fn write_file_can_format_supported_text_without_failing_unsupported_files() {
        let cwd = temp_workspace("write-format");
        let result = write_file(&json!({
            "cwd": cwd,
            "relativePath": "x.js",
            "contents": "const x={a:1}\n",
            "format": true
        }))
        .unwrap();
        assert_eq!(result["formatting"]["applied"], true);
        assert_eq!(
            std::fs::read_to_string(cwd.join("x.js")).unwrap(),
            "const x = { a: 1 };\n"
        );
        let result = write_file(&json!({
            "cwd": cwd,
            "relativePath": "x.rs",
            "contents": "fn main(){}\n",
            "format": true
        }))
        .unwrap();
        assert_eq!(result["formatting"]["applied"], false);
        assert_eq!(
            std::fs::read_to_string(cwd.join("x.rs")).unwrap(),
            "fn main(){}\n"
        );
        std::fs::remove_dir_all(&cwd).ok();
    }
}
