//! The Rust port of `scripts/update-release-package-versions.ts`.
//!
//! Node `path.resolve` and `fs` error strings are reproduced with POSIX semantics;
//! `JSON.stringify(obj, null, 2)` is reproduced with JS number printing and key order.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::io::{Read, Write};

const RELEASE_PACKAGE_FILES: [&str; 4] = [
    "apps/server/package.json",
    "apps/desktop/package.json",
    "apps/web/package.json",
    "packages/contracts/package.json",
];

// --- node:path posix semantics ---

/// `normalizeString`: collapse `//` and `.`, resolve `..` segments.
fn normalize_string(path: &str, allow_above_root: bool) -> String {
    let mut res: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." if res.last().is_some_and(|&s| s != "..") => {
                res.pop();
            }
            ".." if allow_above_root => res.push(".."),
            ".." => {}
            _ => res.push(seg),
        }
    }
    res.join("/")
}

/// `path.resolve(base, segment)` with `base` already absolute (or the process cwd).
fn path_resolve(segment: &str, cwd: &str) -> String {
    let joined = if segment.is_empty() || segment.starts_with('/') {
        if segment.is_empty() { cwd } else { segment }.to_string()
    } else {
        format!("{cwd}/{segment}")
    };
    let out = normalize_string(&joined, false);
    format!("/{out}")
}

// --- JS value views over the wire markers ---

/// A one-key object `{"$k": v}` stands for a non-JSON JS value, as `index.cjs` encodes it.
fn is_marker(v: &Value, key: &str) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == 1 && o.contains_key(key))
}

/// A JS string's view of `v`: `String` plus the `{"$utf16": ...}` marker.
fn as_js_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => v
            .as_object()
            .filter(|o| o.len() == 1)
            .and_then(|o| o.get("$utf16"))
            .and_then(Value::as_array)
            .map(|u| {
                String::from_utf16_lossy(
                    &u.iter()
                        .filter_map(|x| x.as_u64().map(|n| n as u16))
                        .collect::<Vec<u16>>(),
                )
            }),
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

/// Node `ERR_INVALID_ARG_TYPE` for `path.resolve`'s first argument.
fn resolve_type_error(v: &Value) -> String {
    let received = match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Number(n) => {
            format!("type number ({})", num_str(n.as_f64().unwrap_or(f64::NAN)))
        }
        Value::Object(o) if o.len() == 1 => match o.keys().next().map(String::as_str) {
            Some("$num") => format!(
                "type number ({})",
                num_str(as_js_num(v).unwrap_or(f64::NAN))
            ),
            Some("$bigint") => format!(
                "type bigint ({}n)",
                o["$bigint"].as_str().unwrap_or_default()
            ),
            Some("$symbol") => format!(
                "type symbol (Symbol({}))",
                o["$symbol"].as_str().unwrap_or_default()
            ),
            Some("$undefined") => "undefined".to_string(),
            Some(k) => format!(
                "an instance of {}",
                match k {
                    "$date" => "Date",
                    "$map" => "Map",
                    "$set" => "Set",
                    "$bytes" => "Uint8Array",
                    "$buffer" => "Buffer",
                    "$regexp" => "RegExp",
                    _ => "Object",
                }
            ),
            _ => "an instance of Object".to_string(),
        },
        _ => "an instance of Object".to_string(),
    };
    format!("The \"paths[0]\" argument must be of type string. Received {received}")
}

/// `String(n)`; `"-0"` survives here, matching `inspect` in `Received type number (-0)`.
fn num_str(n: f64) -> String {
    if n == 0.0 && n.is_sign_negative() {
        "-0".to_string()
    } else {
        crate::js::number_to_string(n)
    }
}

// --- node:fs error strings ---

fn io_error(e: &std::io::Error, op: &str, path: &str) -> String {
    let (code, msg) = match e.raw_os_error() {
        Some(1) => ("EPERM", "operation not permitted"),
        Some(2) => ("ENOENT", "no such file or directory"),
        Some(5) => ("EIO", "input/output error"),
        Some(13) => ("EACCES", "permission denied"),
        Some(20) => ("ENOTDIR", "not a directory"),
        Some(21) => ("EISDIR", "illegal operation on a directory"),
        Some(28) => ("ENOSPC", "no space left on device"),
        Some(30) => ("EROFS", "read-only file system"),
        Some(36) | Some(63) => ("ENAMETOOLONG", "name too long"),
        Some(40) | Some(62) => ("ELOOP", "too many symbolic links encountered"),
        other => {
            let detail = e.to_string();
            let code = other.map_or_else(|| "ERR".to_string(), |n| format!("E{n}"));
            return format!(
                "{code}: {}, {op} '{path}'",
                detail.split(" (os error").next().unwrap_or(&detail)
            );
        }
    };
    format!("{code}: {msg}, {op} '{path}'")
}

