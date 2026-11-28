//! Project and site favicon HTTP routes, ports of upstream
//! project/Layers/ProjectFaviconResolver.ts + siteFaviconCache.ts and the two
//! routes in http.ts. Candidate scan order, fallback SVGs, cache TTLs and the
//! SSRF public-host guard are exact. Documented divergence: the icon href
//! extraction is string scanning instead of the two regexes (no regex crate
//! in this binary); the covered shapes are identical for well-formed sources.

use axum::{
    extract::RawQuery,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::attachments_http::attachments_http_query;

const PROJECT_FAVICON_CACHE_CONTROL: &str = "public, max-age=300";
const SITE_FAVICON_CACHE_CONTROL_SUCCESS: &str = "public, max-age=86400";
const SITE_FAVICON_CACHE_CONTROL_FALLBACK: &str = "public, max-age=3600";
const MAX_FAVICON_BYTES: usize = 512 * 1024;
const FAVICON_CACHE_MAX: usize = 500;
const FAVICON_SUCCESS_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const FAVICON_FAILURE_TTL: Duration = Duration::from_secs(60 * 60);

const FALLBACK_PROJECT_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"24\" height=\"24\" fill=\"none\" stroke=\"#6b728080\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" data-fallback=\"project-favicon\"><path d=\"M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-8l-2-2H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2Z\"/></svg>";
const FALLBACK_SITE_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"24\" height=\"24\" fill=\"none\" stroke=\"#6b728080\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-linejoin=\"round\" data-fallback=\"site-favicon\"><circle cx=\"12\" cy=\"12\" r=\"10\"/><path d=\"M12 2a14.5 14.5 0 0 0 0 20M12 2a14.5 14.5 0 0 1 0 20M2 12h20\"/></svg>";

const FAVICON_CANDIDATES: &[&str] = &[
    "favicon.svg",
    "favicon.ico",
    "favicon.png",
    "public/favicon.svg",
    "public/favicon.ico",
    "public/favicon.png",
    "app/favicon.ico",
    "app/favicon.png",
    "app/icon.svg",
    "app/icon.png",
    "app/icon.ico",
    "src/favicon.ico",
    "src/favicon.svg",
    "src/app/favicon.ico",
    "src/app/icon.svg",
    "src/app/icon.png",
    "assets/icon.svg",
    "assets/icon.png",
    "assets/logo.svg",
    "assets/logo.png",
];
const NESTED_FAVICON_CANDIDATES: &[&str] = &[
    "apps/web/public/favicon.svg",
    "apps/web/public/favicon.ico",
    "apps/web/public/favicon.png",
    "web/public/favicon.svg",
    "web/public/favicon.ico",
    "web/public/favicon.png",
];
const ICON_SOURCE_FILES: &[&str] = &[
    "index.html",
    "public/index.html",
    "app/routes/__root.tsx",
    "src/routes/__root.tsx",
    "app/root.tsx",
    "src/root.tsx",
    "src/index.html",
];

fn svg_response(svg: &'static str, cache: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml".to_string()),
            (header::CACHE_CONTROL, cache.to_string()),
            (
                header::HeaderName::from_static("content-security-policy"),
                "default-src 'none'; style-src 'unsafe-inline'".to_string(),
            ),
        ],
        svg,
    )
        .into_response()
}

fn is_supported_image(path: &str) -> bool {
    let lower = path.to_lowercase();
    lower.ends_with(".svg")
        || lower.ends_with(".ico")
        || lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".gif")
        || lower.ends_with(".webp")
}

fn contained_existing(root: &Path, candidate: &Path) -> Option<PathBuf> {
    if !candidate.is_file() {
        return None;
    }
    let canonical_root = root.canonicalize().ok()?;
    let canonical = candidate.canonicalize().ok()?;
    if canonical.starts_with(&canonical_root) {
        Some(canonical)
    } else {
        None
    }
}

/// Extracts an icon href from `<link rel="icon" href=...>` html or the
/// `{ rel: "icon", href: ... }` object form used by root route modules.
fn extract_icon_href(source: &str) -> Option<String> {
    let lower = source.to_lowercase();
    let scan = |needle: &str, hay: &str, original: &str| -> Option<String> {
        let mut search_from = 0;
        while let Some(pos) = hay[search_from..].find(needle) {
            let abs = search_from + pos;
            let window_end = (abs + 600).min(hay.len());
            let window = &hay[abs..window_end];
            let rel_hit = window.contains("rel=\"icon\"")
                || window.contains("rel='icon'")
                || window.contains("rel=\"shortcut icon\"")
                || window.contains("rel='shortcut icon'");
            if rel_hit {
                for marker in ["href=\"", "href='"] {
                    if let Some(href_pos) = window.find(marker) {
                        let start = href_pos + marker.len();
                        let quote = marker.chars().last().unwrap();
                        if let Some(end) = window[start..].find(quote) {
                            let href = window[start..start + end].to_string();
                            let href = href.split('?').next().unwrap_or("").to_string();
                            if !href.is_empty() {
                                return Some(href);
                            }
                        }
                    }
                }
            }
            search_from = abs + needle.len();
            if search_from >= hay.len() {
                break;
            }
        }
        let _ = original;
        None
    };
    if let Some(hit) = scan("<link", &lower, source) {
        return Some(hit);
    }
    scan("rel", &lower, source)
}

