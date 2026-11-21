//! The Rust port of `apps/server/src/os-jank.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn home_dir() -> String {
    std::env::var("HOME")
        .ok()
        .filter(|h| !h.is_empty())
        .or_else(|| std::env::var("USERPROFILE").ok().filter(|h| !h.is_empty()))
        .unwrap_or_else(|| "/".to_string())
}

/// POSIX `path.normalize`: collapse `//`, drop `.`, resolve `..` lexically,
/// drop a trailing slash unless the result is the root.
fn path_normalize(p: &str) -> String {
    let absolute = p.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|s| *s != "..") {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let joined = out.join("/");
    if absolute {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

/// POSIX `path.join`: join segments with `/` then normalize; all-empty gives ".".
fn path_join(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    path_normalize(&joined)
}

/// POSIX `path.resolve`: like join but the result is absolute — a relative tail is
/// rooted at the process cwd.
fn path_resolve(p: &str) -> String {
    if p.starts_with('/') {
        return path_normalize(p);
    }
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "/".to_string());
    path_normalize(&format!("{cwd}/{p}"))
}

/// `expandHomePath` from `packages/shared/src/synaraHome.ts` (POSIX).
fn expand(input: &str, home: &str) -> String {
    if input == "~" {
        return home.to_string();
    }
    if input.starts_with("~/") || input.starts_with("~\\") {
        return path_join(&[home, &input[2..]]);
    }
    input.to_string()
}

fn effect_succeed(value: Value) -> Value {
    json!({ "~effect/Effect/args": value })
}

/// Port of `expandHomePath` (`apps/server/src/os-jank.ts:85`).
pub fn expand_home_path(input: String) -> Result<Value, String> {
    Ok(effect_succeed(json!(expand(&input, &home_dir()))))
}

/// Port of `resolveBaseDir` (`apps/server/src/os-jank.ts:88`).
pub fn resolve_base_dir(raw: Option<String>) -> Result<Value, String> {
    let configured = match raw {
        Some(s) => Some(s.trim().to_string()),
        None => std::env::var("SYNARA_HOME")
            .ok()
            .map(|s| s.trim().to_string()),
    }
    .filter(|s| !s.is_empty());
    let home = home_dir();
    let resolved = match configured {
        None => path_join(&[&home, ".synara"]),
        Some(c) => path_resolve(&expand(&c, &home)),
    };
    Ok(effect_succeed(json!(resolved)))
}