/// `readFileSync(path, "utf8")`: lossy utf8 and open/read phase errors like Node's.
fn read_file_utf8(path: &str) -> Result<String, String> {
    let mut file = std::fs::File::open(path).map_err(|e| io_error(&e, "open", path))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|e| io_error(&e, "read", path))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn write_file(path: &str, text: &str) -> Result<(), String> {
    let mut file = std::fs::File::create(path).map_err(|e| io_error(&e, "open", path))?;
    file.write_all(text.as_bytes())
        .map_err(|e| io_error(&e, "write", path))
}

// --- JSON.parse / JSON.stringify ---

/// `JSON.parse`: every number is a double, and non-finite results stringify as `null`.
fn parse_json(text: &str) -> Result<Value, String> {
    let mut v: Value = serde_json::from_str(text).map_err(|e| format!("SyntaxError: {e}"))?;
    let mut stack = vec![&mut v];
    while let Some(v) = stack.pop() {
        match v {
            Value::Number(n) => *v = Value::from(n.as_f64().unwrap_or(f64::NAN)),
            Value::Array(items) => stack.extend(items.iter_mut()),
            Value::Object(o) => stack.extend(o.values_mut()),
            _ => {}
        }
    }
    Ok(v)
}

fn write_json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `JSON.stringify(v, null, 2)`: two-space indent, `"k": v`, JS number printing.
fn write_json(v: &Value, depth: usize, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            out.push_str(&crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)))
        }
        Value::String(s) => write_json_string(s, out),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Object(o) if o.is_empty() => out.push_str("{}"),
        Value::Array(items) => {
            out.push_str("[\n");
            for (k, it) in items.iter().enumerate() {
                if k > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&"  ".repeat(depth + 1));
                write_json(it, depth + 1, out);
            }
            out.push_str(&format!("\n{}]", "  ".repeat(depth)));
        }
        Value::Object(o) => {
            out.push_str("{\n");
            for (k, (key, val)) in o.iter().enumerate() {
                if k > 0 {
                    out.push_str(",\n");
                }
                out.push_str(&"  ".repeat(depth + 1));
                write_json_string(key, out);
                out.push_str(": ");
                write_json(val, depth + 1, out);
            }
            out.push_str(&format!("\n{}}}", "  ".repeat(depth)));
        }
    }
}

// --- the export ---

/// Port of `updateReleasePackageVersions` (`scripts/update-release-package-versions.ts:21`).
pub fn update_release_package_versions(
    version: String,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // `options.rootDir ?? process.cwd()`; non-objects have no `rootDir`, null throws.
    let root_dir_value = match &options {
        Value::Null => {
            return Err("TypeError: Cannot read properties of null (reading 'rootDir')".into());
        }
        Value::Object(o) => o.get("rootDir"),
        _ => None,
    };
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "/".to_string());
    let root_dir = match root_dir_value {
        None | Some(Value::Null) => cwd.clone(),
        Some(v) if is_marker(v, "$undefined") => cwd.clone(),
        Some(v) => match as_js_str(v) {
            Some(s) => path_resolve(&s, &cwd),
            None => return Err(format!("TypeError: {}", resolve_type_error(v))),
        },
    };

    let mut changed = false;
    for relative in RELEASE_PACKAGE_FILES {
        let file_path = path_resolve(relative, &root_dir);
        let mut package_json = parse_json(&read_file_utf8(&file_path)?)?;
        let current = match &package_json {
            Value::Null => {
                return Err("TypeError: Cannot read properties of null (reading 'version')".into());
            }
            Value::Object(o) => o.get("version"),
            _ => None,
        };
        if matches!(current, Some(Value::String(s)) if s == &version) {
            continue;
        }
        match &mut package_json {
            Value::Object(o) => {
                o.insert("version".to_string(), Value::String(version.clone()));
            }
            // `x.version = v` on an array sticks but never serializes; null returned above.
            Value::Array(_) | Value::Null => {}
            Value::Bool(b) => {
                return Err(format!(
                    "TypeError: Cannot create property 'version' on boolean '{b}'"
                ));
            }
            Value::Number(n) => {
                return Err(format!(
                    "TypeError: Cannot create property 'version' on number '{}'",
                    crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
                ));
            }
            Value::String(s) => {
                return Err(format!(
                    "TypeError: Cannot create property 'version' on string '{s}'"
                ));
            }
        }
        let mut rendered = String::new();
        write_json(&package_json, 0, &mut rendered);
        rendered.push('\n');
        write_file(&file_path, &rendered)?;
        changed = true;
    }
    Ok(json!({ "changed": changed }))
}