fn resolve_project_favicon(cwd: &str) -> Option<PathBuf> {
    let root = Path::new(cwd);
    if !root.is_dir() {
        return None;
    }
    for candidates in [FAVICON_CANDIDATES, NESTED_FAVICON_CANDIDATES] {
        for candidate in candidates {
            if !is_supported_image(candidate) {
                continue;
            }
            if let Some(found) = contained_existing(root, &root.join(candidate)) {
                return Some(found);
            }
        }
    }
    // Icon href extraction from root html/route modules.
    for source_file in ICON_SOURCE_FILES {
        let path = root.join(source_file);
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(href) = extract_icon_href(&source) else {
            continue;
        };
        let clean = href.trim_start_matches('/');
        for base in [root.join("public").join(clean), root.join(clean)] {
            if is_supported_image(base.to_string_lossy().as_ref()) {
                if let Some(found) = contained_existing(root, &base) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn mime_for_favicon(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => "image/x-icon",
    }
}

pub async fn project_favicon(RawQuery(query): RawQuery) -> Response {
    let query = query.unwrap_or_default();
    let cwd = attachments_http_query(&query, "cwd").unwrap_or_default();
    if cwd.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing cwd parameter").into_response();
    }
    let fallback_none = attachments_http_query(&query, "fallback").as_deref() == Some("none");
    let Some(found) = resolve_project_favicon(&cwd) else {
        if fallback_none {
            return (
                StatusCode::NO_CONTENT,
                [(header::CACHE_CONTROL, PROJECT_FAVICON_CACHE_CONTROL)],
            )
                .into_response();
        }
        return svg_response(FALLBACK_PROJECT_SVG, PROJECT_FAVICON_CACHE_CONTROL);
    };
    match std::fs::read(&found) {
        Ok(bytes) if mime_for_favicon(&found) == "image/svg+xml" => (
            [
                (header::CONTENT_TYPE, "image/svg+xml".to_string()),
                (
                    header::CACHE_CONTROL,
                    PROJECT_FAVICON_CACHE_CONTROL.to_string(),
                ),
                (
                    header::HeaderName::from_static("content-security-policy"),
                    "default-src 'none'; style-src 'unsafe-inline'".to_string(),
                ),
            ],
            bytes,
        )
            .into_response(),
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, mime_for_favicon(&found).to_string()),
                (
                    header::CACHE_CONTROL,
                    PROJECT_FAVICON_CACHE_CONTROL.to_string(),
                ),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}

#[derive(Clone)]
struct SiteCacheEntry {
    bytes: Option<Vec<u8>>,
    content_type: Option<String>,
    expires_at: Instant,
}

fn site_cache() -> &'static Mutex<HashMap<String, SiteCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, SiteCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn normalize_host(host: &str) -> String {
    host.trim()
        .to_lowercase()
        .trim_start_matches("www.")
        .to_string()
}

fn try_parse_host(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };
    let after_scheme = with_scheme.split("://").nth(1)?;
    let host_part = after_scheme.split(['/', '?', '#']).next()?;
    let host = host_part.split(':').next().unwrap_or(host_part);
    let normalized = normalize_host(host);
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// SSRF guard, exact to upstream: no single-label hosts, no localhost/internal
/// suffixes, no private/loopback/reserved IPv4 ranges.
fn is_public_http_host(host: &str) -> bool {
    if host.is_empty() || host.contains([':', '[', ']']) {
        return false;
    }
    if host == "localhost" || host.ends_with(".localhost") {
        return false;
    }
    if host.ends_with(".local") || host.ends_with(".internal") {
        return false;
    }
    if !host.contains('.') {
        return false;
    }
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() == 4 && parts.iter().all(|p| p.parse::<u8>().is_ok()) {
        let nums: Vec<u8> = parts.iter().map(|p| p.parse().unwrap()).collect();
        let (a, b) = (nums[0], nums[1]);
        if a == 10 || a == 127 || a == 0 {
            return false;
        }
        if a == 172 && (16..=31).contains(&b) {
            return false;
        }
        if a == 192 && b == 168 {
            return false;
        }
        if a == 169 && b == 254 {
            return false;
        }
        if a >= 224 {
            return false;
        }
    }
    true
}

async fn fetch_image(
    client: &reqwest::Client,
    url: &str,
    timeout: Duration,
) -> Option<(Vec<u8>, String)> {
    let response = client.get(url).timeout(timeout).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if !content_type.to_lowercase().starts_with("image/") {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    if bytes.is_empty() || bytes.len() > MAX_FAVICON_BYTES {
        return None;
    }
    Some((bytes.to_vec(), content_type))
}

async fn fetch_favicon_for_host(host: &str) -> Option<(Vec<u8>, String)> {
    let client = reqwest::Client::new();
    if let Some(hit) = fetch_image(
        &client,
        &format!("https://www.google.com/s2/favicons?domain={host}&sz=64"),
        Duration::from_secs(5),
    )
    .await
    {
        return Some(hit);
    }
    if let Some(hit) = fetch_image(
        &client,
        &format!("https://icons.duckduckgo.com/ip3/{host}.ico"),
        Duration::from_secs(5),
    )
    .await
    {
        return Some(hit);
    }
    if is_public_http_host(host) {
        if let Some(hit) = fetch_image(
            &client,
            &format!("https://{host}/favicon.ico"),
            Duration::from_secs(3),
        )
        .await
        {
            return Some(hit);
        }
    }
    None
}

async fn resolve_site_favicon(host: &str) -> SiteCacheEntry {
    {
        let cache = site_cache().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = cache.get(host) {
            if entry.expires_at > Instant::now() {
                return SiteCacheEntry {
                    bytes: entry.bytes.clone(),
                    content_type: entry.content_type.clone(),
                    expires_at: entry.expires_at,
                };
            }
        }
    }
    let result = fetch_favicon_for_host(host).await;
    let success = result.is_some();
    let entry = SiteCacheEntry {
        bytes: result.as_ref().map(|(b, _)| b.clone()),
        content_type: result.map(|(_, c)| c),
        expires_at: Instant::now()
            + if success {
                FAVICON_SUCCESS_TTL
            } else {
                FAVICON_FAILURE_TTL
            },
    };
    let mut cache = site_cache().lock().unwrap_or_else(|e| e.into_inner());
    while cache.len() >= FAVICON_CACHE_MAX {
        if let Some(oldest) = cache.keys().next().cloned() {
            if oldest == host {
                break;
            }
            cache.remove(&oldest);
        } else {
            break;
        }
    }
    cache.insert(host.to_string(), entry.clone());
    entry
}

pub async fn site_favicon(RawQuery(query): RawQuery) -> Response {
    let query = query.unwrap_or_default();
    let domain = attachments_http_query(&query, "domain")
        .or_else(|| attachments_http_query(&query, "url"))
        .unwrap_or_default();
    if domain.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing domain parameter").into_response();
    }
    let Some(host) = try_parse_host(&domain) else {
        return (StatusCode::BAD_REQUEST, "Invalid domain").into_response();
    };
    if !is_public_http_host(&host) {
        return svg_response(FALLBACK_SITE_SVG, SITE_FAVICON_CACHE_CONTROL_FALLBACK);
    }
    let entry = resolve_site_favicon(&host).await;
    match (entry.bytes, entry.content_type) {
        (Some(bytes), content_type) => (
            [
                (
                    header::CONTENT_TYPE,
                    content_type.unwrap_or_else(|| "image/x-icon".to_string()),
                ),
                (
                    header::CACHE_CONTROL,
                    SITE_FAVICON_CACHE_CONTROL_SUCCESS.to_string(),
                ),
            ],
            bytes,
        )
            .into_response(),
        _ => svg_response(FALLBACK_SITE_SVG, SITE_FAVICON_CACHE_CONTROL_FALLBACK),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_parsing_normalizes() {
        assert_eq!(
            try_parse_host("https://www.Example.com/path"),
            Some("example.com".into())
        );
        assert_eq!(try_parse_host("docs.rs"), Some("docs.rs".into()));
        assert!(try_parse_host("").is_none());
    }

    #[test]
    fn ssrf_guard_rejects_internal() {
        assert!(!is_public_http_host("localhost"));
        assert!(!is_public_http_host("192.168.1.1"));
        assert!(!is_public_http_host("10.0.0.5"));
        assert!(!is_public_http_host("foo.internal"));
        assert!(!is_public_http_host("singlelabel"));
        assert!(is_public_http_host("example.com"));
        assert!(is_public_http_host("8.8.8.8"));
    }

    #[test]
    fn extracts_icon_href_from_html() {
        let html = r#"<html><head><link rel="icon" href="/favicon-32.png?v=2"></head></html>"#;
        assert_eq!(extract_icon_href(html), Some("/favicon-32.png".into()));
    }
}
