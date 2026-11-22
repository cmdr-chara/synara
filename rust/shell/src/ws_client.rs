//! Blocking WS-RPC client for the native shell UI.
//!
//! Speaks the `{"_tag":"Request"|"Exit"|"Chunk", ...}` wire protocol used by
//! rust/app's native WS-RPC layer and by the relayed upstream server. One TCP
//! connection, split into a reader thread that routes frames by `requestId` to
//! per-request channels: `call` waits for `Exit`, `call_stream` also receives
//! `Chunk` values. Blocking: always drive it from a background executor, never
//! the UI thread.

use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tungstenite::{connect, Message, WebSocket};

pub struct Client {
    writer: Arc<Mutex<WebSocket<TcpStream>>>,
    pending: Arc<Mutex<HashMap<String, Sender<Value>>>>,
    next_id: AtomicU64,
    alive: Arc<AtomicBool>,
}

impl Client {
    /// Mint a real session cookie + ws token through the front door so relayed
    /// (non-native) RPCs authenticate like the browser does.
    fn mint_session(base: &str) -> (Option<String>, String) {
        let agent = ureq::AgentBuilder::new().build();
        let cookie = agent
            .post(&format!("{base}/api/auth/bootstrap"))
            .set("Origin", base)
            .set("Content-Type", "application/json")
            .send_string("{\"credential\":\"rectoken\"}")
            .ok()
            .and_then(|r| {
                r.all("set-cookie")
                    .into_iter()
                    .next()
                    .map(|c| c.split(';').next().unwrap_or("").to_string())
            });
        let token = if let Some(cookie) = &cookie {
            agent
                .post(&format!("{base}/api/auth/ws-token"))
                .set("Origin", base)
                .set("Cookie", cookie)
                .set("Content-Type", "application/json")
                .send_string("{}")
                .ok()
                .and_then(|r| r.into_json::<serde_json::Value>().ok())
                .and_then(|v| v.get("token").and_then(|t| t.as_str()).map(str::to_owned))
        } else {
            None
        };
        (cookie, token.unwrap_or_else(|| "rectoken".to_string()))
    }

