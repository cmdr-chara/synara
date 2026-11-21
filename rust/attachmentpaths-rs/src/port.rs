//! The Rust port of `apps/server/src/attachmentPaths.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

/// Node's `ERR_INVALID_ARG_TYPE` message for `path` arguments.
fn path_type_error(v: &Value) -> String {
    let received = match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Array(_) => "an instance of Array".to_string(),
        _ => match crate::js::as_number(v) {
            Some(n) => format!("type number ({})", crate::js::number_to_string(n)),
            None if is_undefined(v) => "undefined".to_string(),
            None => "an instance of Object".to_string(),
        },
    };
    format!("The \"path\" argument must be of type string. Received {received}")
}

// --- node:path posix semantics (normalize/resolve/join only) ---

/// `normalizeString`: resolve `.`/`..` segments and collapse empty ones.
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

fn path_normalize(path: &str) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let absolute = path.starts_with('/');
    let mut out = normalize_string(path, !absolute);
    if out.is_empty() {
        return if absolute {
            "/".to_string()
        } else {
            ".".to_string()
        };
    }
    if path.ends_with('/') {
        out.push('/');
    }
    if absolute {
        out.insert(0, '/');
    }
    out
}

fn path_resolve(segment: &str, cwd: &str) -> String {
    let joined = if segment.is_empty() {
        cwd.to_string()
    } else if segment.starts_with('/') {
        segment.to_string()
    } else {
        format!("{cwd}/{segment}")
    };
    let out = normalize_string(&joined, false);
    if out.is_empty() {
        return "/".to_string();
    }
    if joined.starts_with('/') || !cwd.is_empty() {
        format!("/{out}")
    } else {
        out
    }
}

// --- ported functions ---

fn normalize_relative(v: &Value) -> Result<Option<String>, String> {
    let Value::String(raw) = v else {
        return Err(path_type_error(v));
    };
    // Order follows the source: normalize → strip leading slashes → backslash→slash.
    let normalized = path_normalize(raw)
        .trim_start_matches(['/', '\\'])
        .replace('\\', "/");
    if normalized.is_empty()
        || normalized == ".."
        || normalized.starts_with("../")
        || normalized.contains('\0')
    {
        return Ok(None);
    }
    Ok(Some(normalized))
}

/// Port of `normalizeAttachmentRelativePath` (`apps/server/src/attachmentPaths.ts:5`).
pub fn normalize_attachment_relative_path(
    raw_relative_path: String,
) -> Result<Option<String>, String> {
    normalize_relative(&Value::String(raw_relative_path))
}

/// Port of `resolveAttachmentRelativePath` (`apps/server/src/attachmentPaths.ts:21`).
pub fn resolve_attachment_relative_path(
    input: serde_json::Value,
) -> Result<Option<String>, String> {
    let relative = member(&input, "relativePath")?;
    let Some(normalized_relative_path) = normalize_relative(&relative)? else {
        return Ok(None);
    };

    let dir = member(&input, "attachmentsDir")?;
    let Value::String(dir_str) = &dir else {
        return Err(path_type_error(&dir));
    };
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    let attachments_root = path_resolve(dir_str, &cwd);
    // `path.join(root, rel)` then `path.resolve(...)`: both reduce to normalize here.
    let file_path = path_resolve(
        &format!("{attachments_root}/{normalized_relative_path}"),
        &cwd,
    );
    if !file_path.starts_with(&format!("{attachments_root}/")) {
        return Ok(None);
    }
    Ok(Some(file_path))
}
