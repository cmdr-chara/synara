//! Auth HTTP surface, lean port of upstream http.ts auth routes over
//! auth_sessions tokens. Exact wire shapes from packages/contracts/auth.ts.
//! The bootstrap credential is seeded at boot into the secrets directory —
//! the local desktop shell reads it directly, matching the local trust model;
//! upstream's in-memory seeded grant map is the same mechanism one process
//! earlier. Remote-access hardening (rate limits, device keys) is a
//! documented gap.

use axum::{
    body::Bytes,
    extract::RawQuery,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use rusqlite::params;
use serde_json::{json, Value};

use crate::auth_sessions;
use crate::orchestration_engine::{open_state_rw, uuid_v4};

const PAIRING_TOKEN_ALPHABET: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
const PAIRING_TOKEN_LENGTH: usize = 12;

fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    crate::projects_fileops::iso8601(secs)
}

fn iso_plus_ms(ms: u64) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        + ms / 1000;
    crate::projects_fileops::iso8601(secs)
}

fn generate_pairing_token() -> String {
    use std::io::Read;
    let mut bytes = vec![0u8; PAIRING_TOKEN_LENGTH];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut bytes);
    }
    bytes.iter().map(|b| PAIRING_TOKEN_ALPHABET[(b % 32) as usize] as char).collect()
}

pub fn session_cookie_name() -> String {
    match std::env::var("PORT").ok().and_then(|p| p.parse::<u16>().ok()) {
        Some(port) => format!("synara_session_{port}"),
        None => "synara_session".to_string(),
    }
}

fn extract_session_token(headers: &HeaderMap) -> Option<String> {
    if let Some(auth) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            return Some(token.trim().to_string());
        }
    }
    let cookie_header = headers.get(header::COOKIE).and_then(|v| v.to_str().ok())?;
    let cookie_name = session_cookie_name();
    for part in cookie_header.split(';') {
        let part = part.trim();
        if let Some((name, value)) = part.split_once('=') {
            if name == cookie_name || name == "synara_session" {
                return Some(value.to_string());
            }
        }
    }
    None
}

pub(crate) struct AuthenticatedSession {
    pub(crate) session_id: String,
    pub(crate) role: String,
    pub(crate) method: String,
    pub(crate) subject: String,
}

/// Validates the request session: token cryptographic validity plus the
/// session row still active (not revoked, not expired) — mirroring upstream
/// authenticateRequest.
pub(crate) fn authenticate(headers: &HeaderMap) -> Option<AuthenticatedSession> {
    let token = extract_session_token(headers)?;
    let claims = auth_sessions::verify_token(&token)?;
    if claims.kind != "session" {
        return None;
    }
    let conn = open_state_rw().ok()?;
    let now = now_iso();
    conn.query_row(
        "SELECT session_id, role, method, subject FROM auth_sessions WHERE session_id = ?1 AND revoked_at IS NULL AND expires_at > ?2",
        params![claims.session_id, now],
        |r| Ok(AuthenticatedSession {
            session_id: r.get(0)?,
            role: r.get(1)?,
            method: r.get(2)?,
            subject: r.get(3)?,
        }),
    )
    .ok()
}


/// Validates a short-lived websocket ticket and the still-active session row
/// it points at. Unlike `authenticate`, which accepts long-lived session
/// cookies/bearer tokens, this is the credential browsers use for upgrades.
pub(crate) fn authenticate_websocket_ticket(token: &str) -> Option<AuthenticatedSession> {
    let claims = auth_sessions::verify_token(token)?;
    if claims.kind != "websocket" || !auth_sessions::consume_websocket_ticket(&claims) {
        return None;
    }
    let conn = open_state_rw().ok()?;
    let now = now_iso();
    conn.query_row(
        "SELECT session_id, role, method, subject FROM auth_sessions WHERE session_id = ?1 AND revoked_at IS NULL AND expires_at > ?2",
        params![claims.session_id, now],
        |r| Ok(AuthenticatedSession {
            session_id: r.get(0)?,
            role: r.get(1)?,
            method: r.get(2)?,
            subject: r.get(3)?,
        }),
    )
    .ok()
}

