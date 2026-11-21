//! The Rust port of `apps/desktop/src/linuxAppIcon.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const ICON_NAME_CHARS: fn(char) -> bool =
    |c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');

fn icon_name_valid(value: &str) -> bool {
    !value.is_empty()
        && value.chars().all(ICON_NAME_CHARS)
        && value
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
}

fn member<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

fn js_trim(s: &str) -> &str {
    const WS: &[char] = &[
        '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}',
        '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}',
        '\u{2009}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}',
        '\u{feff}',
    ];
    s.trim_matches(|c| WS.contains(&c))
}

fn posix_normalize(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|p| *p != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            s => parts.push(s),
        }
    }
    let joined = parts.join("/");
    match (absolute, joined.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => format!("/{joined}"),
        (false, true) => ".".to_string(),
        (false, false) => joined,
    }
}

fn posix_join(parts: &[String]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("/");
    posix_normalize(&joined)
}

/// `env[key]?.trim()` — TypeError when the member exists but is not a string's trim.
fn env_trimmed<'a>(env: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    let Some(v) = member(env, key) else {
        return Ok(None);
    };
    match v {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(s.as_str())),
        _ => Err(format!("env.{key}.trim is not a function")),
    }
}

/// `synaraDesktopIdentity(flavor).userDataDirectoryName`.
fn user_data_directory_name(flavor: &Value) -> &'static str {
    match flavor.as_str() {
        Some("cua") => "synara-cua",
        Some("canary") => "synara-canary",
        Some("beta") => "synara-beta",
        Some("remote") => "synara-remote",
        Some("development") => "synara-dev",
        _ => "synara",
    }
}

/// Port of `desktopEntryIconName` (`apps/desktop/src/linuxAppIcon.ts:24`).
pub fn desktop_entry_icon_name(contents: String) -> Result<Option<String>, String> {
    let mut in_desktop_entry = false;
    for raw_line in contents.split("\r\n").flat_map(|part| part.split('\n')) {
        let line = js_trim(raw_line);
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry || !line.starts_with("Icon=") {
            continue;
        }
        let value = js_trim(&line["Icon=".len()..]);
        return Ok(if icon_name_valid(value) {
            Some(value.to_string())
        } else {
            None
        });
    }
    Ok(None)
}

fn linux_data_home_inner(env: &Value, home_dir: &str) -> Result<String, String> {
    match env_trimmed(env, "XDG_DATA_HOME") {
        Err(e) => Err(e),
        Ok(Some(configured)) => {
            let configured = js_trim(configured);
            if !configured.is_empty() && configured.starts_with('/') {
                Ok(configured.to_string())
            } else {
                Ok(posix_join(&[
                    home_dir.to_string(),
                    ".local".to_string(),
                    "share".to_string(),
                ]))
            }
        }
        Ok(None) => Ok(posix_join(&[
            home_dir.to_string(),
            ".local".to_string(),
            "share".to_string(),
        ])),
    }
}

/// Port of `linuxDataHome` (`apps/desktop/src/linuxAppIcon.ts:74`).
pub fn linux_data_home(env: Value, home_dir: String) -> Result<String, String> {
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'XDG_DATA_HOME')".to_string());
    }
    if crate::js::is_undefined(&env) {
        return Err("Cannot read properties of undefined (reading 'XDG_DATA_HOME')".to_string());
    }
    linux_data_home_inner(&env, &home_dir)
}

/// Port of `resolveLinuxDesktopIconName` (`apps/desktop/src/linuxAppIcon.ts:46`).
pub fn resolve_linux_desktop_icon_name(input: Value) -> Result<Option<String>, String> {
    let env = member(&input, "env")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let home_dir = member(&input, "homeDir")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let flavor = member(&input, "flavor")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    // `input.env` itself is read first; a null/undefined *input* throws on 'env'
    // while a null/undefined env value throws inside linuxDataHome on XDG_DATA_HOME.
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'env')".to_string());
    }
    if crate::js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'env')".to_string());
    }
    // linuxDataHome(input.env, input.homeDir): reading XDG_DATA_HOME off
    // null/undefined env throws there.
    if env.is_null() {
        return Err("Cannot read properties of null (reading 'XDG_DATA_HOME')".to_string());
    }
    if crate::js::is_undefined(&env) {
        return Err("Cannot read properties of undefined (reading 'XDG_DATA_HOME')".to_string());
    }
    let data_home = linux_data_home_inner(&env, &home_dir)?;
    let data_dirs: Vec<String> = match env_trimmed(&env, "XDG_DATA_DIRS")? {
        Some(v) if !js_trim(v).is_empty() => js_trim(v).to_string(),
        _ => "/usr/local/share:/usr/share".to_string(),
    }
    .split(':')
    .filter(|d| d.starts_with('/'))
    .map(|d| d.to_string())
    .collect();
    let icon_name = user_data_directory_name(&flavor);
    let file_name = format!("{icon_name}.desktop");
    for directory in std::iter::once(data_home).chain(data_dirs) {
        let path = posix_join(&[directory, "applications".to_string(), file_name.clone()]);
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        return Ok(match desktop_entry_icon_name(contents)? {
            Some(name) if name == icon_name => Some(name),
            _ => None,
        });
    }
    Ok(None)
}
