//! Outbound remote host connection intent and renderer attachments.
//!
//! Mirrors the important ownership rule of upstream HostConnectionRegistry:
//! `hosts.connect` records a desired relationship and verifies reachability,
//! while every renderer negotiation dials its own fresh authenticated RPC
//! stream. Streams are consumable once and are never replayed after a close.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::Mutex as AsyncMutex;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use crate::remote_client::{DialSessionInput, SessionWebSocket};

type SplitWrite = futures_util::stream::SplitSink<SessionWebSocket, WsMessage>;
type SplitRead = futures_util::stream::SplitStream<SessionWebSocket>;

#[derive(Clone)]
pub struct DesiredHost {
    pub host_id: String,
    pub host_name: String,
    pub environment_id: String,
    pub endpoints: Vec<crate::remote_client::RemoteEndpoint>,
    pub root_certificate: String,
    pub execution_scope: Value,
    pub state: &'static str,
    pub transport: Option<String>,
    pub started_at: Option<String>,
    pub credential_expires_at: Option<String>,
}

struct Attachment {
    host_id: String,
    server_instance_id: String,
    write: Arc<AsyncMutex<SplitWrite>>,
    read: Arc<AsyncMutex<SplitRead>>,
}

fn desired_hosts() -> &'static Mutex<HashMap<String, DesiredHost>> {
    static HOSTS: OnceLock<Mutex<HashMap<String, DesiredHost>>> = OnceLock::new();
    HOSTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn attachments() -> &'static Mutex<HashMap<String, Arc<Attachment>>> {
    static ATTACHMENTS: OnceLock<Mutex<HashMap<String, Arc<Attachment>>>> = OnceLock::new();
    ATTACHMENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn iso_from_unix(seconds: u64) -> String {
    chrono::DateTime::from_timestamp(seconds as i64, 0)
        .map(|value| value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string())
}

fn now_iso() -> String {
    iso_from_unix(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
    )
}

pub fn set_desired(host: DesiredHost) {
    if let Ok(mut hosts) = desired_hosts().lock() {
        hosts.insert(host.host_id.clone(), host);
    }
}

pub fn desired(host_id: &str) -> Option<DesiredHost> {
    desired_hosts().lock().ok()?.get(host_id).cloned()
}

pub fn remove_desired(host_id: &str) {
    if let Ok(mut hosts) = desired_hosts().lock() {
        hosts.remove(host_id);
    }
    if let Ok(mut live) = attachments().lock() {
        live.retain(|_, attachment| attachment.host_id != host_id);
    }
}

pub fn clear_all() {
    if let Ok(mut hosts) = desired_hosts().lock() { hosts.clear(); }
    if let Ok(mut live) = attachments().lock() { live.clear(); }
}

fn connection_view(host: &DesiredHost) -> Value {
    json!({
        "executionScope": host.execution_scope,
        "state": host.state,
        "environmentId": host.environment_id,
        "hostId": host.host_id,
        "hostName": host.host_name,
        "transport": host.transport.clone().unwrap_or_else(|| "lan".to_string()),
        "startedAt": host.started_at.clone().unwrap_or_else(now_iso),
        "credentialExpiresAt": host.credential_expires_at.clone().unwrap_or_else(now_iso),
        "wsPath": format!("/ws/remote/{}", host.host_id),
    })
}

pub fn list() -> Vec<Value> {
    desired_hosts()
        .lock()
        .map(|hosts| hosts.values().filter(|host| host.started_at.is_some()).map(connection_view).collect())
        .unwrap_or_default()
}

pub fn state(host_id: &str) -> &'static str {
    desired(host_id).map(|host| host.state).unwrap_or("stopped")
}

pub fn mark_state(host_id: &str, state: &'static str) {
    if let Ok(mut hosts) = desired_hosts().lock()
        && let Some(host) = hosts.get_mut(host_id)
    {
        host.state = state;
    }
}

async fn dial(desired: &DesiredHost, client: Value) -> Result<crate::remote_client::DialedSession, String> {
    let state_dir = crate::todo::state_dir();
    let secrets_dir = format!("{state_dir}/secrets");
    let identity = crate::device_identity::load_or_create(&crate::device_identity::device_identity_path(&secrets_dir))
        .map_err(|_| "Remote access operation failed".to_string())?;
    let user_id = desired.execution_scope.get("userId").and_then(Value::as_str).unwrap_or_default().to_string();
    let host_id = desired.host_id.clone();
    let grant_user_id = user_id.clone();
    // Endpoint reachability is account-directory state, not pairing trust.
    // Refresh it for every stream so restored desired hosts and network moves
    // do not reuse stale LAN/Tailscale addresses.
    let endpoints = crate::hosts::fresh_host_endpoints(&desired.host_id, &desired.environment_id)?;
    crate::remote_client::dial_remote_host(
        DialSessionInput {
            endpoints,
            anchor_environment_id: desired.environment_id.clone(),
            root_certificate: desired.root_certificate.clone(),
            client,
        },
        &identity,
        &user_id,
        move || crate::hosts::request_host_grant(&host_id, &grant_user_id),
    ).await
}

