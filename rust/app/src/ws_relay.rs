//! Transparent WebSocket relay for the RPC upgrade paths (`/ws`,
//! `/ws/bootstrap`, device/computer frame sockets). Each client upgrade is
//! answered by opening a matching upgrade against the original and piping
//! frames in both directions — the byte-level equivalent of the HTTP proxy.
//!
//! This is the interception point where native RPC handlers will later answer
//! messages (starting with `bootstrap.negotiate`) while everything else still
//! relays upstream.

use axum::extract::ws::{Message as ClientMessage, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
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

pub fn router() -> Router<Original> {
    Router::new()
        .route("/ws", get(upgrade))
        .route("/ws/bootstrap", get(upgrade))
        .route("/ws/frames/device", get(upgrade))
        .route("/ws/frames/computer", get(upgrade))
}

async fn upgrade(
    State(original): State<Original>,
    ws: WebSocketUpgrade,
    uri: Uri,
    headers: HeaderMap,
) -> Response {
    ws.on_upgrade(move |socket| relay(socket, original, uri, headers))
}

async fn relay(client: WebSocket, original: Original, uri: Uri, headers: HeaderMap) {
    let target = format!(
        "{}{}",
        original.ws_base(),
        uri.path_and_query().map_or("/ws", |p| p.as_str())
    );
    let mut request = http::Request::builder().uri(&target);
    for name in FORWARDED {
        if let Some(value) = headers.get(*name) {
            request = request.header(*name, value);
        }
    }
    let request = match request.body(()) {
        Ok(r) => r,
        Err(_) => return,
    };
    let (upstream, _) = match connect_async(request).await {
        Ok(pair) => pair,
        Err(_) => {
            // The original refused the upgrade: drop the client socket, which
            // surfaces as a normal WS close on the client side.
            return;
        }
    };

    let (mut client_tx, mut client_rx) = client.split();
    let (mut up_tx, mut up_rx) = upstream.split();

    let to_upstream = async move {
        while let Some(msg) = client_rx.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(_) => break,
            };
            let out = match msg {
                ClientMessage::Text(t) => UpstreamMessage::Text(t.to_string().into()),
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
        let _ = up_tx.close().await;
    };

    let to_client = async move {
        while let Some(msg) = up_rx.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(_) => break,
            };
            let out = match msg {
                UpstreamMessage::Text(t) => ClientMessage::Text(t.to_string().into()),
                UpstreamMessage::Binary(b) => ClientMessage::Binary(b),
                UpstreamMessage::Ping(p) => ClientMessage::Ping(p),
                UpstreamMessage::Pong(p) => ClientMessage::Pong(p),
                UpstreamMessage::Frame(f) => {
                    // Raw frames: forward payload as binary — axum has no raw
                    // frame passthrough; unreachable on this server.
                    ClientMessage::Binary(f.into_payload())
                }
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
            };
            if client_tx.send(out).await.is_err() {
                break;
            }
        }
        let _ = client_tx.close().await;
    };

    tokio::select! {
        _ = to_upstream => {}
        _ = to_client => {}
    }
}
