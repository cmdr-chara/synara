//! Outbound pinned remote channel client, port of the transport core of
//! upstream hostConnections/dialer.ts + remoteTransport/tunnel.ts. The outer
//! websocket is only a carrier: every byte inside it is a TLS 1.3 stream
//! pinned to the paired environment root before any pairing or RPC frame is
//! accepted.

use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio_rustls::rustls::pki_types::{CertificateDer, ServerName};
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;
use tokio_tungstenite::tungstenite::Message;

use crate::device_identity::DeviceIdentity;

const HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
const PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(1500);
const STREAM_CHUNK: usize = 64 * 1024;

#[derive(Clone, Debug)]
pub struct RemoteEndpoint {
    pub transport: String,
    pub url: String,
}

impl RemoteEndpoint {
    fn rank(&self) -> usize {
        match self.transport.as_str() {
            "loopback" => 0,
            "lan" => 1,
            "tailscale" => 2,
            "ssh" => 3,
            "cloudflare" => 4,
            "relay" => 5,
            _ => 6,
        }
    }
}

fn sort_endpoints(mut endpoints: Vec<RemoteEndpoint>) -> Vec<RemoteEndpoint> {
    endpoints.sort_by_key(|endpoint| endpoint.rank());
    endpoints
}

fn websocket_url(endpoint: &str) -> Result<String, String> {
    let mut url = reqwest::Url::parse(endpoint).map_err(|_| "Invalid remote endpoint".to_string())?;
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        "wss" => "wss",
        "ws" => "ws",
        _ => return Err("Invalid remote endpoint".to_string()),
    };
    url.set_scheme(scheme).map_err(|_| "Invalid remote endpoint".to_string())?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err("Invalid remote endpoint".to_string());
    }
    url.set_path(crate::remote_tunnel::REMOTE_OUTER_PATH);
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

fn health_url(endpoint: &str) -> Result<String, String> {
    let mut url = reqwest::Url::parse(endpoint).map_err(|_| "Invalid remote endpoint".to_string())?;
    match url.scheme() {
        "https" | "http" => {}
        "wss" => url.set_scheme("https").map_err(|_| "Invalid remote endpoint".to_string())?,
        "ws" => url.set_scheme("http").map_err(|_| "Invalid remote endpoint".to_string())?,
        _ => return Err("Invalid remote endpoint".to_string()),
    }
    url.set_path("/health");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.to_string())
}

async fn probe(endpoint: &RemoteEndpoint) -> bool {
    let Ok(url) = health_url(&endpoint.url) else { return false };
    let Ok(client) = reqwest::Client::builder().timeout(PROBE_TIMEOUT).build() else { return false };
    client.get(url).send().await.is_ok_and(|response| response.status().is_success())
}

/// Cheap liveness race for preference. This preserves ADR ordering — a fast
/// relay never outranks loopback/LAN — while avoiding a multi-second serial
/// walk past paths that clearly do not answer.
async fn order_by_probe(endpoints: Vec<RemoteEndpoint>) -> Vec<RemoteEndpoint> {
    let ordered = sort_endpoints(endpoints);
    let probes = futures_util::future::join_all(ordered.iter().map(probe)).await;
    if let Some((winner, _)) = ordered.iter().zip(probes.iter()).find(|(_, reachable)| **reachable) {
        let winner_url = winner.url.clone();
        let mut result = vec![winner.clone()];
        result.extend(ordered.into_iter().filter(|endpoint| endpoint.url != winner_url));
        return result;
    }
    ordered
}

/// Byte-stream adapter over a client websocket. TLS sees one ordered byte
/// stream; websocket framing remains an outer implementation detail exactly
/// like upstream's ciphertextStream.
struct ClientWsDuplex<S> {
    incoming: VecDeque<u8>,
    ws: tokio_tungstenite::WebSocketStream<S>,
    write_queue: VecDeque<u8>,
    flushing: bool,
}

impl<S> ClientWsDuplex<S> {
    fn new(ws: tokio_tungstenite::WebSocketStream<S>) -> Self {
        Self { incoming: VecDeque::new(), ws, write_queue: VecDeque::new(), flushing: false }
    }
}

