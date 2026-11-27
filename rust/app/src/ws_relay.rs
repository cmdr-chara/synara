//! WebSocket relay for the RPC upgrade paths (`/ws`, `/ws/bootstrap`,
//! device/computer frame sockets). Each client upgrade is answered by opening
//! a matching upgrade against the original and piping frames both ways — the
//! byte-level equivalent of the HTTP proxy.
//!
//! `/ws/bootstrap` additionally intercepts `bootstrap.negotiate` requests and
//! answers them natively (same `WsBootstrapNegotiateResult` the HTTP
//! `/ws/negotiate` route produces); every other frame relays upstream.

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message as ClientMessage, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc::unbounded_channel;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as UpstreamMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::original::Original;

/// Forward the headers that matter to the original's admission and auth.
/// `origin` is deliberately excluded: the original gates upgrades on a trusted
/// origin list and the webview's dev-server origin (e.g. localhost:4577) is not
/// on it — omitting the header passes admission while `token` still auths.
const FORWARDED: &[&str] = &[
    "cookie",
    "authorization",
    "sec-websocket-protocol",
    "user-agent",
    "x-forwarded-for",
    "x-forwarded-proto",
];

const BOOTSTRAP_NEGOTIATE: &str = "bootstrap.negotiate";

fn rpc_id_key(v: &serde_json::Value) -> String {
    // Effect RPC clients use numeric request ids while the native shell uses
    // string ids; responses must echo the original JSON type verbatim. Internally
    // streams are keyed by this normalized string form either way.
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

pub fn router() -> Router<Original> {
    Router::new()
        .route("/ws", get(|s, w, u, h| upgrade(s, w, u, h, false)))
        .route("/ws/bootstrap", get(|s, w, u, h| upgrade(s, w, u, h, true)))
        .route("/ws/computer-frames", get(computer_frames_upgrade))
        .route("/ws/device-frames", get(device_frames_upgrade))
        .route("/ws/host/v2", get(remote_tunnel_upgrade))
        .route("/ws/remote/{host_id}/ws/negotiate", get(remote_negotiate))
        .route("/ws/remote/{host_id}/ws", get(remote_renderer_upgrade))
}


/// Outer remote endpoint: never plaintext RPC — the upgraded socket carries
/// the TLS tunnel exactly as upstream's RemoteTlsServer. Without a paired
/// identity there is nothing to terminate TLS with, so the upgrade refuses.
async fn remote_tunnel_upgrade(
    State(original): State<Original>,
    ws: WebSocketUpgrade,
) -> Response {
    let state_dir = crate::todo::state_dir();
    let secrets_dir = format!("{state_dir}/secrets");
    let environment_id = crate::server_env::environment_id(&state_dir).unwrap_or_default();
    let path = crate::remote_tls::remote_tls_identity_path(&secrets_dir);
    let identity = match crate::remote_tls::load_remote_tls_identity(&path, &environment_id) {
        Ok(identity) => identity,
        Err(_) => return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response(),
    };
    let todos = original.todos.clone();
    ws.on_upgrade(move |socket| async move {
        crate::remote_tunnel::drive_tunnel(socket, &identity, move |tls| async move {
            crate::remote_tunnel::serve_inner(tls, todos).await;
        })
        .await;
    })
}

/// Computer frame streaming serves natively when the backend owns the
/// computer; upstream mode relays exactly like every other socket.

pub(crate) fn query_value(uri: &Uri, wanted: &str) -> Option<String> {
    uri.query()?.split('&').filter_map(|pair| {
        let (key, raw) = pair.split_once('=').unwrap_or((pair, ""));
        (key == wanted).then(|| urlencoding_decode(raw))
    }).next()
}

fn configured_auth_token() -> Option<String> {
    std::env::var("SYNARA_AUTH_TOKEN").ok().filter(|value| !value.trim().is_empty()).or_else(|| {
        let args: Vec<String> = std::env::args().collect();
        args.windows(2)
            .find(|pair| pair[0] == "--auth-token")
            .map(|pair| pair[1].clone())
            .or_else(|| args.iter().find_map(|arg| arg.strip_prefix("--auth-token=").map(str::to_string)))
            .filter(|value| !value.trim().is_empty())
    })
}

fn loopback_host(value: Option<&str>) -> bool {
    let Some(value) = value else { return true };
    matches!(value, "localhost" | "127.0.0.1" | "::1" | "[::1]")
}

fn websocket_auth_required() -> bool {
    configured_auth_token().is_some()
        || std::env::var("SYNARA_PUBLIC_URL").ok().is_some_and(|value| !value.trim().is_empty())
        || !loopback_host(std::env::var("SYNARA_HOST").ok().as_deref())
        || std::env::var("SYNARA_SSH_FORWARD_PORT").ok().is_some_and(|value| !value.trim().is_empty())
}

fn trusted_local_legacy_token(token: Option<&str>) -> bool {
    configured_auth_token().is_some_and(|expected| {
        Some(expected.as_str()) == token
            && loopback_host(std::env::var("SYNARA_HOST").ok().as_deref())
            && std::env::var("SYNARA_PUBLIC_URL").ok().is_none_or(|value| value.trim().is_empty())
    })
}


fn has_session_cookie(cookie_header: &str) -> bool {
    cookie_header.split(';').any(|part| {
        let Some((name, value)) = part.trim().split_once('=') else { return false };
        !value.is_empty()
            && (name == "synara_session"
                || (name.strip_prefix("synara_session_").is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                })))
    })
}

/// Owner admission for the remote-host pivot. This intentionally mirrors the
/// local trust split of the main `/ws` upgrade: authless loopback stays local,
/// the private legacy token stays local, but every explicit browser credential
/// must verify and client-role sessions may not pivot to another machine.
pub(crate) fn remote_owner_authorized(headers: &HeaderMap, uri: &Uri) -> Result<(), axum::http::StatusCode> {
    let legacy_token = query_value(uri, "token");
    let ws_ticket = query_value(uri, "wsToken");
    let explicit = ws_ticket.is_some()
        || headers.get(axum::http::header::AUTHORIZATION).is_some()
        || headers.get(axum::http::header::COOKIE).and_then(|value| value.to_str().ok()).is_some_and(has_session_cookie);
    if !explicit && (!websocket_auth_required() || trusted_local_legacy_token(legacy_token.as_deref())) {
        return Ok(());
    }
    if trusted_local_legacy_token(legacy_token.as_deref()) {
        return Ok(());
    }
    let session = ws_ticket
        .as_deref()
        .and_then(crate::auth_http::authenticate_websocket_ticket)
        .or_else(|| crate::auth_http::authenticate(headers));
    let Some(session) = session else {
        return Err(axum::http::StatusCode::UNAUTHORIZED);
    };
    if session.role != "owner" {
        return Err(axum::http::StatusCode::FORBIDDEN);
    }
    Ok(())
}

/// Feature RPC and frame sockets share upstream's upgrade admission: trusted
/// origin first, then either local loopback trust, the private legacy token,
/// or an explicit authenticated session/ticket. `/ws/bootstrap` stays
/// unauthenticated because it is only the legacy negotiation transport.
fn reject_untrusted_websocket_origin(headers: &HeaderMap) -> bool {
    let cfg = crate::native_ws::config();
    let request_origin = crate::native_ws::request_origin(headers);
    let raw_origin = headers.get("origin").and_then(|value| value.to_str().ok());
    crate::native_ws::should_reject_untrusted_request_origin(raw_origin, &request_origin, cfg)
}

fn websocket_auth_not_required_response(status: axum::http::StatusCode) -> Response {
    (status, [(axum::http::header::CACHE_CONTROL, "no-store")], "Unauthorized").into_response()
}

