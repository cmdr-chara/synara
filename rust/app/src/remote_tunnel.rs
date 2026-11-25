//! Remote TLS tunnel, port of upstream remoteTransport/tunnel.ts.
//! Outer WebSocket frames carry opaque ciphertext bytes; a TLS 1.3 session
//! runs over that byte stream, and inside it HTTP plus inner websocket
//! upgrades at /v2/rpc and /v2/pairing. Only pinned-root clients complete
//! the handshake — the leaf certificate chains to the locally generated root.

use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use axum::extract::ws::{Message as WsMessage, WebSocket};

use crate::remote_tls::RemoteTlsIdentity;

pub const REMOTE_OUTER_PATH: &str = "/ws/host/v2";
pub const REMOTE_INNER_RPC_PATH: &str = "/v2/rpc";
pub const REMOTE_INNER_PAIRING_PATH: &str = "/v2/pairing";
pub const REMOTE_HANDSHAKE_TIMEOUT_MS: u64 = 15_000;
const STREAM_CHUNK: usize = 64 * 1024;

/// Byte-stream adapter over a websocket's binary frames, mirroring upstream's
/// ciphertextStream: inbound binary frames queue as readable bytes, outbound
/// writes chunk at 64 KiB. Text frames terminate the tunnel upstream; here
/// they are simply not produced or consumed.
pub struct WsDuplex {
    incoming: VecDeque<u8>,
    ws: WebSocket,
    write_queue: VecDeque<u8>,
    flushing: bool,
}

impl WsDuplex {
    pub fn new(ws: WebSocket) -> Self {
        Self {
            incoming: VecDeque::new(),
            ws,
            write_queue: VecDeque::new(),
            flushing: false,
        }
    }
}

impl AsyncRead for WsDuplex {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            if !self.incoming.is_empty() {
                let (front, _) = self.incoming.as_slices();
                let n = buf.remaining().min(front.len());
                buf.put_slice(&front[..n]);
                self.incoming.drain(..n);
                return Poll::Ready(Ok(()));
            }
            match self.ws.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(WsMessage::Binary(data)))) => {
                    self.incoming.extend(data.iter());
                }
                Poll::Ready(Some(Ok(_))) => {
                    // Ping/pong/text: tungstenite answers pings internally.
                    continue;
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, e)))
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl AsyncWrite for WsDuplex {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        data: &[u8],
    ) -> Poll<io::Result<usize>> {
        // Flush any queued chunk first so frames stay ordered.
        if self.flushing {
            match self.as_mut().poll_flush(cx) {
                Poll::Ready(Ok(())) => {}
                other => return other.map(|r| r.map(|_| 0)),
            }
        }
        let chunk = data.len().min(STREAM_CHUNK);
        self.write_queue.extend(&data[..chunk]);
        self.flushing = true;
        // Start the flush; the frame leaves when the sink accepts it.
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(chunk)),
            Poll::Pending => {
                // The bytes are queued: report them written so the TLS layer
                // does not retry, the flush continues on later polls.
                Poll::Ready(Ok(chunk))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.flushing && !self.write_queue.is_empty() {
            // Readiness first: draining before the sink accepts would drop the frame.
            match self.ws.poll_ready_unpin(cx) {
                Poll::Ready(Ok(())) => {
                    let frame: Vec<u8> = self.write_queue.drain(..).collect();
                    if let Err(e) = self
                        .ws
                        .start_send_unpin(WsMessage::Binary(frame.into()))
                    {
                        return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, e)));
                    }
                    self.flushing = false;
                }
                Poll::Ready(Err(e)) => {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, e)))
                }
                Poll::Pending => return Poll::Pending,
            }
        }
        match self.ws.poll_flush_unpin(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, e))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => match self.ws.poll_close_unpin(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(e)) => Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, e))),
                Poll::Pending => Poll::Pending,
            },
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn server_config(identity: &RemoteTlsIdentity) -> Result<Arc<ServerConfig>, String> {
    let mut certs: Vec<CertificateDer<'static>> = Vec::new();
    for pem in rustls_pemfile::certs(&mut identity.leaf_certificate.as_bytes()) {
        certs.push(pem.map_err(|e| format!("leaf cert parse failed: {e}"))?);
    }
    // The client pins the root itself; sending the chain helps path building.
    for pem in rustls_pemfile::certs(&mut identity.root_certificate.as_bytes()) {
        certs.push(pem.map_err(|e| format!("root cert parse failed: {e}"))?);
    }
    let key = rustls_pemfile::pkcs8_private_keys(&mut identity.leaf_private_key.as_bytes())
        .next()
        .ok_or_else(|| "leaf private key missing".to_string())?
        .map_err(|e| format!("leaf key parse failed: {e}"))?;
    let config = ServerConfig::builder_with_provider(
            tokio_rustls::rustls::crypto::ring::default_provider().into(),
        )
        // TLS 1.3 only, matching upstream's minVersion/maxVersion pinning.
        .with_protocol_versions(&[&tokio_rustls::rustls::version::TLS13])
        .map_err(|e| format!("tls protocol config failed: {e}"))?
        .with_no_client_auth()
        .with_single_cert(certs, PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key)))
        .map_err(|e| format!("tls config failed: {e}"))?;
    Ok(Arc::new(config))
}

