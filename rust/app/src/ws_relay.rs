//! WebSocket relay for the RPC upgrade paths (`/ws`, `/ws/bootstrap`,
//! device/computer frame sockets). Each client upgrade is answered by opening
//! a matching upgrade against the original and piping frames both ways — the
//! byte-level equivalent of the HTTP proxy.
//!
//! `/ws/bootstrap` additionally intercepts `bootstrap.negotiate` requests and
//! answers them natively (same `WsBootstrapNegotiateResult` the HTTP
//! `/ws/negotiate` route produces); every other frame relays upstream.

use axum::extract::ws::{Message as ClientMessage, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc::unbounded_channel;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message as UpstreamMessage;

use crate::original::Original;

/// Forward the headers that matter to the original's admission and auth.
const FORWARDED: &[&str] = &[
    "origin",
    "cookie",
    "authorization",
    "sec-websocket-protocol",
    "user-agent",
    "x-forwarded-for",
    "x-forwarded-proto",
];

const BOOTSTRAP_NEGOTIATE: &str = "bootstrap.negotiate";

pub fn router() -> Router<Original> {
    Router::new()
        .route("/ws", get(|s, w, u, h| upgrade(s, w, u, h, false)))
        .route("/ws/bootstrap", get(|s, w, u, h| upgrade(s, w, u, h, true)))
        .route(
            "/ws/device-frames",
            get(|s, w, u, h| upgrade(s, w, u, h, false)),
        )
        .route(
            "/ws/computer-frames",
            get(|s, w, u, h| upgrade(s, w, u, h, false)),
        )
        .route("/ws/host/v2", get(|s, w, u, h| upgrade(s, w, u, h, false)))
}

async fn upgrade(
    State(original): State<Original>,
    ws: WebSocketUpgrade,
    uri: Uri,
    headers: HeaderMap,
    intercept: bool,
) -> Response {
    ws.on_upgrade(move |socket| relay(socket, original, uri, headers, intercept))
}

/// `todo.*` requests served by the native store; the string check is a fast
/// prefilter so ordinary traffic still takes the relay path unparsed.
/// `Some(None)` marks a handled stream request (the reply is the chunk flow).
fn intercept_todo(
    text: &str,
    todos: &std::sync::Arc<crate::todo::TodoStore>,
    native_tx: &tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    streams: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>,
    >,
) -> Option<Option<ClientMessage>> {
    if !text.contains("\"todo.") && !text.contains("\"server.get") {
        return None;
    }
    let msg: serde_json::Value = serde_json::from_str(text).ok()?;
    if msg.get("_tag")?.as_str()? != "Request" {
        return None;
    }
    let tag = msg.get("tag")?.as_str()?;
    if !tag.starts_with("todo.") && tag != "server.getEnvironment" && tag != "server.getRuntimeStatus" {
        return None;
    }
    let request_id = msg
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let payload = msg.get("payload").cloned().unwrap_or(serde_json::Value::Null);
    if tag == "todo.subscribe" {
        // Snapshot first, then every change — mirrors `streamChanges`
        // (subscribe-then-read ordering is preserved by the store lock).
        let todos = todos.clone();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let mut rx = todos.subscribe();
            if let Ok(serde_json::Value::Object(mut snapshot)) = todos.handle("todo.list", serde_json::Value::Null) {
                let event = serde_json::json!({"type": "snapshot", "todos": snapshot.remove("todos").unwrap_or(serde_json::json!([]))});
                let chunk = serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx.send(ClientMessage::Text(chunk.to_string().into())).is_err() {
                    return;
                }
            }
            while let Ok(event) = rx.recv().await {
                let chunk = serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx.send(ClientMessage::Text(chunk.to_string().into())).is_err() {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    let exit = match if tag == "server.getEnvironment" {
        crate::server_env::get_environment(&crate::todo::state_dir()).map_err(crate::todo::err_pub)
    } else if tag == "server.getRuntimeStatus" {
        Ok(crate::runtime_status::get_runtime_status())
    } else {
        todos.handle(tag, payload)
    } {
        Ok(value) => serde_json::json!({"_tag": "Success", "value": value}),
        Err(crate::todo::WsErr(message, code)) => serde_json::json!({
            "_tag": "Failure",
            "cause": [{
                "_tag": "Fail",
                "error": {
                    "_tag": "WsRpcError",
                    "message": message,
                    "code": code,
                    "retryable": false,
                }
            }]
        }),
    };
    let reply = serde_json::json!({"_tag": "Exit", "requestId": request_id, "exit": exit});
    Some(Some(ClientMessage::Text(reply.to_string().into())))
}

/// A `bootstrap.negotiate` request answered natively, or `None` to relay.
fn intercept_bootstrap(text: &str) -> Option<(String, ClientMessage)> {
    let msg: serde_json::Value = serde_json::from_str(text).ok()?;
    if msg.get("_tag")?.as_str()? != "Request" || msg.get("tag")?.as_str()? != BOOTSTRAP_NEGOTIATE {
        return None;
    }
    let request_id = msg.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let payload = msg.get("payload").cloned().unwrap_or(serde_json::Value::Null);
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
    Some((
        request_id,
        ClientMessage::Text(reply.to_string().into()),
    ))
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

async fn relay(
    client: WebSocket,
    original: Original,
    uri: Uri,
    headers: HeaderMap,
    intercept: bool,
) {
    let todos = original.todos.clone();
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
                return;
            }
        }
    };
    let _ = resp;
    let (mut client_tx, mut client_rx) = client.split();
    let (mut up_tx, mut up_rx) = upstream.split();
    let (native_tx, mut native_rx) = unbounded_channel::<ClientMessage>();
    let streams: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let streams_in = streams.clone();

    let to_upstream = async move {
        while let Some(msg) = client_rx.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(_) => break,
            };
            let out = match msg {
                ClientMessage::Text(t) => {
                    if intercept && t.contains(BOOTSTRAP_NEGOTIATE) {
                        if let Some((_, reply)) = intercept_bootstrap(&t) {
                            let _ = native_tx.send(reply);
                            continue;
                        }
                    }
                    if let Some(reply) = intercept_todo(&t, &todos, &native_tx, &streams_in) {
                        if let Some(reply) = reply {
                            let _ = native_tx.send(reply);
                        }
                        continue;
                    }
                    // `Interrupt`/`Eof` addressed at a native stream: abort the
                    // pump and swallow the frame; anything else relays upstream.
                    if (t.contains("\"Interrupt\"") || t.contains("\"Eof\""))
                        && let Ok(msg) = serde_json::from_str::<serde_json::Value>(&t)
                    {
                        let tag = msg.get("_tag").and_then(|v| v.as_str()).unwrap_or("");
                        if tag == "Interrupt" || tag == "Eof" {
                            let rid = msg
                                .get("requestId")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let owned = if let Ok(mut map) = streams_in.lock() {
                                if tag == "Eof" {
                                    map.drain().map(|(_, h)| h).collect::<Vec<_>>()
                                } else {
                                    map.remove(rid).into_iter().collect::<Vec<_>>()
                                }
                            } else {
                                Vec::new()
                            };
                            if !owned.is_empty() {
                                for h in owned {
                                    h.abort();
                                }
                                continue;
                            }
                        }
                    }
                    UpstreamMessage::Text(t.to_string().into())
                }
                ClientMessage::Binary(b) => UpstreamMessage::Binary(b),
                ClientMessage::Ping(p) => UpstreamMessage::Ping(p),
                ClientMessage::Pong(p) => UpstreamMessage::Pong(p),
                ClientMessage::Close(c) => {
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
                break;
            }
        }
    };

    let to_client = async move {
        loop {
            let out = tokio::select! {
                msg = up_rx.next() => match msg {
                    Some(Ok(m)) => match m {
                        UpstreamMessage::Text(t) => ClientMessage::Text(t.to_string().into()),
                        UpstreamMessage::Binary(b) => ClientMessage::Binary(b),
                        UpstreamMessage::Ping(p) => ClientMessage::Ping(p),
                        UpstreamMessage::Pong(p) => ClientMessage::Pong(p),
                        UpstreamMessage::Frame(f) => ClientMessage::Binary(f.into_payload()),
                        UpstreamMessage::Close(c) => {
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
                    _ => break,
                },
                msg = native_rx.recv() => match msg {
                    Some(m) => m,
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
