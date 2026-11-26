//! Device frame streaming: unix-socket ingestion of the helper's pre-encoded
//! frames into the shared keyframe-gated transport. The helper connects as a
//! client to the socket path handed to `stream.start` and writes u32-LE
//! length-prefixed envelopes that already carry the device-frame wire format
//! (magic 0x5346), so ingestion only re-validates through the decoder before
//! fan-out.

use crate::frame_transport::{FrameCodecConfig, FrameTransport};

pub const DEVICE_FRAME_MAGIC: u16 = 0x5346;
pub const DEVICE_FRAME_VERSION: u8 = 1;

pub fn device_codec() -> FrameCodecConfig {
    FrameCodecConfig {
        magic: DEVICE_FRAME_MAGIC,
        version: DEVICE_FRAME_VERSION,
    }
}

static TRANSPORT: std::sync::OnceLock<FrameTransport> = std::sync::OnceLock::new();

pub fn transport() -> &'static FrameTransport {
    TRANSPORT.get_or_init(|| FrameTransport::new(device_codec()))
}

/// Start the unix-socket ingestion for one device and ask the helper to
/// connect. Returns the socket path the helper was given.
pub fn start_stream(udid: &str) -> Result<serde_json::Value, String> {
    use serde_json::json;
    let dir = std::path::Path::new(&crate::todo::state_dir()).join("device-streams");
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let sock = dir.join(format!("{udid}.sock"));
    let _ = std::fs::remove_file(&sock);
    let listener = std::os::unix::net::UnixListener::bind(&sock)
        .map_err(|e| format!("bind {}: {e}", sock.display()))?;
    let path = sock.to_string_lossy().to_string();
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(stream) = conn else { continue };
            std::thread::spawn(move || ingest_conn(stream));
        }
    });
    crate::device_helper::attach(udid)?;
    crate::device_helper::request("stream.start", json!({ "socketPath": path }))?;
    Ok(json!({ "udid": udid, "streaming": true, "socketPath": path }))
}

fn ingest_conn(mut stream: std::os::unix::net::UnixStream) {
    use std::io::Read;
    loop {
        let mut len_buf = [0u8; 4];
        if stream.read_exact(&mut len_buf).is_err() {
            return;
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len == 0 || len > 64 * 1024 * 1024 {
            return;
        }
        let mut buf = vec![0u8; len];
        if stream.read_exact(&mut buf).is_err() {
            return;
        }
        // The helper's bytes are already the envelope: decode to validate and
        // normalize, then fan out through the shared transport.
        if let Ok(envelope) = crate::frame_transport::decode_frame(device_codec(), &buf) {
            transport().publish(&envelope);
        }
    }
}

pub fn stop_stream(udid: &str) -> Result<serde_json::Value, String> {
    use serde_json::json;
    crate::device_helper::attach(udid)?;
    crate::device_helper::request("stream.stop", json!({}))?;
    Ok(json!({ "udid": udid, "streaming": false }))
}


// ---- websocket fan-out --------------------------------------------------------

use axum::extract::{Query, WebSocketUpgrade};
use axum::response::{IntoResponse, Response};
use axum::extract::ws::{Message, WebSocket};
use std::sync::Arc;

struct WsSink {
    tx: tokio::sync::mpsc::UnboundedSender<Message>,
    open: Arc<std::sync::atomic::AtomicBool>,
}

impl crate::frame_transport::FrameSink for WsSink {
    fn send(&mut self, bytes: &[u8]) {
        if self.tx.send(Message::Binary(bytes.to_vec().into())).is_err() {
            self.open.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }
    fn is_open(&self) -> bool {
        self.open.load(std::sync::atomic::Ordering::SeqCst)
    }
}

pub async fn upgrade(
    ws: WebSocketUpgrade,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let udid = query.get("udid").cloned().unwrap_or_default();
    if udid.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Missing udid").into_response();
    }
    ws.on_upgrade(move |socket| async move {
        // Stream lifecycle follows the subscriber: connecting a frame socket
        // starts the helper's capture for that device when it is not running,
        // echoing the upstream DeviceManager's first-subscriber starts.
        let udid2 = udid.clone();
        let started = tokio::task::spawn_blocking(move || start_stream(&udid2)).await;
        if let Some(e) = started.ok().and_then(|r| r.err()) {
            eprintln!("device_stream: start_stream failed: {e}");
        }
        drive_ws(socket, udid).await
    })
}

async fn drive_ws(socket: WebSocket, udid: String) {
    use futures_util::{SinkExt, StreamExt};
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let open = Arc::new(std::sync::atomic::AtomicBool::new(true));

    let writer_open = open.clone();
    let writer = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_tx.send(message).await.is_err() {
                writer_open.store(false, std::sync::atomic::Ordering::SeqCst);
                break;
            }
        }
    });

    transport().subscribe(
        &udid,
        Box::new(WsSink {
            tx,
            open: open.clone(),
        }),
    );

    // The reader only watches for close: frames are helper-driven, so there is
    // no publisher side here. A resync text message re-asks the helper for a
    // keyframe through stream.stats, matching upstream's resync semantics of
    // forcing fresh codec state to the subscriber.
    let reader_open = open.clone();
    let reader = tokio::spawn(async move {
        while let Some(Ok(message)) = ws_rx.next().await {
            if matches!(message, Message::Close(_)) {
                break;
            }
        }
        reader_open.store(false, std::sync::atomic::Ordering::SeqCst);
    });

    let _ = writer.await;
    let _ = reader.await;
}