/// Drives one accepted outer websocket: TLS handshake then the inner byte
/// stream handed to the HTTP/WS layer by the caller's `serve` closure.
/// TLS 1.3 only, 15-second handshake ceiling — the upstream envelope.
pub async fn drive_tunnel<S>(
    ws: WebSocket,
    identity: &RemoteTlsIdentity,
    serve: impl FnOnce(tokio_rustls::server::TlsStream<WsDuplex>) -> S,
) where
    S: std::future::Future<Output = ()> + Send,
{
    let config = match server_config(identity) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("remote_tunnel: tls config failed: {e}");
            return;
        }
    };
    let acceptor = TlsAcceptor::from(config);
    let duplex = WsDuplex::new(ws);
    let accept = acceptor.accept(duplex);
    let tls = match tokio::time::timeout(
        std::time::Duration::from_millis(REMOTE_HANDSHAKE_TIMEOUT_MS),
        accept,
    )
    .await
    {
        Ok(Ok(stream)) => stream,
        Ok(Err(e)) => {
            eprintln!("remote_tunnel: tls handshake failed: {e}");
            return;
        }
        Err(_) => {
            eprintln!("remote_tunnel: tls handshake timed out");
            return;
        }
    };
    serve(tls).await;
}

// ── Inner HTTP upgrade + RPC serving ────────────────────────────────────────

fn ws_accept_key(key: &str) -> String {
    use base64::Engine;
    use sha1::Digest as _;
    let mut h = sha1::Sha1::new();
    h.update(key.as_bytes());
    h.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64::engine::general_purpose::STANDARD.encode(h.finalize())
}

/// Minimal HTTP upgrade parser: the inner protocol only ever opens websocket
/// upgrades at the two fixed paths, so a bounded header read suffices — the
/// same surface node http exposes upstream before handleUpgrade.
async fn read_upgrade_request<S: tokio::io::AsyncRead + Unpin>(
    stream: &mut S,
) -> Option<(String, String)> {
    let mut buf = Vec::with_capacity(4096);
    let mut chunk = [0u8; 1024];
    loop {
        if buf.len() > 16 * 1024 {
            return None;
        }
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..pos]).to_string();
            let mut lines = head.lines();
            let request_line = lines.next()?;
            let path = request_line.split_whitespace().nth(1)?.to_string();
            let mut key = String::new();
            for line in lines {
                if let Some((name, value)) = line.split_once(':') {
                    if name.trim().eq_ignore_ascii_case("sec-websocket-key") {
                        key = value.trim().to_string();
                    }
                }
            }
            if key.is_empty() {
                return None;
            }
            return Some((path, key));
        }
        use tokio::io::AsyncReadExt;
        match stream.read(&mut chunk).await {
            Ok(0) | Err(_) => return None,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
}

/// Serves one TLS session: inner upgrade at /v2/rpc or /v2/pairing, then the
/// same native dispatch loop the relay runs in SYNARA_NO_UPSTREAM mode.
pub async fn serve_inner(
    mut tls: tokio_rustls::server::TlsStream<WsDuplex>,
    todos: std::sync::Arc<crate::todo::TodoStore>,
) {
    use futures_util::{SinkExt, StreamExt};
    let Some((path, key)) = read_upgrade_request(&mut tls).await else {
        return;
    };
    if path != REMOTE_INNER_RPC_PATH && path != REMOTE_INNER_PAIRING_PATH {
        return;
    }
    use tokio::io::AsyncWriteExt;
    let response = format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        ws_accept_key(&key)
    );
    if tls.write_all(response.as_bytes()).await.is_err() {
        return;
    }
    let ws = tokio_tungstenite::WebSocketStream::from_raw_socket(
        tls,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    )
    .await;
    let (mut client_tx, mut client_rx) = ws.split();
    let (native_tx, mut native_rx) = tokio::sync::mpsc::unbounded_channel::<axum::extract::ws::Message>();
    let streams: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let streams_in = streams.clone();
    let writer = tokio::spawn(async move {
        while let Some(m) = native_rx.recv().await {
            let text = match m {
                axum::extract::ws::Message::Text(t) => t.to_string(),
                _ => continue,
            };
            if client_tx
                .send(tokio_tungstenite::tungstenite::Message::Text(text.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });
    while let Some(msg) = client_rx.next().await {
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(t)) => {
                if let Some(reply) =
                    crate::ws_relay::intercept_todo_pub(&t, &todos, &native_tx, &streams_in).await
                {
                    if let Some(reply) = reply {
                        let _ = native_tx.send(reply);
                    }
                    continue;
                }
                if let Ok(m2) = serde_json::from_str::<serde_json::Value>(&t) {
                    if m2.get("_tag").and_then(|v| v.as_str()) == Some("Request") {
                        let rid = m2
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let tag_s = m2.get("tag").and_then(|v| v.as_str()).unwrap_or("");
                        let reply = serde_json::json!({
                            "_tag": "Exit", "requestId": rid,
                            "exit": {"_tag": "Failure", "cause": [{"_tag": "Fail", "error": {
                                "_tag": "WsRpcError",
                                "message": format!("{tag_s} is not served over the remote tunnel yet"),
                                "code": null, "retryable": false}}]}
                        });
                        let _ = native_tx.send(axum::extract::ws::Message::Text(
                            reply.to_string().into(),
                        ));
                    }
                }
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
    writer.abort();
}
