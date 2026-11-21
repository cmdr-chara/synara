//! The Rust port of `apps/desktop/src/backendCompileCache.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

use crate::js;

const CACHE_PARENT: &str = "cache/backend-compile";

const JS_WS: &[char] = &[
    '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}',
    '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}',
    '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
];

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c| JS_WS.contains(&c))
}

/// `appVersion.trim().replace(/[^A-Za-z0-9._-]+/g, "-") || "unknown"`.
fn sanitize_version(app_version: &str) -> String {
    let trimmed = js_trim(app_version);
    let mut out = String::new();
    let mut in_run = false;
    for c in trimmed.chars() {
        let ok = c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
        if ok {
            out.push(c);
            in_run = false;
        } else if !in_run {
            out.push('-');
            in_run = true;
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

/// `Path.posix.join`: join non-empty parts, then normalize `.`/`..` segments.
fn posix_join(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    if joined.is_empty() {
        return ".".to_string();
    }
    let absolute = joined.starts_with('/');
    let trailing = joined.len() > 1 && joined.ends_with('/');
    let mut segs: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if matches!(segs.last(), Some(&s) if s != "..") {
                    segs.pop();
                } else if !absolute {
                    segs.push("..");
                }
            }
            s => segs.push(s),
        }
    }
    let mut out = String::new();
    if absolute {
        out.push('/');
    }
    out.push_str(&segs.join("/"));
    if out.is_empty() {
        out.push('.');
    } else if trailing && !out.ends_with('/') {
        out.push('/');
    }
    out
}

/// Port of `resolveBackendCompileCacheDir` (`apps/desktop/src/backendCompileCache.ts:23`).
pub fn resolve_backend_compile_cache_dir(
    state_dir: String,
    app_version: String,
) -> Result<String, String> {
    Ok(posix_join(&[
        &state_dir,
        CACHE_PARENT,
        &sanitize_version(&app_version),
    ]))
}

/// Port of `backendCompileCacheEnv` (`apps/desktop/src/backendCompileCache.ts:30`).
pub fn backend_compile_cache_env(
    inherited: serde_json::Value,
    cache_dir: String,
) -> Result<serde_json::Value, String> {
    if inherited.is_null() {
        return Err("Cannot read properties of null (reading 'NODE_COMPILE_CACHE')".to_string());
    }
    if js::is_undefined(&inherited) {
        return Err(
            "Cannot read properties of undefined (reading 'NODE_COMPILE_CACHE')".to_string(),
        );
    }
    let existing = match &inherited {
        Value::Object(o) => o
            .get("NODE_COMPILE_CACHE")
            .cloned()
            .unwrap_or_else(js::undefined),
        _ => js::undefined(),
    };
    let configured = match &existing {
        Value::String(s) => !js_trim(s).is_empty(),
        v if js::is_undefined(v) || v.is_null() => false,
        _ => {
            return Err("inherited.NODE_COMPILE_CACHE.trim is not a function".to_string());
        }
    };
    if configured {
        return Ok(json!({ "SYNARA_BACKEND_COMPILE_CACHE": js::undefined() }));
    }
    Ok(json!({
        "NODE_COMPILE_CACHE": cache_dir,
        "SYNARA_BACKEND_COMPILE_CACHE": cache_dir,
    }))
}

/// Port of `pruneBackendCompileCaches` (`apps/desktop/src/backendCompileCache.ts:41`).
/// Best-effort: removes sibling directories of `cacheDir`; every failure is swallowed.
pub async fn prune_backend_compile_caches(cache_dir: String) -> Result<(), String> {
    let path = std::path::Path::new(&cache_dir);
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    let Some(current) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
        return Ok(());
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Ok(());
    };
    for entry in entries.flatten() {
        let file_name = entry.file_name();
        if file_name.to_string_lossy() == current {
            continue;
        }
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            let _ = std::fs::remove_dir_all(entry.path());
        }
    }
    Ok(())
}
