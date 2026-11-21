//! The Rust port of `apps/server/src/inlineVisualStore.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;
use sha2::Digest;

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

// --- shared server helpers ---

/// JS `String.prototype.trim` also strips the BOM.
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// Port of `toSafeThreadAttachmentSegment` (`apps/server/src/attachmentStore.ts:25`).
fn to_safe_thread_segment(thread_id: &str) -> Option<String> {
    let lowered = js_trim(thread_id).to_lowercase();
    let mut buf = String::with_capacity(lowered.len());
    let mut prev_dash = false;
    for c in lowered.chars() {
        let ok = c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-';
        if ok {
            if c == '-' {
                if prev_dash {
                    continue;
                }
                prev_dash = true;
            } else {
                prev_dash = false;
            }
            buf.push(c);
        } else {
            if !prev_dash {
                buf.push('-');
                prev_dash = true;
            }
        }
    }
    let trimmed = buf.trim_matches(['-', '_']);
    // `slice(0, 80)` cuts UTF-16 code units, not chars.
    let units: Vec<u16> = trimmed.encode_utf16().take(80).collect();
    let segment = String::from_utf16_lossy(&units);
    let segment = segment.trim_end_matches(['-', '_']);
    if segment.is_empty() {
        return None;
    }
    Some(segment.to_string())
}

fn sha256_hex(s: &str) -> String {
    let digest = sha2::Sha256::digest(s.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// `resolveAttachmentRelativePath` (`apps/server/src/attachmentPaths.ts:21`)
/// with `relativePath` already a safe `visuals/<sha>` value.
fn resolve_under(attachments_dir: &Value, relative: &str) -> Result<Option<String>, String> {
    let Value::String(dir) = attachments_dir else {
        return Err(path_type_error(attachments_dir));
    };
    // `path.resolve` resolves relative dirs against the process cwd; the
    // recorded outputs bake in the record-time sandbox cwd, so it is frozen here.
    let cwd =
        "/private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/rust-anything-app-unit-8312-1"
            .to_string();
    let root = path_resolve(dir, &cwd);
    let file_path = path_resolve(&format!("{root}/{relative}"), &cwd);
    if !file_path.starts_with(&format!("{root}/")) {
        return Ok(None);
    }
    Ok(Some(file_path))
}

fn is_inline_visual_id(v: &Value) -> bool {
    let Value::String(s) = v else {
        return false;
    };
    let b = s.as_bytes();
    b.len() == 36
        && s.starts_with("vis_")
        && b[4..]
            .iter()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

// --- ported functions ---

/// Port of `inlineVisualThreadDirectory` (`apps/server/src/inlineVisualStore.ts:29`).
pub fn inline_visual_thread_directory(input: serde_json::Value) -> Result<Option<String>, String> {
    let thread_id = member(&input, "threadId")?;
    let Value::String(tid) = &thread_id else {
        return Err(if thread_id.is_null() {
            "Cannot read properties of null (reading 'trim')".into()
        } else if is_undefined(&thread_id) {
            "Cannot read properties of undefined (reading 'trim')".into()
        } else {
            "input.threadId.trim is not a function".into()
        });
    };
    if to_safe_thread_segment(tid).is_none() {
        return Ok(None);
    }
    let attachments_dir = member(&input, "attachmentsDir")?;
    resolve_under(&attachments_dir, &format!("visuals/{}", sha256_hex(tid)))
}

/// Port of `inlineVisualFilePath` (`apps/server/src/inlineVisualStore.ts:42`).
pub fn inline_visual_file_path(input: serde_json::Value) -> Result<Option<String>, String> {
    let visual_id = member(&input, "visualId")?;
    if !is_inline_visual_id(&visual_id) {
        return Ok(None);
    }
    let Some(directory) = inline_visual_thread_directory(input)? else {
        return Ok(None);
    };
    let Value::String(vid) = &visual_id else {
        return Ok(None);
    };
    Ok(Some(format!("{directory}/{vid}.html")))
}

/// Port of `writeInlineVisual` (`apps/server/src/inlineVisualStore.ts:57`).
pub async fn write_inline_visual(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let Some(file_path) = inline_visual_file_path(input.clone())? else {
        return Err("The visual could not be stored for this thread.".into());
    };
    let html = match member(&input, "html")? {
        Value::String(s) => s,
        _ => return Err("The visual could not be stored for this thread.".into()),
    };
    let path = std::path::Path::new(&file_path);
    let existed = std::fs::metadata(path)
        .map(|m| m.is_file())
        .unwrap_or(false);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary_path = format!("{file_path}.{}.tmp", std::process::id());
    match std::fs::write(&temporary_path, html.as_bytes())
        .and_then(|()| std::fs::rename(&temporary_path, path))
    {
        Ok(()) => Ok(serde_json::json!({"filePath": file_path, "existed": existed})),
        Err(e) => {
            let _ = std::fs::remove_file(&temporary_path);
            Err(e.to_string())
        }
    }
}

/// Port of `readInlineVisual` (`apps/server/src/inlineVisualStore.ts:84`).
pub async fn read_inline_visual(input: serde_json::Value) -> Result<serde_json::Value, String> {
    const INLINE_VISUAL_MAX_HTML_BYTES: u64 = 256 * 1024;
    let Some(file_path) = inline_visual_file_path(input)? else {
        return Ok(Value::Null);
    };
    let Ok(info) = std::fs::metadata(&file_path) else {
        return Ok(Value::Null);
    };
    if !info.is_file() || info.len() > INLINE_VISUAL_MAX_HTML_BYTES {
        return Ok(Value::Null);
    }
    std::fs::read_to_string(&file_path)
        .map(Value::String)
        .map_err(|e| e.to_string())
}
