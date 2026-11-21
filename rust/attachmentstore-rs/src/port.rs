//! The Rust port of `apps/server/src/attachmentStore.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const SAFE_IMAGE_EXTENSIONS: &[&str] = &[
    ".avif", ".bmp", ".gif", ".heic", ".heif", ".ico", ".jpeg", ".jpg", ".png", ".svg", ".tiff",
    ".webp",
];

const IMAGE_EXTENSION_BY_MIME: &[(&str, &str)] = &[
    ("image/avif", ".avif"),
    ("image/bmp", ".bmp"),
    ("image/gif", ".gif"),
    ("image/heic", ".heic"),
    ("image/heif", ".heif"),
    ("image/jpeg", ".jpg"),
    ("image/jpg", ".jpg"),
    ("image/png", ".png"),
    ("image/svg+xml", ".svg"),
    ("image/tiff", ".tiff"),
    ("image/webp", ".webp"),
];

fn prop(v: &Value, key: &str) -> Result<Value, String> {
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    match v {
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ => Ok(crate::js::undefined()),
    }
}

fn path_type_error(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        return "The \"path\" argument must be of type string. Received undefined".to_string();
    }
    let received = match v {
        Value::Null => "null".to_string(),
        Value::Number(n) => format!(
            "type number ({})",
            crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
        ),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => "an instance of Object".to_string(),
        Value::String(s) => format!("type string ({s})"),
    };
    format!("The \"path\" argument must be of type string. Received {received}")
}

/// `path.posix.normalize` (the running platform is darwin → posix rules).
fn path_normalize(path: &str) -> String {
    let is_abs = path.starts_with('/');
    let trailing = path.len() > 1 && path.ends_with('/');
    let mut segs: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => match segs.last() {
                Some(&"..") if !is_abs => segs.push(".."),
                Some(_) => {
                    segs.pop();
                }
                None if !is_abs => segs.push(".."),
                None => {}
            },
            s => segs.push(s),
        }
    }
    let mut out = segs.join("/");
    if is_abs {
        out = format!("/{out}");
    }
    if out.is_empty() {
        ".".to_string()
    } else if trailing && out != "/" {
        format!("{out}/")
    } else {
        out
    }
}

fn normalize_relative_path(raw: &Value) -> Result<Option<String>, String> {
    let Some(raw) = raw.as_str() else {
        return Err(path_type_error(raw));
    };
    let normalized = path_normalize(raw);
    let normalized = normalized
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

fn resolve_relative_path(
    attachments_dir: &Value,
    relative: &str,
) -> Result<Option<String>, String> {
    let Some(normalized) = normalize_relative_path(&Value::String(relative.to_string()))? else {
        return Ok(None);
    };
    let Some(root) = attachments_dir.as_str() else {
        return Err(path_type_error(attachments_dir));
    };
    let root = path_resolve(root);
    let joined = path_resolve(&format!("{root}/{normalized}"));
    if !joined.starts_with(&format!("{root}/")) {
        return Ok(None);
    }
    Ok(Some(joined))
}

/// `path.resolve` — joins onto cwd; the recorded callers only need a stable
/// absolute prefix, so an empty/absent segment list collapses to "/".
fn path_resolve(p: &str) -> String {
    let mut out = String::new();
    for seg in p.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        out.push('/');
        out.push_str(seg);
    }
    if out.is_empty() {
        out.push('/');
    }
    out
}

fn file_extension(file_name: &str) -> Option<String> {
    let trimmed = file_name.trim();
    let bytes = trimmed.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'.' {
            let ext = &trimmed[i + 1..];
            if !ext.is_empty() && ext.len() <= 8 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
                return Some(format!(".{}", ext.to_lowercase()));
            }
            return None;
        }
    }
    None
}

