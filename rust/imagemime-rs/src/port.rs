//! The Rust port of `apps/server/src/imageMime.ts`.

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

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn image_extension_by_mime_type(mime: &str) -> Option<&'static str> {
    Some(match mime {
        "image/avif" => ".avif",
        "image/bmp" => ".bmp",
        "image/gif" => ".gif",
        "image/heic" => ".heic",
        "image/heif" => ".heif",
        "image/jpeg" | "image/jpg" => ".jpg",
        "image/png" => ".png",
        "image/svg+xml" => ".svg",
        "image/tiff" => ".tiff",
        "image/webp" => ".webp",
        _ => return None,
    })
}

fn is_safe_image_file_extension(ext: &str) -> bool {
    matches!(
        ext,
        ".avif"
            | ".bmp"
            | ".gif"
            | ".heic"
            | ".heif"
            | ".ico"
            | ".jpeg"
            | ".jpg"
            | ".png"
            | ".svg"
            | ".tiff"
            | ".webp"
    )
}

/// Minimal mime-db lookup mirroring `Mime.getExtension` from the `mime` package for the
/// types relevant here. Returns `Some("")` for an empty input string (the JS returns `""`,
/// not null) and `None` for non-string input or unknown types.
fn mime_get_extension(mime_type: &Value) -> Option<String> {
    let raw = mime_type.as_str()?;
    if raw.is_empty() {
        return Some(String::new());
    }
    let type_part = raw.split(';').next().unwrap_or(raw).trim().to_lowercase();
    let ext = match type_part.as_str() {
        "image/jpeg" | "image/pjpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/avif" => "avif",
        "image/bmp" | "image/x-ms-bmp" => "bmp",
        "image/svg+xml" => "svg",
        "image/tiff" => "tiff",
        "image/heic" => "heic",
        "image/heif" | "image/heif-sequence" => "heif",
        "image/x-icon" | "image/vnd.microsoft.icon" => "ico",
        "image/heic-sequence" => "heics",
        "text/plain" => "txt",
        "text/html" => "html",
        "text/css" => "css",
        "text/csv" => "csv",
        "text/markdown" => "md",
        "application/json" => "json",
        "application/pdf" => "pdf",
        "application/zip" => "zip",
        "application/xml" | "text/xml" => "xml",
        "application/octet-stream" => "bin",
        _ => return None,
    };
    Some(ext.to_string())
}

fn file_name_extension(file_name: &str) -> Option<String> {
    let dot = file_name.rfind('.')?;
    let ext = &file_name[dot + 1..];
    if ext.is_empty() || ext.len() > 8 || !ext.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    Some(ext.to_lowercase())
}

/// Port of `inferImageExtension` (`apps/server/src/imageMime.ts:32`).
pub fn infer_image_extension(input: serde_json::Value) -> Result<String, String> {
    let mime = member(&input, "mimeType")?;
    let Value::String(mime_str) = &mime else {
        if is_undefined(&mime) {
            return Err("Cannot read properties of undefined (reading 'toLowerCase')".to_string());
        }
        return Err("input.mimeType.toLowerCase is not a function".to_string());
    };
    let key = mime_str.to_lowercase();
    if let Some(from_mime) = image_extension_by_mime_type(&key) {
        return Ok(from_mime.to_string());
    }

    if let Some(from_mime_extension) = mime_get_extension(&mime)
        && !from_mime_extension.is_empty()
        && is_safe_image_file_extension(&format!(".{from_mime_extension}"))
    {
        return Ok(format!(".{from_mime_extension}"));
    }

    let file_name = member(&input, "fileName")?;
    let file_name = match &file_name {
        v if v.is_null() || is_undefined(v) => String::new(),
        Value::String(s) => js_trim(s).to_string(),
        _ => return Err("input.fileName.trim is not a function".to_string()),
    };
    if let Some(ext) = file_name_extension(&file_name) {
        let dotted = format!(".{ext}");
        if is_safe_image_file_extension(&dotted) {
            return Ok(dotted);
        }
    }
    Ok(".bin".to_string())
}

/// Port of `inferAttachmentExtension` (`apps/server/src/imageMime.ts:56`).
pub fn infer_attachment_extension(input: serde_json::Value) -> Result<String, String> {
    let file_name = member(&input, "fileName")?;
    let file_name = match &file_name {
        v if v.is_null() || is_undefined(v) => String::new(),
        Value::String(s) => js_trim(s).to_string(),
        _ => return Err("input.fileName.trim is not a function".to_string()),
    };
    if !file_name.is_empty() && !file_name.contains(['/', '\\']) {
        // `/^.+\.([a-z0-9]{1,8})$/i`: `.+` needs >= 1 non-line-terminator char before
        // the final dot.
        if let Some(dot) = file_name.rfind('.') {
            let ext = &file_name[dot + 1..];
            let prefix_ok = dot >= 1
                && file_name[..dot]
                    .chars()
                    .all(|c| !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}'));
            if prefix_ok
                && !ext.is_empty()
                && ext.len() <= 8
                && ext.bytes().all(|b| b.is_ascii_alphanumeric())
            {
                return Ok(format!(".{}", ext.to_lowercase()));
            }
        }
    }

    let mime = member(&input, "mimeType")?;
    if let Some(from_mime_extension) = mime_get_extension(&mime)
        && !from_mime_extension.is_empty()
        && from_mime_extension.len() <= 8
        && from_mime_extension
            .bytes()
            .all(|b| b.is_ascii_alphanumeric())
    {
        return Ok(format!(".{}", from_mime_extension.to_lowercase()));
    }
    Ok(".bin".to_string())
}