impl<S> AsyncRead for ClientWsDuplex<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        loop {
            if !self.incoming.is_empty() {
                let (front, _) = self.incoming.as_slices();
                let n = buf.remaining().min(front.len());
                buf.put_slice(&front[..n]);
                self.incoming.drain(..n);
                return Poll::Ready(Ok(()));
            }
            match self.ws.poll_next_unpin(cx) {
                Poll::Ready(Some(Ok(Message::Binary(data)))) => self.incoming.extend(data.iter()),
                Poll::Ready(Some(Ok(_))) => continue,
                Poll::Ready(Some(Err(error))) => {
                    return Poll::Ready(Err(io::Error::new(io::ErrorKind::ConnectionReset, error)))
                }
                Poll::Ready(None) => return Poll::Ready(Ok(())),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

impl<S> AsyncWrite for ClientWsDuplex<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, data: &[u8]) -> Poll<io::Result<usize>> {
        if self.flushing {
            match self.as_mut().poll_flush(cx) {
                Poll::Ready(Ok(())) => {}
                other => return other.map(|result| result.map(|_| 0)),
            }
        }
        let chunk = data.len().min(STREAM_CHUNK);
        self.write_queue.extend(&data[..chunk]);
        self.flushing = true;
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(chunk)),
            Poll::Pending => Poll::Ready(Ok(chunk)),
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.flushing && !self.write_queue.is_empty() {
            match self.ws.poll_ready_unpin(cx) {
                Poll::Ready(Ok(())) => {
                    let frame: Vec<u8> = self.write_queue.drain(..).collect();
                    if let Err(error) = self.ws.start_send_unpin(Message::Binary(frame.into())) {
                        return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, error)));
                    }
                    self.flushing = false;
                }
                Poll::Ready(Err(error)) => return Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, error))),
                Poll::Pending => return Poll::Pending,
            }
        }
        match self.ws.poll_flush_unpin(cx) {
            Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
            Poll::Ready(Err(error)) => Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, error))),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.as_mut().poll_flush(cx) {
            Poll::Ready(Ok(())) => match self.ws.poll_close_unpin(cx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(())),
                Poll::Ready(Err(error)) => Poll::Ready(Err(io::Error::new(io::ErrorKind::BrokenPipe, error))),
                Poll::Pending => Poll::Pending,
            },
            Poll::Ready(Err(error)) => Poll::Ready(Err(error)),
            Poll::Pending => Poll::Pending,
        }
    }
}

fn root_config(root_certificate: &str) -> Result<Arc<ClientConfig>, String> {
    let pem = rustls_pemfile::certs(&mut root_certificate.as_bytes())
        .next()
        .ok_or_else(|| "Remote root certificate missing".to_string())?
        .map_err(|error| format!("Remote root certificate invalid: {error}"))?;
    let mut roots = RootCertStore::empty();
    roots
        .add(CertificateDer::from(pem))
        .map_err(|error| format!("Remote root certificate invalid: {error}"))?;
    let config = ClientConfig::builder_with_provider(
            tokio_rustls::rustls::crypto::ring::default_provider().into(),
        )
        .with_protocol_versions(&[&tokio_rustls::rustls::version::TLS13])
        .map_err(|error| format!("TLS client configuration failed: {error}"))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(Arc::new(config))
}

fn ws_accept_key(key: &str) -> String {
    use sha1::Digest as _;
    let mut hash = sha1::Sha1::new();
    hash.update(key.as_bytes());
    hash.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    base64::engine::general_purpose::STANDARD.encode(hash.finalize())
}

fn random_websocket_key() -> String {
    use std::io::Read as _;
    let mut bytes = [0u8; 16];
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        let _ = file.read_exact(&mut bytes);
    }
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