fn authenticate_rpc_websocket_upgrade(
    headers: &HeaderMap,
    uri: &Uri,
) -> Result<Option<crate::auth_http::AuthenticatedSession>, axum::http::StatusCode> {
    let legacy_token = query_value(uri, "token");
    if trusted_local_legacy_token(legacy_token.as_deref()) {
        return Ok(None);
    }
    let ws_ticket = query_value(uri, "wsToken");
    let explicit = ws_ticket.is_some()
        || headers.get(axum::http::header::AUTHORIZATION).is_some()
        || headers
            .get(axum::http::header::COOKIE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(has_session_cookie);
    if !explicit && !websocket_auth_required() {
        return Ok(None);
    }
    let session = ws_ticket
        .as_deref()
        .and_then(crate::auth_http::authenticate_websocket_ticket)
        .or_else(|| crate::auth_http::authenticate(headers));
    session
        .map(Some)
        .ok_or(axum::http::StatusCode::UNAUTHORIZED)
}

fn authenticated_websocket_path(path: &str) -> bool {
    path == "/ws" || path == "/ws/device-frames" || path == "/ws/computer-frames"
}

async fn remote_negotiate(
    axum::extract::Path(host_id): axum::extract::Path<String>,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let cfg = crate::native_ws::config();
    let request_origin = crate::native_ws::request_origin(&headers);
    let raw_origin = headers.get("origin").and_then(|value| value.to_str().ok());
    let no_store = [(axum::http::header::CACHE_CONTROL, "no-store")];
    if crate::native_ws::should_reject_untrusted_request_origin(raw_origin, &request_origin, cfg) {
        return (axum::http::StatusCode::FORBIDDEN, no_store, "Forbidden").into_response();
    }
    if let Err(status) = remote_owner_authorized(&headers, &uri) {
        return (status, no_store, "Unauthorized").into_response();
    }
    if crate::remote_connections::desired(&host_id).is_none() {
        return (axum::http::StatusCode::NOT_FOUND, no_store, "No open connection to that host").into_response();
    }
    let query = uri.query().unwrap_or_default();
    let mut epoch = None;
    let mut min_revision = None;
    let mut max_revision = None;
    let mut client_build = None;
    let mut required = Vec::new();
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, raw) = pair.split_once('=').unwrap_or((pair, ""));
        let value = urlencoding_decode(raw);
        let integer = (!value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
            .then(|| value.parse::<i64>().ok()).flatten();
        match key {
            "x-synara-protocol-epoch" => epoch = integer,
            "x-synara-protocol-min-revision" => min_revision = integer,
            "x-synara-protocol-max-revision" => max_revision = integer,
            "x-synara-client-build" => client_build = (!value.trim().is_empty()).then_some(value),
            "x-synara-required-capability" => required.push(value),
            _ => {}
        }
    }
    let valid = epoch.is_some() && min_revision.is_some() && max_revision.is_some() && client_build.is_some();
    if !valid {
        let error = crate::native_ws::error_body(
            "WebSocket negotiation parameters are missing or invalid.",
            "WS_NEGOTIATION_REQUIRED",
            "reload",
        );
        return (axum::http::StatusCode::UPGRADE_REQUIRED, no_store, axum::Json(error)).into_response();
    }
    let client = json!({
        "protocolEpoch": epoch.unwrap_or_default(),
        "minRevision": min_revision.unwrap_or_default(),
        "maxRevision": max_revision.unwrap_or_default(),
        "clientBuild": client_build.unwrap_or_default(),
        "requiredCapabilities": required,
    });
    let host_for_worker = host_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        crate::remote_connections::prepare_blocking(&host_for_worker, client)
    }).await;
    match result {
        Ok(Ok(value)) => (axum::http::StatusCode::OK, no_store, axum::Json(value)).into_response(),
        Ok(Err(error)) if error.starts_with("Remote connection requires attention:") => {
            let state = crate::remote_connections::state(&host_id);
            (axum::http::StatusCode::CONFLICT, no_store, axum::Json(json!({
                "code":"REMOTE_CONNECTION_REQUIRES_ATTENTION",
                "state":state,
            }))).into_response()
        }
        _ => (axum::http::StatusCode::SERVICE_UNAVAILABLE, no_store, "Remote host unavailable").into_response(),
    }
}

async fn remote_renderer_upgrade(
    ws: WebSocketUpgrade,
    axum::extract::Path(host_id): axum::extract::Path<String>,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    let cfg = crate::native_ws::config();
    let request_origin = crate::native_ws::request_origin(&headers);
    let raw_origin = headers.get("origin").and_then(|value| value.to_str().ok());
    if crate::native_ws::should_reject_untrusted_request_origin(raw_origin, &request_origin, cfg) {
        return (axum::http::StatusCode::FORBIDDEN, "Forbidden").into_response();
    }
    if let Err(status) = remote_owner_authorized(&headers, &uri) {
        return (status, "Unauthorized").into_response();
    }
    let query = uri.query().unwrap_or_default();
    let mut attachment_id = None;
    let mut server_instance = None;
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, raw) = pair.split_once('=').unwrap_or((pair, ""));
        let value = urlencoding_decode(raw);
        match key {
            "remoteAttachment" => attachment_id = Some(value),
            "x-synara-server-instance" => server_instance = Some(value),
            _ => {}
        }
    }
    let (Some(attachment_id), Some(server_instance)) = (attachment_id, server_instance) else {
        return (axum::http::StatusCode::NOT_FOUND, "Remote stream negotiation expired").into_response();
    };
    if !crate::remote_connections::attachment_valid(&host_id, &attachment_id, Some(&server_instance)) {
        return (axum::http::StatusCode::NOT_FOUND, "Remote stream negotiation expired").into_response();
    }
    ws.on_upgrade(move |socket| async move {
        let _ = crate::remote_connections::attach(&host_id, &attachment_id, Some(&server_instance), socket).await;
    })
}

