//! The Rust port of `packages/shared/src/pathContainment.ts`.
//!
//! Behavior inferred clean-room from recorded cases: both paths are resolved
//! to absolute segments (POSIX semantics, `.`/`..` normalized, `\` is not a
//! separator, relative inputs resolve against `process.cwd()`), then the
//! child is inside when `relative(parent, child)` is empty or does not begin
//! with a `..` segment.

#![forbid(unsafe_code)]

/// Normalize a `/`-split path into segments: drops `''` and `'.'`, resolves
/// `..` (pops a real segment, keeps leading `..` only for relative paths).
fn normalize_segments(segments: &[&str], is_absolute: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for segment in segments {
        match *segment {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|last| last != "..") {
                    out.pop();
                } else if !is_absolute {
                    out.push("..".to_string());
                }
            }
            other => out.push(other.to_string()),
        }
    }
    out
}

/// `path.posix.resolve` on a single input against `cwd` (both absolute).
fn resolve(input: &str, cwd: &str) -> Vec<String> {
    let joined = if input.starts_with('/') {
        input.to_string()
    } else if input.is_empty() {
        cwd.to_string()
    } else {
        format!("{cwd}/{input}")
    };
    let segments: Vec<&str> = joined.split('/').collect();
    normalize_segments(&segments, true)
}

/// Port of `isPathInside` (`packages/shared/src/pathContainment.ts:12`).
pub fn is_path_inside(parent: String, child: String) -> Result<bool, String> {
    let cwd = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    let from = resolve(&parent, &cwd);
    let to = resolve(&child, &cwd);

    let mut i = 0;
    while i < from.len() && i < to.len() && from[i] == to[i] {
        i += 1;
    }
    let mut rel: Vec<String> = vec!["..".to_string(); from.len() - i];
    rel.extend(to[i..].iter().cloned());
    let rel = rel.join("/");

    Ok(rel.is_empty() || (rel != ".." && !rel.starts_with("../")))
}
