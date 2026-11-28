//! Local preview-file serving for `/api/local-image`, lean slice of upstream
//! localImageFiles.ts. Security invariants preserved exactly: only files inside
//! an allowlisted root are served, the decision uses the realpath of both file
//! and root (symlink escapes fail closed), the extension allowlist matches
//! upstream shared/localPreviewFiles.ts, and bytes never come from inline
//! payloads. Grants are validated against the same realpath-issued token store
//! as workspace file access. Codex generated-image roots include the ambient
//! home, the Synara overlay, and every configured Codex provider-instance home;
//! those roots remain image-only as upstream requires.

use axum::{
    extract::RawQuery,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::path::{Path, PathBuf};

const IMAGE_EXTENSIONS: &[&str] = &[
    ".avif", ".bmp", ".gif", ".heic", ".heif", ".ico", ".jpeg", ".jpg", ".png", ".svg", ".tiff",
    ".webp",
];
const VIDEO_EXTENSIONS: &[&str] = &[".mp4", ".webm", ".mov", ".m4v"];
const PDF_EXTENSIONS: &[&str] = &[".pdf"];
/// Preview responses are capped so a hostile or corrupted huge file cannot
/// turn the route into a memory sink; upstream streams, the lean slice reads.
const MAX_PREVIEW_BYTES: u64 = 256 * 1024 * 1024;

fn lower_extension(path: &str) -> Option<String> {
    let dot = path.rfind('.')?;
    Some(path[dot..].to_lowercase())
}

fn is_supported_preview(path: &str) -> bool {
    let Some(ext) = lower_extension(path) else {
        return false;
    };
    IMAGE_EXTENSIONS.contains(&ext.as_str())
        || VIDEO_EXTENSIONS.contains(&ext.as_str())
        || PDF_EXTENSIONS.contains(&ext.as_str())
}

pub(crate) fn local_image_mime_for(path: &str) -> &'static str {
    mime_for(path)
}

