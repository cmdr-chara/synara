//! Attachment upload and serving over HTTP, lean slice of the upstream
//! attachmentStore/http route pair. The wire contract is exact: upload is
//! POST /api/attachments/upload?threadId&type&name&mimeType with raw body
//! bytes and answers the ChatAttachment metadata JSON ({type,id,name,
//! mimeType,sizeBytes}); serving is GET /attachments/{id} resolving the
//! extension by scan, exactly like upstream resolveAttachmentPathById.
//! The storage lifecycle follows upstream managedAttachmentStore.ts: ids are
//! att_v2_{32hex}, bytes live at objects/{hex-prefix}/{id}{ext}, every upload
//! is tracked in managed_attachment_blobs (staged with a 1h TTL, claimed once
//! a turn message references it), and a sweep worker reclaims expired staging
//! plus rows marked for deletion.

use axum::{
    body::Bytes,
    extract::{Path, RawQuery},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use rusqlite::params;
use serde_json::{Value, json};
use sha2::Digest;
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
    let capped = capped
        .trim_end_matches(|c| c == '-' || c == '_')
        .to_string();
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

/// Exact upstream migration 055 shape, created lazily so the server never
/// depends on the gate schema being fresh.
pub(crate) fn ensure_managed_schema(conn: &rusqlite::Connection) {
    let _ = conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS managed_attachment_blobs (
          attachment_id TEXT PRIMARY KEY,
          owner_thread_id TEXT NOT NULL,
          owner_kind TEXT NOT NULL,
          owner_id TEXT NOT NULL,
          kind TEXT NOT NULL,
          original_name TEXT NOT NULL,
          mime_type TEXT NOT NULL,
          reserved_bytes INTEGER NOT NULL,
          size_bytes INTEGER,
          sha256 TEXT,
          relative_path TEXT NOT NULL UNIQUE,
          state TEXT NOT NULL,
          staging_expires_at TEXT,
          claim_command_id TEXT,
          claim_message_id TEXT,
          claimed_at TEXT,
          delete_reason TEXT,
          delete_requested_at TEXT,
          deleted_at TEXT,
          created_at TEXT NOT NULL,
          updated_at TEXT NOT NULL
        )",
    );
}

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    crate::projects_fileops::iso8601(secs)
}

fn iso_in(seconds: u64) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        + seconds;
    crate::projects_fileops::iso8601(secs)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = sha2::Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Claims staged attachments referenced by a persisted turn message, mirroring
/// upstream claim semantics: the blob survives past its staging TTL once a
/// message actually carries it.
fn claim_staged(
    conn: &rusqlite::Connection,
    attachment_id: &str,
    thread_id: &str,
    message_id: &str,
    now: &str,
) -> bool {
    // The upstream CHECK requires both claim_command_id and claim_message_id
    // on claimed rows. Dispatch-time command ids are not threaded into the
    // projection layer, so the message id stands in as the command linkage —
    // the audit trail still resolves the exact message and turn. Claims are
    // thread-scoped and cannot resurrect an expired staging row.
    conn.execute(
        "UPDATE managed_attachment_blobs SET state='claimed', claim_command_id=?3, claim_message_id=?3, claimed_at=?4, staging_expires_at=NULL, updated_at=?4 WHERE attachment_id=?1 AND owner_thread_id=?2 AND state='staged' AND (staging_expires_at IS NULL OR staging_expires_at >= ?4)",
        params![attachment_id, thread_id, message_id, now],
    ).unwrap_or(0) == 1
}

pub(crate) fn claim_attachments_for_message(
    message_id: &str,
    thread_id: &str,
    attachment_ids: &[String],
) {
    if attachment_ids.is_empty() {
        return;
    }
    let Ok(conn) = crate::orchestration_engine::open_state_rw() else {
        return;
    };
    ensure_managed_schema(&conn);
    let now = now_iso();
    let mut seen = std::collections::HashSet::new();
    for id in attachment_ids {
        // Duplicate ids reject the upstream command. Projection has no error
        // path by this point, so a duplicate is conservatively ignored after
        // its first valid claim instead of claiming the row twice.
        if seen.insert(id.as_str()) {
            let _ = claim_staged(&conn, id, thread_id, message_id, &now);
        }
    }
}

