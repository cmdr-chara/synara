//! Upstream `ProjectFaviconResolver` (`apps/server`): locates the project's own
//! favicon on disk — direct candidate paths first, then an `<link rel=icon>`
//! declaration in a root source file, then nested web-app folders. The resolved
//! path is loaded by the app and shown as the project's glyph badge.
use std::path::{Component, Path, PathBuf};

/// Upstream `FAVICON_CANDIDATES` (order preserved — first hit wins).
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

/// Upstream `NESTED_FAVICON_CANDIDATES`.
const NESTED_FAVICON_CANDIDATES: &[&str] = &[
    "apps/web/public/favicon.svg",
    "apps/web/public/favicon.ico",
    "apps/web/public/favicon.png",
    "web/public/favicon.svg",
    "web/public/favicon.ico",
    "web/public/favicon.png",
];

/// Upstream `ICON_SOURCE_FILES` — `<link rel="icon">` declarations to read.
const ICON_SOURCE_FILES: &[&str] = &[
    "index.html",
    "public/index.html",
    "app/routes/__root.tsx",
    "src/routes/__root.tsx",
    "app/root.tsx",
    "src/root.tsx",
    "src/index.html",
];

/// Upstream `LINK_ICON_HTML_RE`: a `<link>` tag whose attributes carry
/// `rel="icon"`/`rel="shortcut icon"` and an `href`. Attributes may appear in
/// either order, matching the upstream regex's lookaheads.
fn link_tag_icon_href(tag: &str) -> Option<String> {
    let rel = attr_value(tag, "rel")?;
    if rel != "icon" && rel != "shortcut icon" {
        return None;
    }
    attr_value(tag, "href").filter(|href| !href.contains('?'))
}

/// Extract a `name="value"` / `name='value'` / `name: "value"` attribute value.
fn attr_value(source: &str, name: &str) -> Option<String> {
    let mut offset = 0;
    while let Some(at) = source[offset..].find(name) {
        let at = offset + at;
        let after = &source[at + name.len()..];
        // The name must be a standalone attribute: preceded by whitespace or
        // `{` boundary and followed by `=` or `:`.
        let boundary = at == 0
            || source[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace() || c == '{' || c == ',');
        let rest = after.trim_start();
        if boundary && (rest.starts_with('=') || rest.starts_with(':')) {
            let rest = rest[1..].trim_start();
            let quote = rest.chars().next()?;
            if quote != '"' && quote != '\'' {
                offset = at + name.len();
                continue;
            }
            let value = &rest[1..];
            let end = value.find(quote)?;
            return Some(value[..end].to_owned());
        }
        offset = at + name.len();
    }
    None
}

/// Upstream `extractIconHref`: try the HTML `<link>` form, then the object
/// literal form (`{ rel: "icon", href: "..." }`).
fn extract_icon_href(source: &str) -> Option<String> {
    let mut offset = 0;
    while let Some(at) = source[offset..].find("<link") {
        let tag_start = offset + at;
        let tag_end = source[tag_start..]
            .find('>')
            .map(|end| tag_start + end + 1)?;
        let tag = &source[tag_start..tag_end];
        if let Some(href) = link_tag_icon_href(tag) {
            return Some(href);
        }
        offset = tag_end;
    }
    // Object-literal form: scan `{...}` spans for rel: "icon" + href: "...".
    let mut offset = 0;
    while let Some(at) = source[offset..].find('{') {
        let block_start = offset + at;
        let Some(block_end) = source[block_start..].find('}').map(|end| block_start + end) else {
            break;
        };
        let block = &source[block_start..block_end];
        let matches_rel = block
            .find("rel")
            .and_then(|_| attr_value(block, "rel"))
            .is_some_and(|rel| rel == "icon" || rel == "shortcut icon");
        if matches_rel
            && let Some(href) = attr_value(block, "href")
            && !href.contains('?')
        {
            return Some(href);
        }
        offset = block_end + 1;
    }
    None
}

