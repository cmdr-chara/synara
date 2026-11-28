//! Static serving for the built web app with SPA fallback. Upstream serves
//! the same dist from the TS server so browser clients reach both the API and
//! the app on one origin; resolving the dist once at startup keeps the Rust
//! backend self-contained for the full-stack port.

use axum::body::Body;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};

fn dist_dir() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("SYNARA_WEB_DIST") {
        let dir = std::path::PathBuf::from(p);
        if dir.join("index.html").exists() {
            return Some(dir);
        }
    }
    // Repo-relative: rust/app is two levels below the checkout root.
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent()?.to_path_buf();
        for _ in 0..8 {
            for rel in ["rust/app/web-dist", "apps/web/dist"] {
                let cand = dir.join(rel);
                if cand.join("index.html").exists() {
                    return Some(cand);
                }
            }
            if !dir.pop() {
                break;
            }
        }
    }
    None
}

fn mime(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "mp4" => "video/mp4",
        "wasm" => "application/wasm",
        "txt" => "text/plain; charset=utf-8",
        "map" => "application/json",
        _ => "application/octet-stream",
    }
}

pub fn configured() -> bool {
    dist_dir().is_some()
}

pub async fn serve(uri: axum::http::Uri, headers: axum::http::HeaderMap) -> Response {
    let Some(dir) = dist_dir() else {
        return (
            StatusCode::NOT_FOUND,
            "No static directory configured and no dev URL set.",
        )
            .into_response();
    };
    let raw = uri.path().trim_start_matches('/');
    // Path traversal guard: normalize away any `..` before joining.
    let mut rel = std::path::PathBuf::new();
    for part in std::path::Path::new(raw).components() {
        if let std::path::Component::Normal(p) = part {
            rel.push(p);
        }
    }
    let mut file = dir.join(&rel);
    let mut is_shell_fallback = false;
    if rel.as_os_str().is_empty() || !file.exists() || file.is_dir() {
        // SPA fallback: client-side routes get the shell page.
        file = dir.join("index.html");
        is_shell_fallback = true;
    }
    // Pre-compressed sidecars emitted by the vite build: prefer brotli over
    // gzip when the client accepts both, mirroring the upstream server. The
    // content type still describes the uncompressed asset.
    let accept_encoding = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mut content_encoding: Option<&'static str> = None;
    let mut bytes_file = file.clone();
    if !is_shell_fallback {
        for (encoding, ext) in [("br", "br"), ("gzip", "gz")] {
            if accepts_encoding(&accept_encoding, encoding) {
                let sidecar = file.with_extension(format!(
                    "{}.{ext}",
                    file.extension().and_then(|e| e.to_str()).unwrap_or("")
                ));
                if sidecar.exists() {
                    content_encoding = Some(encoding);
                    bytes_file = sidecar;
                    break;
                }
            }
        }
    }
    match std::fs::read(&bytes_file) {
        Ok(bytes) => {
            let mut response = Body::from(bytes).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(mime(match file.extension().and_then(|e| e.to_str()) {
                    Some(ext) => ext,
                    None => "",
                })),
            );
            if let Some(encoding) = content_encoding {
                response.headers_mut().insert(
                    header::CONTENT_ENCODING,
                    header::HeaderValue::from_static(encoding),
                );
                response.headers_mut().insert(
                    header::VARY,
                    header::HeaderValue::from_static("Accept-Encoding"),
                );
            }
            response.headers_mut().insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static(cache_control_for(&rel, is_shell_fallback)),
            );
            response
        }
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

fn accepts_encoding(accept_encoding: &str, encoding: &str) -> bool {
    accept_encoding
        .split(',')
        .map(str::trim)
        .any(|entry| entry == encoding || entry.starts_with(&format!("{encoding};")))
}

fn cache_control_for(rel: &std::path::Path, is_shell_fallback: bool) -> &'static str {
    if is_shell_fallback {
        return "no-cache";
    }
    let normalized = rel.to_string_lossy().replace('\\', "/");
    // Vite writes content-hashed filenames under assets/; they never change.
    if normalized.starts_with("assets/") {
        return "public, max-age=31536000, immutable";
    }
    if normalized.starts_with("central-icons-reversed/") || normalized.starts_with("central-icons-fill/") {
        return "public, max-age=86400";
    }
    "no-cache"
}