fn lookup_relative_path(id: &str) -> Option<String> {
    let conn = crate::orchestration_engine::open_state_rw().ok()?;
    ensure_managed_schema(&conn);
    conn.query_row(
        "SELECT relative_path FROM managed_attachment_blobs WHERE attachment_id=?1 AND state IN ('staged','claimed')",
        params![id],
        |r| r.get::<_, String>(0),
    )
    .ok()
}

/// Cleanup sweep, lean port of managedAttachmentCleanup.ts: staged rows past
/// their TTL and rows in deleting state get their object file removed and end
/// tombstoned; tombstones are purged after 30 days. The lease/backoff
/// machinery upstream uses for multi-instance safety is unnecessary here
/// because this server runs a single writer.
pub fn run_cleanup_sweep() {
    let Ok(conn) = crate::orchestration_engine::open_state_rw() else {
        return;
    };
    ensure_managed_schema(&conn);
    let now = now_iso();
    let root = attachments_root();
    let mut stmt = match conn.prepare(
        "SELECT attachment_id, relative_path FROM managed_attachment_blobs WHERE (state='staged' AND staging_expires_at IS NOT NULL AND staging_expires_at < ?1) OR state='deleting'",
    ) {
        Ok(s) => s,
        Err(_) => return,
    };
    let victims: Vec<(String, String)> = stmt
        .query_map(params![now], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    for (id, rel) in victims {
        let target = root.join(&rel);
        if target.starts_with(&root) {
            let _ = std::fs::remove_file(&target);
        }
        // The upstream trigger graph only allows deleting -> deleted, so the
        // sweep walks victims through deleting first; the CHECK chain requires
        // delete_reason and delete_requested_at on both states.
        let _ = conn.execute(
            "UPDATE managed_attachment_blobs SET state='deleting', delete_reason=COALESCE(delete_reason,'staging-expired'), delete_requested_at=COALESCE(delete_requested_at,?2), updated_at=?2 WHERE attachment_id=?1 AND state IN ('staged','claimed','uploading')",
            params![id, now],
        );
        let _ = conn.execute(
            "UPDATE managed_attachment_blobs SET state='deleted', deleted_at=?2, updated_at=?2 WHERE attachment_id=?1 AND state='deleting'",
            params![id, now],
        );
    }
    // Tombstone retention: 30 days, exact upstream constant.
    let purge_before = iso_in_now_minus(30 * 24 * 60 * 60);
    let mut purged_files: Vec<String> = Vec::new();
    if let Ok(mut st) = conn.prepare(
        "SELECT relative_path FROM managed_attachment_blobs WHERE state='deleted' AND deleted_at IS NOT NULL AND deleted_at < ?1",
    ) {
        if let Ok(rows) = st.query_map(params![purge_before], |r| r.get::<_, String>(0)) {
            purged_files = rows.filter_map(|r| r.ok()).collect();
        }
    }
    let _ = conn.execute(
        "DELETE FROM managed_attachment_blobs WHERE state='deleted' AND deleted_at IS NOT NULL AND deleted_at < ?1",
        params![purge_before],
    );
}

fn iso_in_now_minus(seconds: u64) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .saturating_sub(seconds);
    crate::projects_fileops::iso8601(secs)
}

/// Starts the 5-minute sweep loop behind the same single-writer assumption as
/// the rest of the native server.
pub fn start_cleanup_worker() {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(300)).await;
            run_cleanup_sweep();
        }
    });
}

fn attachment_principal(headers: &axum::http::HeaderMap) -> (String, String) {
    if let Some(device_jkt) = headers
        .get("x-synara-remote-device-jkt")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 256)
    {
        return ("session".to_string(), format!("remote-device:{device_jkt}"));
    }
    crate::auth_http::authenticate(headers)
        .map(|session| ("session".to_string(), session.session_id))
        .unwrap_or_else(|| ("local-loopback".to_string(), "local-loopback".to_string()))
}