fn json_response(status: StatusCode, body: Value) -> Response {
    (status, axum::Json(body)).into_response()
}

fn create_session_row(
    subject: &str,
    role: &str,
    method: &str,
    label: Option<&str>,
    user_agent: Option<&str>,
) -> Option<(String, String)> {
    let conn = open_state_rw().ok()?;
    let session_id = uuid_v4();
    let now = now_iso();
    let expires = iso_plus_ms(auth_sessions::SESSION_TTL_MS);
    conn.execute(
        "INSERT INTO auth_sessions (session_id, subject, role, method, client_label, client_user_agent, issued_at, expires_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![session_id, subject, role, method, label, user_agent, now, expires],
    )
    .ok()?;
    let token = auth_sessions::issue_session_token(&session_id, subject, role, method)?;
    Some((session_id, token))
}

/// Seeds a one-time bootstrap pairing credential at boot when none is active,
/// written into the secrets dir for the local shell — exact upstream
/// alphabet/length, 5-minute TTL replaced by a fresh reseed on each boot.
pub fn seed_bootstrap_credential() {
    let Ok(conn) = open_state_rw() else { return };
    let now = now_iso();
    let active: i64 = conn
        .query_row(
            "SELECT COUNT(1) FROM auth_pairing_links WHERE revoked_at IS NULL AND consumed_at IS NULL AND expires_at > ?1",
            params![now],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if active > 0 {
        return;
    }
    let credential = generate_pairing_token();
    let expires = iso_plus_ms(24 * 60 * 60 * 1000);
    let _ = conn.execute(
        "INSERT INTO auth_pairing_links (id, credential, method, role, subject, label, created_at, expires_at) VALUES (?1,?2,'browser-session-cookie','owner','bootstrap','Bootstrap',?3,?4)",
        params![uuid_v4(), credential, now, expires],
    );
    let _ = crate::server_secrets::put_secret("bootstrap-credential", credential.as_bytes());
}

pub async fn bootstrap(headers: HeaderMap, body: Bytes) -> Response {
    let payload: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let credential = payload.get("credential").and_then(Value::as_str).unwrap_or("").trim().to_uppercase();
    if credential.is_empty() {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "Invalid bootstrap credential."}));
    }
    let Ok(conn) = open_state_rw() else {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "auth store unavailable"}));
    };
    let now = now_iso();
    let link: Option<(String, String, String, String)> = conn
        .query_row(
            "SELECT id, role, subject, method FROM auth_pairing_links WHERE credential = ?1 AND revoked_at IS NULL AND consumed_at IS NULL AND expires_at > ?2",
            params![credential, now],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();
    let Some((link_id, role, subject, method)) = link else {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "Invalid bootstrap credential."}));
    };
    let ua = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).map(str::to_string);
    let Some((_session_id, token)) = create_session_row(&subject, &role, &method, None, ua.as_deref()) else {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "failed to create session"}));
    };
    // One-time semantics: the credential is consumed exactly once.
    let _ = conn.execute(
        "UPDATE auth_pairing_links SET consumed_at = ?2 WHERE id = ?1",
        params![link_id, now],
    );
    let expires_at = iso_plus_ms(auth_sessions::SESSION_TTL_MS);
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax; Expires={}",
        session_cookie_name(),
        token,
        expires_at
    );
    (
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (header::SET_COOKIE, cookie),
        ],
        axum::Json(json!({
            "authenticated": true,
            "role": role,
            "sessionMethod": method,
            "expiresAt": expires_at,
        })),
    )
        .into_response()
}