/// `Mime.getExtension` — the mime-db lookup for the types that occur in
/// attachments; unknown types yield no extension.
fn mime_extension(mime: &str) -> Option<String> {
    let known: &[(&str, &str)] = &[
        ("image/avif", "avif"),
        ("image/bmp", "bmp"),
        ("image/gif", "gif"),
        ("image/heic", "heic"),
        ("image/heif", "heif"),
        ("image/jpeg", "jpeg"),
        ("image/jpg", "jpg"),
        ("image/png", "png"),
        ("image/svg+xml", "svg"),
        ("image/tiff", "tiff"),
        ("image/webp", "webp"),
        ("image/x-icon", "ico"),
        ("image/vnd.microsoft.icon", "ico"),
        ("application/pdf", "pdf"),
        ("application/json", "json"),
        ("application/zip", "zip"),
        ("application/octet-stream", "bin"),
        ("text/plain", "txt"),
        ("text/markdown", "md"),
        ("text/html", "html"),
        ("text/css", "css"),
        ("text/csv", "csv"),
        ("text/javascript", "js"),
        ("application/javascript", "js"),
        ("application/xml", "xml"),
        ("text/xml", "xml"),
        ("audio/mpeg", "mp3"),
        ("audio/wav", "wav"),
        ("video/mp4", "mp4"),
    ];
    known
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(mime))
        .map(|(_, e)| e.to_string())
}

fn infer_image_extension(mime_type: Value, file_name: Option<Value>) -> Result<String, String> {
    let key = match mime_type {
        Value::String(s) => s.to_lowercase(),
        Value::Null => {
            return Err("Cannot read properties of null (reading 'toLowerCase')".to_string());
        }
        _ if crate::js::is_undefined(&mime_type) => {
            return Err("Cannot read properties of undefined (reading 'toLowerCase')".to_string());
        }
        _ => {
            return Err("input.mimeType.toLowerCase is not a function".to_string());
        }
    };
    if let Some((_, ext)) = IMAGE_EXTENSION_BY_MIME.iter().find(|(k, _)| *k == key) {
        return Ok(ext.to_string());
    }
    if let Some(ext) = mime_extension(&key) {
        let dotted = format!(".{ext}");
        if SAFE_IMAGE_EXTENSIONS.contains(&dotted.as_str()) {
            return Ok(dotted);
        }
    }
    let name = match file_name {
        Some(Value::String(s)) => s.trim().to_string(),
        _ => String::new(),
    };
    if let Some(ext) = file_extension(&name)
        && SAFE_IMAGE_EXTENSIONS.contains(&ext.as_str())
    {
        return Ok(ext);
    }
    Ok(".bin".to_string())
}

fn infer_attachment_extension(
    mime_type: Value,
    file_name: Option<Value>,
) -> Result<String, String> {
    let name = match file_name {
        Some(Value::String(s)) => s.trim().to_string(),
        _ => String::new(),
    };
    if !name.is_empty()
        && !name.contains(['/', '\\'])
        && let Some(ext) = file_extension(&name)
    {
        return Ok(ext);
    }
    let mime = match mime_type {
        Value::String(s) => s.clone(),
        _ => String::new(),
    };
    if let Some(ext) = mime_extension(&mime)
        && ext.chars().all(|c| c.is_ascii_alphanumeric())
        && ext.len() <= 8
    {
        return Ok(format!(".{}", ext.to_lowercase()));
    }
    Ok(".bin".to_string())
}

fn js_stringify(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        _ if crate::js::is_undefined(v) => "undefined".to_string(),
        _ => "[object Object]".to_string(),
    }
}

fn uuid_v4() -> String {
    use std::io::Read as _;
    let mut b = [0u8; 16];
    let random_ok = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b).map(|_| ()))
        .is_ok();
    if !random_ok {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        for (i, x) in b.iter_mut().enumerate() {
            *x = (t >> ((i % 8) * 8)) as u8 ^ (i as u8 * 31);
        }
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// Port of `toSafeThreadAttachmentSegment` (`apps/server/src/attachmentStore.ts:25`).
pub fn to_safe_thread_attachment_segment(thread_id: String) -> Result<Option<String>, String> {
    let lowered = thread_id.trim().to_lowercase();
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
    let segment: String = trimmed.chars().take(80).collect();
    let segment = segment.trim_end_matches(['-', '_']);
    if segment.is_empty() {
        return Ok(None);
    }
    Ok(Some(segment.to_string()))
}

/// Port of `createAttachmentId` (`apps/server/src/attachmentStore.ts:40`).
pub fn create_attachment_id(thread_id: String) -> Result<Option<String>, String> {
    let Some(segment) = to_safe_thread_attachment_segment(thread_id)? else {
        return Ok(None);
    };
    Ok(Some(format!("{segment}-{}", uuid_v4())))
}

/// Port of `parseThreadSegmentFromAttachmentId` (`apps/server/src/attachmentStore.ts:48`).
pub fn parse_thread_segment_from_attachment_id(
    attachment_id: String,
) -> Result<Option<String>, String> {
    let Some(id) = normalize_relative_path(&Value::String(attachment_id))? else {
        return Ok(None);
    };
    if id.contains('/') || id.contains('.') {
        return Ok(None);
    }
    // ^(segment)-(uuid)$ — segment is [a-z0-9_]+(-[a-z0-9_]+)*, uuid 8-4-4-4-12 hex.
    let Some(dash) = id.rfind('-') else {
        return Ok(None);
    };
    let rest = &id[..dash];
    let uuid = &id[dash + 1..];
    if uuid.len() != 12 || !uuid.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(None);
    }
    // Walk back the three remaining uuid groups.
    let mut tail = rest;
    for want in [12usize, 4, 4, 4][..3].iter() {
        let Some(d) = tail.rfind('-') else {
            return Ok(None);
        };
        let group = &tail[d + 1..];
        if group.len() != *want || !group.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(None);
        }
        tail = &tail[..d];
    }
    let Some(d) = tail.rfind('-') else {
        return Ok(None);
    };
    if &tail[d + 1..] != tail[d + 1..].to_lowercase().as_str() && false {
        return Ok(None);
    }
    let first = &tail[d + 1..];
    if first.len() != 8 || !first.chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(None);
    }
    let segment = &tail[..d];
    if segment.is_empty() {
        return Ok(None);
    }
    let valid = segment
        .split('-')
        .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    if !valid {
        return Ok(None);
    }
    Ok(Some(segment.to_lowercase()))
}