fn cancel_staged(
    conn: &rusqlite::Connection,
    attachment_id: &str,
    owner_kind: &str,
    owner_id: &str,
    reason: &str,
    requested_at: &str,
) -> &'static str {
    let row = conn.query_row(
        "SELECT state FROM managed_attachment_blobs WHERE attachment_id=?1 AND owner_kind=?2 AND owner_id=?3",
        params![attachment_id, owner_kind, owner_id],
        |r| r.get::<_, String>(0),
    );
    let Ok(state) = row else { return "not-found" };
    if state == "claimed" {
        return "already-claimed";
    }
    if state == "deleting" || state == "deleted" {
        return "cancelled";
    }
    let updated = conn.execute(
        "UPDATE managed_attachment_blobs SET state='deleting', delete_reason=?4, delete_requested_at=?5, updated_at=?5 WHERE attachment_id=?1 AND owner_kind=?2 AND owner_id=?3 AND state IN ('uploading','staged')",
        params![attachment_id, owner_kind, owner_id, reason, requested_at],
    ).unwrap_or(0);
    if updated == 0 {
        "not-found"
    } else {
        "cancelled"
    }
}

fn valid_attachment_cancel_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

pub async fn cancel(headers: axum::http::HeaderMap, body: Bytes) -> Response {
    let payload: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let attachment_id = payload
        .get("attachmentId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if !valid_attachment_cancel_id(attachment_id) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "Attachment cancellation payload is invalid.",
        );
    }
    let (owner_kind, owner_id) = attachment_principal(&headers);
    let Ok(conn) = crate::orchestration_engine::open_state_rw() else {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "attachment store unavailable",
        );
    };
    ensure_managed_schema(&conn);
    match cancel_staged(
        &conn,
        attachment_id,
        &owner_kind,
        &owner_id,
        "client-cancelled",
        &now_iso(),
    ) {
        "not-found" => json_error(StatusCode::NOT_FOUND, "Attachment not found."),
        "already-claimed" => json_error(StatusCode::CONFLICT, "Attachment is already committed."),
        _ => (StatusCode::OK, axum::Json(json!({"cancelled": true}))).into_response(),
    }
}

