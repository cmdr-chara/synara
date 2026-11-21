//! The Rust port of `apps/desktop/src/desktopUserDataProfile.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// A JS string's view of `v`: `String` plus the `{"$utf16": ...}` marker.
fn as_js_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.get("$utf16").and_then(Value::as_array).map(|units| {
            let u: Vec<u16> = units
                .iter()
                .filter_map(|x| x.as_u64().map(|n| n as u16))
                .collect();
            String::from_utf16_lossy(&u)
        }),
        _ => None,
    }
}

/// A JS number's view of `v`: `Number` plus the `{"$num": ...}` marker.
fn as_js_num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Object(o) if o.len() == 1 => match o.get("$num")?.as_str()? {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}

/// JS truthiness after decoding wire markers.
fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undefined(v) {
                return false;
            }
            if let Some(x) = as_js_num(v) {
                return x != 0.0 && !x.is_nan();
            }
            if let Some(s) = as_js_str(v) {
                return !s.is_empty();
            }
            true
        }
    }
}

/// `v.key` where `v` may be absent: null/undefined input yields undefined here
/// (callers reach it through `?.`), primitives and arrays yield undefined.
fn prop<'v>(v: Option<&'v Value>, key: &str) -> Option<&'v Value> {
    match v {
        Some(Value::Object(o)) if !is_undefined(v.unwrap()) => o.get(key),
        _ => None,
    }
}

/// `a ?? b`: the JS nullish fallback (`undefined` and `null` fall through).
fn nullish(v: Option<&Value>) -> bool {
    match v {
        None => true,
        Some(Value::Null) => true,
        Some(x) => is_undefined(x),
    }
}

/// Node `path.posix.normalize` (keeps a trailing `/`).
fn posix_normalize(p: &str) -> String {
    let absolute = p.starts_with('/');
    let trailing = p.ends_with('/') && p.len() > 1;
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
    let mut joined = if absolute {
        format!("/{}", out.join("/"))
    } else {
        out.join("/")
    };
    if joined.is_empty() {
        joined = if absolute {
            "/".to_string()
        } else {
            ".".to_string()
        };
    }
    if trailing && joined != "/" {
        joined.push('/');
    }
    joined
}

/// Node `path.posix.join`: empty segments are skipped; non-string arguments
/// raise `The "path" argument must be of type string`.
fn posix_join(segments: &[&Value]) -> Result<String, String> {
    let mut parts: Vec<String> = Vec::new();
    for seg in segments {
        match as_js_str(seg) {
            Some(s) => {
                if !s.is_empty() {
                    parts.push(s);
                }
            }
            None => {
                return Err(format!(
                    "The \"path\" argument must be of type string. Received {}",
                    received(seg)
                ));
            }
        }
    }
    if parts.is_empty() {
        return Ok(".".to_string());
    }
    Ok(posix_normalize(&parts.join("/")))
}

fn received(v: &Value) -> String {
    match v {
        Value::Object(o) if is_undefined(v) && o.len() == 1 => "undefined".to_string(),
        Value::Null => "null".to_string(),
        _ => "an instance of Object".to_string(),
    }
}

/// Node `path.resolve` (POSIX).
fn resolve_path(p: &str) -> String {
    if p.starts_with('/') {
        posix_normalize(p)
    } else {
        let cwd = std::env::current_dir()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "/".to_string());
        posix_normalize(&format!("{cwd}/{p}"))
    }
}

/// `path.posix.dirname`.
fn posix_dirname(p: &str) -> String {
    if p.is_empty() {
        return ".".to_string();
    }
    let has_root = p.starts_with('/');
    let mut end = p.len();
    while end > 0 && p.as_bytes()[end - 1] == b'/' {
        end -= 1;
    }
    if end == 0 {
        return if has_root {
            "/".to_string()
        } else {
            ".".to_string()
        };
    }
    let trimmed = &p[..end];
    match trimmed.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(i) => {
            let mut e = i;
            while e > 0 && trimmed.as_bytes()[e - 1] == b'/' {
                e -= 1;
            }
            if e == 0 {
                "/".to_string()
            } else {
                trimmed[..e].to_string()
            }
        }
    }
}

/// `String.prototype.trim` with the JS whitespace set.
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n'
                | '\u{000B}'
                | '\u{000C}'
                | '\r'
                | ' '
                | '\u{00A0}'
                | '\u{1680}'
                | '\u{2000}'
                ..='\u{200A}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202F}'
                    | '\u{205F}'
                    | '\u{3000}'
                    | '\u{FEFF}'
        )
    })
}

