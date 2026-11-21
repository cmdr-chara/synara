//! The Rust port of `packages/shared/src/wslBridge.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

fn is_win32(platform: &Option<Value>) -> bool {
    matches!(platform, Some(Value::String(s)) if s == "win32")
}

/// The `\\wsl$\` / `\\wsl.localhost\` UNC translation inlined from `parseWindowsWslUncPath`.
fn parse_wsl_unc(value: &str) -> Option<(String, String)> {
    let trimmed = js_trim(value);
    let rest = trimmed.strip_prefix("\\\\")?;
    let (host, tail) = rest.split_once('\\')?;
    let host_lower = host.to_ascii_lowercase();
    if host_lower != "wsl.localhost" && host_lower != "wsl$" {
        return None;
    }
    let (group1, suffix) = match tail.split_once('\\') {
        Some((a, b)) => (a, Some(b)),
        None => (tail, None),
    };
    let distribution = js_trim(group1);
    if distribution.is_empty() {
        return None;
    }
    let linux_path = match suffix {
        Some(s) => format!(
            "/{}",
            s.split('\\')
                .filter(|seg| !seg.is_empty())
                .collect::<Vec<_>>()
                .join("/")
        ),
        None => "/".to_string(),
    };
    Some((distribution.to_string(), linux_path))
}

/// Port of `resolveWslWorkspace` (`packages/shared/src/wslBridge.ts:13`).
pub fn resolve_wsl_workspace(
    cwd: String,
    platform: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    if !is_win32(&platform) {
        return Ok(None);
    }
    Ok(parse_wsl_unc(&cwd).map(|(distribution, linux_path)| {
        json!({
            "distribution": distribution,
            "linuxPath": linux_path,
        })
    }))
}

/// Port of `resolveExecutionWorkingDirectory` (`packages/shared/src/wslBridge.ts:21`).
pub fn resolve_execution_working_directory(
    cwd: String,
    platform: Option<serde_json::Value>,
) -> Result<String, String> {
    if is_win32(&platform)
        && let Some((_, linux_path)) = parse_wsl_unc(&cwd)
    {
        return Ok(linux_path);
    }
    Ok(cwd)
}
