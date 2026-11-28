//! Computer-use panel state: connects the backend's `/ws/computer-frames`
//! socket on a background thread exactly like the ws_client RPC channel,
//! decodes the binary frame envelopes, and keeps the newest PNG still in a
//! shared slot the UI thread samples on its refresh timer. Input actions
//! forward through the ordinary RPC client as computer.* calls.

use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tungstenite::{connect, Message};

pub struct CuaShared {
    /// Newest decoded PNG payload; written by the receiver thread.
    pub frame_png: Option<Vec<u8>>,
    /// Monotonic frame counter — the UI compares to redraw only on change.
    pub sequence: u64,
    pub connected: bool,
    pub error: Option<String>,
}

pub struct CuaHandle {
    pub shared: Arc<Mutex<CuaShared>>,
    pub generation: Arc<AtomicU64>,
}

impl CuaHandle {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(Mutex::new(CuaShared {
                frame_png: None,
                sequence: 0,
                connected: false,
                error: None,
            })),
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Start (or restart) the receiver. Bumping the generation retires any
    /// earlier thread: it exits on its next read.
    pub fn start(&self) {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let generation_counter = self.generation.clone();
        let shared = self.shared.clone();
        std::thread::spawn(move || {
            let base =
                std::env::var("SYNARA_HTTP").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
            let authority = base
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_end_matches('/');
            let url = format!("ws://{authority}/ws/computer-frames?computerId=desktop");
            let sock = connect(&url);
            let (mut socket, _) = match sock {
                Ok(pair) => pair,
                Err(e) => {
                    if let Ok(mut s) = shared.lock() {
                        s.error = Some(format!("frame stream unavailable: {e}"));
                        s.connected = false;
                    }
                    return;
                }
            };
            if let Ok(mut s) = shared.lock() {
                s.connected = true;
                s.error = None;
            }
            loop {
                if generation_counter.load(Ordering::SeqCst) != generation {
                    break;
                }
                match socket.read() {
                    Ok(Message::Binary(bytes)) => {
                        // Decode the transport envelope: 17-byte header then
                        // the stream id, then the payload. Keyframe stills
                        // carry the whole image so late joins render at once.
                        if bytes.len() < 17 {
                            continue;
                        }
                        let sid_len = bytes[16] as usize;
                        if bytes.len() < 17 + sid_len {
                            continue;
                        }
                        let sequence = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
                        let payload = bytes[17 + sid_len..].to_vec();
                        if payload.len() < 8 || payload[0..4] != [137, 80, 78, 71] {
                            continue;
                        }
                        if let Ok(mut s) = shared.lock() {
                            s.frame_png = Some(payload);
                            s.sequence = sequence as u64;
                        }
                    }
                    Ok(Message::Close(_)) | Err(_) => {
                        if let Ok(mut s) = shared.lock() {
                            s.connected = false;
                        }
                        break;
                    }
                    Ok(_) => {}
                }
            }
        });
    }

    pub fn stop(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        if let Ok(mut s) = self.shared.lock() {
            s.connected = false;
        }
    }
}

/// Control-plane actions forwarded as ordinary RPCs on the shell client.
pub fn enable_control(client: &crate::ws_client::Client, thread_id: &str) -> Option<Value> {
    client.call(
        "computer.setControlEnabled",
        json!({"threadId": thread_id, "enabled": true}),
    )
}

pub fn disable_control(client: &crate::ws_client::Client, thread_id: &str) -> Option<Value> {
    client.call(
        "computer.setControlEnabled",
        json!({"threadId": thread_id, "enabled": false}),
    )
}

pub fn click(
    client: &crate::ws_client::Client,
    thread_id: &str,
    x: f64,
    y: f64,
    button: &str,
) -> Option<Value> {
    client.call(
        "computer.input.click",
        json!({"threadId": thread_id, "x": x, "y": y, "button": button}),
    )
}

pub fn key(client: &crate::ws_client::Client, thread_id: &str, key: &str) -> Option<Value> {
    client.call(
        "computer.input.key",
        json!({"threadId": thread_id, "key": key}),
    )
}

pub fn type_text(client: &crate::ws_client::Client, thread_id: &str, text: &str) -> Option<Value> {
    client.call(
        "computer.input.typeText",
        json!({"threadId": thread_id, "text": text}),
    )
}