pub async fn prepare(host_id: &str, client: Value) -> Result<Value, String> {
    let desired = desired(host_id).ok_or_else(|| "Connect to this host from the local controller first".to_string())?;
    if matches!(desired.state, "needs-sign-in" | "revoked" | "incompatible" | "stopped") {
        return Err(format!("Remote connection requires attention: {}", desired.state));
    }
    let session = dial(&desired, client).await?;
    let server_instance_id = session.compatibility.get("serverInstanceId").and_then(Value::as_str).unwrap_or_default().to_string();
    let transport = session.transport.clone();
    let credential_expires = iso_from_unix(session.credential_expires_at_seconds);
    if let Ok(mut hosts) = desired_hosts().lock()
        && let Some(host) = hosts.get_mut(host_id)
    {
        host.state = "connected";
        host.transport = Some(transport);
        host.started_at = Some(now_iso());
        host.credential_expires_at = Some(credential_expires);
    }
    let id = crate::orchestration_engine::uuid_v4();
    let (write, read) = session.socket.split();
    let attachment = Arc::new(Attachment {
        host_id: host_id.to_string(),
        server_instance_id,
        write: Arc::new(AsyncMutex::new(write)),
        read: Arc::new(AsyncMutex::new(read)),
    });
    if let Ok(mut live) = attachments().lock() {
        live.insert(id.clone(), attachment);
    }
    let mut result = session.compatibility.clone();
    if let Some(object) = result.as_object_mut() {
        object.insert("remoteAttachmentId".into(), json!(id));
    }
    Ok(result)
}

pub async fn probe(host_id: &str, client: Value) -> Result<(), String> {
    let result = prepare(host_id, client).await?;
    if let Some(id) = result.get("remoteAttachmentId").and_then(Value::as_str) {
        close_attachment(id).await;
    }
    mark_state(host_id, "idle");
    Ok(())
}

pub fn prepare_blocking(host_id: &str, client: Value) -> Result<Value, String> {
    let host_id = host_id.to_string();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("Remote connection runtime unavailable: {error}"))?;
        runtime.block_on(async move { prepare(&host_id, client).await })
    })
    .join()
    .map_err(|_| "Remote connection worker failed".to_string())?
}

pub fn probe_blocking(host_id: &str, client: Value) -> Result<(), String> {
    let host_id = host_id.to_string();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("Remote connection runtime unavailable: {error}"))?;
        runtime.block_on(async move { probe(&host_id, client).await })
    })
    .join()
    .map_err(|_| "Remote connection worker failed".to_string())?
}

pub async fn close_attachment(id: &str) {
    let attachment = attachments().lock().ok().and_then(|mut live| live.remove(id));
    if let Some(attachment) = attachment {
        let _ = attachment.write.lock().await.send(WsMessage::Close(None)).await;
    }
}

pub fn disconnect_blocking(host_id: &str) -> bool {
    let existed = desired(host_id).is_some();
    remove_desired(host_id);
    existed
}

pub async fn unary_rpc(host_id: &str, tag: &str, payload: Value, client: Value) -> Result<Value, String> {
    let prepared = prepare(host_id, client).await?;
    let id = prepared.get("remoteAttachmentId").and_then(Value::as_str).unwrap_or_default().to_string();
    let attachment = attachments().lock().ok().and_then(|live| live.get(&id).cloned())
        .ok_or_else(|| "Remote tool connection is unavailable".to_string())?;
    let request_id = crate::orchestration_engine::uuid_v4();
    let frame = json!({"_tag":"Request","id":request_id,"tag":tag,"payload":payload,"headers":[]});
    let work = async {
        attachment.write.lock().await
            .send(WsMessage::Text(frame.to_string().into()))
            .await
            .map_err(|_| "Remote connection closed; do not replay mutations automatically.".to_string())?;
        let mut read = attachment.read.lock().await;
        loop {
            let message = read.next().await
                .ok_or_else(|| "Remote connection closed; do not replay mutations automatically.".to_string())?
                .map_err(|_| "Remote connection closed; do not replay mutations automatically.".to_string())?;
            let WsMessage::Text(text) = message else { continue };
            let parsed: Value = serde_json::from_str(&text).map_err(|_| "Invalid remote tool response.".to_string())?;
            if parsed.get("_tag").and_then(Value::as_str) != Some("Exit")
                || parsed.get("requestId").and_then(Value::as_str) != Some(request_id.as_str())
            {
                continue;
            }
            if parsed.pointer("/exit/_tag").and_then(Value::as_str) == Some("Success") {
                return Ok(parsed.pointer("/exit/value").cloned().unwrap_or(Value::Null));
            }
            let message = parsed
                .pointer("/exit/cause")
                .and_then(Value::as_array)
                .and_then(|cause| cause.iter().find(|entry| entry.get("_tag").and_then(Value::as_str) == Some("Fail")))
                .and_then(|failure| failure.pointer("/error/message").and_then(Value::as_str))
                .unwrap_or("Remote tool request was refused or interrupted.");
            return Err(message.to_string());
        }
    };
    let result = match tokio::time::timeout(Duration::from_secs(120), work).await {
        Ok(result) => result,
        Err(_) => Err("Remote tool call timed out; do not replay mutations automatically.".to_string()),
    };
    close_attachment(&id).await;
    result
}

