//! The Rust port of `apps/server/src/providerAccountHomePath.ts`.

#![forbid(unsafe_code)]

/// `path.posix.join` semantics (the host this was recorded on): empty args are
/// skipped, the parts join with '/', then posix normalization resolves `.` and
/// `..` (root clamps, a relative result keeps leading `..`, empty means `.`).
fn posix_join(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    let absolute = joined.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => match out.last() {
                Some(&"..") => out.push(".."),
                Some(_) => {
                    out.pop();
                }
                None => {
                    if !absolute {
                        out.push("..");
                    }
                }
            },
            s => out.push(s),
        }
    }
    let norm = out.join("/");
    if absolute {
        format!("/{norm}")
    } else if norm.is_empty() {
        ".".to_string()
    } else {
        norm
    }
}

/// Port of `expandProviderAccountHomePath` (`apps/server/src/providerAccountHomePath.ts:8`).
pub fn expand_provider_account_home_path(
    input: String,
    home_dir: Option<String>,
) -> Result<String, String> {
    let home_dir =
        home_dir.unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "/".to_string()));
    if input == "~" {
        return Ok(home_dir);
    }
    if !input.starts_with("~/") && !input.starts_with("~\\") {
        return Ok(input);
    }
    // Both leading chars are ASCII, so a byte slice is the UTF-16 slice too.
    let segments: Vec<&str> = input[2..]
        .split(['\\', '/'])
        .filter(|s| !s.is_empty())
        .collect();
    if segments.is_empty() {
        return Ok(home_dir);
    }
    let mut parts: Vec<&str> = Vec::with_capacity(segments.len() + 1);
    parts.push(home_dir.as_str());
    parts.extend(segments);
    Ok(posix_join(&parts))
}