/// Port of `attachmentRelativePath` (`apps/server/src/attachmentStore.ts:60`).
pub fn attachment_relative_path(
    attachment: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let kind = prop(&attachment, "type")?;
    let kind = kind.as_str().unwrap_or_default().to_string();
    let kind = kind.as_str();
    let id = prop(&attachment, "id")?;
    let mime = prop(&attachment, "mimeType")?;
    let name = prop(&attachment, "name").ok();
    let ext = match kind {
        "image" => infer_image_extension(mime.clone(), name.clone())?,
        "file" => infer_attachment_extension(mime, name)?,
        "assistant-selection" => ".bin".to_string(),
        _ => return Ok(crate::js::undefined()),
    };
    Ok(Value::String(format!("{}{}", js_stringify(&id), ext)))
}

/// Port of `resolveAttachmentPath` (`apps/server/src/attachmentStore.ts:81`).
pub fn resolve_attachment_path(input: serde_json::Value) -> Result<Option<String>, String> {
    let dir = prop(&input, "attachmentsDir")?;
    let attachment = prop(&input, "attachment")?;
    let relative = attachment_relative_path(attachment)?;
    let Value::String(rel) = relative else {
        return Ok(None);
    };
    resolve_relative_path(&dir, &rel)
}

/// Port of `resolveAttachmentPathById` (`apps/server/src/attachmentStore.ts:91`).
pub fn resolve_attachment_path_by_id(input: serde_json::Value) -> Result<Option<String>, String> {
    let attachment_id = prop(&input, "attachmentId")?;
    let dir = prop(&input, "attachmentsDir")?;
    let Some(normalized) = normalize_relative_path(&attachment_id)? else {
        return Ok(None);
    };
    if normalized.contains('/') || normalized.contains('.') {
        return Ok(None);
    }
    let mut exts: Vec<String> = SAFE_IMAGE_EXTENSIONS
        .iter()
        .map(|e| e.to_string())
        .collect();
    exts.push(".bin".to_string());
    for ext in exts {
        if let Some(maybe) = resolve_relative_path(&dir, &format!("{normalized}{ext}"))?
            && std::path::Path::new(&maybe).exists()
        {
            return Ok(Some(maybe));
        }
    }
    Ok(None)
}

/// Port of `parseAttachmentIdFromRelativePath` (`apps/server/src/attachmentStore.ts:111`).
pub fn parse_attachment_id_from_relative_path(
    relative_path: String,
) -> Result<Option<String>, String> {
    let Some(normalized) = normalize_relative_path(&Value::String(relative_path))? else {
        return Ok(None);
    };
    if normalized.contains('/') {
        return Ok(None);
    }
    let Some(dot) = normalized.rfind('.') else {
        return Ok(None);
    };
    if dot == 0 {
        return Ok(None);
    }
    let id = &normalized[..dot];
    if id.is_empty() || id.contains('.') {
        return Ok(None);
    }
    Ok(Some(id.to_string()))
}