async fn inner_upgrade<S>(mut tls: S, environment_id: &str, path: &str) -> Result<tokio_tungstenite::WebSocketStream<S>, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let key = random_websocket_key();
    let host = crate::remote_tls::remote_tls_server_name(environment_id)?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    tokio::time::timeout(HANDSHAKE_TIMEOUT, tls.write_all(request.as_bytes()))
        .await
        .map_err(|_| "Remote inner upgrade timed out".to_string())?
        .map_err(|error| format!("Remote inner upgrade failed: {error}"))?;
    let mut head = Vec::with_capacity(1024);
    let mut byte = [0u8; 1];
    let read_head = async {
        while !head.windows(4).any(|window| window == b"\r\n\r\n") {
            if head.len() > 16 * 1024 {
                return Err("Remote inner upgrade refused".to_string());
            }
            let read = tls.read(&mut byte).await.map_err(|error| format!("Remote inner upgrade failed: {error}"))?;
            if read == 0 {
                return Err("Remote inner upgrade refused".to_string());
            }
            head.push(byte[0]);
        }
        Ok::<(), String>(())
    };
    tokio::time::timeout(HANDSHAKE_TIMEOUT, read_head)
        .await
        .map_err(|_| "Remote inner upgrade timed out".to_string())??;
    let text = String::from_utf8_lossy(&head);
    let mut lines = text.lines();
    let status = lines.next().unwrap_or_default();
    if !status.contains(" 101") {
        return Err("Remote inner upgrade refused".to_string());
    }
    let accepted = lines.any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.trim().eq_ignore_ascii_case("sec-websocket-accept") && value.trim() == ws_accept_key(&key)
        })
    });
    if !accepted {
        return Err("Remote inner upgrade refused".to_string());
    }
    Ok(tokio_tungstenite::WebSocketStream::from_raw_socket(
        tls,
        tokio_tungstenite::tungstenite::protocol::Role::Client,
        None,
    ).await)
}

type OuterStream = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type TlsStream = tokio_rustls::client::TlsStream<ClientWsDuplex<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>;

async fn open_tls_channel(endpoint: &RemoteEndpoint, root_certificate: &str, environment_id: &str) -> Result<TlsStream, String> {
    let outer_url = websocket_url(&endpoint.url)?;
    let connect = tokio_tungstenite::connect_async(outer_url);
    let (outer, _) = tokio::time::timeout(HANDSHAKE_TIMEOUT, connect)
        .await
        .map_err(|_| "Could not open the remote socket".to_string())?
        .map_err(|_| "Could not open the remote socket".to_string())?;
    let server_name = ServerName::try_from(crate::remote_tls::remote_tls_server_name(environment_id)?)
        .map_err(|_| "Invalid remote environment identity".to_string())?;
    let connector = TlsConnector::from(root_config(root_certificate)?);
    let tls = tokio::time::timeout(HANDSHAKE_TIMEOUT, connector.connect(server_name, ClientWsDuplex::new(outer)))
        .await
        .map_err(|_| "Remote TLS handshake timed out".to_string())?
        .map_err(|error| format!("No route completed the verified host handshake: {error}"))?;
    Ok(tls)
}

pub async fn open_inner_websocket(
    endpoints: Vec<RemoteEndpoint>,
    root_certificate: &str,
    environment_id: &str,
    path: &str,
) -> Result<(tokio_tungstenite::WebSocketStream<TlsStream>, RemoteEndpoint), String> {
    if endpoints.is_empty() {
        return Err("That host has no published route".to_string());
    }
    let ordered = order_by_probe(endpoints).await;
    let mut last_error = "No route completed the verified host handshake".to_string();
    for endpoint in ordered {
        let result = async {
            let tls = open_tls_channel(&endpoint, root_certificate, environment_id).await?;
            inner_upgrade(tls, environment_id, path).await
        }.await;
        match result {
            Ok(socket) => return Ok((socket, endpoint)),
            Err(error) => last_error = error,
        }
    }
    Err(last_error)
}

