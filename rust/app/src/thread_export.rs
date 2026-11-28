//! Thread export as a ZIP download, lean slice of upstream exportThreadArchive.
//! The archive layout is exact: transcript.md plus thread.json with the same
//! projections. Entries use the stored (uncompressed) ZIP method — valid ZIP
//! everywhere, simpler than the upstream streaming deflater; compression is
//! the documented divergence. The 409 in-flight guard matches
//! shared/threadExport.ts exactly.

use axum::{
    extract::RawQuery,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use rusqlite::params;
use serde_json::{json, Value};

use crate::orchestration_engine::open_state_rw;

fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    !crc
}

struct ZipEntry {
    name: String,
    data: Vec<u8>,
}

fn build_zip(entries: &[ZipEntry]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for entry in entries {
        let offset = out.len() as u32;
        let crc = crc32(&entry.data);
        let size = entry.data.len() as u32;
        let name = entry.name.as_bytes();
        // Local file header.
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0x0800u16.to_le_bytes()); // UTF-8 flag
        out.extend_from_slice(&0u16.to_le_bytes()); // stored
        out.extend_from_slice(&0u16.to_le_bytes()); // time
        out.extend_from_slice(&0x21u16.to_le_bytes()); // date (1980-01-01)
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(&size.to_le_bytes()); // compressed
        out.extend_from_slice(&size.to_le_bytes()); // uncompressed
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name);
        out.extend_from_slice(&entry.data);
        // Central directory record.
        central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&20u16.to_le_bytes());
        central.extend_from_slice(&0x0800u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes());
        central.extend_from_slice(&0x21u16.to_le_bytes());
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&size.to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&0u16.to_le_bytes()); // extra
        central.extend_from_slice(&0u16.to_le_bytes()); // comment
        central.extend_from_slice(&0u16.to_le_bytes()); // disk
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name);
    }
    let cd_offset = out.len() as u32;
    let cd_size = central.len() as u32;
    out.extend_from_slice(&central);
    // End of central directory.
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&cd_size.to_le_bytes());
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn slugify_title(title: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in title.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    let capped: String = trimmed.chars().take(60).collect();
    let capped = capped.trim_end_matches('-');
    if capped.is_empty() {
        "thread".to_string()
    } else {
        capped.to_string()
    }
}

fn role_heading(role: &str) -> &str {
    match role {
        "user" => "User",
        "assistant" => "Assistant",
        _ => "Message",
    }
}

pub async fn thread_export(
    headers: axum::http::HeaderMap,
    RawQuery(query): RawQuery,
) -> Response {
    // Full thread contents: unauthenticated export on a non-loopback bind
    // would expose every conversation to the network. Browser downloads are
    // same-origin and carry the session cookie, so first-party export keeps
    // working.
    let remote_device = headers
        .get("x-synara-remote-device-jkt")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .is_some();
    if !remote_device && crate::auth_http::authenticate(&headers).is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    let query = query.unwrap_or_default();
    let thread_id = crate::attachments_http::attachments_http_query(&query, "threadId").unwrap_or_default();
    if thread_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing threadId parameter").into_response();
    }
    let conn = match open_state_rw() {
        Ok(c) => c,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let thread: Option<(String, String, String, String)> = conn
        .query_row(
            "SELECT title, COALESCE(model_selection_json,'null'), runtime_mode, updated_at FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();
    let Some((title, model_selection, runtime_mode, updated_at)) = thread else {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    };
    // In-flight guard, exactly shared/threadExport.ts: a running turn or a
    // streaming message blocks the export with 409.
    let running: bool = conn
        .query_row(
            "SELECT COUNT(1) FROM projection_turns WHERE thread_id = ?1 AND state = 'running'",
            params![thread_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    if running {
        return (
            StatusCode::CONFLICT,
            "Thread is still running. Wait for the current turn to finish before exporting.",
        )
            .into_response();
    }
    let streaming: bool = conn
        .query_row(
            "SELECT COUNT(1) FROM projection_thread_messages WHERE thread_id = ?1 AND is_streaming = 1",
            params![thread_id],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);
    if streaming {
        return (
            StatusCode::CONFLICT,
            "Thread has a streaming message. Wait for the current response to finish before exporting.",
        )
            .into_response();
    }

    let mut stmt = match conn.prepare(
        "SELECT message_id, role, text, created_at, updated_at, attachments_json FROM projection_thread_messages WHERE thread_id = ?1 ORDER BY rowid ASC",
    ) {
        Ok(s) => s,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };
    let messages: Vec<(String, String, String, String, String, Option<String>)> = stmt
        .query_map(params![thread_id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();

    let mut markdown = format!("# {title}\n\n> Exported from Synara.\n");
    for (_, role, text, created_at, _, _) in &messages {
        markdown.push_str(&format!("\n## {} `{created_at}`\n\n{text}\n", role_heading(role)));
    }

    let metadata = json!({
        "threadId": thread_id,
        "title": title,
        "modelSelection": serde_json::from_str::<Value>(&model_selection).unwrap_or(Value::Null),
        "runtimeMode": runtime_mode,
        "createdAt": updated_at,
        "updatedAt": updated_at,
    });
    let mut thread_json = format!(
        "{},\n  \"messages\": [",
        serde_json::to_string_pretty(&metadata).unwrap_or_default().trim_end_matches('}')
    );
    let mut first = true;
    for (id, role, text, created_at, msg_updated, attachments) in &messages {
        let mut m = json!({
            "id": id,
            "role": role,
            "text": text,
            "source": "native",
            "createdAt": created_at,
            "updatedAt": msg_updated,
        });
        if let Some(att) = attachments {
            if let Ok(v) = serde_json::from_str::<Value>(att) {
                if v.as_array().map(|a| !a.is_empty()).unwrap_or(false) {
                    m["attachments"] = v;
                }
            }
        }
        let serialized = serde_json::to_string_pretty(&m)
            .unwrap_or_default()
            .lines()
            .map(|l| format!("    {l}"))
            .collect::<Vec<_>>()
            .join("\n");
        if !first {
            thread_json.push(',');
        }
        thread_json.push('\n');
        thread_json.push_str(&serialized);
        first = false;
    }
    thread_json.push_str("\n  ]\n}\n");

    let zip = build_zip(&[
        ZipEntry { name: "transcript.md".into(), data: markdown.into_bytes() },
        ZipEntry { name: "thread.json".into(), data: thread_json.into_bytes() },
    ]);

    let date_bucket = updated_at.get(..10).unwrap_or("0000-00-00").replace('-', "");
    let file_name = format!("synara-thread-{}-{date_bucket}.zip", slugify_title(&title));
    (
        [
            (header::CONTENT_TYPE, "application/zip".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{file_name}\""),
            ),
            (header::CACHE_CONTROL, "no-store".to_string()),
        ],
        zip,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zip_has_valid_structure() {
        let zip = build_zip(&[ZipEntry { name: "a.txt".into(), data: b"hello".to_vec() }]);
        assert!(zip.starts_with(&0x0403_4b50u32.to_le_bytes()));
        assert!(zip.windows(4).any(|w| w == 0x0201_4b50u32.to_le_bytes()));
        assert!(zip.windows(4).any(|w| w == 0x0605_4b50u32.to_le_bytes()));
    }

    #[test]
    fn slugify_matches_expected_shape() {
        assert_eq!(slugify_title("Hello World!"), "hello-world");
        assert_eq!(slugify_title("---"), "thread");
    }

    #[test]
    fn crc32_known_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }
}
