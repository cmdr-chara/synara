//! The Rust port of `apps/server/src/codexPathIdentity.ts` (posix, darwin).

#![forbid(unsafe_code)]

/// Node `path.posix.normalize`.
fn posix_normalize(p: &str) -> String {
    let is_abs = p.starts_with('/');
    let trailing = p.len() > 1 && p.ends_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last() == Some(&"..") || (out.is_empty() && !is_abs) {
                    out.push("..");
                } else {
                    out.pop();
                }
            }
            s => out.push(s),
        }
    }
    let mut r = if is_abs {
        format!("/{}", out.join("/"))
    } else {
        out.join("/")
    };
    if r.is_empty() {
        r = if is_abs {
            "/".to_string()
        } else {
            ".".to_string()
        };
    }
    if trailing && r != "/" {
        r.push('/');
    }
    r
}

/// Node `path.posix.resolve` for a single argument.
fn posix_resolve(p: &str) -> String {
    if p.starts_with('/') {
        posix_normalize(p)
    } else {
        let cwd = std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/".to_string());
        posix_normalize(&format!("{cwd}/{p}"))
    }
}

/// Node `path.posix.dirname`.
fn posix_dirname(p: &str) -> String {
    let s = p.trim_end_matches('/');
    let s = if s.is_empty() { "/" } else { s };
    match s.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(i) => s[..i].to_string(),
    }
}

/// Node `path.posix.basename`.
fn posix_basename(p: &str) -> String {
    let s = p.trim_end_matches('/');
    match s.rfind('/') {
        None => s.to_string(),
        Some(i) => s[i + 1..].to_string(),
    }
}

/// Node `path.posix.join` over segments.
fn posix_join(parts: &[String]) -> String {
    posix_normalize(
        &parts
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("/"),
    )
}

/// darwin: no case folding.
fn normalize_identity(p: &str) -> String {
    posix_normalize(p)
}

fn resolve_identity(input_path: &str) -> String {
    let resolved_input = posix_resolve(input_path);
    let mut missing_segments: Vec<String> = Vec::new();
    let mut candidate = resolved_input.clone();
    loop {
        match std::fs::canonicalize(&candidate) {
            Ok(real) => {
                let real = real.to_string_lossy().into_owned();
                let rebuilt = if missing_segments.is_empty() {
                    real
                } else {
                    let mut parts = vec![real];
                    parts.extend(missing_segments.iter().cloned());
                    posix_join(&parts)
                };
                return normalize_identity(&rebuilt);
            }
            Err(_) => {
                let parent = posix_dirname(&candidate);
                if parent == candidate {
                    return normalize_identity(&resolved_input);
                }
                missing_segments.insert(0, posix_basename(&candidate));
                candidate = parent;
            }
        }
    }
}

/// Port of `resolveCodexPathIdentity` (`apps/server/src/codexPathIdentity.ts:14`).
pub fn resolve_codex_path_identity(input_path: String) -> Result<String, String> {
    Ok(resolve_identity(&input_path))
}

/// Port of `codexPathsReferenceSameLocation` (`apps/server/src/codexPathIdentity.ts:40`).
pub fn codex_paths_reference_same_location(left: String, right: String) -> Result<bool, String> {
    Ok(resolve_identity(&left) == resolve_identity(&right))
}
