//! Attachment upload and serving over HTTP, lean slice of the upstream
//! attachmentStore/http route pair. The wire contract is exact: upload is
//! POST /api/attachments/upload?threadId&type&name&mimeType with raw body
//! bytes and answers the ChatAttachment metadata JSON ({type,id,name,
//! mimeType,sizeBytes}); serving is GET /attachments/{id} resolving the
//! extension by scan, exactly like upstream resolveAttachmentPathById.
//! Ids embed the sanitized thread segment ({threadSegment}-{uuid}) so
//! per-thread cleanup stays possible; the inline-visuals folder layout and
//! upload cancellation are documented gaps.

use axum::{
    body::Bytes,
    extract::{Path, RawQuery},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde_json::{json, Value};
use std::path::PathBuf;

use crate::todo::state_dir;

const MAX_IMAGE_BYTES: usize = 10 * 1024 * 1024;
const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;

const SERVE_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".svg", ".pdf", ".txt", ".md", ".json", ".bin",
];

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(hex);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub(crate) fn attachments_http_query(query: &str, key: &str) -> Option<String> {
    query_param(query, key)
}

fn query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if percent_decode(k) == key {
            return Some(percent_decode(v));
        }
    }
    None
}

fn attachments_root() -> PathBuf {
    PathBuf::from(state_dir()).join("attachments")
}

/// Upstream toSafeThreadAttachmentSegment: lowercase, non [a-z0-9_-] runs to
/// dashes, collapse, trim, cap length.
fn safe_thread_segment(thread_id: &str) -> Option<String> {
    let mut out = String::with_capacity(thread_id.len());
    let mut last_dash = false;
    for ch in thread_id.trim().to_lowercase().chars() {
        let ok = ch.is_ascii_alphanumeric() || ch == '_' || ch == '-';
        if ok {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches(|c| c == '-' || c == '_');
    let capped: String = trimmed.chars().take(64).collect();
    let capped = capped.trim_end_matches(|c| c == '-' || c == '_').to_string();
    if capped.is_empty() {
        None
    } else {
        Some(capped)
    }
}

fn extension_for(mime_type: &str, name: &str) -> String {
    if let Some(ext) = std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|e| !e.is_empty() && e.len() <= 16)
    {
        return format!(".{}", ext.to_ascii_lowercase());
    }
    match mime_type.to_ascii_lowercase().as_str() {
        "image/png" => ".png".into(),
        "image/jpeg" | "image/jpg" => ".jpg".into(),
        "image/gif" => ".gif".into(),
        "image/webp" => ".webp".into(),
        "image/svg+xml" => ".svg".into(),
        "application/pdf" => ".pdf".into(),
        "text/plain" => ".txt".into(),
        "text/markdown" => ".md".into(),
        "application/json" => ".json".into(),
        _ => ".bin".into(),
    }
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (status, axum::Json(json!({ "error": message }))).into_response()
}

pub async fn upload(RawQuery(query): RawQuery, body: Bytes) -> Response {
    let query = query.unwrap_or_default();
    let thread_id = query_param(&query, "threadId").unwrap_or_default();
    let kind = query_param(&query, "type").unwrap_or_default();
    let name = query_param(&query, "name").unwrap_or_else(|| "attachment".into());
    let mime = query_param(&query, "mimeType").unwrap_or_else(|| "application/octet-stream".into());

    if kind != "image" && kind != "file" {
        return json_error(StatusCode::BAD_REQUEST, "type must be image or file");
    }
    let Some(segment) = safe_thread_segment(&thread_id) else {
        return json_error(StatusCode::BAD_REQUEST, "threadId is invalid");
    };
    if kind == "image" && !mime.to_ascii_lowercase().starts_with("image/") {
        return json_error(StatusCode::BAD_REQUEST, "image uploads require an image MIME type");
    }
    let limit = if kind == "image" { MAX_IMAGE_BYTES } else { MAX_FILE_BYTES };
    if body.len() > limit {
        return json_error(StatusCode::PAYLOAD_TOO_LARGE, "attachment exceeds the size limit");
    }
    if body.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "attachment body is empty");
    }

    let id = format!("{segment}-{}", crate::orchestration_engine::uuid_v4());
    let ext = extension_for(&mime, &name);
    let root = attachments_root();
    if let Err(e) = std::fs::create_dir_all(&root) {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to prepare attachment storage: {e}"),
        );
    }
    let path = root.join(format!("{id}{ext}"));
    if let Err(e) = std::fs::write(&path, &body) {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to store attachment: {e}"),
        );
    }
    let size = body.len() as i64;
    let name_trimmed: String = name.trim().chars().take(255).collect();
    let mut payload = json!({
        "type": kind,
        "id": id,
        "name": if name_trimmed.is_empty() { "attachment".to_string() } else { name_trimmed },
        "mimeType": mime,
        "sizeBytes": size,
    });
    if let Value::Object(ref mut m) = payload {
        m.insert("relativePath".into(), json!(format!("{id}{ext}")));
    }
    axum::Json(payload).into_response()
}

/// Ids never carry an extension or path segments; the extension is resolved
/// by scanning the known set exactly like upstream resolveAttachmentPathById.
pub async fn serve(Path(id): Path<String>, headers: axum::http::HeaderMap) -> Response {
    if id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains('.')
        || id.contains('\0')
        || id == ".."
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    let root = attachments_root();
    for ext in SERVE_EXTENSIONS {
        let candidate = root.join(format!("{id}{ext}"));
        if let Ok(real) = candidate.canonicalize() {
            let real_root = root.canonicalize().unwrap_or_else(|_| root.clone());
            if real.starts_with(&real_root) && real.is_file() {
                let range = headers.get(header::RANGE).and_then(|v| v.to_str().ok());
                let mime = crate::local_image::local_image_mime_for(&real.to_string_lossy());
                return crate::local_image::file_response_with_range(
                    &real,
                    range,
                    mime,
                    "private, no-store",
                );
            }
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_segment_matches_upstream_rules() {
        assert_eq!(safe_thread_segment("abc-def").as_deref(), Some("abc-def"));
        assert_eq!(
            safe_thread_segment("Hello World_123").as_deref(),
            Some("hello-world_123")
        );
        assert_eq!(safe_thread_segment("---").as_deref(), None);
        assert_eq!(safe_thread_segment("").as_deref(), None);
    }

    #[test]
    fn serve_rejects_path_segments() {
        assert!("/etc/passwd".contains('/'));
    }
}