fn mime_for(path: &str) -> &'static str {
    match lower_extension(path).as_deref().unwrap_or("") {
        ".png" => "image/png",
        ".jpg" | ".jpeg" => "image/jpeg",
        ".gif" => "image/gif",
        ".svg" => "image/svg+xml",
        ".webp" => "image/webp",
        ".avif" => "image/avif",
        ".bmp" => "image/bmp",
        ".ico" => "image/x-icon",
        ".tiff" => "image/tiff",
        ".heic" => "image/heic",
        ".heif" => "image/heif",
        ".mp4" | ".m4v" => "video/mp4",
        ".webm" => "video/webm",
        ".mov" => "video/quicktime",
        ".pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

/// Range-aware file response shared by `/api/local-image` and `/attachments`.
/// Semantics match upstream streamedFileResponse exactly: `bytes=a-b`,
/// `bytes=a-`, `bytes=-suffix`, 416 with `Content-Range: bytes */size` on
/// unsatisfiable ranges, and `Accept-Ranges: bytes` on every response.
pub(crate) fn file_response_with_range(
    path: &Path,
    range_header: Option<&str>,
    content_type: &str,
    cache_control: &str,
) -> Response {
    let size = match std::fs::metadata(path) {
        Ok(m) => m.len(),
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    if size > MAX_PREVIEW_BYTES {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    let mut start: u64 = 0;
    let mut end: u64 = size.saturating_sub(1);
    let mut partial = false;
    if let Some(range) = range_header {
        partial = true;
        if let Some(spec) = range.strip_prefix("bytes=") {
            let mut parts = spec.splitn(2, '-');
            let first = parts.next().unwrap_or("");
            let second = parts.next().unwrap_or("");
            if !first.is_empty() || !second.is_empty() {
                if first.is_empty() {
                    let suffix: u64 = second.parse().unwrap_or(u64::MAX);
                    start = size.saturating_sub(suffix);
                } else {
                    start = first.parse().unwrap_or(u64::MAX);
                    if !second.is_empty() {
                        end = second.parse::<u64>().unwrap_or(u64::MAX).min(end);
                    }
                }
            } else {
                start = u64::MAX;
            }
        } else {
            start = u64::MAX;
        }
        if start == u64::MAX || start > end || start >= size {
            return (
                StatusCode::RANGE_NOT_SATISFIABLE,
                [
                    (header::CONTENT_RANGE, format!("bytes */{size}")),
                    (header::ACCEPT_RANGES, "bytes".to_string()),
                ],
            )
                .into_response();
        }
    }
    let length = end.saturating_sub(start) + 1;
    let data = match std::fs::read(path) {
        Ok(bytes) => {
            let s = (start as usize).min(bytes.len());
            let e = ((end as usize) + 1).min(bytes.len());
            bytes[s..e].to_vec()
        }
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let status = if partial && length < size {
        StatusCode::PARTIAL_CONTENT
    } else {
        StatusCode::OK
    };
    let mut response = axum::response::Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, cache_control)
        .header(header::ACCEPT_RANGES, "bytes");
    if status == StatusCode::PARTIAL_CONTENT {
        response = response.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{size}"));
    }
    response
        .body(axum::body::Body::from(data))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn is_inside(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

fn realpath(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// Allowlisted roots for one request: the workspace cwd, the scratch
/// workspaces root (sessions without a project), and every generated-images
/// root this server can legitimately serve (default codex home, CODEX_HOME
/// override, and the per-instance overlay beneath SYNARA_HOME).
fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let marker = current.join(".git");
        if marker.is_dir() || marker.is_file() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn temporary_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let tmp = std::env::temp_dir();
    if let Some(real) = realpath(&tmp) {
        roots.push(real);
    }
    if cfg!(target_os = "macos") {
        if let Some(real) = realpath(Path::new("/tmp")) {
            if !roots.contains(&real) {
                roots.push(real);
            }
        }
    }
    roots
}

fn allowlisted_roots(cwd: Option<&str>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(cwd) = cwd {
        if let Some(real) = realpath(Path::new(cwd)) {
            // Upstream resolves the workspace root as the git root of the cwd,
            // so previews work anywhere in the enclosing checkout.
            if let Some(git_root) = find_git_root(&real) {
                if let Some(real_root) = realpath(&git_root) {
                    roots.push(real_root);
                }
            }
            roots.push(real);
        }
    }
    for temp in temporary_roots() {
        let scratch = temp.join("synara-scratch-workspaces");
        if let Some(real) = realpath(&scratch) {
            roots.push(real);
        }
    }
    if let Ok(home) = std::env::var("SYNARA_HOME") {
        if let Some(real) = realpath(&Path::new(&home).join("scratch-workspaces")) {
            roots.push(real);
        }
    }
    roots
}

fn codex_generated_roots() -> Vec<PathBuf> {
    let mut homes: Vec<PathBuf> = Vec::new();
    let mut push_home = |home: PathBuf| {
        if !homes.contains(&home) {
            homes.push(home);
        }
    };
    if let Ok(ch) = std::env::var("CODEX_HOME") {
        if !ch.trim().is_empty() {
            push_home(PathBuf::from(ch));
        }
    }
    if let Some(home) = std::env::var_os("HOME") {
        push_home(Path::new(&home).join(".codex"));
    }
    if let Ok(home) = std::env::var("SYNARA_HOME") {
        push_home(Path::new(&home).join("codex-home-overlay"));
    }
    let settings = crate::server_settings::current();
    for instance in crate::provider_instances::derive_instances(&settings) {
        if instance.driver != "codex" || !instance.enabled {
            continue;
        }
        if let Some(home) = crate::provider_instances::instance_home_path(&instance) {
            push_home(PathBuf::from(home));
        }
        for (key, value) in crate::provider_instances::instance_start_environment(&instance) {
            if key == "CODEX_HOME" && !value.trim().is_empty() {
                push_home(PathBuf::from(value));
            }
        }
    }
    homes
        .into_iter()
        .filter_map(|home| realpath(&home.join("generated_images")))
        .collect()
}

fn is_video(path: &str) -> bool {
    lower_extension(path)
        .map(|e| VIDEO_EXTENSIONS.contains(&e.as_str()))
        .unwrap_or(false)
}

fn is_image(path: &str) -> bool {
    lower_extension(path)
        .map(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
        .unwrap_or(false)
}

fn resolve_allowed_with_grant(
    requested: &str,
    cwd: Option<&str>,
    grant: Option<&str>,
) -> Option<(PathBuf, u64)> {
    if requested.is_empty() || requested.contains('\0') || !is_supported_preview(requested) {
        return None;
    }
    let is_absolute = Path::new(requested).is_absolute();
    let resolved = if is_absolute {
        PathBuf::from(requested)
    } else {
        Path::new(cwd?).join(requested)
    };
    let real = realpath(&resolved)?;
    if !is_supported_preview(real.to_str()?) {
        return None;
    }
    let meta = std::fs::metadata(&real).ok()?;
    if !meta.is_file() || meta.len() > MAX_PREVIEW_BYTES {
        return None;
    }
    let real_str = real.to_str()?;
    let roots = allowlisted_roots(cwd);
    if roots.iter().any(|root| is_inside(&real, root)) {
        return Some((real, meta.len()));
    }
    // Opt-in absolute local preview with a live grant — the file panel uses
    // this for intentional previews outside the workspace (e.g. Downloads).
    // Exact upstream shape: absolute path + grant bound to this realpath.
    if is_absolute {
        if let Some(token) = grant {
            if crate::projects_fileops::resolve_grant_real_path(token).as_deref()
                == Some(real.as_path())
            {
                return Some((real, meta.len()));
            }
        }
    }
    // Temp roots serve images and videos (agent tools write recordings such
    // as simulator-*.mp4 to the temp dir); they never serve documents. Codex
    // generated-image roots are stricter and serve images only.
    if is_image(real_str) || is_video(real_str) {
        let temp_roots = temporary_roots();
        if temp_roots.iter().any(|root| is_inside(&real, root)) {
            return Some((real, meta.len()));
        }
    }
    if is_image(real_str) {
        let generated_roots = codex_generated_roots();
        if generated_roots.iter().any(|root| is_inside(&real, root)) {
            return Some((real, meta.len()));
        }
    }
    None
}

fn resolve_allowed(requested: &str, cwd: Option<&str>) -> Option<(PathBuf, u64)> {
    resolve_allowed_with_grant(requested, cwd, None)
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(hex);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if percent_decode(k) == key {
            return Some(percent_decode(v));
        }
    }
    None
}

pub async fn local_image(RawQuery(query): RawQuery, headers: axum::http::HeaderMap) -> Response {
    let query = query.unwrap_or_default();
    let path_param = query_param(&query, "path");
    let cwd_param = query_param(&query, "cwd");
    let Some(requested) = path_param
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let grant_param = query_param(&query, "grant");
    let Some((real, _len)) =
        resolve_allowed_with_grant(requested, cwd_param.as_deref(), grant_param.as_deref())
    else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !real.is_file() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
    file_response_with_range(
        &real,
        range,
        mime_for(real.to_str().unwrap_or("")),
        "private, no-store",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsupported_extension() {
        assert!(resolve_allowed("/etc/passwd", None).is_none());
        assert!(resolve_allowed("/tmp/evil.sh", None).is_none());
    }

    #[test]
    fn rejects_null_bytes_and_empty() {
        assert!(resolve_allowed("/tmp/a.png\0x", None).is_none());
        assert!(resolve_allowed("", None).is_none());
    }

    #[test]
    fn supported_extensions_match_upstream() {
        assert!(is_supported_preview("/x/a.png"));
        assert!(is_supported_preview("/x/b.webp"));
        assert!(is_supported_preview("/x/c.mp4"));
        assert!(is_supported_preview("/x/d.pdf"));
        assert!(!is_supported_preview("/x/e.txt"));
        assert!(!is_supported_preview("/x/f"));
    }
}