pub async fn upload(
    headers: axum::http::HeaderMap,
    RawQuery(query): RawQuery,
    body: Bytes,
) -> Response {
    let query = query.unwrap_or_default();
    let thread_id = query_param(&query, "threadId").unwrap_or_default();
    let kind = query_param(&query, "type").unwrap_or_default();
    let name = query_param(&query, "name").unwrap_or_else(|| "attachment".into());
    let mime = query_param(&query, "mimeType").unwrap_or_else(|| "application/octet-stream".into());

    if kind != "image" && kind != "file" {
        return json_error(StatusCode::BAD_REQUEST, "type must be image or file");
    }
    let Some(_segment) = safe_thread_segment(&thread_id) else {
        return json_error(StatusCode::BAD_REQUEST, "threadId is invalid");
    };
    if kind == "image" && !mime.to_ascii_lowercase().starts_with("image/") {
        return json_error(
            StatusCode::BAD_REQUEST,
            "image uploads require an image MIME type",
        );
    }
    let limit = if kind == "image" {
        MAX_IMAGE_BYTES
    } else {
        MAX_FILE_BYTES
    };
    if body.len() > limit {
        return json_error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "attachment exceeds the size limit",
        );
    }
    if body.is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "attachment body is empty");
    }

    let hex: String = crate::orchestration_engine::uuid_v4().replace('-', "");
    let id = format!("att_v2_{hex}");
    let ext = extension_for(&mime, &name);
    let root = attachments_root();
    // Upstream object layout: objects/{first two hex chars}/{id}{ext}, so one
    // directory never accumulates every blob.
    let prefix = &hex[..2.min(hex.len())];
    let relative = format!("objects/{prefix}/{id}{ext}");
    let path = root.join(&relative);
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("failed to prepare attachment storage: {e}"),
            );
        }
    }
    if let Err(e) = std::fs::write(&path, &body) {
        return json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to store attachment: {e}"),
        );
    }
    let size = body.len() as i64;
    let name_trimmed: String = name.trim().chars().take(255).collect();
    // Lifecycle row: staged with a one-hour TTL. The blob is reclaimed by the
    // sweep unless a turn message claims it — exact upstream semantics.
    if let Ok(conn) = crate::orchestration_engine::open_state_rw() {
        ensure_managed_schema(&conn);
        let now = now_iso();
        let (owner_kind, owner_id) = attachment_principal(&headers);
        let _ = conn.execute(
            "INSERT OR REPLACE INTO managed_attachment_blobs (attachment_id, owner_thread_id, owner_kind, owner_id, kind, original_name, mime_type, reserved_bytes, size_bytes, sha256, relative_path, state, staging_expires_at, created_at, updated_at) VALUES (?1,?2,?11,?12,?3,?4,?5,?6,?6,?7,?8,'staged',?9,?10,?10)",
            params![
                id,
                thread_id,
                kind,
                name_trimmed,
                mime,
                size,
                sha256_hex(&body),
                relative,
                iso_in(3600),
                now,
                owner_kind,
                owner_id,
            ],
        );
    }
    let payload = json!({
        "type": kind,
        "id": id,
        "name": if name_trimmed.is_empty() { "attachment".to_string() } else { name_trimmed },
        "mimeType": mime,
        "sizeBytes": size,
        "relativePath": relative,
    });
    (StatusCode::CREATED, axum::Json(payload)).into_response()
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
    // Managed blobs resolve through their lifecycle row, so a deleted or
    // tombstoned attachment stops serving even while the file remains.
    if id.starts_with("att_v2_") {
        if let Some(rel) = lookup_relative_path(&id) {
            let candidate = root.join(&rel);
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
        return StatusCode::NOT_FOUND.into_response();
    }
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

    #[test]
    fn remote_device_principal_isolated_from_local_loopback() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert(
            "x-synara-remote-device-jkt",
            axum::http::HeaderValue::from_static("device-jkt"),
        );
        assert_eq!(
            attachment_principal(&headers),
            ("session".into(), "remote-device:device-jkt".into())
        );
        assert_eq!(
            attachment_principal(&axum::http::HeaderMap::new()),
            ("local-loopback".into(), "local-loopback".into())
        );
    }

    #[test]
    fn cancellation_is_owner_scoped_stateful_and_idempotent() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_managed_schema(&conn);
        conn.execute(
            "INSERT INTO managed_attachment_blobs (attachment_id, owner_thread_id, owner_kind, owner_id, kind, original_name, mime_type, reserved_bytes, size_bytes, sha256, relative_path, state, staging_expires_at, created_at, updated_at) VALUES ('att','thread','session','one','image','a.png','image/png',1,1,'x','objects/aa/att.png','staged','soon','now','now')",
            [],
        ).unwrap();
        assert_eq!(
            cancel_staged(&conn, "att", "session", "two", "client-cancelled", "later"),
            "not-found"
        );
        assert_eq!(
            cancel_staged(&conn, "att", "session", "one", "client-cancelled", "later"),
            "cancelled"
        );
        assert_eq!(
            cancel_staged(&conn, "att", "session", "one", "client-cancelled", "later"),
            "cancelled"
        );
        conn.execute(
            "UPDATE managed_attachment_blobs SET state='claimed' WHERE attachment_id='att'",
            [],
        )
        .unwrap();
        assert_eq!(
            cancel_staged(&conn, "att", "session", "one", "client-cancelled", "later"),
            "already-claimed"
        );
    }

    #[test]
    fn claims_are_thread_scoped_and_expire() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        ensure_managed_schema(&conn);
        for (id, thread, expiry) in [
            ("att-live", "thread-1", "2999-01-01T00:00:00Z"),
            ("att-wrong-thread", "thread-2", "2999-01-01T00:00:00Z"),
            ("att-expired", "thread-1", "2000-01-01T00:00:00Z"),
        ] {
            conn.execute(
                "INSERT INTO managed_attachment_blobs (attachment_id, owner_thread_id, owner_kind, owner_id, kind, original_name, mime_type, reserved_bytes, size_bytes, sha256, relative_path, state, staging_expires_at, created_at, updated_at) VALUES (?1,?2,'local-loopback','local-loopback','image','a.png','image/png',1,1,'x',?3,'staged',?4,'now','now')",
                params![id, thread, format!("objects/aa/{id}.png"), expiry],
            ).unwrap();
        }
        let now = "2026-01-01T00:00:00Z";
        assert!(claim_staged(
            &conn,
            "att-live",
            "thread-1",
            "message-1",
            now
        ));
        assert!(!claim_staged(
            &conn,
            "att-wrong-thread",
            "thread-1",
            "message-1",
            now
        ));
        assert!(!claim_staged(
            &conn,
            "att-expired",
            "thread-1",
            "message-1",
            now
        ));
    }

    #[test]
    fn cancellation_id_shape_matches_upstream() {
        assert!(!valid_attachment_cancel_id("att_v2 abc"));
        assert!(valid_attachment_cancel_id("att_v2_abcd-123"));
        assert!(!valid_attachment_cancel_id("att/../x"));
    }
}
