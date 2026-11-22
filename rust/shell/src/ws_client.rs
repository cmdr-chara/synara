//! Minimal blocking WS-RPC client for the native shell UI.
//!
//! Speaks the `{"_tag":"Request"|"Success"|"Failure", ...}` wire protocol used by
//! rust/app's native WS-RPC layer. Blocking: always drive it from a background
//! executor, never the UI thread.

use serde_json::{json, Value};
use std::io::ErrorKind;
use std::net::TcpStream;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message};

pub struct Client {
    sock: tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
    next_id: u64,
}

impl Client {
    pub fn connect() -> Option<Self> {
        let url = std::env::var("SYNARA_WS").unwrap_or_else(|_| {
            "ws://127.0.0.1:8080/ws?x-synara-client-build=1.0.0&x-synara-protocol-epoch=1&x-synara-protocol-revision=3&x-synara-server-instance=inst-1&token=rectoken"
                .to_string()
        });
        match connect(&url) {
            Ok((sock, _resp)) => Some(Self { sock, next_id: 0 }),
            Err(e) => {
                eprintln!("ws_client: connect failed: {e}");
                None
            }
        }
    }

    /// One request -> one Success value (or None on failure/timeout).
    pub fn call(&mut self, tag: &str, payload: Value) -> Option<Value> {
        self.next_id += 1;
        let id = format!("native-{}", self.next_id);
        let req = json!({"_tag": "Request", "id": id, "tag": tag, "payload": payload, "headers": []});
        if let Err(e) = self.sock.send(Message::Text(req.to_string().into())) {
            eprintln!("ws_client: send failed: {e}");
            return None;
        }
        for _ in 0..200 {
            match self.sock.read() {
                Ok(Message::Text(text)) => {
                    let Ok(msg) = serde_json::from_str::<Value>(&text) else {
                        continue;
                    };
                    if msg.get("_tag").and_then(Value::as_str) != Some("Exit")
                        || msg.get("requestId").and_then(Value::as_str) != Some(id.as_str())
                    {
                        continue;
                    }
                    let exit = msg.get("exit").cloned().unwrap_or(Value::Null);
                    if exit.get("_tag").and_then(Value::as_str) == Some("Success") {
                        return Some(exit.get("value").cloned().unwrap_or(Value::Null));
                    }
                    eprintln!("ws_client: {tag} failed: {}", &text[..text.len().min(300)]);
                    return None;
                }
                Ok(_) => continue,
                Err(tungstenite::Error::Io(e)) if e.kind() == ErrorKind::WouldBlock => continue,
                Err(_) => return None,
            }
        }
        None
    }
}
