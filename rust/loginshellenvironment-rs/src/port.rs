//! The Rust port of `packages/shared/src/loginShellEnvironment.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const SYNARA_HOME_ENV_NAME: &str = "SYNARA_HOME";
const DEFAULT_DIRECTORY_NAME: &str = ".synara";
const CACHE_FILE_NAME: &str = "login-shell-environment.json";

fn js_ws(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r'
            | ' '
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(js_ws)
}

/// Node `path.posix.normalize`.
fn normalize(path: &str) -> String {
    let absolute = path.starts_with('/');
    let trailing = path.ends_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." {
            if let Some(&last) = parts.last() {
                if last != ".." {
                    parts.pop();
                    continue;
                }
            }
            if !absolute {
                parts.push("..");
            }
        } else {
            parts.push(seg);
        }
    }
    let mut out = String::new();
    if absolute {
        out.push('/');
    }
    out += &parts.join("/");
    if out.is_empty() {
        out.push(if absolute { '/' } else { '.' });
    } else if trailing {
        out.push('/');
    }
    out
}

/// Node `path.posix.join` (args are already validated strings; empty ones are skipped).
fn join(segments: &[String]) -> String {
    normalize(
        &segments
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("/"),
    )
}

/// Node `path.posix.resolve`.
fn resolve(segments: &[String]) -> String {
    let mut resolved = String::new();
    let mut absolute = false;
    for p in segments.iter().rev() {
        if p.is_empty() {
            continue;
        }
        resolved = format!("{p}/{resolved}");
        if p.starts_with('/') {
            absolute = true;
            break;
        }
    }
    if !absolute {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/".to_string());
        resolved = format!("{cwd}/{resolved}");
    }
    normalize(&resolved)
}

fn expand_home_path(input: &str, home_directory: &str) -> String {
    if input == "~" {
        return home_directory.to_string();
    }
    if let Some(rest) = input
        .strip_prefix("~/")
        .or_else(|| input.strip_prefix("~\\"))
    {
        return join(&[home_directory.to_string(), rest.to_string()]);
    }
    input.to_string()
}

fn prop<'a>(options: &'a Value, name: &str) -> Option<&'a Value> {
    options.get(name).filter(|v| !v.is_null())
}

/// `loginShellEnvironmentCachePath` (`packages/shared/src/loginShellEnvironment.ts:51`):
/// `Path.join(resolveSynaraHomeDirectory(options), "cache", CACHE_FILE_NAME)`.
pub fn login_shell_environment_cache_path(options: serde_json::Value) -> Result<String, String> {
    if options.is_null() {
        return Err("Cannot read properties of null (reading 'homeDirectory')".to_string());
    }
    let options = Some(options);
    // Property reads on primitives yield undefined without throwing.
    let get = |name: &str| -> Option<&Value> { options.as_ref().and_then(|o| prop(o, name)) };

    // os.homedir(): $HOME on POSIX.
    let home_directory = match get("homeDirectory") {
        None => std::env::var("HOME").map_err(|e| e.to_string())?,
        Some(v) => v
            .as_str()
            .ok_or_else(|| "The \"path\" argument must be of type string".to_string())?
            .to_string(),
    };

    // configuredHome ?? (env ?? process.env)[SYNARA_HOME]
    let configured: Option<String> = match get("configuredHome") {
        Some(v) => match v {
            Value::String(s) => Some(js_trim(s).to_string()),
            _ => return Err("configured.trim is not a function".to_string()),
        },
        None => match get("env") {
            Some(Value::Object(map)) => {
                match map.get(SYNARA_HOME_ENV_NAME).filter(|v| !v.is_null()) {
                    None => None,
                    Some(Value::String(s)) => Some(js_trim(s).to_string()),
                    Some(_) => return Err("configured.trim is not a function".to_string()),
                }
            }
            Some(_) => None,
            None => std::env::var(SYNARA_HOME_ENV_NAME)
                .ok()
                .map(|s| js_trim(&s).to_string()),
        },
    };

    let home = match configured.filter(|c| !c.is_empty()) {
        None => {
            let dir_name = match get("directoryName") {
                None => DEFAULT_DIRECTORY_NAME.to_string(),
                Some(v) => v
                    .as_str()
                    .ok_or_else(|| "The \"path\" argument must be of type string".to_string())?
                    .to_string(),
            };
            join(&[home_directory, dir_name])
        }
        Some(configured) => resolve(&[expand_home_path(&configured, &home_directory)]),
    };

    Ok(join(&[
        home,
        "cache".to_string(),
        CACHE_FILE_NAME.to_string(),
    ]))
}