fn axum_to_tungstenite(message: axum::extract::ws::Message) -> Option<WsMessage> {
    match message {
        axum::extract::ws::Message::Text(text) => Some(WsMessage::Text(text.to_string().into())),
        axum::extract::ws::Message::Binary(bytes) => Some(WsMessage::Binary(bytes.to_vec().into())),
        axum::extract::ws::Message::Ping(bytes) => Some(WsMessage::Ping(bytes.to_vec().into())),
        axum::extract::ws::Message::Pong(bytes) => Some(WsMessage::Pong(bytes.to_vec().into())),
        axum::extract::ws::Message::Close(frame) => Some(WsMessage::Close(frame.map(|frame| {
            tokio_tungstenite::tungstenite::protocol::CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(frame.code),
                reason: frame.reason.to_string().into(),
            }
        }))),
    }
}

fn tungstenite_to_axum(message: WsMessage) -> Option<axum::extract::ws::Message> {
    match message {
        WsMessage::Text(text) => Some(axum::extract::ws::Message::Text(text.to_string().into())),
        WsMessage::Binary(bytes) => Some(axum::extract::ws::Message::Binary(bytes.to_vec().into())),
        WsMessage::Ping(bytes) => Some(axum::extract::ws::Message::Ping(bytes.to_vec().into())),
        WsMessage::Pong(bytes) => Some(axum::extract::ws::Message::Pong(bytes.to_vec().into())),
        WsMessage::Close(frame) => Some(axum::extract::ws::Message::Close(frame.map(|frame| axum::extract::ws::CloseFrame {
            code: frame.code.into(),
            reason: frame.reason.to_string().into(),
        }))),
        WsMessage::Frame(_) => None,
    }
}

pub fn attachment_valid(host_id: &str, attachment_id: &str, server_instance_id: Option<&str>) -> bool {
    attachments().lock().ok().is_some_and(|live| {
        live.get(attachment_id).is_some_and(|attachment| {
            attachment.host_id == host_id
                && server_instance_id.is_some_and(|expected| expected == attachment.server_instance_id)
        })
    })
}

/// Bridges one local renderer websocket to its freshly negotiated remote
/// stream. Either side closing consumes the stream; renewal requires another
/// negotiation, so no mutation bytes are replayed.
pub async fn attach(host_id: &str, attachment_id: &str, server_instance_id: Option<&str>, socket: axum::extract::ws::WebSocket) -> Result<(), String> {
    // Negotiated streams are consumable once. Removing the reservation before
    // bridging prevents a second renderer from attaching to the same byte
    // stream while the first one is active.
    let attachment = attachments().lock().ok().and_then(|mut live| live.remove(attachment_id))
        .ok_or_else(|| "Remote connection is unavailable".to_string())?;
    if attachment.host_id != host_id
        || server_instance_id != Some(attachment.server_instance_id.as_str())
    {
        return Err("Remote stream negotiation expired".to_string());
    }
    let (mut local_write, mut local_read) = socket.split();
    let remote_read = attachment.read.clone();
    let remote_write = attachment.write.clone();
    let mut remote_to_local = tokio::spawn(async move {
        let mut read = remote_read.lock().await;
        while let Some(message) = read.next().await {
            let Ok(message) = message else { break };
            let Some(message) = tungstenite_to_axum(message) else { continue };
            if local_write.send(message).await.is_err() { break }
        }
    });
    while let Some(message) = local_read.next().await {
        let Ok(message) = message else { break };
        let Some(message) = axum_to_tungstenite(message) else { continue };
        if remote_write.lock().await.send(message).await.is_err() { break }
    }
    remote_to_local.abort();
    close_attachment(attachment_id).await;
    if attachments().lock().ok().is_some_and(|live| !live.values().any(|item| item.host_id == host_id)) {
        mark_state(host_id, "idle");
    }
    Ok(())
}