pub async fn session_state(headers: HeaderMap) -> Response {
    match authenticate(&headers) {
        Some(session) => json_response(
            StatusCode::OK,
            json!({
                "authenticated": true,
                "role": session.role,
                "sessionMethod": session.method,
                "subject": session.subject,
            }),
        ),
        None => json_response(StatusCode::OK, json!({ "authenticated": false })),
    }
}

fn iso_millis(text: &str) -> Option<u64> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .and_then(|value| u64::try_from(value.timestamp_millis()).ok())
}

fn iso_from_millis(ms: u64) -> String {
    crate::projects_fileops::iso8601(ms / 1000)
}

pub async fn ws_token(headers: HeaderMap) -> Response {
    let Some(session) = authenticate(&headers) else {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "authentication required"}));
    };
    let session_expires_at_ms = open_state_rw()
        .ok()
        .and_then(|conn| {
            conn.query_row(
                "SELECT expires_at FROM auth_sessions WHERE session_id = ?1",
                params![session.session_id],
                |row| row.get::<_, String>(0),
            )
            .ok()
        })
        .and_then(|expires| iso_millis(&expires))
        .unwrap_or_else(|| {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0);
            now_ms + auth_sessions::WEBSOCKET_TOKEN_TTL_MS
        });
    let (token, expires_at_ms) = match auth_sessions::issue_websocket_ticket(&session.session_id, session_expires_at_ms) {
        Ok(issued) => issued,
        Err(auth_sessions::WebSocketTicketIssueError::Capacity { active }) => {
            return json_response(
                StatusCode::TOO_MANY_REQUESTS,
                json!({
                    "error": "Outstanding websocket ticket capacity exceeded.",
                    "scope": "websocket-tickets",
                    "active": active,
                    "limit": auth_sessions::MAX_OUTSTANDING_WEBSOCKET_TICKETS_PER_SESSION,
                    "retryAfterSeconds": 1,
                }),
            );
        }
        Err(auth_sessions::WebSocketTicketIssueError::SigningUnavailable) => {
            return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "ticket issuance failed"}));
        }
    };
    json_response(
        StatusCode::OK,
        json!({
            "token": token,
            "expiresAt": iso_from_millis(expires_at_ms),
        }),
    )
}

pub async fn pairing_token(headers: HeaderMap, body: Bytes) -> Response {
    let Some(session) = authenticate(&headers) else {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "authentication required"}));
    };
    if session.role != "owner" {
        return json_response(
            StatusCode::FORBIDDEN,
            json!({"error": "Only owner sessions can create pairing credentials."}),
        );
    }
    let payload: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let label = payload.get("label").and_then(Value::as_str).map(str::to_string);
    let role = payload.get("role").and_then(Value::as_str).unwrap_or("client");
    let credential = generate_pairing_token();
    let id = uuid_v4();
    let now = now_iso();
    let expires = iso_plus_ms(5 * 60 * 1000);
    let Ok(conn) = open_state_rw() else {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "auth store unavailable"}));
    };
    let inserted = conn.execute(
        "INSERT INTO auth_pairing_links (id, credential, method, role, subject, label, created_at, expires_at) VALUES (?1,?2,'browser-session-cookie',?3,?4,?5,?6,?7)",
        params![id, credential, role, session.subject, label, now, expires],
    );
    if inserted.is_err() {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "failed to create pairing credential"}));
    }
    json_response(
        StatusCode::OK,
        json!({
            "id": id,
            "credential": credential,
            "label": label,
            "expiresAt": expires,
        }),
    )
}

pub async fn logout(headers: HeaderMap) -> Response {
    let Some(session) = authenticate(&headers) else {
        return json_response(StatusCode::OK, json!({ "revoked": false }));
    };
    if let Ok(conn) = open_state_rw() {
        let _ = conn.execute(
            "UPDATE auth_sessions SET revoked_at = ?2 WHERE session_id = ?1",
            params![session.session_id, now_iso()],
        );
    }
    crate::auth_sessions::clear_websocket_tickets(&[session.session_id.clone()]);
    crate::auth_connections::interrupt_sessions(&[session.session_id]);
    json_response(StatusCode::OK, json!({ "revoked": true }))
}

