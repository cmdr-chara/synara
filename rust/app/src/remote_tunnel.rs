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
) -> Option<(String, String, String, String)> {
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
            let mut authorization = String::new();
            let mut dpop = String::new();
            for line in lines {
                if let Some((name, value)) = line.split_once(':') {
                    if name.trim().eq_ignore_ascii_case("sec-websocket-key") {
                        key = value.trim().to_string();
                    } else if name.trim().eq_ignore_ascii_case("authorization") {
                        authorization = value.trim().to_string();
                    } else if name.trim().eq_ignore_ascii_case("dpop") {
                        dpop = value.trim().to_string();
                    }
                }
            }
            if key.is_empty() {
                return None;
            }
            return Some((path, key, authorization, dpop));
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
    let Some(head) = crate::remote_resource::read_inner_head(&mut tls).await else {
        return;
    };
    let path = head.path.split('?').next().unwrap_or("").to_string();
    let listening_port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080);
    if path == crate::remote_resource::REMOTE_RESOURCE_PATH {
        crate::remote_resource::serve_resource(&mut tls, head, listening_port).await;
        return;
    }
    if path != REMOTE_INNER_RPC_PATH && path != REMOTE_INNER_PAIRING_PATH {
        return;
    }
    let key = head.header("sec-websocket-key").unwrap_or("").to_string();
    if key.is_empty() {
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
    if path == REMOTE_INNER_PAIRING_PATH {
        let state_dir = crate::todo::state_dir();
        let secrets_dir = format!("{state_dir}/secrets");
        let environment_id = crate::server_env::environment_id(&state_dir).unwrap_or_default();
        if let Some(scope) = crate::remote_pairing::local_scope(&secrets_dir, &environment_id) {
            crate::pairing_gateway::run_pairing(ws, scope).await;
        }
        return;
    }
    let mut session_registration: Option<(String, tokio::sync::mpsc::UnboundedReceiver<u16>)> = None;
    // In-band session handshake, port of upstream remoteSessions/gateway.ts:
    // the upgrade stays open and the first frames authenticate — a mint
    // request (proof of possession of a trusted device key) earns a fresh
    // short-lived credential, then session_authorize binds that credential
    // to this socket with a DPoP proof before any RPC is bridged. Frames cap
    // at 64 KiB and 8 messages with a 15 second deadline, exactly upstream.
    let (mut client_tx, mut client_rx) = ws.split();
    {
        use futures_util::SinkExt as _;
        let state_dir = crate::todo::state_dir();
        let secrets_dir = format!("{state_dir}/secrets");
        let environment_id = crate::server_env::environment_id(&state_dir).unwrap_or_default();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(15);
        let mut state = "mint";
        let mut frames_seen = 0u32;
        let mut authed = false;
        while !authed {
            let frame = match tokio::time::timeout_at(deadline, client_rx.next()).await {
                Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text)))) => text.to_string(),
                _ => return,
            };
            frames_seen += 1;
            if frames_seen > 8 || frame.len() > 64 * 1024 {
                let _ = client_tx
                    .send(tokio_tungstenite::tungstenite::Message::Close(Some(
                        tokio_tungstenite::tungstenite::protocol::CloseFrame {
                            code: 1009u16.into(),
                            reason: "Remote authorization frame limit exceeded".into(),
                        },
                    )))
                    .await;
                return;
            }
            let parsed: serde_json::Value = match serde_json::from_str(&frame) {
                Ok(value) => value,
                Err(_) => return,
            };
            let frame_type = parsed.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if state == "mint" && frame_type == "mint_request" {
                let request = parsed.get("request").and_then(|v| v.as_str()).unwrap_or("");
                let identity_path = crate::host_identity::host_identity_path(&secrets_dir);
                let minted = (|| -> Result<crate::host_auth_verify::MintedSessionCredential, String> {
                    let identity = crate::host_identity::read_host_identity(&identity_path)?
                        .ok_or_else(|| "host identity missing".to_string())?;
                    let conn = crate::remote_pairing::open()?;
                    let scope = crate::remote_pairing::local_scope(&secrets_dir, &environment_id)
                        .ok_or_else(|| "scope unavailable".to_string())?;
                    let account = crate::account::read_session_file()
                        .ok_or_else(|| "Remote authorization unavailable".to_string())?;
                    let account_url = account.get("accountUrl").and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
                    let host_id = account.get("hostId").and_then(serde_json::Value::as_str).unwrap_or_default().to_string();
                    let key_generation = account.get("hostKeyGeneration").and_then(serde_json::Value::as_u64).unwrap_or(0);
                    if account_url.is_empty() || host_id.is_empty() || key_generation == 0 {
                        return Err("Remote authorization unavailable".to_string());
                    }
                    crate::host_auth_verify::mint_session_credential(
                        request,
                        &identity,
                        &environment_id,
                        key_generation,
                        |device_jkt, grant, subject| {
                            let verified = crate::host_grant_verify::verify_host_grant(
                                grant,
                                &account_url,
                                &scope.account_authority,
                                &environment_id,
                                &host_id,
                                &scope.user_id,
                                device_jkt,
                                Some(subject),
                            )?;
                            let generation = crate::remote_pairing::device_trust_generation(&conn, &scope, device_jkt)?;
                            crate::host_grant_verify::consume_host_grant(&verified)?;
                            Ok(generation)
                        },
                    )
                })();
                let Ok(minted) = minted else { return };
                let reply = serde_json::json!({
                    "v": 1,
                    "type": "session_credential",
                    "credential": minted.credential,
                    "expiresAtSeconds": minted.expires_at_seconds,
                });
                if client_tx
                    .send(tokio_tungstenite::tungstenite::Message::Text(reply.to_string().into()))
                    .await
                    .is_err()
                {
                    return;
                }
                state = "authorize";
                continue;
            }
            if frame_type != "session_authorize" {
                return;
            }
            let credential = parsed.get("credential").and_then(|v| v.as_str()).unwrap_or("");
            let dpop = parsed.get("dpop").and_then(|v| v.as_str()).unwrap_or("");
            let verified = (|| -> Result<crate::host_auth_verify::VerifiedSessionCredential, String> {
                let identity_path = crate::host_identity::host_identity_path(&secrets_dir);
                let identity = crate::host_identity::read_host_identity(&identity_path)?
                    .ok_or_else(|| "host identity missing".to_string())?;
                crate::host_auth_verify::verify_session_credential(
                    credential,
                    dpop,
                    &identity,
                    &environment_id,
                    1,
                    "synara://remote/session",
                    "CONNECT",
                    None,
                )
            })();
            let Ok(verified) = verified else { return };
            let trusted = (|| -> Result<bool, String> {
                let conn = crate::remote_pairing::open()?;
                let scope = crate::remote_pairing::local_scope(&secrets_dir, &environment_id)
                    .ok_or_else(|| "scope unavailable".to_string())?;
                crate::remote_pairing::authorize_device(&conn, &scope, &verified.device_jkt)
            })();
            if trusted != Ok(true) {
                return;
            }
            let (session_id, close_rx) = crate::remote_sessions::add(&verified.device_jkt, &verified.user_id, "direct");
            session_registration = Some((session_id, close_rx));
            let client = parsed.get("client").cloned().unwrap_or_else(|| serde_json::json!({}));
            let required: Vec<String> = client
                .get("requiredCapabilities")
                .and_then(serde_json::Value::as_array)
                .map(|items| items.iter().filter_map(|item| item.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let compatibility = crate::native_ws::negotiate(
                client.get("protocolEpoch").and_then(serde_json::Value::as_i64).unwrap_or(-1),
                client.get("minRevision").and_then(serde_json::Value::as_i64).unwrap_or(-1),
                client.get("maxRevision").and_then(serde_json::Value::as_i64).unwrap_or(-1),
                &required,
            );
            let Ok(compatibility) = compatibility else {
                let refusal = compatibility.err().unwrap_or_else(|| serde_json::json!({"message":"incompatible"}));
                let frame = serde_json::json!({"v":2,"type":"session_incompatible","error":refusal});
                let _ = client_tx
                    .send(tokio_tungstenite::tungstenite::Message::Text(frame.to_string().into()))
                    .await;
                return;
            };
            let ready = serde_json::json!({
                "v": 2,
                "type": "session_ready",
                "environmentId": environment_id,
                "compatibility": compatibility,
            });
            if client_tx
                .send(tokio_tungstenite::tungstenite::Message::Text(ready.to_string().into()))
                .await
                .is_err()
            {
                return;
            }
            authed = true;
        }
    }
    let desktop_update_authority = session_registration
        .as_ref()
        .map(|(session_id, _)| crate::server_desktop_update::authorize_remote_session(session_id));
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
    let mut close_rx = session_registration.as_mut().map(|(_, rx)| rx);
    loop {
        let msg = if let Some(rx) = close_rx.as_mut() {
            tokio::select! {
                signal = rx.recv() => {
                    break;
                }
                frame = client_rx.next() => match frame {
                    Some(frame) => frame,
                    None => break,
                },
            }
        } else {
            match client_rx.next().await {
                Some(frame) => frame,
                None => break,
            }
        };
        match msg {
            Ok(tokio_tungstenite::tungstenite::Message::Text(t)) => {
                if let Some(reply) =
                    crate::ws_relay::intercept_todo_pub(&t, &todos, &native_tx, &streams_in, desktop_update_authority.as_deref()).await
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
    if let Some((session_id, _)) = session_registration {
        crate::server_desktop_update::revoke_remote_session(&session_id);
        crate::remote_sessions::remove(&session_id);
    }
}