/// Upstream `isPathWithinProject`: `path.resolve`-style normalization — the
/// candidate must not escape the project root through `..` or absolute parts.
fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            _ => result.push(component),
        }
    }
    result
}

fn is_path_within_project(root: &Path, candidate: &Path) -> bool {
    normalize(candidate).strip_prefix(normalize(root)).is_ok()
}

fn find_existing_file(root: &Path, candidates: &[PathBuf]) -> Option<PathBuf> {
    for candidate in candidates {
        if !is_path_within_project(root, candidate) {
            continue;
        }
        if candidate.is_file() {
            return Some(candidate.clone());
        }
    }
    None
}

/// Upstream `resolveIconHref`: a leading `/` is project-relative (`public/`
/// first, then the root itself).
fn resolve_icon_href(root: &Path, href: &str) -> [PathBuf; 2] {
    let clean = href.trim_start_matches('/');
    [root.join("public").join(clean), root.join(clean)]
}

/// First-level directory names upstream treats as app-like:
/// `/(?:web|front|dash|app|client|site)/i` — substring, case-insensitive.
fn app_like_folder(name: &str) -> bool {
    if name.starts_with('.') {
        return false;
    }
    let lowered = name.to_lowercase();
    ["web", "front", "dash", "app", "client", "site"]
        .iter()
        .any(|needle| lowered.contains(needle))
}