async fn exchange_frame<S>(
    socket: &mut S,
    request: Value,
    response_type: &str,
) -> Result<Value, String>
where
    S: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + futures_util::Sink<Message>
        + Unpin,
    S::Error: std::fmt::Display,
{
    socket
        .send(Message::Text(request.to_string().into()))
        .await
        .map_err(|error| format!("Remote handshake send failed: {error}"))?;
    let frame = tokio::time::timeout(HANDSHAKE_TIMEOUT, socket.next())
        .await
        .map_err(|_| "Remote handshake timed out".to_string())?
        .ok_or_else(|| "Remote peer refused the session".to_string())?
        .map_err(|error| format!("Remote handshake failed: {error}"))?;
    let Message::Text(text) = frame else {
        return Err("Unexpected remote handshake response".to_string());
    };
    let parsed: Value = serde_json::from_str(&text).map_err(|_| "Unexpected remote handshake response".to_string())?;
    if parsed.get("type").and_then(Value::as_str) == Some("session_incompatible") {
        let message = parsed
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("Remote host protocol is incompatible");
        return Err(message.to_string());
    }
    if parsed.get("type").and_then(Value::as_str) != Some(response_type) {
        return Err("Unexpected remote handshake response".to_string());
    }
    Ok(parsed)
}

pub struct PairingOutcome {
    pub transport: String,
}

/// Runs the v2 pairing frame machine over a pinned TLS channel. The returned
/// future intentionally waits through the owner's manual approval window;
/// expiry is the invitation expiry, not an arbitrary local timeout.
pub async fn pair_remote_host(
    endpoints: Vec<RemoteEndpoint>,
    bundle: &Value,
    identity: &DeviceIdentity,
    label: &str,
    request_grant: impl Fn() -> Result<String, String> + Send + Sync,
) -> Result<PairingOutcome, String> {
    let environment_id = bundle.get("environmentId").and_then(Value::as_str).unwrap_or_default();
    let root_certificate = bundle.get("rootCertificate").and_then(Value::as_str).unwrap_or_default();
    let invite_id = bundle.get("inviteId").and_then(Value::as_str).unwrap_or_default();
    let secret = bundle.get("secret").and_then(Value::as_str).unwrap_or_default();
    let user_id = bundle.get("userId").and_then(Value::as_str).unwrap_or_default();
    if environment_id.is_empty() || root_certificate.is_empty() || invite_id.is_empty() || secret.is_empty() || user_id.is_empty() {
        return Err("Review a fresh pairing code and compare the host fingerprint".to_string());
    }
    crate::remote_tls::validate_anchor(
        environment_id,
        root_certificate,
        bundle.get("rootFingerprint").and_then(Value::as_str).unwrap_or_default(),
    )?;
    let expires_at = bundle
        .get("expiresAt")
        .and_then(Value::as_str)
        .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
        .map(|date| date.timestamp_millis())
        .unwrap_or(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_millis() as i64).unwrap_or(0);
    if expires_at <= now {
        return Err("Invitation expired or invalid".to_string());
    }
    let (mut socket, endpoint) = open_inner_websocket(
        endpoints,
        root_certificate,
        environment_id,
        crate::remote_tunnel::REMOTE_INNER_PAIRING_PATH,
    ).await?;
    let challenge = exchange_frame(
        &mut socket,
        json!({
            "v": 2,
            "type": "pairing_begin",
            "inviteId": invite_id,
            "secret": secret,
            "publicKey": identity.public_jwk,
            "label": label,
        }),
        "pairing_challenge",
    ).await?;
    let nonce = challenge.get("nonce").and_then(Value::as_str).unwrap_or_default();
    if nonce.len() != 43 || !nonce.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid host pairing challenge".to_string());
    }
    let grant = request_grant().ok();
    let now_seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_secs()).unwrap_or(0);
    let proof = identity.sign_jwt(
        json!({"typ": crate::pairing_gateway::REMOTE_PAIRING_PROOF_TYPE}),
        json!({
            "nonce": nonce,
            "inviteId": invite_id,
            "iss": "synara-device",
            "sub": user_id,
            "aud": format!("synara-host:{environment_id}"),
            "iat": now_seconds,
            "exp": now_seconds + 60,
            "jti": crate::orchestration_engine::uuid_v4(),
        }),
    )?;
    let mut proof_frame = json!({"v": 2, "type": "pairing_proof", "proof": proof});
    if let Some(grant) = grant.filter(|grant| !grant.is_empty()) {
        proof_frame["grant"] = json!(grant);
    }
    let mut status = exchange_frame(&mut socket, proof_frame, "pairing_status").await?;
    loop {
        if status.get("deviceJkt").and_then(Value::as_str) != Some(identity.jkt.as_str()) {
            return Err("Host approved a different device".to_string());
        }
        match status.get("state").and_then(Value::as_str) {
            Some("approved") => return Ok(PairingOutcome { transport: endpoint.transport }),
            Some("pending") => {}
            _ => return Err("Host denied this device".to_string()),
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_millis() as i64).unwrap_or(0);
        if now >= expires_at {
            return Err("Pairing cancelled or expired".to_string());
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        status = exchange_frame(&mut socket, json!({"v": 2, "type": "pairing_status"}), "pairing_status").await?;
    }
}