    pub fn connect() -> Option<Arc<Self>> {
        let base = std::env::var("SYNARA_HTTP").unwrap_or_else(|_| "http://127.0.0.1:8080".into());
        let (cookie, token) = Self::mint_session(&base);
        let url = std::env::var("SYNARA_WS").unwrap_or_else(|_| {
            format!("ws://127.0.0.1:8080/ws?x-synara-client-build=1.0.0&x-synara-protocol-epoch=1&x-synara-protocol-revision=3&x-synara-server-instance=inst-1&token={token}")
        });
        let host = http::Uri::try_from(&url)
            .ok()
            .and_then(|u| u.authority().map(|a| a.as_str().to_string()))
            .unwrap_or_else(|| "127.0.0.1:8080".into());
        let mut req = http::Request::builder()
            .uri(&url)
            .header("Host", host)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tungstenite::handshake::client::generate_key(),
            );
        if let Some(cookie) = &cookie {
            req = req.header("Cookie", cookie);
        }
        let req = match req.body(()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("ws_client: request build failed: {e}");
                return None;
            }
        };
        let (mut sock, _resp) = match connect(req) {
            Ok(ok) => ok,
            Err(e) => {
                eprintln!("ws_client: connect failed: {e}");
                return None;
            }
        };
        // Split the socket: the upgraded stream is a plain TcpStream locally;
        // a second client-role WebSocket on a cloned stream writes while the
        // original reads.
        let raw = match sock.get_mut() {
            tungstenite::stream::MaybeTlsStream::Plain(s) => match s.try_clone() {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("ws_client: try_clone failed: {e}");
                    return None;
                }
            },
            _ => {
                eprintln!("ws_client: non-plain stream, cannot split");
                return None;
            }
        };
        let writer = Arc::new(Mutex::new(WebSocket::from_raw_socket(
            raw,
            tungstenite::protocol::Role::Client,
            None,
        )));
        let pending: Arc<Mutex<HashMap<String, Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_reader = pending.clone();
        let pong_writer = writer.clone();
        let alive = Arc::new(AtomicBool::new(true));
        let reader_alive = alive.clone();
        thread::spawn(move || {
            loop {
            match sock.read() {
                Ok(Message::Text(text)) => {
                    let Ok(msg) = serde_json::from_str::<Value>(&text) else {
                        continue;
                    };
                    let rid = msg
                        .get("requestId")
                        .and_then(Value::as_str)
                        .map(str::to_owned);
                    if let Some(rid) = rid {
                        if let Some(tx) = pending_reader.lock().ok().and_then(|m| m.get(&rid).cloned()) {
                            let _ = tx.send(msg);
                        }
                    }
                }
                Ok(Message::Ping(p)) => {
                    if let Ok(mut w) = pong_writer.lock() {
                        let _ = w.send(Message::Pong(p));
                    }
                }
                Ok(Message::Close(c)) => {
                    eprintln!("ws_client: close frame: {c:?}");
                    break;
                }
                Err(e) => {
                    eprintln!("ws_client: read error: {e}");
                    break;
                }
                Ok(_) => {}
            }
            }
            reader_alive.store(false, Ordering::Relaxed);
        });
        // The upstream closes relayed sockets after ~40s of silence; the
        // browser keeps them alive with an application-level Ping frame.
        let heartbeat_writer = writer.clone();
        let heartbeat_alive = alive.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(15));
            let frame = json!({"_tag": "Ping"});
            let ok = heartbeat_writer
                .lock()
                .map(|mut w| w.send(Message::Text(frame.to_string().into())).is_ok())
                .unwrap_or(false);
            if !ok {
                heartbeat_alive.store(false, Ordering::Relaxed);
                break;
            }
        });
        Some(Arc::new(Self {
            writer,
            pending,
            next_id: AtomicU64::new(0),
            alive,
        }))
    }

    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }

    /// Issue a Request frame; returns (request_id, receiver for its frames).
    /// Caller drains the receiver (or drops it) and later calls
    /// [`Self::finish_request`] to unregister.
    pub fn request(&self, tag: &str, payload: Value) -> Option<(String, Receiver<Value>)> {
        // RequestId is a bigint on the wire: ids must be numeric strings.
        let id = format!(
            "{}",
            9_000_000_000u64 + self.next_id.fetch_add(1, Ordering::Relaxed) + 1
        );
        let (tx, rx) = channel();
        self.pending.lock().ok()?.insert(id.clone(), tx);
        let req = json!({"_tag": "Request", "id": id, "tag": tag, "payload": payload, "headers": []});
        let sent = self
            .writer
            .lock()
            .ok()
            .map(|mut w| w.send(Message::Text(req.to_string().into())));
        match sent {
            Some(Ok(())) => {}
            other => {
                eprintln!("ws_client: send failed for {tag}: {other:?}");
                let _ = self.pending.lock().map(|mut m| m.remove(&id));
                return None;
            }
        }
        Some((id, rx))
    }

    /// Drop the per-request routing entry once its frames are done.
    pub fn finish_request(&self, request_id: &str) {
        let _ = self.pending.lock().map(|mut m| m.remove(request_id));
    }

    /// One request -> one Success value (or None on failure/timeout).
    pub fn call(&self, tag: &str, payload: Value) -> Option<Value> {
        let (id, rx) = self.request(tag, payload)?;
        let mut out = None;
        for _ in 0..600 {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(msg) => {
                    if msg.get("_tag").and_then(Value::as_str) != Some("Exit") {
                        continue;
                    }
                    let exit = msg.get("exit").cloned().unwrap_or(Value::Null);
                    if exit.get("_tag").and_then(Value::as_str) == Some("Success") {
                        out = Some(exit.get("value").cloned().unwrap_or(Value::Null));
                    } else {
                        eprintln!("ws_client: {tag} failed: {msg}");
                    }
                    break;
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => break,
            }
        }
        let _ = self.pending.lock().map(|mut m| m.remove(&id));
        out
    }

    /// Ask the server to stop an in-flight request or stream.
    pub fn interrupt(&self, request_id: &str) {
        let _ = self.pending.lock().map(|mut m| m.remove(request_id));
        let frame = json!({"_tag": "Interrupt", "requestId": request_id});
        if let Ok(mut w) = self.writer.lock() {
            let _ = w.send(Message::Text(frame.to_string().into()));
        }
    }

    /// One request -> Chunk values streamed to `on_chunk` until Exit.
    /// Returns (request_id, final Exit's Success value if any).
    pub fn call_stream(
        &self,
        tag: &str,
        payload: Value,
        mut on_chunk: impl FnMut(Vec<Value>),
    ) -> (Option<String>, Option<Value>) {
        let Some((id, rx)) = self.request(tag, payload) else {
            return (None, None);
        };
        let mut out = None;
        loop {
            match rx.recv() {
                Ok(msg) => match msg.get("_tag").and_then(Value::as_str) {
                    Some("Chunk") => {
                        let values = msg
                            .get("values")
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default();
                        if !values.is_empty() {
                            on_chunk(values);
                        }
                    }
                    Some("Exit") => {
                        let exit = msg.get("exit").cloned().unwrap_or(Value::Null);
                        if exit.get("_tag").and_then(Value::as_str) == Some("Success") {
                            out = exit.get("value").cloned();
                        } else {
                            eprintln!("ws_client: {tag} stream failed: {msg}");
                        }
                        break;
                    }
                    _ => continue,
                },
                Err(_) => break,
            }
        }
        let _ = self.pending.lock().map(|mut m| m.remove(&id));
        (Some(id), out)
    }
}