/// Upstream `resolvePath` — returns the favicon's absolute path or `None`.
pub fn resolve_project_favicon(cwd: &Path) -> Option<PathBuf> {
    for candidate in FAVICON_CANDIDATES {
        if let Some(existing) = find_existing_file(cwd, &[cwd.join(candidate)]) {
            return Some(existing);
        }
    }
    for source_file in ICON_SOURCE_FILES {
        let source_path = cwd.join(source_file);
        let Ok(source) = std::fs::read_to_string(&source_path) else {
            continue;
        };
        let Some(href) = extract_icon_href(&source) else {
            continue;
        };
        if let Some(existing) = find_existing_file(cwd, &resolve_icon_href(cwd, &href)) {
            return Some(existing);
        }
    }
    // Nested app discovery is a fallback to the project's own icon and
    // declaration.
    for candidate in NESTED_FAVICON_CANDIDATES {
        if let Some(existing) = find_existing_file(cwd, &[cwd.join(candidate)]) {
            return Some(existing);
        }
    }
    // Some repositories keep the web app one directory below the project root.
    // App-like first-level folders only, so a sidebar refresh does not scan
    // unrelated source trees or dependency directories.
    let mut app_folders: Vec<PathBuf> = std::fs::read_dir(cwd)
        .map(|entries| {
            entries
                .filter_map(std::result::Result::ok)
                .map(|entry| entry.file_name())
                .filter_map(|name| name.to_str().map(str::to_owned))
                .filter(|name| app_like_folder(name))
                .map(|name| cwd.join(name))
                .collect()
        })
        .unwrap_or_default();
    app_folders.sort();
    app_folders.truncate(16);
    for folder in app_folders {
        let candidates: Vec<PathBuf> = ["favicon.svg", "favicon.ico", "favicon.png"]
            .iter()
            .map(|name| folder.join("public").join(name))
            .collect();
        if let Some(existing) = find_existing_file(cwd, &candidates) {
            return Some(existing);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write(root: &Path, path: &str, contents: &str) {
        let full = root.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, contents).unwrap();
    }

    #[test]
    fn finds_favicons_in_direct_candidate_order() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "public/favicon.ico", "icon");
        write(root.path(), "favicon.svg", "<svg/>");
        assert_eq!(
            resolve_project_favicon(root.path()),
            Some(root.path().join("favicon.svg"))
        );
    }

    #[test]
    fn declared_link_icon_beats_direct_candidates() {
        // Upstream: `FAVICON_CANDIDATES` run before source files — a public/
        // favicon still wins over a declared brand mark.
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "index.html",
            r#"<html><head><link rel="icon" href="/brand/logo.svg"></head></html>"#,
        );
        write(root.path(), "public/brand/logo.svg", "<svg>brand</svg>");
        write(root.path(), "public/favicon.png", "icon");
        assert_eq!(
            resolve_project_favicon(root.path()),
            Some(root.path().join("public/favicon.png"))
        );
    }

    #[test]
    fn declared_link_icon_without_public_candidate_wins() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "index.html",
            r#"<head><link href="/brand/logo.svg" rel="icon"></head>"#,
        );
        write(root.path(), "public/brand/logo.svg", "<svg>brand</svg>");
        assert_eq!(
            resolve_project_favicon(root.path()),
            Some(root.path().join("public/brand/logo.svg"))
        );
        // Root-relative fallback covers hrefs that resolve under the root.
        write(
            root.path(),
            "index.html",
            r#"<link rel="icon" href="mark.svg">"#,
        );
        write(root.path(), "mark.svg", "<svg>mark</svg>");
        assert_eq!(
            resolve_project_favicon(root.path()),
            Some(root.path().join("mark.svg"))
        );
    }

    #[test]
    fn nested_and_app_like_folders_are_fallbacks() {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "apps/web/public/favicon.png", "icon");
        assert_eq!(
            resolve_project_favicon(root.path()),
            Some(root.path().join("apps/web/public/favicon.png"))
        );
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "vatrium-dashboard/public/favicon.png", "icon");
        write(root.path(), "docs/public/favicon.png", "not-app-like");
        assert_eq!(
            resolve_project_favicon(root.path()),
            Some(root.path().join("vatrium-dashboard/public/favicon.png"))
        );
    }

    #[test]
    fn prefers_declared_icon_over_nested_candidates() {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "index.html",
            r#"<link rel="icon" href="/brand/logo.svg">"#,
        );
        write(
            root.path(),
            "public/brand/logo.svg",
            "<svg>project brand</svg>",
        );
        write(root.path(), "apps/web/public/favicon.png", "nested icon");
        assert_eq!(
            resolve_project_favicon(root.path()),
            Some(root.path().join("public/brand/logo.svg"))
        );
    }

    #[test]
    fn link_href_is_untrusted_and_stays_inside_the_project() {
        // An escaped href (dot segments, absolute path) never resolves outside
        // the project root, even when the file exists there.
        let root = tempfile::tempdir().unwrap();
        let outside = root.path().parent().unwrap().join("favicon-outside.svg");
        fs::write(&outside, "<svg>outside</svg>").unwrap();
        write(
            root.path(),
            "index.html",
            r#"<link rel="icon" href="../favicon-outside.svg">"#,
        );
        assert_eq!(resolve_project_favicon(root.path()), None);
        let _ = fs::remove_file(outside);
    }

    #[test]
    fn empty_project_and_missing_files_resolve_none() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(resolve_project_favicon(root.path()), None);
        write(root.path(), "index.html", "<html><head></head></html>");
        assert_eq!(resolve_project_favicon(root.path()), None);
        write(
            root.path(),
            "index.html",
            r#"<link rel="icon" href="gone.svg">"#,
        );
        assert_eq!(resolve_project_favicon(root.path()), None);
    }

    #[test]
    fn object_literal_icon_declarations_resolve() {
        // TanStack Router root files declare icons as an object literal, which
        // upstream's LINK_ICON_OBJ_RE covers.
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "app/routes/__root.tsx",
            r#"links: [{ rel: "icon", href: "/favicon-mini.svg" }]"#,
        );
        write(root.path(), "public/favicon-mini.svg", "<svg/>");
        assert_eq!(
            resolve_project_favicon(root.path()),
            Some(root.path().join("public/favicon-mini.svg"))
        );
    }
}
