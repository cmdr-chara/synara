//! The Rust port of `apps/desktop/src/updatePendingCache.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

const JS_WS: &[char] = &[
    '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}',
    '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}', '\u{2009}',
    '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
];

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c| JS_WS.contains(&c))
}

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => js::is_undefined(v),
        _ => js::as_number(v).is_some_and(|n| n == 0.0 || n.is_nan()),
    }
}

fn path_arg_err(v: Option<&Value>) -> String {
    let desc = match v {
        Some(Value::Null) => "null".to_string(),
        Some(Value::Bool(b)) => format!("type boolean ({b})"),
        Some(Value::Number(n)) => format!("type number ({})", n),
        Some(Value::String(s)) => format!("type string ('{s}')"),
        Some(Value::Array(_)) => "an instance of Array".to_string(),
        Some(Value::Object(o)) if !js::is_undefined(v.unwrap_or(&Value::Null)) => {
            match o.get("$num").and_then(|x| x.as_str()) {
                Some(m) => format!("type number ({m})"),
                None => "an instance of Object".to_string(),
            }
        }
        _ => "undefined".to_string(),
    };
    format!("\"path\" argument must be of type string. Received {desc}")
}

fn norm(segs: &[&str], abs: bool, trailing: bool, sep: char) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in segs {
        match *seg {
            "" | "." => {}
            ".." => {
                if out.last().is_some_and(|s| *s != "..") {
                    out.pop();
                } else if !abs {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let mut res = out.join(&sep.to_string());
    if abs {
        res = format!("{sep}{res}");
    }
    if res.is_empty() {
        res = if abs {
            sep.to_string()
        } else if trailing {
            format!(".{sep}")
        } else {
            ".".to_string()
        };
    } else if trailing && res.len() > 1 {
        res.push(sep);
    }
    res
}

fn join(parts: &[Option<&Value>], win32: bool) -> Result<String, String> {
    let mut segs: Vec<&str> = Vec::new();
    for part in parts {
        match part {
            Some(Value::String(s)) => {
                if !s.is_empty() {
                    segs.push(s);
                }
            }
            _ => return Err(path_arg_err(*part)),
        }
    }
    if segs.is_empty() {
        return Ok(".".to_string());
    }
    let joined = segs.join(if win32 { "\\" } else { "/" });
    if !win32 {
        let abs = joined.starts_with('/');
        let trailing = joined.ends_with('/');
        return Ok(norm(
            &joined.split('/').collect::<Vec<_>>(),
            abs,
            trailing,
            '/',
        ));
    }
    // win32: split on both separators; first segment may carry a drive letter
    // (e.g. "C:" or a "C:\\" prefix) or a leading UNC root.
    let trailing = joined.ends_with(['\\', '/']);
    let mut device = String::new();
    let mut rest = joined.as_str();
    let mut abs = false;
    if joined.len() >= 2
        && joined.as_bytes()[0].is_ascii_alphabetic()
        && joined.as_bytes()[1] == b':'
    {
        device = joined[..2].to_string();
        rest = &joined[2..];
        abs = rest.starts_with(['\\', '/']);
    } else if joined.starts_with("\\\\") || joined.starts_with("//") {
        // UNC root: the device is "\\server\share" (first two real segments).
        let parts: Vec<&str> = joined
            .split(['\\', '/'])
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 2 {
            device = format!("\\\\{}\\{}", parts[0], parts[1]);
            let idx = 2 + parts[0].len() + 1 + parts[1].len();
            rest = joined.get(idx..).unwrap_or("");
            abs = true;
        }
    } else if joined.starts_with(['\\', '/']) {
        abs = true;
    }
    Ok(format!(
        "{device}{}",
        norm(
            &rest.split(['\\', '/']).collect::<Vec<_>>(),
            abs,
            trailing,
            '\\'
        )
    ))
}

fn get<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.as_object().and_then(|o| o.get(key))
}

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn cache_dir(args: &Value) -> Result<Option<String>, String> {
    if args.is_null() || js::is_undefined(args) {
        let kind = if args.is_null() { "null" } else { "undefined" };
        return Err(format!(
            "Cannot read properties of {kind} (reading 'cacheDirName')"
        ));
    }
    let name = get(args, "cacheDirName");
    match name {
        Some(v) if falsy(v) => return Ok(None),
        None => return Ok(None),
        Some(Value::String(_)) => {}
        Some(v) => return Err(path_arg_err(Some(v))),
    }
    let is_win = matches!(get(args, "platform"), Some(Value::String(p)) if p == "win32");
    let is_mac = matches!(get(args, "platform"), Some(Value::String(p)) if p == "darwin");
    let home = get(args, "homeDir");
    let base = if is_win {
        match get(args, "localAppData") {
            Some(v) if !falsy(v) => match v {
                Value::String(x) => x.clone(),
                _ => return Err(path_arg_err(Some(v))),
            },
            _ => join(&[home, Some(&s("AppData")), Some(&s("Local"))], true)?,
        }
    } else if is_mac {
        join(&[home, Some(&s("Library")), Some(&s("Caches"))], false)?
    } else {
        match get(args, "xdgCacheHome") {
            Some(v) if !falsy(v) => match v {
                Value::String(x) => x.clone(),
                _ => return Err(path_arg_err(Some(v))),
            },
            _ => join(&[home, Some(&s(".cache"))], false)?,
        }
    };
    Ok(Some(join(&[Some(&s(&base)), name], is_win)?))
}

/// `rawConfig?.updaterCacheDirName?.trim() || appName`
/// Port of `resolveElectronUpdaterCacheDirName` (`apps/desktop/src/updatePendingCache.ts:8`).
pub fn resolve_electron_updater_cache_dir_name(
    raw_config: Option<serde_json::Value>,
    app_name: String,
) -> Result<String, String> {
    let name = raw_config
        .as_ref()
        .filter(|c| !js::is_undefined(c))
        .and_then(|c| c.as_object())
        .and_then(|o| o.get("updaterCacheDirName"));
    match name {
        Some(Value::String(v)) => {
            let t = js_trim(v);
            Ok(if t.is_empty() {
                app_name
            } else {
                t.to_string()
            })
        }
        Some(Value::Null) | None => Ok(app_name),
        Some(v) if js::is_undefined(v) => Ok(app_name),
        Some(_) => Err("rawConfig.updaterCacheDirName.trim is not a function".to_string()),
    }
}

/// Port of `resolveElectronUpdaterPendingCacheDir` (`apps/desktop/src/updatePendingCache.ts:15`).
pub fn resolve_electron_updater_pending_cache_dir(
    args: serde_json::Value,
) -> Result<Option<String>, String> {
    leaf(&args, "pending")
}

/// Port of `resolveElectronUpdaterCacheDir` (`apps/desktop/src/updatePendingCache.ts:31`).
pub fn resolve_electron_updater_cache_dir(
    args: serde_json::Value,
) -> Result<Option<String>, String> {
    cache_dir(&args)
}

/// Port of `resolveElectronUpdaterLegacyZipPath` (`apps/desktop/src/updatePendingCache.ts:53`).
pub fn resolve_electron_updater_legacy_zip_path(
    args: serde_json::Value,
) -> Result<Option<String>, String> {
    leaf(&args, "update.zip")
}

fn leaf(args: &Value, name: &str) -> Result<Option<String>, String> {
    let Some(dir) = cache_dir(args)? else {
        return Ok(None);
    };
    let win = matches!(get(args, "platform"), Some(Value::String(p)) if p == "win32");
    Ok(Some(join(&[Some(&s(&dir)), Some(&s(name))], win)?))
}
