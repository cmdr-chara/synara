//! Local preview-file serving for `/api/local-image`, lean slice of upstream
//! localImageFiles.ts. Security invariants preserved exactly: only files inside
//! an allowlisted root are served, the decision uses the realpath of both file
//! and root (symlink escapes fail closed), the extension allowlist matches
//! upstream shared/localPreviewFiles.ts, and bytes never come from inline
//! payloads. Grant tokens, Range streaming for video, and the multi-home
//! overlay enumeration are documented gaps — the default CODEX_HOME plus the
//! SYNARA_HOME overlay cover the configurations this server actually launches.

use axum::{
    extract::RawQuery,
    http::{header, StatusCode},
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
    let Some(ext) = lower_extension(path) else { return false };
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
fn allowlisted_roots(cwd: Option<&str>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(cwd) = cwd {
        if let Some(real) = realpath(Path::new(cwd)) {
            roots.push(real);
        }
    }
    if let Ok(home) = std::env::var("SYNARA_HOME") {
        if let Some(real) = realpath(&Path::new(&home).join("scratch-workspaces")) {
            roots.push(real);
        }
    }
    let codex_homes: Vec<PathBuf> = {
        let mut v = Vec::new();
        if let Ok(ch) = std::env::var("CODEX_HOME") {
            v.push(PathBuf::from(ch));
        }
        if let Some(home) = std::env::var_os("HOME") {
            v.push(Path::new(&home).join(".codex"));
        }
        v
    };
    for codex_home in codex_homes {
        let generated = codex_home.join("generated_images");
        if let Some(real) = realpath(&generated) {
            roots.push(real);
        }
    }
    roots
}

fn resolve_allowed(requested: &str, cwd: Option<&str>) -> Option<(PathBuf, u64)> {
    if requested.is_empty() || requested.contains('\0') || !is_supported_preview(requested) {
        return None;
    }
    let resolved = if Path::new(requested).is_absolute() {
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
    let roots = allowlisted_roots(cwd);
    if roots.iter().any(|root| is_inside(&real, root)) {
        return Some((real, meta.len()));
    }
    None
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

pub async fn local_image(RawQuery(query): RawQuery) -> Response {
    let query = query.unwrap_or_default();
    let path_param = query_param(&query, "path");
    let cwd_param = query_param(&query, "cwd");
    let Some(requested) = path_param.as_deref().map(str::trim).filter(|s| !s.is_empty()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Some((real, _len)) = resolve_allowed(requested, cwd_param.as_deref()) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let Ok(bytes) = std::fs::read(&real) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    (
        [
            (header::CONTENT_TYPE, mime_for(real.to_str().unwrap_or(""))),
            (header::CACHE_CONTROL, "private, no-store"),
        ],
        bytes,
    )
        .into_response()
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
