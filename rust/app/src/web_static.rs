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
            let cand = dir.join("apps/web/dist");
            if cand.join("index.html").exists() {
                return Some(cand);
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

pub async fn serve(uri: axum::http::Uri) -> Response {
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
    if rel.as_os_str().is_empty() || !file.exists() || file.is_dir() {
        // SPA fallback: client-side routes get the shell page.
        file = dir.join("index.html");
    }
    match std::fs::read(&file) {
        Ok(bytes) => {
            let mut response = Body::from(bytes).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                header::HeaderValue::from_static(mime(match file.extension().and_then(|e| e.to_str()) {
                    Some(ext) => ext,
                    None => "",
                })),
            );
            // Hashed assets cache forever; the html shell never does.
            if file.file_name().and_then(|n| n.to_str()) == Some("index.html") {
                response.headers_mut().insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static("no-cache"),
                );
            }
            response
        }
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}
