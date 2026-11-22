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
        .route("/ws/frames/device", get(|s, w, u, h| upgrade(s, w, u, h, false)))
        .route("/ws/frames/computer", get(|s, w, u, h| upgrade(s, w, u, h, false)))
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

async fn relay(
    client: WebSocket,
    original: Original,
    uri: Uri,
    headers: HeaderMap,
    intercept: bool,
) {
    let target = format!(
        "{}{}",
        original.ws_base(),
        uri.path_and_query().map_or("/ws", |p| p.as_str())
    );
    let mut request = match target.as_str().into_client_request() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("ws_relay: request build failed: {e}");
            return;
        }
    };
    for name in FORWARDED {
        if let Some(value) = headers.get(*name) {
            request.headers_mut().insert(
                tokio_tungstenite::tungstenite::http::header::HeaderName::from_bytes(
                    name.as_bytes(),
                )
                .unwrap(),
                tokio_tungstenite::tungstenite::http::header::HeaderValue::from_bytes(
                    value.as_bytes(),
                )
                .unwrap(),
            );
        }
    }
    let (upstream, resp) = match connect_async(request).await {
        Ok(pair) => pair,
        Err(e) => {
            eprintln!("ws_relay: upstream connect to {target} failed: {e}");
            return;
        }
    };
    let _ = resp;

    let (mut client_tx, mut client_rx) = client.split();
    let (mut up_tx, mut up_rx) = upstream.split();
    let (native_tx, mut native_rx) = unbounded_channel::<ClientMessage>();

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