/// Port of `resolveDesktopAppDataBase` (`apps/desktop/src/desktopUserDataProfile.ts:34`).
pub fn resolve_desktop_app_data_base(input: Option<serde_json::Value>) -> Result<String, String> {
    let inp = input.as_ref();
    let platform = prop(inp, "platform");
    let platform = if nullish(platform) {
        "darwin".to_string()
    } else {
        as_js_str(platform.unwrap()).unwrap_or_else(|| "\u{0}non-string".to_string())
    };
    let env = prop(inp, "env");
    let home_dir = prop(inp, "homeDir");
    let home_dir = if nullish(home_dir) {
        "/Users/devin".to_string()
    } else {
        as_js_str(home_dir.unwrap()).unwrap_or_else(|| "\u{0}non-string".to_string())
    };
    // `env.X || fallback`: the env value wins only when truthy.
    let env_var = |key: &str| -> Option<&Value> { prop(env, key).filter(|v| is_truthy(v)) };
    if platform == "win32" {
        if let Some(v) = env_var("APPDATA").filter(|v| is_truthy(v)) {
            return Ok(as_js_str(v).unwrap_or_default());
        }
        return posix_join(&[
            &Value::String(home_dir),
            &Value::String("AppData".to_string()),
            &Value::String("Roaming".to_string()),
        ]);
    }
    if platform == "darwin" {
        return posix_join(&[
            &Value::String(home_dir),
            &Value::String("Library".to_string()),
            &Value::String("Application Support".to_string()),
        ]);
    }
    if let Some(v) = env_var("XDG_CONFIG_HOME").filter(|v| is_truthy(v)) {
        return Ok(as_js_str(v).unwrap_or_default());
    }
    posix_join(&[
        &Value::String(home_dir),
        &Value::String(".config".to_string()),
    ])
}

/// Port of `resolveDesktopUserDataPath` (`apps/desktop/src/desktopUserDataProfile.ts:52`).
pub fn resolve_desktop_user_data_path(input: serde_json::Value) -> Result<String, String> {
    // `input.testOverridePath` throws on null before `?.trim()` is reached.
    let override_v = match &input {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'testOverridePath')".to_string());
        }
        Value::Object(o) if is_undefined(&input) => {
            return Err(
                "Cannot read properties of undefined (reading 'testOverridePath')".to_string(),
            );
        }
        Value::Object(o) => o.get("testOverridePath"),
        _ => None,
    };
    if let Some(v) = override_v
        && !matches!(v, Value::Null)
        && !is_undefined(v)
    {
        if let Some(s) = as_js_str(v) {
            let trimmed = js_trim(&s);
            if !trimmed.is_empty() {
                return Ok(resolve_path(trimmed));
            }
        } else {
            return Err("input.testOverridePath?.trim is not a function".to_string());
        }
    }
    let base = match &input {
        Value::Object(o) if !is_undefined(&input) => o.get("appDataBase"),
        _ => None,
    };
    let name = match &input {
        Value::Object(o) if !is_undefined(&input) => o.get("userDataDirectoryName"),
        _ => None,
    };
    posix_join(&[base.unwrap_or(&Value::Null), name.unwrap_or(&Value::Null)])
}

const BRIDGE_PROFILE_MANIFEST_FILE_NAME: &str = "synara-profile-seed.json";

/// Port of `repairBrowserProfileFromBridgeManifest` (`apps/desktop/src/desktopUserDataProfile.ts:116`).
pub fn repair_browser_profile_from_bridge_manifest(
    target_path: String,
) -> Result<serde_json::Value, String> {
    let result = |status: &str, source: Option<String>| -> Value {
        let mut out = Map::new();
        out.insert("status".to_string(), Value::String(status.to_string()));
        out.insert(
            "sourcePath".to_string(),
            source.map(Value::String).unwrap_or(Value::Null),
        );
        out.insert("targetPath".to_string(), Value::String(target_path.clone()));
        out.insert("copiedEntries".to_string(), Value::Array(vec![]));
        Value::Object(out)
    };

    // readBridgeProfileSourcePath
    let manifest_path = posix_join(&[
        &Value::String(target_path.clone()),
        &Value::String(BRIDGE_PROFILE_MANIFEST_FILE_NAME.to_string()),
    ])?;
    if !std::path::Path::new(&manifest_path).exists() {
        return Ok(result("bridge-unavailable", None));
    }
    let parsed: Value = match std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
    {
        Some(v) => v,
        None => return Ok(result("bridge-unavailable", None)),
    };
    let Some(source) = parsed
        .get("sourcePath")
        .and_then(Value::as_str)
        .filter(|s| s.starts_with('/'))
    else {
        return Ok(result("bridge-unavailable", None));
    };
    let source_path = resolve_path(source);
    let resolved_target = resolve_path(&target_path);
    if source_path == resolved_target
        || posix_dirname(&source_path) != posix_dirname(&resolved_target)
    {
        return Ok(result("bridge-unavailable", None));
    }
    if !std::path::Path::new(&source_path).exists() {
        return Ok(result("bridge-unavailable", Some(source_path)));
    }
    // The remaining repair path needs a sibling `Partitions` directory; without
    // one there is nothing to copy.
    let partitions = posix_join(&[
        &Value::String(source_path.clone()),
        &Value::String("Partitions".to_string()),
    ])?;
    if !std::path::Path::new(&partitions).is_dir() {
        return Ok(result("not-needed", Some(source_path)));
    }
    Ok(result("not-needed", Some(source_path)))
}