pub async fn clients(headers: HeaderMap) -> Response {
    let Some(session) = authenticate(&headers) else {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "authentication required"}));
    };
    let Ok(conn) = open_state_rw() else {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "auth store unavailable"}));
    };
    let now = now_iso();
    let mut stmt = match conn.prepare(
        "SELECT session_id, subject, role, method, client_label, client_user_agent, issued_at, expires_at, last_connected_at FROM auth_sessions WHERE revoked_at IS NULL AND expires_at > ?1 ORDER BY issued_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "query failed"})),
    };
    let rows: Vec<Value> = stmt
        .query_map(params![now], |r| {
            Ok(json!({
                "sessionId": r.get::<_, String>(0)?,
                "subject": r.get::<_, String>(1)?,
                "role": r.get::<_, String>(2)?,
                "sessionMethod": r.get::<_, String>(3)?,
                "client": {
                    "label": r.get::<_, Option<String>>(4)?,
                    "userAgent": r.get::<_, Option<String>>(5)?,
                },
                "issuedAt": r.get::<_, String>(6)?,
                "expiresAt": r.get::<_, String>(7)?,
                "lastConnectedAt": r.get::<_, Option<String>>(8)?,
                "connected": crate::auth_connections::is_session_connected(&r.get::<_, String>(0)?),
            }))
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    // Mark the caller's own session, exactly like upstream's `current` flag.
    let rows: Vec<Value> = rows
        .into_iter()
        .map(|mut row| {
            if let Value::Object(ref mut m) = row {
                let current = m.get("sessionId").and_then(Value::as_str) == Some(&session.session_id);
                m.insert("current".into(), json!(current));
            }
            row
        })
        .collect();
    json_response(StatusCode::OK, json!({ "clients": rows }))
}

pub async fn clients_revoke(headers: HeaderMap, body: Bytes) -> Response {
    let Some(session) = authenticate(&headers) else {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "authentication required"}));
    };
    if session.role != "owner" {
        return json_response(StatusCode::FORBIDDEN, json!({"error": "Only owner sessions can revoke clients."}));
    }
    let payload: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let target = payload.get("sessionId").and_then(Value::as_str).unwrap_or("");
    if target.is_empty() {
        return json_response(StatusCode::BAD_REQUEST, json!({"error": "sessionId is required"}));
    }
    if let Ok(conn) = open_state_rw() {
        let _ = conn.execute(
            "UPDATE auth_sessions SET revoked_at = ?2 WHERE session_id = ?1",
            params![target, now_iso()],
        );
    }
    crate::auth_sessions::clear_websocket_tickets(&[target.to_string()]);
    crate::auth_connections::interrupt_sessions(&[target.to_string()]);
    json_response(StatusCode::OK, json!({ "revoked": true }))
}

/// Bearer bootstrap variant: same exchange, but the session token travels in
/// the JSON body instead of a cookie — for non-browser clients.
pub async fn bootstrap_bearer(headers: HeaderMap, body: Bytes) -> Response {
    let payload: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let credential = payload.get("credential").and_then(Value::as_str).unwrap_or("").trim().to_uppercase();
    if credential.is_empty() {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "Invalid bootstrap credential."}));
    }
    let Ok(conn) = open_state_rw() else {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "auth store unavailable"}));
    };
    let now = now_iso();
    let link: Option<(String, String, String, String)> = conn
        .query_row(
            "SELECT id, role, subject, method FROM auth_pairing_links WHERE credential = ?1 AND revoked_at IS NULL AND consumed_at IS NULL AND expires_at > ?2",
            params![credential, now],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok();
    let Some((link_id, role, subject, _method)) = link else {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "Invalid bootstrap credential."}));
    };
    let ua = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()).map(str::to_string);
    let Some((_session_id, token)) =
        create_session_row(&subject, &role, "bearer-session-token", None, ua.as_deref())
    else {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "failed to create session"}));
    };
    let _ = conn.execute(
        "UPDATE auth_pairing_links SET consumed_at = ?2 WHERE id = ?1",
        params![link_id, now],
    );
    json_response(
        StatusCode::OK,
        json!({
            "authenticated": true,
            "role": role,
            "sessionMethod": "bearer-session-token",
            "expiresAt": iso_plus_ms(auth_sessions::SESSION_TTL_MS),
            "sessionToken": token,
        }),
    )
}

