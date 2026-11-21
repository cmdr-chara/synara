//! The Rust port of `packages/shared/src/synaraHome.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

use serde_json::Value;

fn os_home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/root".to_string())
}

/// POSIX `path.normalize` on one path.
fn normalize(p: &str) -> String {
    let absolute = p.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|&t| t != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            s => parts.push(s),
        }
    }
    let mut out = String::new();
    if absolute {
        out.push('/');
    }
    out.push_str(&parts.join("/"));
    if out.is_empty() {
        out.push_str(if absolute { "/" } else { "." });
    }
    out
}

/// POSIX `path.join(a, b)`.
fn join(a: &str, b: &str) -> String {
    if b.is_empty() {
        normalize(a)
    } else if a.is_empty() {
        normalize(b)
    } else {
        normalize(&format!("{a}/{b}"))
    }
}

/// POSIX `path.resolve(cwd, p)`.
fn resolve(p: &str) -> String {
    if p.starts_with('/') {
        normalize(p)
    } else {
        let cwd = std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/".to_string());
        normalize(&format!("{cwd}/{p}"))
    }
}

/// Port of `expandHomePath`: `~` or `~/` / `~\` expands to the given home
/// directory (or the OS home); anything else is returned verbatim.
pub fn expand_home_path(input: String, home_directory: Option<String>) -> Result<String, String> {
    let rest = match input.strip_prefix('~') {
        Some(r) => r,
        None => return Ok(input),
    };
    if !rest.is_empty() && !rest.starts_with('/') && !rest.starts_with('\\') {
        return Ok(input);
    }
    let home = home_directory.unwrap_or_else(os_home);
    let rest = rest.trim_start_matches(['/', '\\']);
    if rest.is_empty() {
        return Ok(home);
    }
    Ok(format!("{}/{}", home.trim_end_matches('/'), rest))
}

fn is_undef(v: &Value) -> bool {
    v.get("$undefined") == Some(&Value::Bool(true))
}

/// Port of `resolveSynaraHomeDirectory`.
pub fn resolve_synara_home_directory(options: Value) -> Result<String, String> {
    // `null` throws on `.homeDirectory`; `undefined` (missing arg) defaults to `{}`.
    let options = match options {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'homeDirectory')".to_string());
        }
        v if is_undef(&v) => Value::Object(serde_json::Map::new()),
        v => v,
    };
    let home_dir = options
        .get("homeDirectory")
        .and_then(Value::as_str)
        .map(str::to_string);
    let configured = match options.get("configuredHome") {
        Some(v) if !v.is_null() && !is_undef(v) => Some(v.clone()),
        _ => None,
    };
    let configured = configured.or_else(|| {
        let env = options.get("env");
        match env {
            Some(v) if !v.is_null() && !is_undef(v) => match v {
                Value::Object(m) => m.get("SYNARA_HOME").cloned(),
                _ => None,
            },
            _ => std::env::var("SYNARA_HOME").ok().map(Value::String),
        }
    });
    if let Some(v) = configured {
        match v {
            Value::String(s) => {
                let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
                if !t.is_empty() {
                    return Ok(resolve(&expand_home_path(t.to_string(), home_dir)?));
                }
            }
            _ => {
                return Err("(options.configuredHome ?? (options.env ?? process.env)\
[SYNARA_HOME_ENV_NAME])?.trim is not a function"
                    .to_string());
            }
        }
    }
    let home = home_dir.unwrap_or_else(os_home);
    let dir = options
        .get("directoryName")
        .and_then(Value::as_str)
        .unwrap_or(".synara");
    Ok(join(&home, dir))
}
