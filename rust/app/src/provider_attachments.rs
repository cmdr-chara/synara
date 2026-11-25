//! Shared conversion of persisted chat attachments into provider-native prompt
//! inputs. The resolver deliberately stays inside `<stateDir>/attachments` and
//! supports both the legacy flat filename layout and managed relative object
//! paths when those are present in the serialized attachment metadata.

use rusqlite::params;
use serde_json::{json, Value};

use crate::orchestration_engine::open_state_rw;
use crate::todo::{state_dir, WsErr};

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("PROVIDER"))
}

fn normalize_relative_path(raw: &str) -> Option<String> {
    let normalized = raw.replace('\\', "/");
    let normalized = normalized.trim_start_matches('/');
    if normalized.is_empty()
        || normalized == ".."
        || normalized.starts_with("../")
        || normalized.contains('\0')
    {
        return None;
    }
    Some(normalized.to_string())
}

fn extension_for(mime_type: &str, name: &str) -> String {
    if let Some(ext) = std::path::Path::new(name)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty() && ext.len() <= 16)
    {
        return format!(".{}", ext.to_ascii_lowercase());
    }
    match mime_type.to_ascii_lowercase().as_str() {
        "image/png" => ".png".to_string(),
        "image/jpeg" | "image/jpg" => ".jpg".to_string(),
        "image/gif" => ".gif".to_string(),
        "image/webp" => ".webp".to_string(),
        "image/svg+xml" => ".svg".to_string(),
        "application/pdf" => ".pdf".to_string(),
        "text/plain" => ".txt".to_string(),
        "text/markdown" => ".md".to_string(),
        "application/json" => ".json".to_string(),
        _ => ".bin".to_string(),
    }
}

fn attachment_relative_path(attachment: &Value) -> Option<String> {
    if let Some(relative) = attachment
        .get("relativePath")
        .and_then(Value::as_str)
        .and_then(normalize_relative_path)
    {
        return Some(relative);
    }
    let id = attachment.get("id").and_then(Value::as_str)?;
    if id.contains('/') || id.contains('\\') || id.contains("..") || id.contains('.') {
        return None;
    }
    let mime = attachment.get("mimeType").and_then(Value::as_str).unwrap_or("");
    let name = attachment.get("name").and_then(Value::as_str).unwrap_or("");
    Some(format!("{id}{}", extension_for(mime, name)))
}

pub fn resolve_attachment_path(attachment: &Value) -> Option<std::path::PathBuf> {
    let relative = attachment_relative_path(attachment)?;
    let root = std::path::PathBuf::from(state_dir()).join("attachments");
    let path = root.join(&relative);
    let root = root.canonicalize().unwrap_or(root);
    let candidate = path.canonicalize().unwrap_or(path);
    if candidate.starts_with(&root) {
        Some(candidate)
    } else {
        None
    }
}

pub fn message_attachments(thread_id: &str, message_id: &str) -> Result<Vec<Value>, WsErr> {
    let conn = open_state_rw()?;
    let raw: Option<String> = conn
        .query_row(
            "SELECT attachments_json FROM projection_thread_messages WHERE thread_id=?1 AND message_id=?2",
            params![thread_id, message_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    Ok(raw
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default())
}

fn format_bytes(bytes: i64) -> String {
    if bytes >= 1_000_000 {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.1} kB", bytes as f64 / 1_000.0)
    } else {
        format!("{bytes} B")
    }
}

fn quote_prompt_value(value: &str) -> String {
    if value.chars().any(|ch| ch.is_whitespace() || ch == '"' || ch == '\'') {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

/// Mirror upstream's `<attached_files>` block. Image attachments are included
/// as disk-backed files too unless the caller filters them, because providers
/// can still inspect them with tools even when they also receive native image
/// input.
pub fn append_file_attachments_prompt_block(
    text: &str,
    attachments: &[Value],
    include_images: bool,
) -> String {
    let mut lines = Vec::new();
    for attachment in attachments {
        let kind = attachment.get("type").and_then(Value::as_str).unwrap_or("");
        if kind != "file" && !(include_images && kind == "image") {
            continue;
        }
        let Some(path) = resolve_attachment_path(attachment) else {
            continue;
        };
        if !path.is_file() {
            continue;
        }
        let name = attachment.get("name").and_then(Value::as_str).unwrap_or("attachment");
        let mime = attachment.get("mimeType").and_then(Value::as_str).unwrap_or("application/octet-stream");
        let size = attachment.get("sizeBytes").and_then(Value::as_i64).unwrap_or(0);
        lines.push(format!(
            "- {} - {} - {} - {}",
            quote_prompt_value(name),
            mime,
            format_bytes(size),
            path.display()
        ));
    }
    if lines.is_empty() {
        return text.to_string();
    }
    let block = format!(
        "<attached_files>\nThe user attached the following file(s), saved on disk. Read/extract them with your tools as needed; do not assume their contents.\n{}\n</attached_files>",
        lines.join("\n")
    );
    if text.is_empty() {
        block
    } else {
        format!("{text}\n\n{block}")
    }
}


fn file_url(path: &std::path::Path) -> String {
    let mut out = "file://".to_string();
    for byte in path.to_string_lossy().as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// OpenCode native file parts for image attachments. Regular files stay in
/// the prompt block so the model chooses when to read them; images are also
/// native inputs because OpenCode can consume them directly.
pub fn opencode_image_file_parts(attachments: &[Value]) -> Vec<Value> {
    attachments
        .iter()
        .filter(|attachment| attachment.get("type").and_then(Value::as_str) == Some("image"))
        .filter_map(|attachment| {
            let path = resolve_attachment_path(attachment)?;
            if !path.is_file() {
                return None;
            }
            Some(json!({
                "uri": file_url(&path),
                "name": attachment.get("name").and_then(Value::as_str).unwrap_or("image"),
                "mime": attachment.get("mimeType").and_then(Value::as_str).unwrap_or("image/png"),
                "filename": attachment.get("name").and_then(Value::as_str).unwrap_or("image"),
                "url": file_url(&path),
            }))
        })
        .collect()
}

fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { ALPHABET[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { ALPHABET[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn image_blocks(attachments: &[Value]) -> Result<Vec<Value>, WsErr> {
    let mut images = Vec::new();
    for attachment in attachments {
        if attachment.get("type").and_then(Value::as_str) != Some("image") {
            continue;
        }
        let id = attachment.get("id").and_then(Value::as_str).unwrap_or("unknown");
        let path = resolve_attachment_path(attachment)
            .ok_or_else(|| err(format!("Invalid attachment id '{id}'.")))?;
        let bytes = std::fs::read(&path)
            .map_err(|e| err(format!("Failed to read attachment '{id}': {e}")))?;
        images.push(json!({
            "type": "image",
            "data": base64_encode(&bytes),
            "mimeType": attachment.get("mimeType").and_then(Value::as_str).unwrap_or("image/png"),
        }));
    }
    Ok(images)
}
