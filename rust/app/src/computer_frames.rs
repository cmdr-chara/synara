//! Live computer frame streaming, port of computer/computerFrameRoute.ts plus
//! stillFramePublisher.ts. The upgraded socket carries binary frame envelopes
//! exactly on the shared wire format; stills publish on a fixed interval,
//! deduplicated by content digest so an idle desktop costs nothing, every
//! still a keyframe because each is independently decodable. A resync text
//! message forces the next frame out regardless of the digest.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::Query;
use axum::response::{IntoResponse, Response};
use std::sync::Arc;

use crate::frame_transport::{
    encode_frame, FrameCodecConfig, FrameEnvelope, FrameHeader, FrameSink, FrameTransport,
};

pub const COMPUTER_FRAME_WS_PATH: &str = "/ws/computer-frames";
pub const COMPUTER_FRAME_RESYNC_MESSAGE: &str = "computer.frame.resync";
pub const DEFAULT_STILL_INTERVAL_MS: u64 = 500;
pub const MIN_STILL_INTERVAL_MS: u64 = 100;

const COMPUTER_FRAME_CODEC: FrameCodecConfig = FrameCodecConfig {
    magic: 0x5343,
    version: 1,
};

fn transport() -> &'static FrameTransport {
    static TRANSPORT: std::sync::OnceLock<FrameTransport> = std::sync::OnceLock::new();
    TRANSPORT.get_or_init(|| FrameTransport::new(COMPUTER_FRAME_CODEC))
}

struct WsSink {
    tx: tokio::sync::mpsc::UnboundedSender<Message>,
    open: Arc<std::sync::atomic::AtomicBool>,
}

impl FrameSink for WsSink {
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
    let requested = query.get("computerId").cloned().unwrap_or_default();
    if requested.is_empty() {
        return (axum::http::StatusCode::BAD_REQUEST, "Missing computerId").into_response();
    }
    if requested != crate::computer_native::COMPUTER_ID {
        return (axum::http::StatusCode::NOT_FOUND, "Unknown computer").into_response();
    }
    ws.on_upgrade(move |socket| drive(socket, requested))
}

fn sha1_hex(bytes: &[u8]) -> String {
    use sha1::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn capture_still() -> Option<Vec<u8>> {
    #[cfg(target_os = "macos")]
    {
        let dir = crate::todo::state_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = std::path::Path::new(&dir).join("computer-frame-capture.png");
        let out = std::process::Command::new("screencapture")
            .args(["-x", "-C"])
            .arg(&path)
            .output()
            .ok()?;
        if !out.status.success() {
            eprintln!("computer_frames: screencapture failed: {}", String::from_utf8_lossy(&out.stderr).trim());
            return None;
        }
        let bytes = match std::fs::read(&path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("computer_frames: read failed: {e}");
                return None;
            }
        };
        let _ = std::fs::remove_file(&path);
        if bytes.is_empty() {
            eprintln!("computer_frames: empty capture");
            return None;
        }
        Some(bytes)
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

async fn drive(socket: WebSocket, computer_id: String) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    use futures_util::{SinkExt, StreamExt};
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let open = Arc::new(std::sync::atomic::AtomicBool::new(true));

    // Writer task: everything the transport publishes reaches the wire.
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
        &computer_id,
        Box::new(WsSink {
            tx: tx.clone(),
            open: open.clone(),
        }),
    );

    // Resync listener: the text control message forces a keyframe exactly
    // upstream — a decoder that lost its way does not wait for change.
    let force = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let force_reader = force.clone();
    let reader_open = open.clone();
    let reader = tokio::spawn(async move {
        while let Some(Ok(message)) = ws_rx.next().await {
            match message {
                Message::Text(text) if text.as_str() == COMPUTER_FRAME_RESYNC_MESSAGE => {
                    force_reader.store(true, std::sync::atomic::Ordering::SeqCst);
                }
                Message::Close(_) => {
                    reader_open.store(false, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
                _ => {}
            }
        }
        reader_open.store(false, std::sync::atomic::Ordering::SeqCst);
    });

    // Still publisher: interval capture with content dedupe. Capture runs on
    // the blocking pool because screencapture is a subprocess.
    let publisher_open = open.clone();
    let publisher = tokio::spawn(async move {
        let mut sequence: u32 = 0;
        let mut last_digest = String::new();
        let interval = std::time::Duration::from_millis(DEFAULT_STILL_INTERVAL_MS);
        loop {
            tokio::time::sleep(interval).await;
            if !publisher_open.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }
            let forced = force.swap(false, std::sync::atomic::Ordering::SeqCst);
            let captured = tokio::task::spawn_blocking(capture_still).await.ok().flatten();
            let Some(bytes) = captured else {
                continue;
            };
            let digest = format!("{}:{}", bytes.len(), sha1_hex(&bytes));
            if !forced && digest == last_digest {
                continue;
            }
            last_digest = digest;
            sequence = sequence.wrapping_add(1);
            let timestamp_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0);
            transport().publish(&FrameEnvelope {
                header: FrameHeader {
                    stream_id: computer_id.clone(),
                    sequence,
                    timestamp_ms,
                    keyframe: true,
                    codec_config: false,
                },
                payload: bytes,
            });
        }
    });

    let _ = reader.await;
    open.store(false, std::sync::atomic::Ordering::SeqCst);
    publisher.abort();
    writer.abort();
}

/// Encode helper exposed for the shell-side tests and the native screenshot
/// single-shot paths sharing the exact wire format.
pub fn encode_computer_frame(envelope: &FrameEnvelope) -> Result<Vec<u8>, String> {
    encode_frame(COMPUTER_FRAME_CODEC, envelope)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codec_matches_upstream_contract() {
        // COMPUTER_FRAME_MAGIC 0x5343 and version 1 are wire-fixed upstream.
        assert_eq!(COMPUTER_FRAME_CODEC.magic, 0x5343);
        assert_eq!(COMPUTER_FRAME_CODEC.version, 1);
    }

    #[test]
    fn still_frames_encode_as_keyframes() {
        let envelope = FrameEnvelope {
            header: FrameHeader {
                stream_id: "host".to_string(),
                sequence: 7,
                timestamp_ms: 1000.0,
                keyframe: true,
                codec_config: false,
            },
            payload: b"png-bytes".to_vec(),
        };
        let encoded = encode_computer_frame(&envelope).unwrap();
        assert!(encoded[3] & crate::frame_transport::FRAME_FLAG_KEYFRAME != 0);
    }
}