async fn device_frames_upgrade(
    State(original): State<Original>,
    ws: WebSocketUpgrade,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    if reject_untrusted_websocket_origin(&headers) {
        return (axum::http::StatusCode::FORBIDDEN, "Forbidden").into_response();
    }
    if let Err(status) = authenticate_rpc_websocket_upgrade(&headers, &uri) {
        return websocket_auth_not_required_response(status);
    }
    if std::env::var("SYNARA_NO_UPSTREAM").is_err() {
        return upgrade(State(original), ws, uri, headers, false).await;
    }
    let params: std::collections::HashMap<String, String> = uri
        .query()
        .map(|q| {
            q.split('&')
                .filter_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    Some((parts.next()?.to_string(), parts.next().unwrap_or("").to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    crate::device_stream::upgrade(ws, axum::extract::Query(params)).await
}

async fn computer_frames_upgrade(
    State(original): State<Original>,
    ws: WebSocketUpgrade,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    if reject_untrusted_websocket_origin(&headers) {
        return (axum::http::StatusCode::FORBIDDEN, "Forbidden").into_response();
    }
    if let Err(status) = authenticate_rpc_websocket_upgrade(&headers, &uri) {
        return websocket_auth_not_required_response(status);
    }
    if std::env::var("SYNARA_NO_UPSTREAM").is_err() {
        return upgrade(State(original), ws, uri, headers, false).await;
    }
    let params: std::collections::HashMap<String, String> = uri
        .query()
        .map(|q| {
            q.split('&')
                .filter_map(|pair| {
                    let mut parts = pair.splitn(2, '=');
                    Some((parts.next()?.to_string(), parts.next().unwrap_or("").to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    crate::computer_frames::upgrade(ws, axum::extract::Query(params)).await
}


fn compatibility_error_body(
    action: &str,
    message: &str,
    code: &str,
) -> serde_json::Value {
    json!({
        "_tag": "WsCompatibilityError",
        "message": message,
        "code": code,
        "retryable": false,
        "action": action,
        "serverBuild": crate::native_ws::SERVER_BUILD,
        "protocolEpoch": crate::native_ws::PROTOCOL_EPOCH,
        "minRevision": crate::native_ws::MIN_REVISION,
        "maxRevision": crate::native_ws::MAX_REVISION,
    })
}

fn compatibility_refusal(uri: &Uri) -> Option<Response> {
    let params: std::collections::HashMap<String, String> = uri
        .query()
        .map(|query| {
            qstring_pairs(query).into_iter().collect()
        })
        .unwrap_or_default();
    let epoch = params
        .get("x-synara-protocol-epoch")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(-1);
    let revision = params
        .get("x-synara-protocol-revision")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(-1);
    let client_build = params
        .get("x-synara-client-build")
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    let expected_instance = params
        .get("x-synara-server-instance")
        .map(|value| value.trim().to_string())
        .unwrap_or_default();
    let (action, message, code) = if epoch != crate::native_ws::PROTOCOL_EPOCH {
        ("reload", "Feature RPC rejected before compatible WebSocket protocol negotiation.", "WS_NEGOTIATION_REQUIRED")
    } else if revision < crate::native_ws::MIN_REVISION || revision > crate::native_ws::MAX_REVISION {
        if revision < crate::native_ws::MIN_REVISION {
            ("update-client", "Feature RPC rejected because its negotiated protocol revision is unsupported.", "WS_PROTOCOL_INCOMPATIBLE")
        } else {
            ("update-server", "Feature RPC rejected because its negotiated protocol revision is unsupported.", "WS_PROTOCOL_INCOMPATIBLE")
        }
    } else if client_build.is_empty() {
        ("reload", "Feature RPC rejected because the client build identity is missing.", "WS_NEGOTIATION_REQUIRED")
    } else if expected_instance != crate::native_ws::server_instance_id() {
        ("reload", "The Synara server restarted during WebSocket negotiation. Reload to reconnect to the new server generation.", "WS_SERVER_GENERATION_CHANGED")
    } else {
        return None;
    };
    let body = compatibility_error_body(action, message, code).to_string();
    Some(Response::builder()
        .status(axum::http::StatusCode::UPGRADE_REQUIRED)
        .header(axum::http::header::CONTENT_TYPE, "application/json")
        .header(axum::http::header::CACHE_CONTROL, "no-store")
        .body(axum::body::Body::from(body))
        .expect("compatibility refusal"))
}

fn qstring_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = urlencoding_decode(parts.next()?);
            let value = urlencoding_decode(parts.next().unwrap_or(""));
            Some((key, value))
        })
        .collect()
}

pub(crate) fn urlencoding_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = |byte: u8| -> Option<u8> {
                match byte {
                    b'0'..=b'9' => Some(byte - b'0'),
                    b'a'..=b'f' => Some(byte - b'a' + 10),
                    b'A'..=b'F' => Some(byte - b'A' + 10),
                    _ => None,
                }
            };
            if let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2])) {
                out.push((high << 4) | low);
                index += 3;
                continue;
            }
        }
        out.push(if bytes[index] == b'+' { b' ' } else { bytes[index] });
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

async fn upgrade(
    State(original): State<Original>,
    ws: WebSocketUpgrade,
    uri: Uri,
    headers: HeaderMap,
    intercept: bool,
) -> Response {
    if reject_untrusted_websocket_origin(&headers) {
        return (axum::http::StatusCode::FORBIDDEN, "Forbidden").into_response();
    }
    // `/ws` is the feature socket negotiated by `/ws/negotiate`; the legacy
    // bootstrap socket is itself the negotiation transport and stays exempt.
    // Frame sockets have their own query shape and are never compatibility-gated.
    if uri.path() == "/ws" && let Some(refusal) = compatibility_refusal(&uri) {
        return refusal;
    }
    let authenticated_session = if authenticated_websocket_path(uri.path()) {
        match authenticate_rpc_websocket_upgrade(&headers, &uri) {
            Ok(session) => session,
            Err(status) => return websocket_auth_not_required_response(status),
        }
    } else {
        None
    };
    ws.on_upgrade(move |socket| relay(socket, original, uri, headers, intercept, authenticated_session))
}

fn owner_only_rpc(tag: &str, payload: &serde_json::Value) -> bool {
    tag.starts_with("account.")
        || tag.starts_with("hosts.")
        || (tag == "server.updateSettings"
            && (payload.get("legacyProviders").is_some()
                || payload.get("legacyProviderInstances").is_some()))
}

fn strip_provider_account_identity(value: &mut serde_json::Value) {
    let Some(providers) = value.get_mut("providers").and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    for status in providers {
        if let Some(object) = status.as_object_mut() {
            object.remove("accountEmail");
            object.remove("accountOrganization");
        }
    }
}

/// `todo.*` requests served by the native store; the string check is a fast
/// prefilter so ordinary traffic still takes the relay path unparsed.
/// `Some(None)` marks a handled stream request (the reply is the chunk flow).
/// Crate-visible wrapper so the remote tunnel serving loop reuses the exact
/// native dispatch instead of duplicating three hundred lines of routing.
pub(crate) async fn intercept_todo_pub(
    text: &str,
    todos: &std::sync::Arc<crate::todo::TodoStore>,
    native_tx: &tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    streams: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>,
    >,
    desktop_update_authority: Option<&str>,
    session_role: &str,
) -> Option<Option<ClientMessage>> {
    intercept_todo(text, todos, native_tx, streams, desktop_update_authority, session_role).await
}

async fn intercept_todo(
    text: &str,
    todos: &std::sync::Arc<crate::todo::TodoStore>,
    native_tx: &tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    streams: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>,
    >,
    desktop_update_authority: Option<&str>,
    session_role: &str,
) -> Option<Option<ClientMessage>> {
    if !text.contains("\"todo.")
        && !text.contains("\"server.get")
        && !text.contains("\"shell.openInEditor\"")
        && !text.contains("\"git.")
        && !text.contains("\"editor.intelligence")
        && !text.contains("\"agentGateway.call")
        && !text.contains("\"filesystem.browse")
        && !text.contains("server.listWorktrees")
        && !text.contains("projects.listDirectories")
        && !text.contains("projects.discoverScripts")
        && !text.contains("projects.prewarmSearchIndex")
        && !text.contains("projects.searchEntries")
        && !text.contains("projects.searchLocalEntries")
        && !text.contains("projects.resolveWorkspaceFileReferences")
        && !text.contains("projects.resolveOutOfRootFileReference")
        && !text.contains("projects.readFile")
        && !text.contains("projects.writeFile")
        && !text.contains("projects.formatText")
        && !text.contains("projects.formatFile")
        && !text.contains("projects.searchContent")
        && !text.contains("projects.createLocalFilePreviewGrant")
        && !text.contains("projects.listDevServers")
        && !text.contains("projects.runDevServer")
        && !text.contains("projects.stopDevServer")
        && !text.contains("projects.subscribeDevServerEvents")
        && !text.contains("projects.subscribeFileChange")
        && !text.contains("projects.provisionFromGitHub")
        && !text.contains("server.generateAutomationIntent")
        && !text.contains("server.generateComposerPrediction")
        && !text.contains("server.generateThreadRecap")
        && !text.contains("server.consumeCodexResetCredit")
        && !text.contains("server.listProviderUsage")
        && !text.contains("server.getProviderUsageSnapshot")
        && !text.contains("server.upsertKeybinding")
        && !text.contains("server.editKeybindings")
        && !text.contains("server.listLocalServers")
        && !text.contains("server.stopLocalServer")
        && !text.contains("server.getDiagnostics")
        && !text.contains("server.desktopUpdateStatus")
        && !text.contains("server.desktopUpdate")
        && !text.contains("server.getConfig")
        && !text.contains("server.getSettings")
        && !text.contains("server.updateSettings")
        && !text.contains("server.subscribeSettings")
        && !text.contains("server.subscribeConfig")
        && !text.contains("server.subscribeProviderStatuses")
        && !text.contains("server.subscribeLifecycle")
        && !text.contains("server.subscribeKeepAwake")
        && !text.contains("stats.")
        && !text.contains("terminal.")
        && !text.contains("orchestration.")
        && !text.contains("githubInbox.")
        && !text.contains("pullRequests.")
        && !text.contains("automation.")
        && !text.contains("studio.")
        && !text.contains("projectAgent.")
        && !text.contains("provider.")
        && !text.contains("computer.")
        && !text.contains("account.")
        && !text.contains("hosts.")
        && !text.contains("device.")
        && !text.contains("editor.")
        && !text.contains("ExternalMcp")
        && !text.contains("server.readThreadDiagnostics")
        && !text.contains("server.listMcp")
        && !text.contains("server.getMcp")
        && !text.contains("server.connectMcp")
        && !text.contains("server.disconnectMcp")
        && !text.contains("server.resolveMcp")
        && !text.contains("server.transcribeVoice")
        && !text.contains("server.prewarmVoice")
        && !text.contains("server.refreshProviders")
        && !text.contains("server.updateProvider")
    {
        return None;
    }
    let msg: serde_json::Value = serde_json::from_str(text).ok()?;
    if msg.get("_tag")?.as_str()? != "Request" {
        return None;
    }
    let tag = msg.get("tag")?.as_str()?;
    if !tag.starts_with("todo.")
        && tag != "server.getEnvironment"
        && tag != "server.getRuntimeStatus"
        && tag != "shell.openInEditor"
        && !crate::git_domain::handles(tag)
        && !crate::git_pr::handles(tag)
        && !crate::git_handoff::handles(tag)
        && tag != "filesystem.browse"
        && tag != "server.listWorktrees"
        && tag != "projects.listDirectories"
        && tag != "projects.discoverScripts"
        && tag != "projects.prewarmSearchIndex"
        && tag != "projects.searchEntries"
        && tag != "projects.searchLocalEntries"
        && tag != "projects.resolveWorkspaceFileReferences"
        && tag != "projects.resolveOutOfRootFileReference"
        && tag != "projects.readFile"
        && tag != "projects.writeFile"
        && tag != "projects.formatText"
        && tag != "projects.formatFile"
        && tag != "projects.searchContent"
        && tag != "projects.createLocalFilePreviewGrant"
        && tag != "projects.listDevServers"
        && tag != "projects.runDevServer"
        && tag != "projects.stopDevServer"
        && tag != "projects.subscribeDevServerEvents"
        && tag != "projects.subscribeFileChange"
        && !crate::projects_provision::handles(tag)
        && !crate::mcp_connectors::handles(tag)
        && !crate::provider_usage::handles(tag)
        && !crate::voice_transcription::handles(tag)
        && tag != "agentGateway.call"
        && !crate::textgen::handles(tag)
        && tag != "server.upsertKeybinding"
        && tag != "server.editKeybindings"
        && tag != "server.listLocalServers"
        && tag != "server.stopLocalServer"
        && tag != "server.getDiagnostics"
        && tag != "server.desktopUpdateStatus"
        && tag != "server.desktopUpdate"
        && tag != "server.getConfig"
        && tag != "server.getSettings"
        && tag != "server.updateSettings"
        && tag != "server.subscribeSettings"
        && tag != "server.subscribeConfig"
        && tag != "server.subscribeProviderStatuses"
        && tag != "server.subscribeLifecycle"
        && tag != "server.subscribeKeepAwake"
        && tag != "orchestration.getSnapshot"
        && tag != "orchestration.getShellSnapshot"
        && tag != "orchestration.getThreadDetailSnapshot"
        && tag != "orchestration.searchThreads"
        && tag != "orchestration.subscribeShell"
        && tag != "orchestration.subscribeThread"
        && tag != "orchestration.subscribeDomainEvents"
        && !crate::orchestration_reads::handles(tag)
        && !crate::orchestration_engine::handles(tag)
        && !crate::provider_cache::handles(tag)
        && !crate::skills_catalog::handles(tag)
        && !crate::project_imports::handles(tag)
        && !crate::computer_audit::handles(tag)
        && !tag.starts_with("computer.") && !tag.starts_with("device.")
        && !crate::account::handles(tag)
        && !crate::hosts::handles(tag)
        && !crate::device::handles(tag)
        && !crate::editor::handles(tag)
        && !crate::editor_intelligence::handles(tag)
        && !crate::editor_debug::handles(tag)
        && !tag.starts_with("githubInbox.")
        && !crate::pull_requests::handles(tag)
        && !crate::automation::handles(tag)
        && tag != "automation.subscribe"
        && tag != "projectAgent.subscribe"
        && !crate::studio::handles(tag)
        && !crate::project_agent::handles(tag)
        && !tag.starts_with("stats.")
        && !tag.starts_with("terminal.")
    {
        return None;
    }
    let request_id = msg
        .get("id")
        .cloned()
        .unwrap_or(serde_json::Value::String(String::new()));
    let payload = msg
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    if session_role != "owner" && owner_only_rpc(tag, &payload) {
        let reply = serde_json::json!({
            "_tag": "Exit", "requestId": request_id,
            "exit": {"_tag": "Failure", "cause": [{"_tag": "Fail", "error": {
                "_tag": "WsRpcError",
                "message": "Owner authorization is required for this operation.",
                "code": null,
                "retryable": false
            }}]}
        });
        return Some(Some(ClientMessage::Text(reply.to_string().into())));
    }
    if tag == "git.createDetachedWorktree" {
        // Same streaming shape as runStackedAction: phase progress chunks, the
        // terminal `completed` event carries the worktree result.
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let (ptx, mut prx) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
            let payload3 = payload.clone();
            let join = tokio::task::spawn_blocking(move || {
                let mut forward = move |event: serde_json::Value| {
                    let _ = ptx.send(event);
                };
                let cwd = payload3
                    .get("cwd")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                crate::git_pr::create_detached_worktree_streaming(&cwd, &payload3, &mut forward)
            });
            let tx2 = tx.clone();
            let rid2 = rid.clone();
            let forwarder = tokio::spawn(async move {
                while let Some(event) = prx.recv().await {
                    let chunk = serde_json::json!({
                        "_tag": "Chunk", "requestId": rid2, "values": [event],
                    });
                    if tx2.send(ClientMessage::Text(chunk.to_string().into())).is_err() {
                        break;
                    }
                }
            });
            let result = join.await;
            let _ = forwarder.await;
            let reply = match result {
                Ok(Ok(_value)) => serde_json::json!({
                    "_tag": "Exit", "requestId": rid,
                    "exit": {"_tag": "Success", "value": null},
                }),
                Ok(Err(e)) => serde_json::json!({
                    "_tag": "Exit", "requestId": rid,
                    "exit": {"_tag": "Failure", "cause": [{"_tag": "Fail", "error": error_json(e)}]},
                }),
                Err(_) => serde_json::json!({
                    "_tag": "Exit", "requestId": rid,
                    "exit": {"_tag": "Failure", "cause": [{"_tag": "Fail", "error": error_json(crate::todo::WsErr("worktree setup task panicked".to_string(), None))}]},
                }),
            };
            let _ = tx.send(ClientMessage::Text(reply.to_string().into()));
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }

    if tag == "git.runStackedAction" {
        // Upstream serves this RPC as a stream of GitActionProgressEvent chunks
        // through a keyed action registry: a client that reconnects and resumes
        // attaches to the in-flight or completed action instead of duplicating
        // the mutation. The pipeline runs detached in spawn_blocking; progress
        // events flow through the registry into Chunk frames on this socket.
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let payload2 = payload.clone();
        let task = tokio::spawn(async move {
            let action_id = payload2
                .get("actionId")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let fp = crate::git_action_registry::fingerprint(&payload2);
            let resume = payload2.get("resume").and_then(|v| v.as_bool()).unwrap_or(false);
            use crate::git_action_registry::Attach;
            let fail_exit = |message: &str| serde_json::json!({
                "_tag": "Exit", "requestId": rid,
                "exit": {"_tag": "Failure", "cause": [{"_tag": "Fail", "error": {
                    "_tag": "WsRpcError", "message": message, "code": null, "retryable": false,
                }}]},
            });
            let (history, mut rx) = match crate::git_action_registry::attach(&action_id, &fp, resume) {
                Attach::New { rx } => {
                    // Spawn the mutation itself; it publishes every progress
                    // event into the registry, which fans out to subscribers.
                    let payload3 = payload2.clone();
                    let aid = action_id.clone();
                    tokio::task::spawn_blocking(move || {
                        let aid2 = aid.clone();
                        let mut forward = move |event: serde_json::Value| {
                            crate::git_action_registry::publish(&aid2, &event);
                        };
                        let cwd = payload3
                            .get("cwd")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let result =
                            crate::git_pr::run_stacked_action_streaming(&cwd, &payload3, &mut forward);
                        if let Err(e) = &result {
                            crate::git_action_registry::publish(
                                &aid,
                                &serde_json::json!({
                                    "kind": "action_failed", "phase": null, "message": e.0,
                                }),
                            );
                            crate::git_action_registry::finish(&aid, Err(e.0.clone()));
                        } else {
                            crate::git_action_registry::finish(&aid, Ok(()));
                        }
                    });
                    (Vec::new(), rx)
                }
                Attach::Existing {
                    events,
                    outcome: _,
                    subscriber,
                } => match subscriber {
                    Some(rx) => (events, rx),
                    None => (events, {
                        // Completed action: an empty closed receiver — the
                        // terminal event is already inside the replayed history.
                        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
                        rx
                    }),
                },
                Attach::Mismatch => {
                    let _ = tx.send(ClientMessage::Text(
                        fail_exit("Git action ID was reused with different inputs.")
                            .to_string()
                            .into(),
                    ));
                    return;
                }
                Attach::ResumeMissing => {
                    let _ = tx.send(ClientMessage::Text(
                        fail_exit(
                            "The Git action result is unavailable. The server may have restarted or the request may not have arrived. Check the repository status before trying again.",
                        )
                        .to_string()
                        .into(),
                    ));
                    return;
                }
            };
            // Replay the recorded history, then forward live events. The
            // terminal action_finished/action_failed event produces the Exit.
            let mut replay = history.into_iter();
            let mut pending: Vec<serde_json::Value> = replay.by_ref().collect();
            loop {
                let event = if !pending.is_empty() {
                    pending.remove(0)
                } else {
                    match rx.recv().await {
                        Some(event) => event,
                        None => break,
                    }
                };
                let kind = event.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                let terminal = kind == "action_finished" || kind == "action_failed";
                let fail_message = if kind == "action_failed" {
                    event
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Git action failed.")
                        .to_string()
                } else {
                    String::new()
                };
                let chunk = serde_json::json!({
                    "_tag": "Chunk", "requestId": rid, "values": [event],
                });
                if tx.send(ClientMessage::Text(chunk.to_string().into())).is_err() {
                    break;
                }
                if terminal {
                    let reply = if kind == "action_finished" {
                        serde_json::json!({
                            "_tag": "Exit", "requestId": rid,
                            "exit": {"_tag": "Success", "value": null},
                        })
                    } else {
                        serde_json::json!({
                            "_tag": "Exit", "requestId": rid,
                            "exit": {"_tag": "Failure", "cause": [{"_tag": "Fail", "error": {
                                "_tag": "WsRpcError", "message": fail_message, "code": null, "retryable": false,
                            }}]},
                        })
                    };
                    let _ = tx.send(ClientMessage::Text(reply.to_string().into()));
                    break;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }

    if tag == "device.subscribeEvents" {
        // Upstream answers `Stream.never` when no device engine can run: the
        // subscription stays open and silent, since an instantly-completing
        // stream makes the client treat the socket as a zombie and reconnect.
        let task = tokio::spawn(async move {
            std::future::pending::<()>().await;
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "projects.subscribeDevServerEvents" {
        let mut rx = crate::projects_devserver::subscribe();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let chunk = serde_json::json!({
                "_tag": "Chunk", "requestId": rid,
                "values": [crate::projects_devserver::current_snapshot()],
            });
            if tx
                .send(ClientMessage::Text(chunk.to_string().into()))
                .is_err()
            {
                return;
            }
            while let Ok(event) = rx.recv().await {
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "projects.subscribeFileChange" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let cwd = payload
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(std::path::PathBuf::from);
        let rel = payload
            .get("relativePath")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let (Some(cwd), Some(rel)) = (cwd, rel) else {
            let exit = serde_json::json!({
                "_tag": "Failure",
                "cause": [{
                    "_tag": "Fail",
                    "error": error_json(crate::todo::WsErr(
                        "cwd and relativePath are required".to_string(),
                        Some("VALIDATION"),
                    )),
                }],
            });
            let reply = serde_json::json!({
                "_tag": "Exit", "requestId": request_id, "exit": exit,
            });
            return Some(Some(ClientMessage::Text(reply.to_string().into())));
        };
        let task = tokio::spawn(async move {
            let tx2 = tx.clone();
            let rid2 = rid.clone();
            let _ = crate::projects_devserver::poll_watch(cwd, rel, move |event| {
                let chunk = serde_json::json!({
                    "_tag": "Chunk", "requestId": rid2, "values": [event],
                });
                tx2.send(ClientMessage::Text(chunk.to_string().into()))
                    .is_ok()
            })
            .await;
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "computer.subscribeEvents" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let mut rx = crate::computer_native::action_events();
            let send = |tx: &tokio::sync::mpsc::UnboundedSender<ClientMessage>,
                        rid: &serde_json::Value,
                        ev: serde_json::Value|
             -> bool {
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [ev]});
                tx.send(ClientMessage::Text(chunk.to_string().into())).is_ok()
            };
            let mut last_sig = String::new();
            if let Ok(w) = crate::computer_native::windows_value() {
                last_sig = w.to_string();
                if !send(&tx, &rid, serde_json::json!({"type": "computer.windows-changed", "windows": w})) {
                    return;
                }
            }
            let mut tick = tokio::time::interval(std::time::Duration::from_millis(800));
            tick.tick().await;
            loop {
                tokio::select! {
                    _ = tick.tick() => {
                        if let Ok(w) = crate::computer_native::windows_value() {
                            let sig = w.to_string();
                            if sig != last_sig {
                                last_sig = sig;
                                if !send(&tx, &rid, serde_json::json!({"type": "computer.windows-changed", "windows": w})) {
                                    return;
                                }
                            }
                        }
                    }
                    ev = rx.recv() => {
                        if let Ok(ev) = ev {
                            if !send(&tx, &rid, ev) {
                                return;
                            }
                        }
                    }
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "orchestration.subscribeShell" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let mut last_sequence = -1i64;
            loop {
                let sequence = crate::orchestration_snapshot::current_snapshot_sequence().unwrap_or(-1);
                // The projection sequence is the dirty bit. Rebuilding and
                // sending a full shell on a wall-clock-only `updatedAt` floods
                // cold boot with multi-megabyte duplicates every 1.2 seconds.
                if sequence != last_sequence {
                    if let Ok(snapshot) = crate::orchestration_snapshot::get_shell_snapshot() {
                        if !snapshot.is_null() {
                            last_sequence = snapshot
                                .get("snapshotSequence")
                                .and_then(|value| value.as_i64())
                                .unwrap_or(sequence);
                            let chunk = serde_json::json!({
                                "_tag": "Chunk", "requestId": rid,
                                "values": [{"kind": "snapshot", "snapshot": snapshot}],
                            });
                            if tx
                                .send(ClientMessage::Text(chunk.to_string().into()))
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "orchestration.subscribeThread" {
        let thread_id = payload
            .get("threadId")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let Some(thread_id) = thread_id else {
            let exit = serde_json::json!({
                "_tag": "Failure",
                "cause": [{
                    "_tag": "Fail",
                    "error": error_json(crate::todo::WsErr(
                        "threadId is required".to_string(),
                        Some("VALIDATION"),
                    )),
                }],
            });
            let reply = serde_json::json!({
                "_tag": "Exit", "requestId": request_id, "exit": exit,
            });
            return Some(Some(ClientMessage::Text(reply.to_string().into())));
        };
        let after_seq = payload.get("afterSequence").and_then(|v| v.as_i64());
        let batch_replay = payload
            .get("batchReplay")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            fn send(tx: &tokio::sync::mpsc::UnboundedSender<ClientMessage>, chunk: serde_json::Value) -> bool {
                tx.send(ClientMessage::Text(chunk.to_string().into())).is_ok()
            }
            let mut last_seen: i64 = 0;
            if let Some(after) = after_seq {
                // Cursor resume: replay the gap instead of a full snapshot.
                match crate::orchestration_reads::thread_events_after(&thread_id, after) {
                    Ok(events) => {
                        last_seen = events.last().map(|(s, _)| *s).unwrap_or(after);
                        if batch_replay {
                            let items: Vec<serde_json::Value> =
                                events.into_iter().map(|(_, e)| e).collect();
                            let chunk = serde_json::json!({
                                "_tag": "Chunk", "requestId": rid,
                                "values": [{"kind": "replay", "events": items}],
                            });
                            if !send(&tx, chunk) {
                                return;
                            }
                        } else {
                            for (_, event) in events {
                                let chunk = serde_json::json!({
                                    "_tag": "Chunk", "requestId": rid,
                                    "values": [{"kind": "event", "event": event}],
                                });
                                if !send(&tx, chunk) {
                                    return;
                                }
                            }
                        }
                    }
                    Err(_) => {
                        last_seen = after;
                    }
                }
            } else {
                // Snapshot first, then live events past its sequence.
                // A missing thread yields no snapshot at all: emitting
                // `snapshot: null` violates the stream schema and the client
                // drops the whole socket as a protocol defect.
                if let Ok(snapshot) = crate::orchestration_snapshot::get_thread_detail_snapshot(
                    &serde_json::json!({"threadId": thread_id}),
                ) {
                    if !snapshot.is_null() {
                        last_seen = snapshot
                            .get("snapshotSequence")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let chunk = serde_json::json!({
                            "_tag": "Chunk", "requestId": rid,
                            "values": [{"kind": "snapshot", "snapshot": snapshot}],
                        });
                        if !send(&tx, chunk) {
                            return;
                        }
                    }
                }
            }
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                match crate::orchestration_reads::thread_events_after(&thread_id, last_seen) {
                    Ok(events) => {
                        for (seq, event) in events {
                            last_seen = last_seen.max(seq);
                            let chunk = serde_json::json!({
                                "_tag": "Chunk", "requestId": rid,
                                "values": [{"kind": "event", "event": event}],
                            });
                            if !send(&tx, chunk) {
                                return;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "orchestration.subscribeDomainEvents" {
        let mut last_seen = payload
            .get("afterSequence")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            loop {
                match crate::orchestration_reads::domain_events_after(last_seen) {
                    Ok(events) => {
                        for (seq, event) in events {
                            last_seen = last_seen.max(seq);
                            let chunk = serde_json::json!({
                                "_tag": "Chunk", "requestId": rid,
                                "values": [event],
                            });
                            if tx
                                .send(ClientMessage::Text(chunk.to_string().into()))
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                    Err(_) => {}
                }
                tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "automation.subscribe" {
        let project_id = payload
            .get("projectId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let include_archived = payload
            .get("includeArchived")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let mut last: Option<String> = None;
            loop {
                match crate::automation::snapshot_event(project_id.as_deref(), include_archived) {
                    Ok(event) => {
                        let enc = event.to_string();
                        if last.as_deref() != Some(enc.as_str()) {
                            last = Some(enc);
                            let chunk = serde_json::json!({
                                "_tag": "Chunk", "requestId": rid,
                                "values": [event],
                            });
                            if tx
                                .send(ClientMessage::Text(chunk.to_string().into()))
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                    Err(_) => {}
                }
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "projectAgent.subscribe" {
        let project_id = payload
            .get("projectId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let mut last: Option<String> = None;
            loop {
                match crate::project_agent::snapshot_event(&project_id) {
                    Ok(event) => {
                        let enc = event.to_string();
                        if last.as_deref() != Some(enc.as_str()) {
                            last = Some(enc);
                            let chunk = serde_json::json!({
                                "_tag": "Chunk", "requestId": rid,
                                "values": [event],
                            });
                            if tx
                                .send(ClientMessage::Text(chunk.to_string().into()))
                                .is_err()
                            {
                                return;
                            }
                        }
                    }
                    Err(_) => {}
                }
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "terminal.subscribeEvents" {
        let mut rx = crate::terminal::subscribe();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "server.subscribeSettings" {
        let (snapshot, mut rx) = crate::server_settings::subscribe();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            // Upstream subscribeServerSettings streams raw {settings} objects
            // (the typed settingsUpdated envelope belongs to subscribeConfig).
            let chunk =
                serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [{"settings": snapshot}]});
            if tx
                .send(ClientMessage::Text(chunk.to_string().into()))
                .is_err()
            {
                return;
            }
            while let Ok(settings) = rx.recv().await {
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [{"settings": settings}]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "server.subscribeLifecycle" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let events = crate::server_lifecycle::snapshot_events();
            let chunk = serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": events});
            let _ = tx.send(ClientMessage::Text(chunk.to_string().into()));
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "server.subscribeConfig" {
        let mut rx = crate::server_settings::changes();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let event = serde_json::json!({
                "type": "snapshot",
                "config": crate::server_lifecycle::get_config(),
            });
            let chunk = serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
            if tx
                .send(ClientMessage::Text(chunk.to_string().into()))
                .is_err()
            {
                return;
            }
            while let Ok(settings) = rx.recv().await {
                let event = serde_json::json!({"type": "settingsUpdated", "payload": {"settings": settings}});
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "server.subscribeProviderStatuses" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let hide_identity = session_role != "owner";
        let task = tokio::spawn(async move {
            // Serve the cached provider-status snapshots upstream writes to
            // userdata/provider-status/*.json; re-emit when they change.
            let mut last = String::new();
            loop {
                let mut providers = crate::provider_cache::provider_statuses();
                if hide_identity {
                    for status in &mut providers {
                        if let Some(object) = status.as_object_mut() {
                            object.remove("accountEmail");
                            object.remove("accountOrganization");
                        }
                    }
                }
                let text = serde_json::to_string(&providers).unwrap_or_default();
                if text != last {
                    last = text;
                    let chunk = serde_json::json!({
                        "_tag": "Chunk", "requestId": rid,
                        "values": [{"providers": providers}],
                    });
                    if tx
                        .send(ClientMessage::Text(chunk.to_string().into()))
                        .is_err()
                    {
                        return;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "server.subscribeKeepAwake" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let mut last_state: Option<String> = None;
            let send_state = |state_hint: &mut Option<String>| -> Option<String> {
                let state = crate::server_lifecycle::keep_awake_state();
                let comparable = state.to_string();
                if comparable == *state_hint.as_ref().unwrap_or(&String::new()) {
                    return None;
                }
                *state_hint = Some(comparable);
                Some(serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [{"keepAwake": state}]}).to_string())
            };
            if let Some(chunk) = send_state(&mut last_state) {
                if tx.send(ClientMessage::Text(chunk.into())).is_err() {
                    return;
                }
            }
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if let Some(chunk) = send_state(&mut last_state) {
                    if tx.send(ClientMessage::Text(chunk.into())).is_err() {
                        return;
                    }
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    if tag == "todo.subscribe" {
        // Snapshot first, then every change — mirrors `streamChanges`
        // (subscribe-then-read ordering is preserved by the store lock).
        let todos = todos.clone();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let mut rx = todos.subscribe();
            if let Ok(serde_json::Value::Object(mut snapshot)) =
                todos.handle("todo.list", serde_json::Value::Null)
            {
                let event = serde_json::json!({"type": "snapshot", "todos": snapshot.remove("todos").unwrap_or(serde_json::json!([]))});
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
            while let Ok(event) = rx.recv().await {
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(rpc_id_key(&request_id), task);
        }
        return Some(None);
    }
    let outcome = if tag == "filesystem.browse" {
        crate::fs_browse::browse(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.listWorktrees" {
        crate::list_worktrees::list_worktrees()
            .map(Some)
            .map_err(error_json)
    } else if tag.starts_with("terminal.") {
        crate::terminal::handle(tag, &payload).map_err(error_json)
    } else if tag == "server.upsertKeybinding" {
        crate::server_keybindings::upsert_keybinding(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.editKeybindings" {
        crate::server_keybindings::edit_keybindings(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.getDiagnostics" {
        crate::server_diagnostics::get_diagnostics()
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.listLocalServers" {
        crate::server_local_servers::list_local_servers()
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.stopLocalServer" {
        crate::server_local_servers::stop_local_server(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.desktopUpdateStatus" {
        crate::server_desktop_update::desktop_update_status(desktop_update_authority)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.desktopUpdate" {
        crate::server_desktop_update::desktop_update(&payload, desktop_update_authority)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.getConfig" {
        Ok(Some(crate::server_lifecycle::get_config()))
    } else if tag.starts_with("stats.") {
        crate::stats::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.getSettings" {
        crate::server_settings::get_settings(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.updateSettings" {
        crate::server_settings::update_settings(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag.starts_with("projects.readFile")
        || tag == "projects.writeFile"
        || tag == "projects.formatText"
        || tag == "projects.formatFile"
        || tag == "projects.searchContent"
        || tag == "projects.createLocalFilePreviewGrant"
    {
        crate::projects_fileops::handle(tag, &payload).map_err(error_json)
    } else if tag == "projects.listDevServers" {
        crate::projects_devserver::list_dev_servers()
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.runDevServer" {
        crate::projects_devserver::run_dev_server(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.stopDevServer" {
        crate::projects_devserver::stop_dev_server(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.resolveWorkspaceFileReferences" {
        crate::projects_refs::resolve_workspace_file_references(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.resolveOutOfRootFileReference" {
        crate::projects_refs::resolve_out_of_root_file_reference(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.prewarmSearchIndex" {
        crate::projects_search::prewarm_search_index(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.searchEntries" {
        crate::projects_search::search_entries(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.searchLocalEntries" {
        crate::projects_search::search_local_entries(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.discoverScripts" {
        crate::projects_fs::discover_scripts(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.listDirectories" {
        crate::projects_fs::list_directories(&payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::git_domain::handles(tag)
        || crate::git_pr::handles(tag)
        || crate::git_handoff::handles(tag)
    {
        let result = if crate::git_pr::handles(tag) {
            crate::git_pr::handle(tag, &payload)
        } else if crate::git_handoff::handles(tag) {
            crate::git_handoff::handle(&payload).map(Some)
        } else {
            crate::git_domain::handle(tag, &payload)
        };
        match result {
            Ok(value) => Ok(value),
            Err(e) => Err(error_json(e)),
        }
    } else if tag == "server.getEnvironment" {
        crate::server_env::get_environment(&crate::todo::state_dir())
            .map_err(crate::todo::err_pub)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.getRuntimeStatus" {
        Ok(Some(crate::runtime_status::get_runtime_status()))
    } else if tag == "shell.openInEditor" {
        crate::shell_editor::open_in_editor(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "orchestration.getSnapshot" {
        crate::orchestration_snapshot::get_snapshot()
            .map(Some)
            .map_err(error_json)
    } else if tag == "orchestration.getShellSnapshot" {
        crate::orchestration_snapshot::get_shell_snapshot()
            .map(Some)
            .map_err(error_json)
    } else if tag == "orchestration.getThreadDetailSnapshot" {
        crate::orchestration_snapshot::get_thread_detail_snapshot(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "orchestration.searchThreads" {
        crate::orchestration_snapshot::search_threads(&payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::orchestration_engine::handles(tag) {
        crate::orchestration_engine::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::orchestration_reads::handles(tag) {
        crate::orchestration_reads::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::skills_catalog::handles(tag) {
        crate::skills_catalog::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::project_imports::handles(tag) {
        crate::project_imports::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::computer_audit::handles(tag) {
        crate::computer_audit::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::device_sim::handles(tag) {
        crate::device_sim::handle(tag, &payload)
            .map(Some)
            .map_err(|m| error_json(crate::todo::WsErr(m, None)))
    } else if tag.starts_with("computer.") {
        match crate::computer_native::handle(tag, &payload) {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(message)) => Err(error_json(crate::todo::WsErr(message, None))),
            None => Ok(None),
        }
    } else if crate::provider_cache::handles(tag) {
        crate::provider_cache::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::projects_provision::handles(tag) {
        crate::projects_provision::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::mcp_connectors::handles(tag) {
        crate::mcp_connectors::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::editor_debug::handles(tag) {
        crate::editor_debug::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::editor_intelligence::handles(tag) {
        crate::editor_intelligence::handle(tag, &payload)
            .map_err(error_json)
    } else if tag == "agentGateway.call" {
        crate::agent_gateway_mcp::handle_ws_call(&payload)
            .await
            .map(Some)
            .map_err(error_json)
    } else if crate::voice_transcription::handles(tag) {
        crate::voice_transcription::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::provider_usage::handles(tag) {
        crate::provider_usage::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::textgen::handles(tag) {
        crate::textgen::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::account::handles(tag) {
        crate::account::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::hosts::handles(tag) {
        crate::hosts::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::device::handles(tag) {
        crate::device::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::editor::handles(tag) {
        crate::editor::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::studio::handles(tag) {
        crate::studio::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::project_agent::handles(tag) {
        crate::project_agent::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::automation::handles(tag) {
        crate::automation::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::pull_requests::handles(tag) {
        crate::pull_requests::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if tag.starts_with("githubInbox.") {
        crate::github_inbox::handle(tag, &payload)
            .map_err(error_json)
            .and_then(|v| match v {
                Some(v) => Ok(Some(v)),
                None => Err(error_json(crate::todo::WsErr(
                    format!("unhandled native tag {tag}"),
                    Some("DISPATCH"),
                ))),
            })
    } else {
        todos.handle(tag, payload).map(Some).map_err(error_json)
    };
    let outcome = outcome.map(|value| {
        value.map(|mut value| {
            if session_role != "owner"
                && (tag == "server.refreshProviders" || tag == "server.updateProvider")
            {
                strip_provider_account_identity(&mut value);
            }
            value
        })
    });
    let exit = match outcome {
        // `Schema.Void` successes encode without a `value` key.
        Ok(value) => match value {
            Some(value) => serde_json::json!({"_tag": "Success", "value": value}),
            None => serde_json::json!({"_tag": "Success"}),
        },
        Err(error) => serde_json::json!({
            "_tag": "Failure",
            "cause": [{
                "_tag": "Fail",
                "error": error,
            }]
        }),
    };
    let reply = serde_json::json!({"_tag": "Exit", "requestId": request_id, "exit": exit});
    Some(Some(ClientMessage::Text(reply.to_string().into())))
}

fn error_json(crate::todo::WsErr(message, code): crate::todo::WsErr) -> serde_json::Value {
    if let Some(reason) = code.and_then(|c| c.strip_prefix("PR_UNAVAILABLE:")) {
        return serde_json::json!({
            "_tag": "PullRequestsUnavailableError",
            "reason": reason,
            "message": message,
        });
    }
    serde_json::json!({
        "_tag": "WsRpcError",
        "message": message,
        "code": code,
        "retryable": false,
    })
}

/// A `bootstrap.negotiate` request answered natively, or `None` to relay.
fn intercept_bootstrap(text: &str) -> Option<(String, ClientMessage)> {
    let msg: serde_json::Value = serde_json::from_str(text).ok()?;
    if msg.get("_tag")?.as_str()? != "Request" || msg.get("tag")?.as_str()? != BOOTSTRAP_NEGOTIATE {
        return None;
    }
    let request_id = msg
        .get("id")
        .cloned()
        .unwrap_or(serde_json::Value::String(String::new()));
    let payload = msg
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let get_int = |k: &str| payload.get(k).and_then(|v| v.as_i64()).unwrap_or(-1);
    let required: Vec<String> = payload
        .get("requiredCapabilities")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|c| c.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let exit = match crate::native_ws::negotiate(
        get_int("protocolEpoch"),
        get_int("minRevision"),
        get_int("maxRevision"),
        &required,
    ) {
        Ok(value) => json!({"_tag": "Success", "value": value}),
        Err(err) => json!({
            "_tag": "Failure",
            "cause": [{"_tag": "Fail", "error": err}]
        }),
    };
    let reply = json!({"_tag": "Exit", "requestId": request_id, "exit": exit});
    Some((rpc_id_key(&request_id), ClientMessage::Text(reply.to_string().into())))
}

/// Replaces the `x-synara-server-instance` query value in a socket target.
fn rewrite_server_instance(target: &str, id: &str) -> String {
    let Some((base, query)) = target.split_once('?') else {
        return target.to_string();
    };
    let mut params: Vec<String> = query
        .split('&')
        .filter(|p| !p.starts_with("x-synara-server-instance="))
        .map(str::to_string)
        .collect();
    params.push(format!("x-synara-server-instance={id}"));
    format!("{base}?{}", params.join("&"))
}

/// Client request for the upstream socket with the admission headers copied.
fn build_upstream_request(
    target: &str,
    headers: &HeaderMap,
) -> Option<tokio_tungstenite::tungstenite::handshake::client::Request> {
    let mut request = target.into_client_request().ok()?;
    for name in FORWARDED {
        if let Some(value) = headers.get(*name)
            && let Ok(header_name) =
                tokio_tungstenite::tungstenite::http::header::HeaderName::from_bytes(
                    name.as_bytes(),
                )
            && let Ok(header_value) =
                tokio_tungstenite::tungstenite::http::header::HeaderValue::from_bytes(
                    value.as_bytes(),
                )
        {
            request.headers_mut().insert(header_name, header_value);
        }
    }
    Some(request)
}

async fn serve_native_socket(
    client: WebSocket,
    todos: std::sync::Arc<crate::todo::TodoStore>,
    desktop_update_authority: Option<String>,
    session_role: String,
) {
        // No usable original is running: serve every frame natively. Handled tags get
        // their normal reply; anything else answers an Exit failure so callers
        // never hang waiting for an upstream that does not exist.
        eprintln!("ws_relay: serving socket natively (upstream unavailable or disabled)");
        let (mut client_tx, mut client_rx) = client.split();
        let (native_tx, mut native_rx) = unbounded_channel::<ClientMessage>();
        let streams: std::sync::Arc<
            std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>,
        > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let streams_in = streams.clone();
        let writer = tokio::spawn(async move {
            while let Some(m) = native_rx.recv().await {
                if client_tx.send(m).await.is_err() {
                    break;
                }
            }
        });
        while let Some(msg) = client_rx.next().await {
            match msg {
                Ok(ClientMessage::Text(t)) => {
                    if std::env::var("SYNARA_TAP_NATIVE").is_ok() {
                        let snippet: String = t.chars().take(220).collect();
                        eprintln!("ws_native_in: {snippet}");
                    }
                    // Effect-rpc application-level liveness: the client's pinger
                    // replaces the connection after one missed pong, so these
                    // must be answered before anything else.
                    if t.contains("\"_tag\":\"Ping\"") || t.contains("\"_tag\": \"Ping\"") {
                        let _ = native_tx.send(ClientMessage::Text(
                            "{\"_tag\":\"Pong\"}".to_string().into(),
                        ));
                        continue;
                    }
                    if let Some(reply) = intercept_todo(&t, &todos, &native_tx, &streams_in, desktop_update_authority.as_deref(), session_role.as_str()).await {
                        if let Some(reply) = reply {
                            let _ = native_tx.send(reply);
                        }
                        continue;
                    }
                    if let Ok(m2) = serde_json::from_str::<serde_json::Value>(&t)
                        && m2.get("_tag").and_then(|v| v.as_str()) == Some("Request")
                    {
                        let rid = m2
                            .get("id")
                            .cloned()
                            .unwrap_or(serde_json::Value::String(String::new()));
                        let tag_s = m2.get("tag").and_then(|v| v.as_str()).unwrap_or("");
                        let reply = serde_json::json!({
                            "_tag": "Exit", "requestId": rid,
                            "exit": {"_tag": "Failure", "cause": [{"_tag": "Fail", "error": {
                                "_tag": "WsRpcError",
                                "message": format!("{tag_s} requires the original server, which is unavailable"),
                                "code": null, "retryable": false}}]}
                        });
                        let _ = native_tx.send(ClientMessage::Text(reply.to_string().into()));
                    }
                }
                Ok(ClientMessage::Ping(p)) => {
                    let _ = native_tx.send(ClientMessage::Pong(p));
                }
                Ok(ClientMessage::Close(_)) | Err(_) => break,
                _ => {}
            }
        }
        drop(native_tx);
        let _ = writer.await;
}

async fn relay(
    client: WebSocket,
    original: Original,
    uri: Uri,
    headers: HeaderMap,
    intercept: bool,
    authenticated_session: Option<crate::auth_http::AuthenticatedSession>,
) {
    let todos = original.todos.clone();
    let session_role = authenticated_session
        .as_ref()
        .map(|session| session.role.clone())
        .unwrap_or_else(|| "owner".to_string());
    let desktop_update_authority = authenticated_session.and_then(|session| {
        (session.role == "owner").then(|| format!("owner:{}", session.session_id))
    });
    if std::env::var("SYNARA_NO_UPSTREAM").is_ok() || !original.upstream_running() {
        eprintln!("ws_relay: serving socket natively (upstream unavailable or disabled)");
        return serve_native_socket(client, todos, desktop_update_authority, session_role.clone()).await;
    }
    // The native `/ws/negotiate` issues OUR server-instance id, but upstream
    // gates upgrades on its own — swap the param so relayed sockets pass the
    // generation check (fetched lazily; refetched once after a 426).
    let mut target = format!(
        "{}{}",
        original.ws_base(),
        uri.path_and_query().map_or("/ws", |p| p.as_str())
    );
    let mut refreshed = false;
    if target.contains("x-synara-server-instance=")
        && let Some(id) = original.upstream_instance(false).await
    {
        target = rewrite_server_instance(&target, &id);
    }
    let (upstream, resp) = loop {
        let request = match build_upstream_request(&target, &headers) {
            Some(r) => r,
            None => {
                eprintln!("ws_relay: request build failed for {target}");
                return;
            }
        };
        match connect_async(request).await {
            Ok(pair) => break pair,
            Err(e) => {
                if !refreshed
                    && target.contains("x-synara-server-instance=")
                    && let Some(id) = original.upstream_instance(true).await
                {
                    refreshed = true;
                    target = rewrite_server_instance(&target, &id);
                    continue;
                }
                eprintln!("ws_relay: upstream connect to {target} failed: {e}");
                // Do not strand an already accepted cold-boot socket on a dead
                // upstream: degrade this same connection to native serving. Tags
                // still missing natively fail fast instead of hanging forever.
                eprintln!("ws_relay: upstream unavailable after upgrade; serving socket natively");
                return serve_native_socket(client, todos, desktop_update_authority, session_role.clone()).await;
            }
        }
    };
    let _ = resp;
    let (mut client_tx, mut client_rx) = client.split();
    let (mut up_tx, mut up_rx) = upstream.split();
    // Debug tap: SYNARA_TAP_WS=/path appends every text frame (">" to
    // upstream, "<" from upstream) so relayed wire shapes can be studied.
    let tap_path = std::env::var("SYNARA_TAP_WS").ok();
    let tap = std::sync::Arc::new(move |dir: &str, t: &str| {
        if let Some(p) = &tap_path
            && let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
        {
            use std::io::Write;
            let _ = writeln!(f, "{dir}{t}");
        }
    });
    let (native_tx, mut native_rx) = unbounded_channel::<ClientMessage>();
    let streams: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let streams_in = streams.clone();

    let tap_out = tap.clone();
    let to_upstream = async move {
        while let Some(msg) = client_rx.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("ws_relay: client read error: {e}");
                    break;
                }
            };
            let out = match msg {
                ClientMessage::Text(t) => {
                    if intercept && t.contains(BOOTSTRAP_NEGOTIATE) {
                        if let Some((_, reply)) = intercept_bootstrap(&t) {
                            eprintln!("ws_relay: native bootstrap.negotiate reply sent");
                            let _ = native_tx.send(reply);
                            continue;
                        }
                    }
                    if let Some(reply) = intercept_todo(&t, &todos, &native_tx, &streams_in, desktop_update_authority.as_deref(), session_role.as_str()).await {
                        eprintln!("ws_relay: native todo reply");
                        if let Some(reply) = reply {
                            let _ = native_tx.send(reply);
                        }
                        continue;
                    }
                    // `Ack`/`Interrupt`/`Eof` addressed at a native stream stay
                    // on this side: upstream never saw the request id, so the
                    // frame would be a protocol violation there and poison the
                    // whole socket. Interrupt aborts the pump; Eof drains all.
                    if (t.contains("\"Interrupt\"") || t.contains("\"Eof\"") || t.contains("\"Ack\""))
                        && let Ok(msg) = serde_json::from_str::<serde_json::Value>(&t)
                    {
                        let tag = msg.get("_tag").and_then(|v| v.as_str()).unwrap_or("");
                        if tag == "Interrupt" || tag == "Eof" || tag == "Ack" {
                            let rid = msg.get("requestId").map(rpc_id_key).unwrap_or_default();
                            let owned = if let Ok(mut map) = streams_in.lock() {
                                if tag == "Eof" {
                                    map.drain().map(|(_, h)| h).collect::<Vec<_>>()
                                } else if tag == "Interrupt" {
                                    map.remove(&rid).into_iter().collect::<Vec<_>>()
                                } else {
                                    Vec::new()
                                }
                            } else {
                                Vec::new()
                            };
                            let known_native = if let Ok(map) = streams_in.lock() {
                                map.contains_key(rid.as_str())
                            } else {
                                false
                            };
                            if !owned.is_empty() {
                                for h in owned {
                                    h.abort();
                                }
                                continue;
                            }
                            if known_native {
                                continue;
                            }
                        }
                    }
                    tap_out(">", &t);
                    UpstreamMessage::Text(t.to_string().into())
                }
                ClientMessage::Binary(b) => {
                    tap_out(">", &format!(
                        "[bin {}B] {}",
                        b.len(),
                        b.iter()
                            .take(160)
                            .map(|x| format!("{x:02x}"))
                            .collect::<String>()
                    ));
                    UpstreamMessage::Binary(b)
                },
                ClientMessage::Ping(p) => UpstreamMessage::Ping(p),
                ClientMessage::Pong(p) => UpstreamMessage::Pong(p),
                ClientMessage::Close(c) => {
                    eprintln!("ws_relay: client closed socket: {c:?}");
                    let _ = up_tx
                        .send(UpstreamMessage::Close(c.map(|f| {
                            tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(
                                    f.code,
                                ),
                                reason: f.reason.to_string().into(),
                            }
                        })))
                        .await;
                    break;
                }
            };
            if up_tx.send(out).await.is_err() {
                eprintln!("ws_relay: upstream send channel closed");
                break;
            }
        }
        eprintln!("ws_relay: client->upstream loop ended");
    };

    let to_client = async move {
        loop {
            let out = tokio::select! {
                msg = up_rx.next() => match msg {
                    Some(Err(e)) => {
                        eprintln!("ws_relay: upstream read error: {e}");
                        break;
                    }
                    None => {
                        eprintln!("ws_relay: upstream socket ended");
                        break;
                    }
                    Some(Ok(m)) => match m {
                        UpstreamMessage::Text(t) => {
                            tap("<", &t);
                            ClientMessage::Text(t.to_string().into())
                        },
                        UpstreamMessage::Binary(b) => {
                            tap("<", &format!(
                                "[bin {}B] {}",
                                b.len(),
                                b.iter()
                                    .take(160)
                                    .map(|x| format!("{x:02x}"))
                                    .collect::<String>()
                            ));
                            ClientMessage::Binary(b)
                        },
                        UpstreamMessage::Ping(p) => ClientMessage::Ping(p),
                        UpstreamMessage::Pong(p) => ClientMessage::Pong(p),
                        UpstreamMessage::Frame(f) => {
                            let p = f.into_payload();
                            tap("<", &format!("[frame {}B]", p.len()));
                            ClientMessage::Binary(p)
                        },
                        UpstreamMessage::Close(c) => {
                            eprintln!("ws_relay: upstream close: {c:?}");
                            let _ = client_tx
                                .send(ClientMessage::Close(c.map(|f| {
                                    axum::extract::ws::CloseFrame {
                                        code: u16::from(f.code),
                                        reason: f.reason.to_string().into(),
                                    }
                                })))
                                .await;
                            break;
                        }
                    },
                },
                msg = native_rx.recv() => match msg {
                    Some(m) => {
                        if let ClientMessage::Text(t) = &m {
                            tap("<", t);
                        }
                        m
                    }
                    None => continue,
                },
            };
            if client_tx.send(out).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = to_upstream => {}
        _ = to_client => {}
    }
}


#[cfg(test)]
mod admission_tests {
    use axum::http::{HeaderMap, Uri};

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut out = HeaderMap::new();
        for (name, value) in pairs {
            out.insert(
                axum::http::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                axum::http::HeaderValue::from_str(value).unwrap(),
            );
        }
        out
    }

    #[test]
    fn loopback_feature_socket_without_credentials_is_local() {
        let headers = headers(&[]);
        let uri: Uri = "/ws".parse().unwrap();
        assert!(super::authenticate_rpc_websocket_upgrade(&headers, &uri).unwrap().is_none());
    }

    #[test]
    fn explicit_invalid_websocket_ticket_is_rejected() {
        let headers = headers(&[]);
        let uri: Uri = "/ws?wsToken=not-a-valid-ticket".parse().unwrap();
        assert!(matches!(
            super::authenticate_rpc_websocket_upgrade(&headers, &uri),
            Err(axum::http::StatusCode::UNAUTHORIZED)
        ));
    }

    #[test]
    fn authenticated_socket_paths_match_upstream() {
        assert!(super::authenticated_websocket_path("/ws"));
        assert!(super::authenticated_websocket_path("/ws/device-frames"));
        assert!(super::authenticated_websocket_path("/ws/computer-frames"));
        assert!(!super::authenticated_websocket_path("/ws/bootstrap"));
    }

    #[test]
    fn provider_identity_filter_removes_only_account_labels() {
        let mut value = serde_json::json!({"providers": [{
            "provider": "codex",
            "accountEmail": "owner@example.com",
            "accountOrganization": "private-org",
            "available": true
        }]});
        super::strip_provider_account_identity(&mut value);
        let status = &value["providers"][0];
        assert!(status.get("accountEmail").is_none());
        assert!(status.get("accountOrganization").is_none());
        assert_eq!(status.get("available"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn owner_only_rpc_tags_match_upstream_boundary() {
        assert!(super::owner_only_rpc("account.status", &serde_json::json!({})));
        assert!(super::owner_only_rpc("hosts.list", &serde_json::json!({})));
        assert!(super::owner_only_rpc(
            "server.updateSettings",
            &serde_json::json!({"legacyProviders": {}})
        ));
        assert!(!super::owner_only_rpc("server.updateSettings", &serde_json::json!({})));
        assert!(!super::owner_only_rpc("orchestration.getSnapshot", &serde_json::json!({})));
    }

    #[tokio::test]
    async fn client_role_cannot_read_owner_account_state() {
        let dir = std::env::temp_dir().join(format!(
            "synara-ws-auth-test-{}",
            crate::orchestration_engine::uuid_v4()
        ));
        let todos = crate::todo::TodoStore::open(dir.to_str().unwrap());
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let streams = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
        let request = r#"{"_tag":"Request","id":"1","tag":"account.status","payload":{}}"#;
        let reply = super::intercept_todo(request, &todos, &tx, &streams, None, "client")
            .await
            .flatten()
            .expect("owner-only RPC should produce a refusal");
        let text = match reply {
            super::ClientMessage::Text(text) => text.to_string(),
            _ => panic!("expected text refusal"),
        };
        assert!(text.contains("Owner authorization is required"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn unrelated_browser_origin_is_rejected_but_absent_origin_is_allowed() {
        let browser = headers(&[("host", "127.0.0.1:8080"), ("origin", "http://evil.example")]);
        assert!(super::reject_untrusted_websocket_origin(&browser));
        let cli = headers(&[("host", "127.0.0.1:8080")]);
        assert!(!super::reject_untrusted_websocket_origin(&cli));
    }

    #[test]
    fn session_cookie_detection_matches_upstream_shape() {
        assert!(super::has_session_cookie("synara_session=abc"));
        assert!(super::has_session_cookie("other=1; synara_session_8580=abc"));
        assert!(!super::has_session_cookie("x-synara_session=abc"));
        assert!(!super::has_session_cookie("synara_session_x=abc"));
        assert!(!super::has_session_cookie("synara_session="));
    }
}