pub async fn pairing_links(headers: HeaderMap) -> Response {
    let Some(session) = authenticate(&headers) else {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "authentication required"}));
    };
    if session.role != "owner" {
        return json_response(StatusCode::FORBIDDEN, json!({"error": "Only owner sessions can list pairing links."}));
    }
    let Ok(conn) = open_state_rw() else {
        return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "auth store unavailable"}));
    };
    let now = now_iso();
    let mut stmt = match conn.prepare(
        "SELECT id, label, created_at, expires_at, consumed_at FROM auth_pairing_links WHERE revoked_at IS NULL AND expires_at > ?1 ORDER BY created_at DESC",
    ) {
        Ok(s) => s,
        Err(_) => return json_response(StatusCode::INTERNAL_SERVER_ERROR, json!({"error": "query failed"})),
    };
    let rows: Vec<Value> = stmt
        .query_map(params![now], |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "label": r.get::<_, Option<String>>(1)?,
                "createdAt": r.get::<_, String>(2)?,
                "expiresAt": r.get::<_, String>(3)?,
                "consumed": r.get::<_, Option<String>>(4)?.is_some(),
            }))
        })
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default();
    json_response(StatusCode::OK, json!({ "links": rows }))
}

pub async fn pairing_links_revoke(headers: HeaderMap, body: Bytes) -> Response {
    let Some(session) = authenticate(&headers) else {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "authentication required"}));
    };
    if session.role != "owner" {
        return json_response(StatusCode::FORBIDDEN, json!({"error": "Only owner sessions can revoke pairing links."}));
    }
    let payload: Value = serde_json::from_slice(&body).unwrap_or(json!({}));
    let target = payload.get("id").and_then(Value::as_str).unwrap_or("");
    if target.is_empty() {
        return json_response(StatusCode::BAD_REQUEST, json!({"error": "id is required"}));
    }
    if let Ok(conn) = open_state_rw() {
        let _ = conn.execute(
            "UPDATE auth_pairing_links SET revoked_at = ?2 WHERE id = ?1",
            params![target, now_iso()],
        );
    }
    json_response(StatusCode::OK, json!({ "revoked": true }))
}

pub async fn clients_revoke_others(headers: HeaderMap) -> Response {
    let Some(session) = authenticate(&headers) else {
        return json_response(StatusCode::UNAUTHORIZED, json!({"error": "authentication required"}));
    };
    if let Ok(conn) = open_state_rw() {
        let mut stmt = conn.prepare(
            "SELECT session_id FROM auth_sessions WHERE session_id != ?1 AND revoked_at IS NULL",
        );
        let revoked_ids: Vec<String> = stmt
            .ok()
            .and_then(|mut statement| {
                statement
                    .query_map(params![session.session_id], |row| row.get::<_, String>(0))
                    .ok()
                    .map(|rows| rows.filter_map(Result::ok).collect())
            })
            .unwrap_or_default();
        let _ = conn.execute(
            "UPDATE auth_sessions SET revoked_at = ?2 WHERE session_id != ?1 AND revoked_at IS NULL",
            params![session.session_id, now_iso()],
        );
        crate::auth_sessions::clear_websocket_tickets(&revoked_ids);
        crate::auth_connections::interrupt_sessions(&revoked_ids);
    }
    json_response(StatusCode::OK, json!({ "revoked": true }))
}