pub struct DialedSession {
    pub socket: tokio_tungstenite::WebSocketStream<TlsStream>,
    pub transport: String,
    pub credential: String,
    pub credential_expires_at_seconds: u64,
    pub compatibility: Value,
}

fn credential_expiration(credential: &str) -> Result<u64, String> {
    let parts: Vec<&str> = credential.split('.').collect();
    if parts.len() != 3 {
        return Err("Host returned an expired credential".to_string());
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| "Host returned an expired credential".to_string())?;
    let parsed: Value = serde_json::from_slice(&payload).map_err(|_| "Host returned an expired credential".to_string())?;
    let exp = parsed.get("exp").and_then(Value::as_u64).unwrap_or(0);
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|duration| duration.as_secs()).unwrap_or(0);
    if exp <= now {
        return Err("Host returned an expired credential".to_string());
    }
    Ok(exp)
}

/// Opens the post-pairing RPC channel and completes mint_request →
/// session_credential → session_authorize → session_ready. No plaintext RPC
/// fallback exists: this socket is already inside the pinned TLS channel.
pub async fn dial_remote_host(
    endpoints: Vec<RemoteEndpoint>,
    anchor_environment_id: &str,
    root_certificate: &str,
    identity: &DeviceIdentity,
    user_id: &str,
    request_grant: impl Fn() -> Result<String, String> + Send + Sync,
) -> Result<DialedSession, String> {
    let (mut socket, endpoint) = open_inner_websocket(
        endpoints,
        root_certificate,
        anchor_environment_id,
        crate::remote_tunnel::REMOTE_INNER_RPC_PATH,
    ).await?;
    let grant = request_grant()?;
    let mint_request = identity.sign_mint_request(user_id, &grant, anchor_environment_id)?;
    let minted = exchange_frame(
        &mut socket,
        json!({"v":1,"type":"mint_request","request":mint_request}),
        "session_credential",
    ).await?;
    let credential = minted.get("credential").and_then(Value::as_str).unwrap_or_default().to_string();
    if credential.is_empty() {
        return Err("Host returned no session credential".to_string());
    }
    let dpop = identity.sign_dpop("CONNECT", "synara://remote/session", Some(&credential))?;
    let client = json!({
        "protocolEpoch": crate::native_ws::PROTOCOL_EPOCH,
        "minRevision": crate::native_ws::MIN_REVISION,
        "maxRevision": crate::native_ws::MAX_REVISION,
        "clientBuild": crate::native_ws::SERVER_BUILD,
        "requiredCapabilities": [
            "orchestration.cursor-safe-streams",
            "orchestration.thread-detail-snapshot",
            "rpc.typed-errors",
            "git.worktree-setup-progress",
        ],
    });
    let ready = exchange_frame(
        &mut socket,
        json!({"v":1,"type":"session_authorize","credential":credential,"dpop":dpop,"client":client}),
        "session_ready",
    ).await?;
    if ready.get("environmentId").and_then(Value::as_str) != Some(anchor_environment_id) {
        return Err("Host protocol identity does not match its paired root".to_string());
    }
    let compatibility = ready.get("compatibility").cloned().unwrap_or(Value::Null);
    if compatibility.get("negotiatedRevision").and_then(Value::as_i64).is_none() {
        return Err("Host returned an incomplete protocol negotiation".to_string());
    }
    let credential_expires_at_seconds = credential_expiration(&credential)?;
    Ok(DialedSession {
        socket,
        transport: endpoint.transport,
        credential,
        credential_expires_at_seconds,
        compatibility,
    })
}
